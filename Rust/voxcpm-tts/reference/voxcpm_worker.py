#!/usr/bin/env python3
"""Persistent VoxCPM2 (mlx-audio) TTS worker speaking JSON Lines over stdin/stdout.

This is the Python reference engine for voxcpm-tts: a fallback engine for the
Rust CLI today, and the numerical oracle for a later native port.

Protocol (one JSON object per line; see docs/REFERENCE_ENGINE.md):

  startup  -> {"event":"ready","pid":<int>}            (before any weights load)
  load     -> {"id","ok":true,"sample_rate","load_seconds",...}
  synthesize -> optional {"id","event":"progress",...} lines, then
                {"id","ok":true,"audio_seconds","elapsed_seconds","sample_rate",...}
  unload   -> {"id","ok":true,...}
  shutdown -> {"id","ok":true} then exit 0.   EOF on stdin also exits 0.
  failure  -> {"id","ok":false,"error":<str>} and the worker keeps running.

stdout carries protocol lines only. Every other byte (library prints, tqdm,
warnings, logs) goes to stderr: file descriptor 1 is redirected to stderr at
startup and the protocol writes to a private duplicate of the original stdout.
"""

from __future__ import annotations

import argparse
import ctypes
import ctypes.util
import gc
import json
import math
import os
import struct
import sys
import time
import traceback

# --------------------------------------------------------------------------
# stdout isolation: must happen before importing anything that might print.
# --------------------------------------------------------------------------
_PROTO = sys.stdout


def isolate_stdout() -> None:
    """Keep importing this module harmless; isolate descriptors only as a worker."""
    global _PROTO
    _PROTO = os.fdopen(os.dup(1), "w", buffering=1, encoding="utf-8")
    os.dup2(2, 1)  # C and Python library writes now land on stderr.
    sys.stdout = sys.stderr

os.environ.setdefault("TOKENIZERS_PARALLELISM", "false")
os.environ.setdefault("HF_HUB_DISABLE_PROGRESS_BARS", "1")


def emit(obj: dict) -> None:
    _PROTO.write(json.dumps(obj, ensure_ascii=False, separators=(",", ":")) + "\n")
    _PROTO.flush()


def log(msg: str) -> None:
    sys.stderr.write(f"[voxcpm_worker {os.getpid()}] {msg}\n")
    sys.stderr.flush()


# --------------------------------------------------------------------------
# Process memory (macOS): proc_pid_rusage gives phys_footprint, which counts
# Metal/unified-memory allocations; plain RSS under-reports MLX buffers.
# --------------------------------------------------------------------------
_libproc = None
try:
    _libproc = ctypes.CDLL(ctypes.util.find_library("proc") or "/usr/lib/libproc.dylib")
except OSError:
    _libproc = None

_RUSAGE_INFO_V4 = 4


def process_memory(pid: int | None = None) -> dict:
    """Return resident / phys_footprint / lifetime-max footprint in MB (macOS)."""
    pid = os.getpid() if pid is None else pid
    out: dict = {}
    if _libproc is not None:
        buf = (ctypes.c_uint64 * 64)()
        if _libproc.proc_pid_rusage(ctypes.c_int(pid), ctypes.c_int(_RUSAGE_INFO_V4), ctypes.byref(buf)) == 0:
            # rusage_info_v4: 16-byte uuid, then uint64 fields.
            fields = list(buf)[2:]
            out["rss_mb"] = round(fields[6] / 2**20, 1)  # ri_resident_size
            out["footprint_mb"] = round(fields[7] / 2**20, 1)  # ri_phys_footprint
            out["max_footprint_mb"] = round(fields[28] / 2**20, 1)  # ri_lifetime_max_phys_footprint
    return out


def mlx_memory() -> dict:
    try:
        import mlx.core as mx

        return {
            "mlx_active_gb": round(mx.get_active_memory() / 1e9, 3),
            "mlx_cache_gb": round(mx.get_cache_memory() / 1e9, 3),
            "mlx_peak_gb": round(mx.get_peak_memory() / 1e9, 3),
        }
    except Exception:  # pragma: no cover - mlx missing
        return {}


