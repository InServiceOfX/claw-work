# claw-lm

A model-specific Rust inference library and a separate Rust chat server for
[nvidia/Nemotron-Labs-Diffusion-3B](https://huggingface.co/nvidia/Nemotron-Labs-Diffusion-3B).
Model computations run in original CUDA C++ kernels and direct cuBLAS calls.
Generation control, local asset loading, tokenization and HTTP/SSE are Rust.

Supports autoregressive, block-diffusion and hybrid self-speculation decoding.
The runtime uses no Transformers, PyTorch, LibTorch, Candle, Triton or NVRTC.
Project kernels are compiled ahead of time for exactly two targets:

| Family | Native target | Evidence |
| --- | --- | --- |
| GeForce RTX 30 series, including 3060/3070 | sm_86 | RTX 3060 execution tested |
| GeForce RTX 50 series, current GeForce generation as of 2026-11-08 | sm_120 | compilation; hardware testing pending |

The allowed families follow [NVIDIA's table](https://developer.nvidia.com/cuda/gpus).
Device and memory checks run before model allocation. CUDA ordinals can differ
from nvidia-smi; enumerate them first. Some family members have insufficient VRAM
for this checkpoint at a given context length.

## Build and run

Verified development environment: Linux x86_64, Rust 1.97.1, CMake 3.24 or newer,
CUDA Toolkit 13, and a compatible NVIDIA driver. Rust utility crates are fetched
by Cargo. Existing local model files are used directly; no conversion is needed.

From this folder:

```bash
export MODEL_DIR=/path/to/Nemotron-Labs-Diffusion-3B
export CARGO_TARGET_DIR=/tmp/claw-lm-target
cargo build --release --workspace --bins --examples --features cuda
cargo run --release -p claw-lm --features cuda --example devices
```

Select a supported CUDA index from that output, then start the server:

```bash
"$CARGO_TARGET_DIR/release/claw-lm-server" \
  --model-dir "$MODEL_DIR" --device 0 --context 16384 \
  --bind 127.0.0.1:8093
```

Use Ctrl-C for clean shutdown. The server retains model weights between requests
and frees its owned GPU resources at shutdown. The default alias is
`nemotron-labs-diffusion-3b`. Health is `/health`; model listing is `/v1/models`.

```bash
curl http://127.0.0.1:8093/v1/chat/completions \
  -H 'Content-Type: application/json' \
  -d '{"model":"nemotron-labs-diffusion-3b","messages":[{"role":"user","content":"What is 2 + 2?"}],"temperature":0,"max_tokens":64,"generation_mode":"self-speculation","stream":true,"stream_options":{"include_usage":true}}'
```

`generation_mode` accepts `autoregressive`, `diffusion`, or `self-speculation`.
`--mode` sets the server default. Requests can specify `block_length` (1..128) and
`threshold` (0..1). Only deterministic `temperature: 0` is implemented; other
sampling settings fail explicitly. Use `max_tokens` or `max_completion_tokens`.
One request is admitted at a time; another receives HTTP 429. Disconnecting an
SSE client cancels its generation. Only finalized text/tool calls are returned.

For direct inference without a server:

```bash
cargo run --release -p claw-lm --features cuda --example infer -- \
  --model-dir "$MODEL_DIR" --device 0 --mode diffusion \
  --prompt 'What is 2 + 2? Answer briefly.' --max-tokens 32
```

The example also supports all three modes and prints generated IDs, text, actual
forward counts and timing. `--backend cpu` explicitly selects the scalar
correctness reference; it builds without the CUDA feature and is intended for
small numerical fixtures. GPU failures never select CPU automatically.

## Library

`claw-lm` has no dependency on the server crate. See
[the direct inference example](crates/claw-lm/examples/infer.rs) for a complete
Rust consumer: open `Assets`, construct `CudaDecoder`, render/encode with `Chat`,
then call `generate` with `GenerationOptions`, a cancellation flag and a callback
for finalized tokens. `Decoder` is the single explicit CPU/CUDA computation
boundary. Each call resets per-request cache state on success and failure.

The precision policy is BF16 weights and tensor-core GEMM inputs, with FP32
activations, residuals, reductions, logits and KV. Weight storage is about
7.137 GiB. FP32 KV costs 208 KiB per token at batch one: 16384 tokens use 3.25 GiB
before scratch space and NVIDIA runtime allocations. The 262144-token advertised
model context is subject to device memory. Lower `--context` when needed.

## Pi and other clients

The portable [models.json](examples/pi/models.json) and
[settings.json](examples/pi/settings.json) describe all three modes and their
compaction settings. Copy both into a dedicated agent directory outside this
public repository, then run:

```bash
PI_CODING_AGENT_DIR=/path/to/local-pi-profile pi --offline \
  --provider claw-lm --model nemotron-labs-diffusion-3b-autoregressive --thinking off
```

For an existing Pi configuration, merge the providers from `models.json` and
the `compaction.modelOverrides` entries from `settings.json`; preserve existing
providers, authentication, defaults and unrelated settings. After changing the
configuration, run `/reload` and `/model nemotron`, then select a mode:

| Mode | Provider | Model ID shown by Pi |
| --- | --- | --- |
| Autoregressive | `claw-lm` | `nemotron-labs-diffusion-3b-autoregressive` |
| Block diffusion | `claw-lm-diffusion` | `nemotron-labs-diffusion-3b-block-diffusion` |
| Self-speculation | `claw-lm-self-speculation` | `nemotron-labs-diffusion-3b-self-speculation` |

Each named model selects its own decoding mode and uses the same resident
weights. Pi retains the conversation when switching. JSON and SSE responses
echo the selected ID. The base alias `nemotron-labs-diffusion-3b` remains
available with explicit `generation_mode`; contradictory named-mode options
are rejected. The `local` API key is a placeholder for the loopback endpoint.

The profiles advertise the recommended 16384-token server context. Each has a
2048-token compaction reserve and keeps about 2048 recent tokens, scoped only
to that exact provider/model pair. Pi's larger general-purpose defaults can
leave a small local context unable to compact. Summary quality still depends
on the model. Switching modes does not reset or enlarge conversation history.

The explicit `samplingParams.max_tokens=512` prevents Pi's conservative fixed
4096-token context margin from shrinking an otherwise valid reply to one token.
The server tokenizes the complete prompt and checks prompt plus output against
its actual context; it never silently drops history. A real overflow returns
`context_length_exceeded` with token counts, recognized by Pi's ordinary
compact-and-retry flow. If you lower `--context` for device memory, update
`contextWindow` and the compaction settings to agree. Keep output caps within
`--max-output`.

Use the same base URL `http://127.0.0.1:8093/v1`, alias and temperature zero in
other OpenAI Chat Completions clients. Pi chat and a real arithmetic tool cycle
are verified; Hermes-specific configuration remains unverified.

[The synthetic add extension](examples/pi/add.ts) is used for a repeatable tool
test. For this small research model, give explicit tool-use guidance, for example:

> Use the requested tool exactly once. A tool message is the completed result
> of the previous call. After a tool result, answer with that result and stop.
> Never repeat a successful call.

Looser prompts produced repeated calls during testing. The server validates and
translates calls; the harness executes tools. Its parser handles the public XML
format and the checkpoint's observed compact function form. Tool results are
associated by call ID, including out-of-order completion. Malformed/unknown calls
fail rather than executing anything in the server.

## Verification and continuation

```bash
cargo test --workspace --features cuda
cargo test --workspace --no-default-features
cargo clippy --workspace --all-targets --features cuda -- -D warnings
CLAW_LM_TEST_DEVICE=0 cargo test -p claw-lm --features cuda \
  --test cuda_reference -- --ignored --nocapture
```

Independent development-only checks under `verification/` use NumPy, Jinja2 and
Python's standard library, with no PyTorch or Transformers imports. Their real
model dumps belong in an explicitly selected directory outside Git. Source pins,
shapes and mathematical conventions are in [MODEL_PORT.md](docs/MODEL_PORT.md).
Current evidence and continuation state are in [HANDOFF.md](docs/HANDOFF.md).
[Product intent](docs/PRODUCT_INTENT.md), [eight stories](user_stories/README.md)
and versioned `prompts/` preserve the accepted behavior.

The implementation is intentionally limited to this model and these GPU targets.
Optional drafter LoRA, quantization, multimodality, temperature sampling and other
model families remain outside this milestone. PDD's automated architecture run
recorded intent but timed out; no successful automated synchronization is claimed.

PyTorch supports eager execution and optional [graph compilation](https://pytorch.org/get-started/pytorch-2-x/).
[llama.cpp](https://github.com/ggml-org/llama.cpp) uses its own GGML/native backends,
including CUDA C++, rather than relying on PyTorch for inference.
