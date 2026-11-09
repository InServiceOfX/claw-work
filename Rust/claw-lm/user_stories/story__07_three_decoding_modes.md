# 07 — Select any of the model's three decoding modes

As a user of Nemotron-Labs-Diffusion-3B, I want autoregressive, block-diffusion
and hybrid self-speculation generation so I can use every advertised decoding
mode from the same Rust library and local server.

Status: implemented; scoped evidence and limits in [VERIFICATION.md](../docs/VERIFICATION.md).

- Library and server accept explicit `autoregressive`, `diffusion` and
  `self-speculation` selection and report the chosen mode. Unknown values fail.
- All modes use the same validated base weights and tokenizer without PyTorch,
  Transformers, LibTorch or a Python inference process.
- AR performs causal decoding. Diffusion refines blocks under bidirectional
  attention with a preserved prefix, then causally commits each block.
- Self-speculation drafts bidirectionally, verifies causally and accepts only a
  contiguous valid prefix plus the causal bonus token. Rejecting a suffix crops
  its cached state before the next draft. Optional drafter LoRA is not required.
- With deterministic greedy settings, self-speculation agrees with ordinary AR
  token-for-token for the same input and stopping rules. Test zero-match,
  partial-match and full-match cases, EOS, short budgets and cache boundaries.
- Generation never exceeds token/context limits, never exposes rejected drafts,
  and distinguishes accepted tokens, forward evaluations and wall-clock timing.
- All three modes run on the available supported GPU. Compile evidence for the
  second family is labeled separately from actual hardware verification.

Evidence: independent numerical/cache tests and real-weight output comparisons;
mode selection through actual library and HTTP entry points. Do not claim a
speedup from fewer forwards alone; report measurements for each mode separately.

- Each mode is also a distinct server model ID, visible in Pi's selected-model
  status. Switching between them preserves the conversation while the server
  shares its resident weights. JSON and SSE identify the requested model.
- The legacy model alias remains usable with explicit mode selection. A named
  mode and contradictory generation option fail rather than silently changing
  the intended mode.
