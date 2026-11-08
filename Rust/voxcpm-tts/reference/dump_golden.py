#!/usr/bin/env python3
"""Dump golden intermediate tensors from the mlx-audio VoxCPM2 reference.

The Rust/candle port is verified module by module against these tensors.
Every random draw (the CFM initial noise) is dumped, so Rust can *inject*
the same noise and reproduce the AR loop deterministically.

Usage (from Rust/voxcpm-tts/reference, with the shared venv):

    .venv/bin/python dump_golden.py                      # defaults
    .venv/bin/python dump_golden.py --text "Hi there." --seed 7 --out golden/
    .venv/bin/python dump_golden.py --vae-encoder-padding mlx   # raw mlx-audio behaviour

Model: mlx-community/VoxCPM2-bf16 (resolved from the HF cache or downloaded).
Reference audio: test_en.wav shipped in that HF snapshot.

Precision policy: weights stay as stored (bf16 for the LM stack, f32 for the
AudioVAE); all *activations* are float32. This is exactly what mlx-audio 0.6.4
does in practice (its masks/noise/rope are f32, so bf16 activations get promoted).
MLX_ENABLE_TF32 defaults to 0 here before importing MLX: the M5 reduced-precision
convolution path otherwise prevents tight comparison with full-f32 candle.
The effective environment setting is saved in meta.json.
The script cross-checks its hand-written loop against ``model.generate`` with the
same seed and records the max abs difference in meta.json.

Output files (all .safetensors are float32 unless noted; int32 for ids):
    golden/meta.json                  run parameters, shapes, token strings, checks
    golden/tokens.json                tokenizer test cases (transformers vs tokenizers crate)
    golden/00_inputs.safetensors      text ids, masks, ref audio (48k raw, 16k, padded)
    golden/01_vae_enc_unit.safetensors  VAE encoder layer-by-layer on a 2-patch crop
    golden/02_ref_latents.safetensors   ref latents (T,4,64) [+ mlx-padding variant]
    golden/03_locenc.safetensors        LocEnc prefill in/out + per-layer trace (patch 0)
    golden/04_base_lm_prefill.safetensors  TSLM prefill per-layer trace, rope tables
    golden/05_residual_lm_prefill.safetensors  RALM prefill
    golden/06_steps.safetensors         per AR step: lm/res hidden, mu, cond, noise, pred, stop
    golden/07_cfm_trace.safetensors     Euler/CFG trace for the first K AR steps
    golden/08_vae_dec_unit.safetensors  VAE decoder layer-by-layer on first 8 latent frames
    golden/09_decode.safetensors        all latents + decoded 48 kHz waveform
    golden/output.wav                   decoded waveform (for listening)
"""

from __future__ import annotations

import argparse
import glob
import json
import math
import os
import sys
import time
from pathlib import Path

# MLX reads this once at process startup. Keep numerical verification in full
# float32 unless the caller explicitly requests a reduced-precision experiment.
os.environ.setdefault("MLX_ENABLE_TF32", "0")

import mlx.core as mx
import mlx.nn as nn
import numpy as np

REPO_ID = "mlx-community/VoxCPM2-bf16"
DEFAULT_TEXT = "Hello, this is a short test of the Voice C P M two model, running in Rust."


# --------------------------------------------------------------------------- #
# helpers
# --------------------------------------------------------------------------- #
def resolve_snapshot(model: str) -> Path:
    p = Path(model)
    if p.is_dir():
        return p
    pattern = os.path.expanduser(
        f"~/.cache/huggingface/hub/models--{model.replace('/', '--')}/snapshots/*/config.json"
    )
    hits = sorted(glob.glob(pattern))
    if hits:
        return Path(hits[-1]).parent
    from huggingface_hub import snapshot_download

    return Path(snapshot_download(model))


def f32(x) -> mx.array:
    return x.astype(mx.float32)


