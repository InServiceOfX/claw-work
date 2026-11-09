"""Independent mixed-precision NumPy forward reference; no model-framework imports.

Usage: MODEL_DIR=... python3 verification/numpy_reference.py GPU_LOGITS_JSON OUTPUT_JSON
Input is the inference example's --dump-logits artifact. All output remains at
an explicitly selected path outside Git. This reference does not execute model Python.
"""
import json
import math
import os
import pathlib
import struct
import sys

import numpy as np

root = pathlib.Path(os.environ["MODEL_DIR"])
config = json.loads((root / "config.json").read_text())
source = json.loads(pathlib.Path(sys.argv[1]).read_text())
with (root / "model.safetensors").open("rb") as handle:
    header_size = struct.unpack("<Q", handle.read(8))[0]
    metadata = json.loads(handle.read(header_size))
mapped = np.memmap(root / "model.safetensors", mode="r", dtype=np.uint8)


def bf16(x):
    x = np.asarray(x, dtype=np.float32, order="C")
    bits = x.view(np.uint32)
    rounded = bits + np.uint32(0x7FFF) + ((bits >> np.uint32(16)) & np.uint32(1))
    return (rounded & np.uint32(0xFFFF0000)).view(np.float32)


def weight(name):
    entry = metadata[name]
    assert entry["dtype"] == "BF16"
    start, end = entry["data_offsets"]
    data = mapped[8 + header_size + start:8 + header_size + end]
    bits = data.view("<u2").astype(np.uint32) << np.uint32(16)
    return bits.view(np.float32).reshape(entry["shape"])


trace_directory = os.environ.get("CLAW_LM_TRACE_DIR")
trace_results = []
replay_layers = os.environ.get("CLAW_LM_REFERENCE_REPLAY") == "1"

def capture(name, reference):
    if not trace_directory:
        return
    actual = np.fromfile(pathlib.Path(trace_directory) / (name + ".f32"), dtype="<f4").reshape(reference.shape)
    error = actual - reference
    trace_results.append({"stage": name, "max": float(np.max(np.abs(error))),
                          "rmse": float(np.sqrt(np.mean(error * error))),
                          "mean_error": float(np.mean(error)),
                          "same_fraction": float(np.mean(actual == reference))})


def linear(x, name):
    return bf16(x) @ weight(name).T


def norm(x, name):
    normalized = x / np.sqrt(np.mean(x * x, axis=-1, keepdims=True) + np.float32(config["rms_norm_eps"]))
    return normalized * weight(name)


