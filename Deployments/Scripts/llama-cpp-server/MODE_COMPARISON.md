# Qwythos and three native Nemotron modes on RTX 3060

Measured November 8, 2026 on one **RTX 3060 12GB**, one request at a time.
**Keep Qwythos 9B Q5_K_M with low thinking for Pi.** It generated fastest here,
passed the Rust check, and left substantially more VRAM available. Q5 is the
quantization currently selected; it is not Q8.

## Throughput and response latency

| Model / backend | Median generated tokens/sec | First visible text, seconds | Idle / sampled peak VRAM, MiB |
| --- | ---: | ---: | ---: |
| Qwythos 9B Q5 (low thinking) | 40.1 | 0.239 | 5939 / 5945 |
| Nemotron AR | 33.7 | 0.060 | 10863 / 10865 |
| Nemotron block diffusion | 26.5 | 0.867 | 10865 / 10865 |
| Nemotron self-speculation | 6.7 | 0.939 | 10865 / 10865 |

These are **end-to-end generated tokens/sec**: actual API completion tokens
(including thinking) divided by request-to-finish time. The table uses medians
of three fresh 512-token capped tutorial generations, after one excluded warmup.
All twelve throughput probes reached the cap. This is a throughput measurement,
not a completed-tutorial score: Qwythos's 512 tokens were entirely thinking, so
its first visible text is thinking and it had no final answer within that cap.
Native Nemotron emits answer text without a separate thinking field.
The earlier [seven-model results](BENCHMARKS.md) use server decode timings;
those rates exclude more overhead and are not the same metric.

| Model | ~5.5K input: first visible text | Rust: first visible text / first answer |
| --- | ---: | --- |
| Qwythos 9B Q5 (low thinking) | 3.79s | 0.50s / 5.19s |
| Nemotron AR | 11.90s | Error; no completed answer |
| Nemotron block diffusion | 12.06s | Error; no completed answer |
| Nemotron self-speculation | 12.27s | Error; no completed answer |


The long-prefix probe uses the same synthetic text, tokenized to 5551 tokens
by Qwythos and 5529 by Nemotron, with a 128-token output cap. Qwythos began
thinking but reached that cap without its final READY answer. Native modes
finished READY. These first-text times do not compare completed-answer speed.

The native server currently **buffers the whole output whenever tools are
declared**, even when the user explicitly forbids tool calls. The Rust probe
exercises that path; the errors below produced no completed text answer.
llama.cpp streams thinking and answer content with tools declared. This is a
concrete implementation difference affecting how responsive Pi feels.
Bare native AR can start text quickly, as its short-prompt number shows.

## Bounded correctness check

| Model | Exactly 20 primes / response seconds | Rust with one tool declared, execution forbidden |
| --- | --- | --- |
| Qwythos 9B Q5 (low thinking) | Pass / 10.18s | Function and authored tests pass |
| Nemotron AR | Pass / 1.43s | Incomplete compact tool output |
| Nemotron block diffusion | Fail / 0.55s | Model requested an unknown tool |
| Nemotron self-speculation | Pass / 0.67s | Incomplete compact tool output |

The block-diffusion prime answer contained 21 numbers, not the requested 20.
Self-speculation finished this short prime request faster than AR, despite being
slower on the tutorial. Workload and acceptance behavior matter; one throughput
row is not a universal ranking. Qwythos's prime response time includes thinking.
Rust is compiled and checked against six independent empty/negative/duplicate/
integer-boundary cases as well as all model-authored tests. Tool declarations
are synthetic; no generated tool is executed in this comparison. Errors and
output caps are retained, not counted as successful answers or replaced by
selected reruns. This small check cannot establish general model quality, nor
attribute every native failure to the checkpoint rather than our implementation.

## Conditions and limits

- Common server context **16384**, one slot, no simultaneous GPU backends.
  Prompt caching is disabled for llama.cpp comparison requests. Native requests
  reset their cache. Model loading and the excluded warmup are outside timing.
- llama.cpp uses GGUF Q5_K_M weights, Flash Attention, q4_0 KV, Jinja, a 512-token
  thinking budget, temperature .6, top_p .95, top_k 20, min_p 0, repeat penalty
  1.05, seed 42, and low reasoning effort. Image revision/digest are recorded in
  [BENCHMARKS.md](BENCHMARKS.md).