class Dump:
    """Accumulate named tensors, write them as safetensors."""

    def __init__(self, out: Path):
        self.out = out
        self.index: dict[str, dict] = {}

    def save(self, fname: str, tensors: dict):
        arrs = {}
        for k, v in tensors.items():
            if isinstance(v, np.ndarray):
                v = mx.array(v)
            if v.dtype not in (mx.int32, mx.float32):
                v = v.astype(mx.int32) if v.dtype in (mx.int64, mx.uint32, mx.int16) else v.astype(mx.float32)
            arrs[k] = v
        mx.eval(list(arrs.values()))
        mx.save_safetensors(str(self.out / fname), arrs)
        self.index[fname] = {k: {"shape": list(v.shape), "dtype": str(v.dtype)} for k, v in arrs.items()}


def rope_cos_sin(rope, offset: int, length: int):
    pos = mx.arange(offset, offset + length).astype(mx.int32)
    cos, sin = rope(pos)
    return cos[None], sin[None]  # (1, L, head_dim)


def run_minicpm(lm, x, cache=None, causal=True, trace=None, prefix=""):
    """Re-implementation of MiniCPMModel.__call__ that can record per-layer outputs.

    x: (B, L, H) float32. cache: list of (k, v) per layer or None.
    Returns (normed hidden, new_cache).
    """
    B, L, _ = x.shape
    offset = 0 if cache is None else cache[0][0].shape[1]
    cos = sin = None
    if lm.rope is not None:
        cos, sin = rope_cos_sin(lm.rope, offset, L)
        if trace is not None:
            trace[f"{prefix}rope_cos"] = cos[0]
            trace[f"{prefix}rope_sin"] = sin[0]
    mask = None
    if causal and L > 1:
        mask = mx.triu(mx.full((L, L), float("-inf")), k=1)[None, None]
        if cache is not None:
            # prefix positions are always visible
            mask = mx.concatenate([mx.zeros((1, 1, L, offset)), mask], axis=-1)
    h = x
    new_cache = []
    for i, layer in enumerate(lm.layers):
        h, c = layer(h, cos, sin, mask=mask, cache=None if cache is None else cache[i])
        new_cache.append(c)
        if trace is not None:
            trace[f"{prefix}layer{i:02d}_out"] = h[0]
    h = lm.norm(h)
    if trace is not None:
        trace[f"{prefix}final_norm_out"] = h[0]
    return h, new_cache


def locenc_forward(model, feats, trace=None):
    """feats: (B, T, P, D) float32 -> (B, T, H_enc) before enc_to_lm_proj."""
    enc = model.feat_encoder
    B, T, P, D = feats.shape
    x = enc.in_proj(feats)
    st = mx.broadcast_to(f32(enc.special_token), (B, T, 1, x.shape[-1]))
    x = mx.concatenate([st, x], axis=2).reshape(B * T, P + 1, -1)
    if trace is not None:
        trace["patch0_seq_in"] = x[0]
        sub = {}
        out, _ = run_minicpm(enc.encoder, x[:1], causal=False, trace=sub, prefix="patch0_")
        trace.update(sub)
    out, _ = run_minicpm(enc.encoder, x, causal=False)
    return out[:, 0, :].reshape(B, T, -1)


def dit_forward(est, x, mu, t, cond, dt, trace=None):
    """VoxCPMLocDiTV2.__call__ with optional tracing. x/cond: (N, C, T)."""
    xt = x.transpose(0, 2, 1)
    x_proj = est.in_proj(xt)
    ct = cond.transpose(0, 2, 1)
    c_proj = est.cond_proj(ct)
    t_emb = est.time_mlp(est.time_embeddings(t))
    dt_emb = est.delta_time_mlp(est.time_embeddings(dt))
    t_comb = t_emb + dt_emb
    H = x_proj.shape[-1]
    mu_tok = mu.reshape(x.shape[0], -1, H)
    seq = mx.concatenate([mu_tok, t_comb[:, None, :], c_proj, x_proj], axis=1)
    hidden, _ = run_minicpm(est.decoder, seq, causal=False)
    start = mu_tok.shape[1] + 1 + c_proj.shape[1]
    out = est.out_proj(hidden[:, start:, :]).transpose(0, 2, 1)
    if trace is not None:
        trace["t_sinusoid"] = est.time_embeddings(t)
        trace["t_emb"] = t_emb
        trace["dt_emb"] = dt_emb
        trace["seq_in"] = seq
        trace["seq_out_normed"] = hidden
        trace["velocity"] = out
    return out


