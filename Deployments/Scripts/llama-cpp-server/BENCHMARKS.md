# Local model comparison and Pi recommendation

Measured on a GeForce RTX 3060 12GB with the existing local GGUFs. This is one
bounded workflow smoke suite, not a general model leaderboard or a guarantee
about future generated code. No weights were downloaded or converted.

## Recommendation

Use **Qwythos 9B Q5_K_M**, API alias `qwythos-9b-q5-k-m`. It tied Q6 at all six
checks while generating faster and using about 720 MiB less device memory in
this suite. That made it the practical choice for local Pi coding/tool use.

```bash
# After copying the templates and editing local model paths:
./run-recommended.sh --dry-run
./run-recommended.sh
```

The final local profile uses a 65536-token context, one slot, Flash Attention and
q4_0 KV. The wrapper retains the existing launcher and adds a 512-token thinking
budget and Jinja support. Model weights stay resident until the container is
stopped. `./run-recommended.sh --stop` releases the GPU allocation; it does not
delete weights. Stop a separately running native Rust server with its own normal
graceful shutdown before giving the GPU to this deployment.

## Text results

Each row used context 32768, one slot, the same cached image, no prompt reuse,
reasoning budget 512, output cap 2048, temperature 0.6, top_p 0.95, top_k 20,
min_p 0, repeat_penalty 1.05 and seed 42. Quantizations can produce different
samples even with the same seed. This is not enough data to rank their general
capabilities or establish a statistically reliable quality difference.

| Local model | Checks | Median generated tok/s | Device VRAM MiB | Suite response seconds | Observation |
| --- | ---: | ---: | ---: | ---: | --- |
| Qwen3.8 distilled Q5 | 4/6 | 41.1 | 6161 | 70.3 | Greeting constraint; authored test fails on unsorted inputs |
| Qwen3.8 distilled Q4 | 5/6 | 44.8 | 5483 | 50.0 | Greeting constraint |
| Qwythos Q5 | 6/6 | 40.9 | 6161 | 51.8 | All checks passed |
| Qwythos Q6 | 6/6 | 35.7 | 6881 | 57.5 | All checks passed |
| Qwen3.5 distilled Q4 | 3/6 | 44.7 | 5525 | 51.5 | Greeting constraint; generated Rust does not compile |
| Qwen3.5 distilled Q5 | 4/6 | 41.0 | 6203 | 79.7 | Rust output hit the 2048-token cap; not graded |
| Qwen3.5 distilled Q8 | 5/6 | 31.3 | 8755 | 87.8 | Greeting constraint |

Generation rates are medians of the server's short-case timings and include
reasoning tokens. Suite seconds sum response latency, excluding startup. VRAM is
total device usage after the suite, not peak allocation or a model-only allocator
measurement; the idle device baseline was about 15 MiB. Results need not transfer
to other GPUs, image revisions, sampling settings or long-context workloads.

The six checks are a short one-sentence greeting, exactly twenty primes ending
in 71, correct Rust merge behavior, runnable model-authored tests, one correct
addition call, and a final answer using the tool result. Rust compilation and
six independent edge cases are executed with bounded timeouts. The checker
separates correct function behavior from incorrect authored tests. A response
ending because of its output cap is incomplete, not evidence of incorrect logic.
All seven models passed the tool-call and tool-follow-up checks.

Raw responses, compiler output, local configuration, container logs and private
Pi traces are excluded from this repository. `benchmark.py` reproduces the test
and writes to an explicitly selected directory outside Git. Missing weights are
reported rather than silently downloaded or counted as tested.

## Projector and asset checks

Both available projector configurations loaded at context 16384 and identified
a synthetic red PNG correctly:

| Profile | Result | Device VRAM MiB |
| --- | --- | ---: |
| Qwen3.5 Q8 with BF16 projector | Red image passed | 9683 |
| Qwythos Q5 with F16 projector | Red image passed | 7085 |

These checks establish basic loading and image transport, not general vision
accuracy. The selected Pi setup is text-only and does not load a projector.

The Gemma profiles' referenced files were missing. Qwythos Q4 and several other
quantization/MTP files were Git LFS pointers rather than complete GGUFs. They
were excluded from loading and the ranking. `audit_profiles.py` checks local
volume mappings and GGUF headers before a runtime test.

## Actual Pi verification

Merge [models.json](examples/pi/models.json) and
[settings.json](examples/pi/settings.json) into the corresponding Pi files,
preserving other providers/authentication/settings. Only the selected local
model is configured. It advertises a 65536-token context and 4096-token output
cap, with reserveTokens=8192 and keepRecentTokens=8192 for compaction.
A per-model `modelThinkingLevels` entry selects `low`; other models' defaults
are not changed. Thinking off remains available through the normal Pi controls.

```text
/reload
/model llama-cpp-local/qwythos-9b-q5-k-m
```

A normal Pi process with workspace instructions and default extensions loaded
passed the greeting, prime-list and Rust-with-tests prompts in one conversation.
The Rust output compiled and passed both its authored tests and six independent
checks. MCP was disabled; a guard blocked tool execution for these no-tool tasks.
A separate Pi process actually invoked a registered addition extension once,
received sum 42 and answered 42. Streaming usage, reasoning parsing and the local
model's default low thinking level were observed, not inferred from exit status.

Thinking-off tests are retained as a limitation: the merge function was correct,
but it added an invalid unsorted-input test. The passing normal Pi check used
bounded thinking, matching the reasoning-enabled benchmark policy. These are
small acceptance checks, not a claim that the model never writes incorrect code.

Switching models retains the conversation. For an independent comparison, start
a fresh conversation rather than including the previous model's answers.

## Runtime provenance

The tested cached CUDA server image was llama.cpp build **10603**, revision
`c060ca974c773c7c3d17fd1b66dc9d312bc292c0`, repository digest:

```text
ghcr.io/ggml-org/llama.cpp@sha256:41ebf873c2e085dcc3186dc4717ce8112bf8011aabd98038b6bb2b1fe66c86b9
```

The machine-local config pins that digest. The generic template can be pinned
to reproduce it. See the [versioned server reference](https://github.com/ggml-org/llama.cpp/blob/c060ca974c773c7c3d17fd1b66dc9d312bc292c0/tools/server/README.md),
[Qwythos model card](https://huggingface.co/empero-ai/Qwythos-9B-Claude-Mythos-5-1M-GGUF),
[Qwen3.8 card](https://huggingface.co/empero-ai/Qwen3.8-9B-Distill-GGUF), and
[Qwen3.5 distilled card](https://huggingface.co/Jackrong/Qwen3.5-9B-Claude-4.6-Opus-Reasoning-Distilled-v2-GGUF).
The sampling policy is compatible with the reasoning-model cards' temperature,
top-p and top-k guidance; the bounded budgets are local evaluation choices.