- Native Rust/CUDA uses BF16 weights, FP32 KV, deterministic temperature 0,
  block length 32, threshold .9, and source revision `eecedd2`. All three aliases
  were verified against their returned generation-mode metadata. These settings
  measure our current implementation, not an optimized NVIDIA reference runtime.
  In particular threshold .9 is also used for self-speculation; it is not a claim
  that this is its fastest policy.
- Tokenizers, weight precision, cache precision, sampling and kernels differ.
  This is a comparison of the usable local stacks, not an isolated experiment
  proving one decoding algorithm faster or one model universally better.
  Both inference stacks run native CUDA without PyTorch or Transformers.
- Self-speculation's first tutorial run drafted 1800 tokens
  and accepted 452; its 1277
  forward calls include 1217 denoising calls. Draft work
  does not automatically translate into lower latency with this implementation.
- GPU memory is whole-device usage sampled approximately every .2 seconds.
  A sampled peak can miss brief allocations; it is not an allocator trace.
  Raw SSE, generated code, host receipts and private model paths stay outside Git.

## Memory in the normal Pi deployment

After the comparison the native process was stopped and Qwythos restored with
its normal **65536-token context**, low thinking and prompt cache enabled.
The normal profile used **6587 MiB idle** and a sampled peak
of **6593 MiB** during a synthetic
16551-token input and a separate Rust generation
with a 2048-token output cap. That leaves about
5.6 GiB on this card in those checks.
This is not a full-64K stress test or a guarantee against every future OOM.

Idle usage already includes resident weights, cache allocations and workspaces;
it does not imply that generation needs another copy of the model.
`./run-recommended.sh --stop` frees this allocation without deleting weights.
A replacement model can then use most of the card, rather than only the current
free VRAM. Do not stack two resident servers on this GPU for this setup.

## Could a larger model fit?

A concrete candidate is **Qwen3-14B Q4_K_M**: its
[official GGUF](https://huggingface.co/Qwen/Qwen3-14B-GGUF/tree/main) is about
9GB decimal (~8.4GiB). Start with one slot and an **8K or 16K context** and
quantized KV. From the [official configuration](https://huggingface.co/Qwen/Qwen3-14B/raw/main/config.json),
40 layers, 8 KV heads and head dimension 128 imply roughly 360MiB at 8K or
720MiB at 16K for q4_0 K+V payload, before alignment and runtime buffers.
That makes a full-GPU fit plausible, not verified on this machine. Do not assume
it retains the current 64K context, runs as fast, or outperforms this distilled
9B on your tasks. No larger model was downloaded or benchmarked for this report.

The already-tested Qwythos Q6 passed the same six workflow checks as Q5, used
about 720 MiB more at 32K context, and decoded at 35.7 rather than 40.9 tokens/sec.
It did not demonstrate an improvement worth its latency here. Qwythos Q8 would
still have 9B parameters, with less quantization; it is not a larger model.
The local Q8 entry was an LFS pointer, not complete weights, so it was not tested.

## Reproduce

Use the pinned image, real weights, and edited ignored local configuration.
The measurement client only accepts an already-running loopback server with
context 16384; it never downloads models or starts competing GPU backends.
From this deployment directory, for Qwythos:

```bash
./run-recommended.sh -c 16384
python3 compare_modes.py --url http://127.0.0.1:8080 \
  --model qwythos-9b-q5-k-m --backend llama --label 'Qwythos Q5' \
  --gpu-index GPU_INDEX --repetitions 3 --out /tmp/qwythos-speed
./run-recommended.sh --stop
```

Replace GPU_INDEX with the RTX 3060's nvidia-smi index. Then build the native
server following [its README](../../../Rust/claw-lm/README.md), and run it with
`--model-dir "$MODEL_DIR" --device CUDA_ORDINAL --context 16384 --max-output 2048
--bind 127.0.0.1:8093`. CUDA ordinal and nvidia-smi index need not match.
In a separate terminal run the client with `--backend native` for each alias
`nemotron-labs-diffusion-3b-autoregressive`,
`nemotron-labs-diffusion-3b-block-diffusion`, and
`nemotron-labs-diffusion-3b-self-speculation`, a distinct output directory and
`--url http://127.0.0.1:8093`. Each completes before starting the next.
Stop the native server gracefully, then run `./run-recommended.sh` to restore
the normal Pi deployment. Keep all raw captures outside Git.