# --------------------------------------------------------------------------
# WAV writer: mono IEEE float32 (format tag 3), stdlib only, no resampling.
# float32 keeps the oracle output bit-exact (no int16 quantization).
# --------------------------------------------------------------------------
def write_wav_f32(path: str, samples, sample_rate: int) -> None:
    import numpy as np

    data = np.ascontiguousarray(np.asarray(samples, dtype="<f4").reshape(-1))
    n_bytes = data.nbytes
    d = os.path.dirname(os.path.abspath(path))
    os.makedirs(d, exist_ok=True)
    tmp = path + ".part"
    with open(tmp, "wb") as f:
        # RIFF header + 'fmt ' (18-byte, cbSize=0) + 'fact' + 'data'
        f.write(b"RIFF")
        f.write(struct.pack("<I", 4 + (8 + 18) + (8 + 4) + (8 + n_bytes)))
        f.write(b"WAVE")
        f.write(b"fmt ")
        f.write(struct.pack("<IHHIIHHH", 18, 3, 1, sample_rate, sample_rate * 4, 4, 32, 0))
        f.write(b"fact")
        f.write(struct.pack("<II", 4, data.shape[0]))
        f.write(b"data")
        f.write(struct.pack("<I", n_bytes))
        f.write(data.tobytes())
    os.replace(tmp, path)


# --------------------------------------------------------------------------
# Engine
# --------------------------------------------------------------------------
DEFAULT_MODEL = "mlx-community/VoxCPM2-bf16"
PROGRESS_EVERY = 25  # patches (1 patch = 4 latent frames = 0.16 s of 48 kHz audio)
PATCH_SECONDS = 0.16
DEFAULT_CACHE_LIMIT_GB = 2.0


class BadGenerationError(RuntimeError):
    """Recoverable generation failure, distinguished by the Rust job runner."""