def cfm_sample(model, mu, cond, z, n_timesteps, cfg_value, trace=None):
    """UnifiedCFM.sample + solve_euler with injected noise z: (1, 64, 4)."""
    est = model.feat_decoder.estimator
    mean_mode = model.feat_decoder.mean_mode
    t_span = mx.linspace(1, 0, n_timesteps + 1)
    t_span = t_span + 1.0 * (mx.cos(math.pi / 2 * t_span) - 1 + t_span)
    x = z
    t = t_span[0]
    dt = t_span[0] - t_span[1]
    zero_init_steps = max(1, int(len(t_span) * 0.04))
    xs, vpos, vneg, sts, ts, dts = [x], [], [], [], [], []
    for step in range(1, len(t_span)):
        if step <= zero_init_steps:
            v = mx.zeros_like(x)
            vp = vn = mx.zeros_like(x)
            st = mx.zeros((1, 1, 1))
        else:
            b = x.shape[0]
            x_in = mx.concatenate([x, x], axis=0)
            mu_in = mx.concatenate([mu, mx.zeros_like(mu)], axis=0)
            t_in = mx.full((2 * b,), t)
            dt_in = mx.full((2 * b,), dt) if mean_mode else mx.zeros((2 * b,))
            cond_in = mx.concatenate([cond, cond], axis=0)
            sub = {} if (trace is not None and step == zero_init_steps + 1) else None
            out = dit_forward(est, x_in, mu_in, t_in, cond_in, dt_in, trace=sub)
            if sub is not None:
                for k, val in sub.items():
                    trace[f"first_eval_{k}"] = val
            vp, vn = out[:b], out[b:]
            pf, nf = vp.reshape(b, -1), vn.reshape(b, -1)
            st = (mx.sum(pf * nf, axis=1, keepdims=True) / (mx.sum(nf**2, axis=1, keepdims=True) + 1e-8)).reshape(
                b, 1, 1
            )
            v = vn * st + cfg_value * (vp - vn * st)
        ts.append(t)
        dts.append(dt)
        x = x - dt * v
        t = t - dt
        if step < len(t_span) - 1:
            dt = t - t_span[step + 1]
        xs.append(x)
        vpos.append(vp)
        vneg.append(vn)
        sts.append(st)
    if trace is not None:
        trace["t_span"] = t_span
        trace["t_used"] = mx.stack(ts)
        trace["dt_used"] = mx.stack(dts)
        trace["x_per_step"] = mx.concatenate(xs, axis=0)  # (n+1, 64, 4); [0] = z
        trace["v_pos"] = mx.concatenate(vpos, axis=0)
        trace["v_neg"] = mx.concatenate(vneg, axis=0)
        trace["st_star"] = mx.concatenate(sts, axis=0).reshape(-1)
    return x


def vae_encoder_trace(vae, audio_16k: mx.array, trace: dict):
    """audio_16k: (L,) already padded. Records every layer output (channels-last (T, C))."""
    x = audio_16k[None, :, None]
    x = vae.preprocess(x, vae.sample_rate)
    enc = vae.encoder
    x = enc.conv_in(x)
    trace["conv_in_out"] = x[0]
    for bi, blk in enumerate(enc.blocks.layers):
        for name in ("res1", "res2", "res3", "snake", "conv"):
            x = getattr(blk, name)(x)
            trace[f"block{bi}_{name}_out"] = x[0]
    x = enc.fc_mu(x)
    trace["fc_mu_out"] = x[0]
    return x


def vae_decoder_trace(vae, z: mx.array, trace: dict):
    """z: (1, T, 64) channels-last. Records each stage."""
    dec = vae.decoder
    x = dec.conv_in(z)
    trace["conv_in_out"] = x[0]
    sr_idx = dec.get_sr_idx(mx.array([vae.out_sample_rate], dtype=mx.int32))
    trace["sr_idx"] = sr_idx
    for bi, (blk, cl) in enumerate(zip(dec.blocks.layers, dec.sr_cond_layers)):
        x = cl(x, sr_idx)
        trace[f"block{bi}_srcond_out"] = x[0]
        x = blk.snake(x)
        x = blk.conv_t(x)
        trace[f"block{bi}_conv_t_out"] = x[0]
        for name in ("res1", "res2", "res3"):
            x = getattr(blk, name)(x)
        trace[f"block{bi}_out"] = x[0]
    x = dec.snake_out(x)
    x = dec.conv_out(x)
    trace["conv_out_pre_tanh"] = x[0]
    y = mx.tanh(x)
    trace["wav"] = y[0, :, 0]
    return y


