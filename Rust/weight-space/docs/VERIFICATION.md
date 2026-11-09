# Verification

Date: 2026-11-09. Runtime experiments are in progress; final results will be
reported here after all frozen arms finish. No general capability gain is claimed.

## Software checks

- `cargo test --locked --offline`: 14 acceptance/property tests passed.
- `cargo clippy --locked --offline --all-targets -- -D warnings`: passed.
- `cargo fmt --check`: passed.
- PDD deterministic `contracts check prompts`: all five prompts, zero warnings
  or errors. No model-based story alignment or generated story contracts claimed.

Coverage includes source preservation, exact sparse reconstruction, protected
names, quantized targets, no-op/duplicate/out-of-range/nonfinite replacements,
wrong revisions, output collisions/aliases, corruption outside permitted entries,
truncation/count limits, projection geometry, strict final-answer grading,
deterministic cases, repetition detection, paired-denominator/error handling,
budget mismatch rejection, HTTP error diagnostics and source-tree output refusal.

The sandbox provides an empty read-only `.git` guard under some scratch roots.
The output check recognizes real gitdir files or `.git/HEAD`, so an empty guard
does not masquerade as a source checkout. Negative source-tree tests still pass.

## Exact reconstruction

Source model: pinned ggml-org BF16 GGUF, 1,557,662,496 bytes. Whole-file hash
matches the upstream Hugging Face linked file SHA-256:
`9a7bed4041b7975e0f71fa34670d1e9025213bc92905ac0db75d36c4fa3fa623`.
All three pinned assets were also retrieved and verified through the Rust
`fetch` command, including the full model's HTTPS storage redirect path.

Reconstructed model:
`f67c865e3961f13e03ac002de1b797d6806e3f80f5443926ab07a8cf1e023597`.

The Rust full-file audit compared all 1,557,662,496 bytes. Only 187,521 bytes in
130,188 BF16 entries changed. Both target tensor hashes match the published
`p50c` manifest. Every other byte, including metadata, tokenizer, embeddings,
normalizations and all other matrix entries, is identical. An independently
authored Python standard-library/ZIP/struct/mmap oracle reconstructed the sparse
arrays and confirmed the same file hashes, counts and unchanged regions. Python
was used only for this independent validation, not by the implemented pipeline.

| Tensor | Changed entries | Relative Frobenius norm of BF16 change |
|---|---:|---:|
| `blk.15.ffn_down.weight` | 65,260 | 0.1063618105 |
| `blk.19.ffn_down.weight` | 64,928 | 0.0810627653 |

The paper's 131,072 touched weights and 130,188 changed BF16 entries are distinct
counts. The norm ratios here are recomputed from rounded BF16 values and need
not exactly match the original FP32 edit's ratios.

## PDD status

Approved intent:
`rust-weight-space-research-and-rtx-3060-experime-1926821a`.
The exact apply command recorded the intent and Product Intent successfully,
then the local architecture route failed. It attempted stages requiring a
tool-capable coding agent; the exclusive local chat route refused them and did
not fall back to a remote provider. No usable architecture/prompt graph was
generated. The current graph, prompts, stories and tests were maintained directly
under the approved scope. Automated PDD synchronization is not certified.

## Runtime status

The corrected practice pilot loaded and generated on the RTX 3060. Observed
primary-run GPU allocation was approximately 2,145 MiB at context 40,960, with
roughly 80–90 tokens/second during the early sampled interval. These are observed
samples, not a peak-memory or controlled throughput benchmark.
The same 2,145 MiB allocation was observed during the long confirmation after
26K generated tokens; the unquantized BF16 setup remained within the 3060's
12 GiB capacity. This does not certify every workload or the full context limit.

All models, raw requests/responses, token IDs, server logs and receipts remain
outside Git. No normal model/provider configuration was changed. No push/merge
has been performed.

### Completed primary experiment (8,192-token cap)

All three frozen arms finished without protocol errors. Math and exact-string
probes are separated below; every completed final answer was correct, and every
incorrect score was an unfinished thinking sequence at the cap.

| Arm | Math / 20 | Exact probes / 4 | Overall / 24 | Capped | Tokens | Request seconds |
|---|---:|---:|---:|---:|---:|---:|
| Base | 8 | 3 | 11 | 13 | 130,079 | 1,561.35 |
| Published BF16 edit | 18 | 4 | 22 | 2 | 85,219 | 1,025.48 |
| Base + repetition penalty 1.1 | 18 | 4 | 22 | 2 | 86,390 | 1,067.49 |

The paired comparison has 11 recoveries and zero regressions. The fixed-seed
case bootstrap gives a 95% percentile interval of +25.0 to +66.7 percentage
points; exact two-sided McNemar p = 0.0009765625. The decoding control has 12
recoveries and one regression versus base (p = 0.00341796875). Between edit and
decoding there are two wins each and 20 ties: paired difference zero, bootstrap
interval −16.7 to +16.7 percentage points, McNemar p = 1.

These are descriptive, unadjusted statistics on a small suite sharing prompt
templates, not a generalization guarantee. The weight edit reduced total tokens
by approximately 34.5% versus base; the decoding control reduced them by 33.6%.
The small time/token difference between edit and decoding is not a controlled
throughput result or evidence of reliable superiority.

An independently authored standard-library validator recomputed each integer
answer from the prompt text, regraded all 72 raw responses, checked exact
rendered-prompt/request equality across arms except the declared repetition
penalty, and recomputed paired counts and exact McNemar values. All checks passed.
No external model judged answers. The validator is separate from the Rust
pipeline; production grading and comparisons require no Python.

The fixed 32,768-token confirmation is still running; the experiment is not
closed until those results are reported.