class Engine:
    def __init__(self) -> None:
        self.model = None
        self.model_id: str | None = None

    # ---- load / unload ---------------------------------------------------
    def load(self, req: dict) -> dict:
        import mlx.core as mx
        from mlx_audio.tts.utils import load as load_tts

        model_id = req.get("model") or DEFAULT_MODEL
        if self.model is not None and self.model_id == model_id:
            return {"sample_rate": int(self.model.sample_rate), "load_seconds": 0.0, "already_loaded": True}
        if self.model is not None:
            self._drop()
        # Bound MLX's freed-buffer cache. Without a limit the cache grows by
        # several GB per request (KV-cache shapes change every step, so freed
        # buffers rarely get reused) and phys_footprint reached 18 GB after five
        # short requests. See docs/REFERENCE_ENGINE.md.
        cache_limit_gb = float(req.get("cache_limit_gb", DEFAULT_CACHE_LIMIT_GB))
        mx.set_cache_limit(int(cache_limit_gb * 1e9))
        mx.reset_peak_memory()
        t0 = time.perf_counter()
        model = load_tts(model_id)  # lazy=False: weights are materialized here
        mx.eval(model.parameters())
        compiled = bool(req.get("compile", False))
        if compiled:
            model.compile_model()
        load_seconds = time.perf_counter() - t0
        self.model, self.model_id = model, model_id
        log(f"loaded {model_id} in {load_seconds:.2f}s")
        return {
            "sample_rate": int(model.sample_rate),
            "load_seconds": round(load_seconds, 3),
            "compiled": compiled,
            "cache_limit_gb": cache_limit_gb,
            **mlx_memory(),
            **process_memory(),
        }

    def _drop(self) -> None:
        import mlx.core as mx

        self.model = None
        self.model_id = None
        gc.collect()
        mx.clear_cache()

    def unload(self, req: dict) -> dict:
        was_loaded = self.model is not None
        self._drop()
        return {"was_loaded": was_loaded, **mlx_memory(), **process_memory()}

    # ---- synthesize ------------------------------------------------------
    def synthesize(self, req: dict, rid) -> dict:
        if self.model is None:
            raise RuntimeError("model not loaded; send op=load first")
        import mlx.core as mx
        import numpy as np

        model = self.model

        text = req.get("text")
        if not isinstance(text, str) or not text.strip():
            raise ValueError("'text' must be a non-empty string")
        out_wav = req.get("out_wav")
        if not isinstance(out_wav, str) or not out_wav:
            raise ValueError("'out_wav' must be a path string")
        ref_audio = req.get("ref_audio")
        ref_text = req.get("ref_text")
        if ref_audio is not None:
            ref_audio = os.path.expanduser(str(ref_audio))
            if not os.path.isfile(ref_audio):
                raise FileNotFoundError(f"ref_audio not found: {ref_audio}")
        if ref_text is not None and not isinstance(ref_text, str):
            raise ValueError("'ref_text' must be a string or null")
        if ref_text is not None and not ref_text.strip():
            ref_text = None
        if ref_text is not None and ref_audio is None:
            raise ValueError("'ref_text' requires 'ref_audio'")

        cfg_value = float(req.get("cfg_value") if req.get("cfg_value") is not None else 2.0)
        if not math.isfinite(cfg_value) or cfg_value <= 0:
            raise ValueError("cfg_value must be finite and positive")
        timesteps = int(req.get("inference_timesteps") if req.get("inference_timesteps") is not None else 10)
        if timesteps < 1:
            raise ValueError("inference_timesteps must be >= 1")
        seed = req.get("seed")
        max_tokens = int(req.get("max_tokens", 2000))
        if max_tokens < 1:
            raise ValueError("max_tokens must be >= 1")
        seconds_cap = req.get("max_audio_seconds")
        if seconds_cap is not None:
            seconds_cap = float(seconds_cap)
            if not math.isfinite(seconds_cap) or seconds_cap < PATCH_SECONDS:
                raise ValueError("max_audio_seconds must be finite and >= 0.16")
            max_tokens = min(max_tokens, int(math.floor(seconds_cap / PATCH_SECONDS)))
        retry_badcase = bool(req.get("retry_badcase", True))
        retry_max = int(req.get("retry_badcase_max_times", 3))
        retry_ratio = float(req.get("retry_badcase_ratio_threshold", 6.0))
        if retry_max < 0 or not math.isfinite(retry_ratio) or retry_ratio <= 0:
            raise ValueError("retry count must be nonnegative and ratio finite and positive")
        want_progress = bool(req.get("progress", True))

        # Mode selection (upstream VoxCPM2 naming):
        #   ref_audio + ref_text -> "ultimate cloning": reference_wav_path +
        #       prompt_wav_path + prompt_text, all pointing at the same clip.
        #       In mlx-audio: ref_audio=..., prompt_audio=..., prompt_text=...
        #   ref_audio only       -> "controllable cloning": reference_wav_path.
        #   neither              -> zero-shot (text may carry "(voice design)" prefix).
        # NOTE: mlx-audio's VoxCPM2.generate() accepts `ref_text` but IGNORES it.
        gen_kwargs: dict = dict(
            text=text,
            cfg_value=cfg_value,
            inference_timesteps=timesteps,
            max_tokens=max_tokens,
        )
        if ref_audio is not None:
            gen_kwargs["ref_audio"] = ref_audio
            mode = "controllable_clone"
            if ref_text is not None:
                prompt_text = ref_text
                # Upstream concatenates prompt_text + text verbatim. Insert one
                # space between two Latin-script runs so words don't fuse.
                if not prompt_text[-1].isspace() and not text[0].isspace() and prompt_text[-1].isascii():
                    prompt_text = prompt_text + " "
                gen_kwargs["prompt_audio"] = ref_audio
                gen_kwargs["prompt_text"] = prompt_text
                mode = "ultimate_clone"
        else:
            mode = "zero_shot"

        n_text_tokens = len(model._tokenize(text))
        if not n_text_tokens:
            raise ValueError("text tokenized to an empty sequence")
        # Keep the PyTorch per-text cap even when the Rust caller owns retries.
        badcase_limit = min(max_tokens, int(6 * n_text_tokens + 10))
        gen_kwargs["max_tokens"] = badcase_limit

        # Per-patch progress via a counting wrapper around the CFM sampler.
        # generate() calls feat_decoder.sample() exactly once per audio patch and
        # syncs (stop-flag .item()) each step, so the count is real-time.
        counter = {"n": 0}
        orig_sample = model.feat_decoder.sample

        def counting_sample(*a, **k):
            generated = orig_sample(*a, **k)
            counter["n"] += 1
            n = counter["n"]
            if want_progress and n % PROGRESS_EVERY == 0:
                emit({
                    "id": rid,
                    "event": "progress",
                    "patches": n,
                    "audio_seconds": round(n * PATCH_SECONDS, 2),
                    "max_patches": gen_kwargs["max_tokens"],
                    "elapsed_seconds": round(time.perf_counter() - t0, 3),
                })
            return generated

        if seed is not None:
            mx.random.seed(int(seed))
        mx.reset_peak_memory()
        t0 = time.perf_counter()
        attempts = 0
        badcase = False
        model.feat_decoder.sample = counting_sample
        try:
            while True:
                attempts += 1
                counter["n"] = 0
                if seed is not None:
                    mx.random.seed(int(seed) + attempts - 1)
                result = None
                for result in model.generate(**gen_kwargs):
                    pass
                if result is None:
                    raise RuntimeError("generate() produced no audio")
                patches = counter["n"]
                badcase = patches >= badcase_limit or (retry_badcase and patches >= retry_ratio * n_text_tokens)
                if badcase and retry_badcase and attempts <= retry_max:
                    log(f"badcase: {patches} patches for {n_text_tokens} text tokens; retry {attempts}/{retry_max}")
                    continue
                if badcase:
                    raise BadGenerationError(
                        f"generation hit the {badcase_limit}-patch cap (possible runaway); "
                        f"exhausted {attempts} attempt(s)"
                    )
                # mlx-audio 0.6.4's stride-5 transposed convolution adds eight
                # spurious samples at 48 kHz. Preserve the exact patch duration.
                audio = np.array(result.audio, dtype=np.float32).reshape(-1)[:patches * 7680]
                if audio.size == 0 or not np.isfinite(audio).all():
                    raise BadGenerationError("generation produced empty or non-finite audio")
                break
        finally:
            model.feat_decoder.sample = orig_sample
            mx.clear_cache()
        sr = int(model.sample_rate)
        elapsed = time.perf_counter() - t0
        peak = mx.get_peak_memory()
        cache_before_clear = mx.get_cache_memory()
        del result
        mx.clear_cache()  # return per-request scratch to the OS between requests
        write_wav_f32(out_wav, audio, sr)
        audio_seconds = audio.shape[0] / sr
        return {
            "audio_seconds": round(audio_seconds, 4),
            "elapsed_seconds": round(elapsed, 4),
            "sample_rate": sr,
            "rtf": round(elapsed / audio_seconds, 4) if audio_seconds > 0 else None,
            "mode": mode,
            "patches": patches,
            "text_tokens": n_text_tokens,
            "attempts": attempts,
            "badcase": bool(badcase),
            "hit_max_tokens": patches >= gen_kwargs["max_tokens"],
            "mlx_peak_gb": round(peak / 1e9, 3),
            "mlx_cache_before_clear_gb": round(cache_before_clear / 1e9, 3),
            **process_memory(),
        }