def patch_encoder_padding(mode: str):
    """mlx-audio pads the encoder's stride-5 downsample conv by 6 on the left;
    PyTorch (OpenBMB) pads by 2*ceil(s/2) - s%2 = 5. 'pytorch' restores the
    original semantics; 'mlx' keeps mlx-audio behaviour."""
    from mlx_audio.tts.models.voxcpm2 import audio_vae as av

    if getattr(av.CausalConv1d, "_golden_patched", False):
        return
    orig = av.CausalConv1d.__call__

    def call(self, x):
        lp = getattr(self, "left_pad_override", None)
        if lp is None:
            return orig(self, x)
        if lp > 0:
            x = mx.pad(x, ((0, 0), (lp, 0), (0, 0)))
        return nn.Conv1d.__call__(self, x)

    av.CausalConv1d.__call__ = call
    av.CausalConv1d._golden_patched = True


def set_encoder_padding(model, mode: str):
    rates = model.args.audio_vae_config.encoder_rates
    for blk, s in zip(model.audio_vae.encoder.blocks.layers, rates):
        if mode == "pytorch":
            blk.conv.left_pad_override = 2 * math.ceil(s / 2) - (s % 2)
        else:
            blk.conv.left_pad_override = None


def tokenizer_cases(model, snap: Path) -> list:
    from tokenizers import Tokenizer

    rt = Tokenizer.from_file(str(snap / "tokenizer.json"))
    cases = [
        "Hello, this is a test.",
        "(A young woman, gentle and sweet voice)Hello, welcome to VoxCPM2!",
        "你好世界，今天天气很好。",
        "Mixed 中文 and English 123.",
        " Hello",
        "Hello  world",
        "[laughing] ok",
        "Café déjà vu — 😀",
    ]
    out = []
    for s in cases:
        hf_tokens = model.tokenizer.tokenize(s)
        split = model._split_multichar_chinese_tokens(hf_tokens)
        out.append(
            {
                "text": s,
                "hf_tokens": hf_tokens,
                "hf_ids_raw": model.tokenizer.convert_tokens_to_ids(hf_tokens),
                "tokens_after_cjk_split": split,
                "ids_final": model._tokenize(s),
                "tokenizers_crate_tokens": rt.encode(s, add_special_tokens=False).tokens,
                "tokenizers_crate_ids": rt.encode(s, add_special_tokens=False).ids,
            }
        )
    return out


