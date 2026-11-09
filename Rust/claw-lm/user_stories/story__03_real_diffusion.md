# 03 — Generate with actual block diffusion

As a user, I want the model's native block-diffusion generation so I can use its
diffusion capability.

Status: implemented; scoped evidence and limits in [VERIFICATION.md](../docs/VERIFICATION.md).

- Known prompt IDs produce intermediate activations and logits agreeing with an
  independent numerical reference within declared dtype-specific tolerances.
- Generation seeds blocks from causal context, refines the current block with
  bidirectional attention, and commits causal context between blocks.
- Denoising never mutates cached prefix state, the prompt or committed tokens.
- Thresholding makes progress even if every confidence is below threshold. EOS
  becomes final only when earlier positions are resolved.
- Token/context limits hold, including a final partial block. Invalid options
  fail explicitly and unresolved masks never appear as completed output. Document
  this token-limit adaptation because upstream asserts divisibility by block size.
- Report decoding mode and actual forward-pass count. AR-only output or plausible
  text is insufficient proof of diffusion correctness.

Evidence: synthetic numerical/cache tests, real-weight activation/logit checks,
deterministic AR and diffusion runs, EOS/threshold/short-budget/context tests.
