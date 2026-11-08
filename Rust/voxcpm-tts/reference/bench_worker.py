#!/usr/bin/env python3
"""Benchmark + protocol exercise for voxcpm_worker.py.

Drives the worker exactly as the Rust CLI would (subprocess, JSON Lines), then
(optionally, after the worker exits so the two models never share memory)
transcribes every output with a local Whisper to get a listen-free WER.

Example (from Rust/voxcpm-tts/reference):
  .venv/bin/python bench_worker.py --ref "$REF" --ref-text "$REF_TEXT" \
      --out-dir out/bench --timesteps 10 7 --lengths 100 300 600 1200 --asr
"""

from __future__ import annotations

import argparse
import json
import os
import re
import subprocess
import sys
import time

HERE = os.path.dirname(os.path.abspath(__file__))

# Original prose (written for this benchmark; no third-party text). Plain
# English, no digits or abbreviations, so WER does not depend on normalization.
PASSAGE = (
    "The lighthouse at the end of the harbor had not been lit for many years, and its great lamp was cold. "
    "Every evening, the old keeper still climbed the narrow stairs and wiped the salt from the great glass lens. "
    "Nobody in the village asked him why he did it, and he never offered an explanation. "
    "On calm nights you could hear his footsteps ringing on the iron steps, slow and steady, like a clock that refused to stop. "
    "One winter a storm rolled in from the north, faster than anyone expected. "
    "The fishing boats were still out past the reef, and the radio at the harbor office had gone silent. "
    "Without a word, the keeper climbed the stairs one more time, carrying a lantern and a can of oil. "
    "Nobody knew whether the old machinery would still turn, or whether the lamp would catch at all. "
    "For a long moment there was only wind and the sound of waves breaking over the stone wall. "
    "Then, slowly, a pale beam swept across the water, faltered, and steadied into a bright and patient circle. "
    "Out in the dark, the sailors saw it and turned their bows toward home, one boat after another. "
    "By morning every crew was safe in the harbor, wet and exhausted but alive. "
    "The village gathered at the foot of the tower to thank the keeper, but he had already gone to sleep. "
    "After that night, nobody ever again wondered why he kept the glass so clean. "
    "They simply left a basket of bread and fish at his door, and the light burned every evening from then on. "
    "Children who grew up in that village learned to count the seconds between each sweep of the beam, "
    "and they would tell visitors that the light was older than the harbor itself."
)


def text_of_length(target: int) -> str:
    """Sentence-bounded prefix of PASSAGE closest to `target` characters."""
    sents = re.findall(r"[^.]+\.\s*", PASSAGE)
    best, acc = "", ""
    for s in sents:
        acc += s
        if abs(len(acc.strip()) - target) <= abs(len(best) - target):
            best = acc.strip()
    return best


def silence_stats(path: str) -> dict:
    """Leading/trailing/longest-internal silence (s) and tail level, 20 ms frames."""
    import numpy as np
    from mlx_audio.audio_io import read

    a, sr = read(path, dtype="float32")
    a = a.reshape(-1) if a.ndim == 1 else a.mean(axis=1)
    hop = int(0.02 * sr)
    n = len(a) // hop
    if n == 0:
        return {}
    frames = a[: n * hop].reshape(n, hop)
    rms = np.sqrt((frames**2).mean(axis=1) + 1e-12)
    db = 20 * np.log10(rms / (rms.max() + 1e-12))
    voiced = db > -40.0
    idx = np.nonzero(voiced)[0]
    if idx.size == 0:
        return {"all_silent": True}
    lead = idx[0] * 0.02
    trail = (n - 1 - idx[-1]) * 0.02
    longest = 0
    run = 0
    for v in voiced[idx[0] : idx[-1] + 1]:
        run = 0 if v else run + 1
        longest = max(longest, run)
    tail_db = float(20 * np.log10(np.sqrt((a[-int(0.1 * sr) :] ** 2).mean() + 1e-12)))
    return {
        "lead_silence_s": round(float(lead), 2),
        "trail_silence_s": round(float(trail), 2),
        "longest_gap_s": round(longest * 0.02, 2),
        "tail_100ms_dbfs": round(tail_db, 1),
        "peak_dbfs": round(float(20 * np.log10(np.abs(a).max() + 1e-12)), 1),
    }


def norm_words(s: str) -> list[str]:
    s = s.lower().replace("’", "'")
    s = re.sub(r"[^a-z0-9' ]+", " ", s)
    return s.split()


def wer(ref: str, hyp: str) -> float:
    r, h = norm_words(ref), norm_words(hyp)
    d = list(range(len(h) + 1))
    for i in range(1, len(r) + 1):
        prev, d[0] = d[0], i
        for j in range(1, len(h) + 1):
            cur = d[j]
            d[j] = min(d[j] + 1, d[j - 1] + 1, prev + (r[i - 1] != h[j - 1]))
            prev = cur
    return d[len(h)] / max(1, len(r))


class Worker:
    def __init__(self, python: str, log_path: str):
        self.log = open(log_path, "ab")
        self.p = subprocess.Popen(
            [python, os.path.join(HERE, "voxcpm_worker.py")],
            stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=self.log, text=True, bufsize=1,
        )
        self.ready = json.loads(self.p.stdout.readline())
        assert self.ready.get("event") == "ready", self.ready
        self.n = 0
        self.progress: list[dict] = []

    def close(self) -> None:
        """Release the child even if a benchmark or output analysis fails."""
        if self.p.poll() is None:
            self.p.terminate()
            try:
                self.p.wait(timeout=5)
            except subprocess.TimeoutExpired:
                self.p.kill()
                self.p.wait(timeout=5)
        self.log.close()

    def call(self, op: str, **kw) -> dict:
        self.n += 1
        rid = f"r{self.n}"
        self.p.stdin.write(json.dumps({"id": rid, "op": op, **kw}) + "\n")
        self.p.stdin.flush()
        while True:
            line = self.p.stdout.readline()
            if not line:
                raise RuntimeError("worker closed stdout")
            msg = json.loads(line)
            if msg.get("event") == "progress":
                self.progress.append(msg)
                continue
            assert msg.get("id") == rid, msg
            return msg