# --------------------------------------------------------------------------- #
# main
# --------------------------------------------------------------------------- #
def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--model", default=REPO_ID)
    ap.add_argument("--text", default=DEFAULT_TEXT)
    ap.add_argument("--ref-audio", default=None, help="default: test_en.wav in the model snapshot")
    ap.add_argument("--no-ref", action="store_true", help="zero-shot mode (no reference audio)")
    ap.add_argument("--seed", type=int, default=1234)
    ap.add_argument("--timesteps", type=int, default=10)
    ap.add_argument("--cfg", type=float, default=2.0)
    ap.add_argument("--min-len", type=int, default=2)
    ap.add_argument("--max-len", type=int, default=2000)
    ap.add_argument("--trace-steps", type=int, default=2, help="AR steps with full CFM trace")
    ap.add_argument("--vae-encoder-padding", choices=["pytorch", "mlx"], default="pytorch")
    ap.add_argument("--skip-generate-check", action="store_true")
    ap.add_argument("--out", default=str(Path(__file__).resolve().parent / "golden"))
    args = ap.parse_args()

    out = Path(args.out)
    out.mkdir(parents=True, exist_ok=True)
    dump = Dump(out)
    t0 = time.time()

    from mlx_audio.audio_io import read as read_audio
    from mlx_audio.tts.utils import load
    from mlx_audio.utils import resample_audio
    import mlx_audio

    snap = resolve_snapshot(args.model)
    patch_encoder_padding(args.vae_encoder_padding)
    model = load(str(snap))
    set_encoder_padding(model, args.vae_encoder_padding)
    cfgm = model.args
    P = model.patch_size
    D = model.feat_dim
    vae = model.audio_vae

    meta = {
        "model": args.model,
        "mlx_audio_version": getattr(mlx_audio, "__version__", "unknown"),
        "mlx_version": mx.__version__,
        "mlx_enable_tf32": os.environ["MLX_ENABLE_TF32"],
        "text": args.text,
        "seed": args.seed,
        "inference_timesteps": args.timesteps,
        "cfg_value": args.cfg,
        "min_len": args.min_len,
        "max_len": args.max_len,
        "vae_encoder_padding": args.vae_encoder_padding,
        "activation_dtype": "float32",
        "layout_note": "VAE traces are channels-last (T, C); CFM tensors are (N, C=64, P=4); "
        "pred_feat / latents are (T, P=4, D=64).",
    }

    # ---------------- tokenizer ----------------
    with open(out / "tokens.json", "w", encoding="utf-8") as f:
        json.dump(tokenizer_cases(model, snap), f, ensure_ascii=False, indent=1)

    text_ids = model._tokenize(args.text)
    meta["text_tokens"] = model._split_multichar_chinese_tokens(model.tokenizer.tokenize(args.text))
    meta["text_ids"] = text_ids
    meta["pytorch_max_len_cap"] = int(len(text_ids) * 6.0 + 10)

    # ---------------- reference audio ----------------
    inputs = {}
    ref_feat = None
    if not args.no_ref:
        ref_path = args.ref_audio or str(snap / "test_en.wav")
        meta["ref_audio"] = os.path.basename(ref_path)
        audio, sr = read_audio(ref_path, dtype="float32")
        if audio.ndim > 1:
            audio = audio.mean(axis=1)
        meta["ref_audio_sr"] = int(sr)
        inputs["ref_audio_raw"] = np.asarray(audio, dtype=np.float32)
        a16 = audio if sr == 16000 else resample_audio(audio, sr, 16000)
        a16 = np.asarray(a16, dtype=np.float32)
        inputs["ref_audio_16k"] = a16
        patch_len = P * vae.chunk_size  # 4 * 640 = 2560
        pad = (patch_len - len(a16) % patch_len) % patch_len
        a16p = np.pad(a16, (0, pad))  # ref audio: right padding
        inputs["ref_audio_16k_padded"] = a16p
        meta["ref_audio_16k_len"] = len(a16)
        meta["ref_audio_pad_right"] = int(pad)

        # VAE encoder unit test: first 2 patches (5120 samples)
        tr = {"input": mx.array(a16p[: 2 * patch_len])}
        vae_encoder_trace(vae, tr["input"], tr)
        dump.save("01_vae_enc_unit.safetensors", tr)

        lat = vae.encode(mx.array(a16p)[None, None, :], 16000)[0]  # (T', 64)
        ref_feat = lat.reshape(-1, P, D)
        lat_tensors = {"ref_latents": ref_feat}
        # quantify the mlx-vs-pytorch padding discrepancy
        set_encoder_padding(model, "mlx" if args.vae_encoder_padding == "pytorch" else "pytorch")
        other = vae.encode(mx.array(a16p)[None, None, :], 16000)[0].reshape(-1, P, D)
        set_encoder_padding(model, args.vae_encoder_padding)
        other_name = "ref_latents_mlx_padding" if args.vae_encoder_padding == "pytorch" else "ref_latents_pytorch_padding"
        lat_tensors[other_name] = other
        meta["encoder_padding_maxabs_diff"] = float(mx.max(mx.abs(other - ref_feat)).item())
        meta["ref_latents_absmax"] = float(mx.max(mx.abs(ref_feat)).item())
        dump.save("02_ref_latents.safetensors", lat_tensors)

    # ---------------- sequence layout ----------------
    tt = text_ids + [model.audio_start_token]
    Lt = len(tt)
    if ref_feat is not None:
        R = ref_feat.shape[0]
        tokens = [model.ref_audio_start_token] + [0] * R + [model.ref_audio_end_token] + tt
        z1 = mx.zeros((1, P, D))
        audio_feat = mx.concatenate([z1, ref_feat, z1, mx.zeros((Lt, P, D))], axis=0)
        text_mask = [1.0] + [0.0] * R + [1.0] + [1.0] * Lt
        audio_mask = [0.0] + [1.0] * R + [0.0] + [0.0] * Lt
        meta["mode"] = "reference"
    else:
        tokens = tt
        audio_feat = mx.zeros((Lt, P, D))
        text_mask = [1.0] * Lt
        audio_mask = [0.0] * Lt
        meta["mode"] = "zero_shot"
    S = len(tokens)
    meta["seq_len"] = S
    text_token = mx.array(tokens, dtype=mx.int32)[None]
    tmask = mx.array(text_mask)[None]
    amask = mx.array(audio_mask)[None]
    audio_feat = f32(audio_feat)[None]
    inputs.update({"text_token": text_token[0], "text_mask": tmask[0], "audio_mask": amask[0], "audio_feat": audio_feat[0]})
    dump.save("00_inputs.safetensors", inputs)

    # ---------------- LocEnc prefill ----------------
    le = {"input": audio_feat[0]}
    feat_embed_enc = locenc_forward(model, audio_feat, trace=le)
    feat_embed = model.enc_to_lm_proj(feat_embed_enc)
    le["locenc_out"] = feat_embed_enc[0]
    le["enc_to_lm_out"] = feat_embed[0]
    dump.save("03_locenc.safetensors", le)

    # ---------------- TSLM prefill ----------------
    scale_emb = cfgm.lm_config.scale_emb if cfgm.lm_config.use_mup else 1.0
    meta["scale_emb_applied"] = scale_emb
    text_embed = f32(model.base_lm.embed_tokens(text_token)) * scale_emb
    combined = tmask[:, :, None] * text_embed + amask[:, :, None] * feat_embed
    bl = {"text_embed": text_embed[0], "combined_embed": combined[0]}
    enc_out, lm_cache = run_minicpm(model.base_lm, combined, trace=bl, prefix="")
    fsq_all = model.fsq_layer(enc_out)
    enc_out_mixed = fsq_all * amask[:, :, None] + enc_out * tmask[:, :, None]
    bl["fsq_out_all_positions"] = fsq_all[0]
    bl["enc_outputs_mixed"] = enc_out_mixed[0]
    lm_hidden = enc_out_mixed[:, -1, :]
    bl["lm_hidden0"] = lm_hidden
    # FSQ internals for one row (pre-round values matter for rounding ties)
    pre = mx.tanh(model.fsq_layer.in_proj(enc_out[:, -1, :]))
    bl["fsq_last_tanh"] = pre
    bl["fsq_last_quant"] = mx.round(pre * model.fsq_layer.scale) / model.fsq_layer.scale
    bl["k_cache_layer0"] = lm_cache[0][0][0]
    bl["v_cache_layer0"] = lm_cache[0][1][0]
    dump.save("04_base_lm_prefill.safetensors", bl)

    # ---------------- RALM prefill ----------------
    rl = {}
    res_in = model.fusion_concat_proj(mx.concatenate([enc_out_mixed, amask[:, :, None] * feat_embed], axis=-1))
    rl["residual_input"] = res_in[0]
    res_out, res_cache = run_minicpm(model.residual_lm, res_in, trace=rl, prefix="")
    residual_hidden = res_out[:, -1, :]
    rl["residual_hidden0"] = residual_hidden
    dump.save("05_residual_lm_prefill.safetensors", rl)

    # ---------------- AR loop ----------------
    mx.random.seed(args.seed)
    prefix_cond = audio_feat[:, -1, :, :]  # (1, P, D)
    steps = {k: [] for k in ("lm_hidden", "residual_hidden", "mu", "cond", "noise", "pred_feat", "curr_embed",
                             "stop_logits", "lm_step_out_pre_fsq")}
    cfm_tr = {}
    pred_seq = []
    n_steps = 0
    stopped = False
    for i in range(args.max_len):
        mu = mx.concatenate([model.lm_to_dit_proj(lm_hidden), model.res_to_dit_proj(residual_hidden)], axis=-1)
        cond = prefix_cond.transpose(0, 2, 1)  # (1, D, P)
        z = mx.random.normal((1, D, P))  # identical draw order to UnifiedCFM.sample
        tr = {} if i < args.trace_steps else None
        pred = cfm_sample(model, mu, cond, z, args.timesteps, args.cfg, trace=tr).transpose(0, 2, 1)  # (1,P,D)
        if tr is not None:
            for k, v in tr.items():
                cfm_tr[f"step{i}_{k}"] = v
        pred_seq.append(pred)
        curr = model.enc_to_lm_proj(locenc_forward(model, pred[:, None]))  # (1,1,H)
        stop_logits = model.stop_head(nn.silu(model.stop_proj(lm_hidden)))
        for k, v in (("lm_hidden", lm_hidden[0]), ("residual_hidden", residual_hidden[0]), ("mu", mu[0]),
                     ("cond", cond[0]), ("noise", z[0]), ("pred_feat", pred[0]), ("curr_embed", curr[0, 0]),
                     ("stop_logits", stop_logits[0])):
            steps[k].append(v)
        mx.eval(pred, curr, stop_logits)
        n_steps = i + 1
        stop_flag = int(mx.argmax(stop_logits, axis=-1).item())
        if i > args.min_len and stop_flag == 1:
            stopped = True
            break
        h, lm_cache = run_minicpm(model.base_lm, curr, cache=lm_cache)
        steps["lm_step_out_pre_fsq"].append(h[0, -1])
        lm_hidden = model.fsq_layer(h[:, -1, :])
        r_in = model.fusion_concat_proj(mx.concatenate([lm_hidden[:, None, :], curr], axis=-1))
        r, res_cache = run_minicpm(model.residual_lm, r_in, cache=res_cache)
        residual_hidden = r[:, -1, :]
        prefix_cond = pred
    meta["n_ar_steps"] = n_steps
    meta["stopped_by_stop_head"] = stopped
    st = {k: mx.stack(v) for k, v in steps.items() if v}
    dump.save("06_steps.safetensors", st)
    dump.save("07_cfm_trace.safetensors", cfm_tr)

    # ---------------- decode ----------------
    all_feats = mx.concatenate(pred_seq, axis=0).reshape(1, -1, D)  # (1, T*P, 64)
    dec_tr = {"input": all_feats[0, :8]}
    vae_decoder_trace(vae, all_feats[:, :8], dec_tr)
    dump.save("08_vae_dec_unit.safetensors", dec_tr)
    wav = vae.decode(all_feats).reshape(-1)
    mx.eval(wav)
    dump.save("09_decode.safetensors", {"latents": all_feats[0], "wav_48k": wav})
    meta["wav_samples"] = int(wav.shape[0])
    meta["wav_seconds"] = wav.shape[0] / vae.out_sample_rate
    try:
        from mlx_audio.audio_io import write as write_audio

        write_audio(str(out / "output.wav"), np.asarray(wav), vae.out_sample_rate)
    except Exception as e:  # pragma: no cover
        meta["wav_write_error"] = repr(e)

    # ---------------- cross-check against model.generate ----------------
    if not args.skip_generate_check:
        mx.random.seed(args.seed)
        kw = dict(max_tokens=args.max_len, min_tokens=args.min_len, inference_timesteps=args.timesteps,
                  cfg_value=args.cfg)
        if ref_feat is not None:
            kw["ref_audio"] = args.ref_audio or str(snap / "test_en.wav")
        res = next(model.generate(args.text, **kw))
        ref_wav = res.audio.reshape(-1)
        meta["generate_check_samples"] = int(ref_wav.shape[0])
        if ref_wav.shape[0] == wav.shape[0]:
            meta["generate_check_maxabs_diff"] = float(mx.max(mx.abs(ref_wav - wav)).item())
        else:
            meta["generate_check_maxabs_diff"] = None
            meta["generate_check_note"] = "length mismatch (stop step diverged)"

    meta["files"] = dump.index
    meta["elapsed_s"] = round(time.time() - t0, 2)
    with open(out / "meta.json", "w", encoding="utf-8") as f:
        json.dump(meta, f, ensure_ascii=False, indent=1)
    print(json.dumps({k: v for k, v in meta.items() if k != "files"}, ensure_ascii=False, indent=1))


if __name__ == "__main__":
    sys.exit(main())
