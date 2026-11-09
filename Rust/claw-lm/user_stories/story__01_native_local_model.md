# 01 — Run local assets without Transformers

As a user, I want to load my existing Nemotron-Labs-Diffusion-3B directory in
native Rust so I can use it without Transformers.

Status: implemented; scoped evidence and limits in [VERIFICATION.md](../docs/VERIFICATION.md).

- Valid local configuration, tokenizer and BF16 safetensors produce real inference.
- No runtime or verification path imports Transformers, PyTorch or LibTorch, starts a Python model worker, executes
  downloaded Python, automatically downloads assets or uses the network.
- Missing, truncated, unsupported or incorrectly shaped assets fail clearly.
- Read-only model files work; loading and shutdown leave the directory unchanged.
- Model location is explicit and examples contain no personal paths or weights.

Evidence: dependency/process review, offline real-model run, corrupted synthetic
assets and directory before/after check. Header inspection alone is insufficient.