def sys_free_pct() -> float | None:
    try:
        out = subprocess.run(["memory_pressure"], capture_output=True, text=True, timeout=30).stdout
        m = re.search(r"free percentage:\s*(\d+)%", out)
        return float(m.group(1)) if m else None
    except Exception:
        return None


def other_ml_processes() -> list[str]:
    out = subprocess.run(["ps", "-Ao", "pid,rss,pcpu,comm"], capture_output=True, text=True).stdout
    rows = []
    for ln in out.splitlines()[1:]:
        parts = ln.split(None, 3)
        if len(parts) == 4 and int(parts[1]) > 1_000_000 and int(parts[0]) != os.getpid():
            rows.append(f"{parts[0]} rss={int(parts[1]) // 1024}MB cpu={parts[2]}% {os.path.basename(parts[3])}")
    return rows


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--ref", required=True)
    ap.add_argument("--ref-text", default=None)
    ap.add_argument("--modes", nargs="+", default=["controllable"], choices=["ultimate", "controllable", "zero"])
    ap.add_argument("--out-dir", default=os.path.join(HERE, "out", "bench"))
    ap.add_argument("--timesteps", type=int, nargs="+", default=[10, 7])
    ap.add_argument("--lengths", type=int, nargs="+", default=[100, 300, 600, 1200])
    ap.add_argument("--seed", type=int, default=42)
    ap.add_argument("--no-retry", action="store_true", help="disable badcase retry (raw model behaviour)")
    ap.add_argument("--asr", action="store_true", help="transcribe outputs with whisper after the worker exits")
    ap.add_argument("--json", default=None)
    args = ap.parse_args()

    if "ultimate" in args.modes and not args.ref_text:
        ap.error("--modes ultimate requires the exact --ref-text transcript")

    os.makedirs(args.out_dir, exist_ok=True)
    py = os.path.join(HERE, ".venv", "bin", "python")
    report: dict = {"started": time.strftime("%Y-%m-%d %H:%M:%S"), "other_big_processes": other_ml_processes(),
                    "sys_free_pct_before": sys_free_pct(), "cases": []}

    w = Worker(py, os.path.join(args.out_dir, "worker.log"))
    try:
        report["ready"] = w.ready
        r = w.call("load", model="mlx-community/VoxCPM2-bf16")
        print("load", r, file=sys.stderr)
        report["load"] = r
        report["sys_free_pct_loaded"] = sys_free_pct()

        # Warm-up (first call pays Metal kernel JIT); excluded from the table.
        r = w.call("synthesize", text="Warm up.", ref_audio=args.ref, out_wav=os.path.join(args.out_dir, "warmup.wav"), seed=0)
        report["warmup"] = r

        for mode in args.modes:
            for steps in args.timesteps:
                for L in args.lengths:
                    text = text_of_length(L)
                    out = os.path.join(args.out_dir, f"{mode}_t{steps}_{L}.wav")
                    kw = dict(text=text, out_wav=out, inference_timesteps=steps, cfg_value=2.0, seed=args.seed,
                              retry_badcase=not args.no_retry)
                    if mode != "zero":
                        kw["ref_audio"] = args.ref
                    if mode == "ultimate":
                        kw["ref_text"] = args.ref_text
                    r = w.call("synthesize", **kw)
                    case = {"mode": mode, "timesteps": steps, "target_chars": L, "chars": len(text),
                            "words": len(text.split()), "text": text, "out_wav": out, **r}
                    if r.get("ok"):
                        case.update(silence_stats(out))
                        case["chars_per_audio_s"] = round(len(text) / r["audio_seconds"], 2)
                    print(json.dumps({k: v for k, v in case.items() if k != "text"}), file=sys.stderr)
                    report["cases"].append(case)

        report["progress_events"] = len(w.progress)
        r = w.call("unload")
        report["unload"] = r
        time.sleep(2)
        report["after_unload_ping"] = w.call("ping")
        report["sys_free_pct_unloaded"] = sys_free_pct()
        r = w.call("shutdown")
        report["shutdown"] = r
        report["exit_code"] = w.p.wait(timeout=30)
        time.sleep(2)
        report["sys_free_pct_after_exit"] = sys_free_pct()
        report["other_big_processes_end"] = other_ml_processes()

    finally:
        w.close()

    if args.asr:
        from mlx_audio.stt import load as load_stt
        from mlx_audio.utils import load_audio

        stt = load_stt("mlx-community/whisper-large-v3-turbo-asr-fp16")
        for c in report["cases"]:
            if not c.get("ok"):
                continue
            # Whisper expects 16 kHz input; resample explicitly.
            hyp = stt.generate(load_audio(c["out_wav"], sample_rate=16000), language="en").text.strip()
            c["asr"] = hyp
            c["wer"] = round(wer(c["text"], hyp), 3)
            print(f"{os.path.basename(c['out_wav'])} WER={c['wer']}", file=sys.stderr)

    path = args.json or os.path.join(args.out_dir, "bench.json")
    with open(path, "w") as f:
        json.dump(report, f, indent=1)
    print(path)
    return 0


if __name__ == "__main__":
    sys.exit(main())