r = config["rope_parameters"]
dim = config["head_dim"]
correction = lambda rotations: dim * math.log(r["original_max_position_embeddings"] / (rotations * 2 * math.pi)) / (2 * math.log(r["rope_theta"]))
low = max(math.floor(correction(r["beta_fast"])), 0)
high = min(math.ceil(correction(r["beta_slow"])), dim - 1)
ramp = np.clip((np.arange(dim // 2, dtype=np.float64) - low) / (high - low if low != high else .001), 0, 1)
frequency = ((r["rope_theta"] ** (-2 * np.arange(dim // 2, dtype=np.float64) / dim)) * (1 - ramp + ramp / r["factor"])).astype(np.float32)
scale = lambda m: 1 if r["factor"] <= 1 else 1 + .1 * m * math.log(r["factor"])
magnitude = scale(r["mscale"]) / scale(r["mscale_all_dim"])
ids = np.array(source["token_ids"], dtype=np.int64)
positions = np.arange(len(ids), dtype=np.float32)
angles = positions[:, None] * frequency[None, :]
cosine = (np.cos(angles) * magnitude)[:, None, :]
sine = (np.sin(angles) * magnitude)[:, None, :]
query_scale = (1 + r["llama_4_scaling_beta"] * np.log1p(np.floor(positions / r["original_max_position_embeddings"])))[:, None, None]


def rotate(x):
    first, second = np.split(x, 2, axis=-1)
    return np.concatenate((first * cosine - second * sine, second * cosine + first * sine), axis=-1)


x = weight("encoder.embed_tokens.weight")[ids]
capture("embedding", x)
qh = config["num_attention_heads"]
kh = config["num_key_value_heads"]
for layer in range(config["num_hidden_layers"]):
    if replay_layers and layer > 0:
        x = np.fromfile(pathlib.Path(trace_directory) / f"layer-{layer-1}-output.f32", dtype="<f4").reshape(len(ids), config["hidden_size"])
    p = f"encoder.layers.{layer}."
    normalized = norm(x, p + "input_layernorm.weight")
    capture(f"layer-{layer}-input-norm", normalized)
    q = rotate(linear(normalized, p + "self_attn.q_proj.weight").reshape(len(ids), qh, dim)) * query_scale
    k = rotate(linear(normalized, p + "self_attn.k_proj.weight").reshape(len(ids), kh, dim))
    v = linear(normalized, p + "self_attn.v_proj.weight").reshape(len(ids), kh, dim)
    capture(f"layer-{layer}-query", q)
    capture(f"layer-{layer}-key", k)
    capture(f"layer-{layer}-value", v)
    k = np.repeat(k, qh // kh, axis=1)
    v = np.repeat(v, qh // kh, axis=1)
    scores = np.einsum("thd,shd->hts", q, k, optimize=True) / np.float32(math.sqrt(dim))
    scores = np.where(np.triu(np.ones((len(ids), len(ids)), dtype=bool), 1)[None, :, :], -np.inf, scores)
    probabilities = np.exp(scores - np.max(scores, axis=-1, keepdims=True))
    probabilities /= probabilities.sum(axis=-1, keepdims=True)
    attention = np.einsum("hts,shd->thd", probabilities, v, optimize=True).reshape(len(ids), qh * dim)
    capture(f"layer-{layer}-attention", attention)
    x = x + linear(attention, p + "self_attn.o_proj.weight")
    normalized = norm(x, p + "post_attention_layernorm.weight")
    capture(f"layer-{layer}-post-norm", normalized)
    gate = linear(normalized, p + "mlp.gate_proj.weight")
    up = linear(normalized, p + "mlp.up_proj.weight")
    with np.errstate(over="ignore"):
        silu = gate / (1 + np.exp(-gate))
    product = silu * up
    capture(f"layer-{layer}-gate-product", product)
    x = x + linear(product, p + "mlp.down_proj.weight")
    capture(f"layer-{layer}-output", x)

final = norm(x[-1:], "encoder.norm.weight")
capture("final-norm", final)
reference = linear(final, "diffusion_head.weight")[0]
actual = np.asarray(source["logits"], dtype=np.float32)
difference = actual - reference
rmse = float(np.sqrt(np.mean(difference * difference)))
maximum = float(np.max(np.abs(difference)))
cos = float(np.dot(actual, reference) / (np.linalg.norm(actual) * np.linalg.norm(reference)))
result = {"precision": "BF16 weights and GEMM inputs, FP32 state/output", "numpy_version": np.__version__, "intermediates": trace_results, "max_abs_error": maximum, "rmse": rmse, "cosine": cos,
          "cuda_argmax": int(actual.argmax()), "numpy_argmax": int(reference.argmax()),
          "tolerances": {"max_abs": .75, "rmse": .15, "min_cosine": .999},
          "passed": bool(maximum < .75 and rmse < .15 and cos > .999 and actual.argmax() == reference.argmax()),
          "scope": "Independent per-layer replay on captured native inputs" if replay_layers else "One real prompt, complete mixed-precision NumPy forward versus native CUDA logits; no PyTorch"}
pathlib.Path(sys.argv[2]).write_text(json.dumps(result, indent=2) + "\n")
print(json.dumps(result))
assert result["passed"], {k:v for k,v in result.items() if k != "intermediates"}