# --------------------------------------------------------------------------
# Main loop
# --------------------------------------------------------------------------
def serve() -> int:
    emit({"event": "ready", "pid": os.getpid()})
    engine = Engine()
    for line in sys.stdin:
        line = line.strip()
        if not line:
            continue
        rid = None
        try:
            req = json.loads(line)
            if not isinstance(req, dict):
                raise ValueError("request must be a JSON object")
            rid = req.get("id")
            op = req.get("op")
            if op == "load":
                emit({"id": rid, "ok": True, **engine.load(req)})
            elif op == "synthesize":
                emit({"id": rid, "ok": True, **engine.synthesize(req, rid)})
            elif op == "unload":
                emit({"id": rid, "ok": True, **engine.unload(req)})
            elif op == "shutdown":
                emit({"id": rid, "ok": True})
                return 0
            elif op == "ping":
                emit({"id": rid, "ok": True, "loaded": engine.model is not None, **process_memory()})
            else:
                raise ValueError(f"unknown op: {op!r}")
        except Exception as e:  # keep serving after any failure
            log(traceback.format_exc())
            emit({"id": rid, "ok": False, "error": f"{type(e).__name__}: {e}",
                  "code": "bad_generation" if isinstance(e, BadGenerationError) else "request_error"})
    return 0  # EOF


def selftest(ref_audio: str | None, out_dir: str) -> int:
    """Run load -> synthesize -> unload in-process and print a summary to stderr."""
    engine = Engine()
    log(json.dumps(engine.load({"model": DEFAULT_MODEL})))
    r = engine.synthesize(
        {"text": "This is a self test of the VoxCPM2 reference engine.", "ref_audio": ref_audio,
         "out_wav": os.path.join(out_dir, "selftest.wav"), "seed": 0, "progress": False},
        "selftest",
    )
    log(json.dumps(r))
    log(json.dumps(engine.unload({})))
    return 0


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--selftest", action="store_true", help="load, synthesize one line, unload, exit")
    ap.add_argument("--ref-audio", default=None, help="reference clip for --selftest")
    ap.add_argument("--out-dir", default="out", help="output dir for --selftest")
    args = ap.parse_args()
    if args.selftest:
        return selftest(args.ref_audio, args.out_dir)
    return serve()


if __name__ == "__main__":
    isolate_stdout()
    sys.exit(main())
