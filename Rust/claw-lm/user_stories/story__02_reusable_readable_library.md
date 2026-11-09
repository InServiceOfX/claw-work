# 02 — Embed a readable Rust library

As a Rust developer, I want a reusable library with a direct, inspectable loading
and inference path so I can diagnose problems and embed it in my tools.

Status: implemented; scoped evidence and limits in [VERIFICATION.md](../docs/VERIFICATION.md).

- A small external Rust consumer generates without HTTP, the server crate or Python.
- Configuration, tensor layouts, numerical operations and generation state have
  explicit ownership, documented interfaces and useful errors.
- CPU correctness builds without CUDA. An explicitly requested unavailable backend
  fails clearly rather than silently switching backends.
- Unicode, special tokens, system messages and multi-turn history preserve the
  model's tokenizer and chat-template behavior.
- Requests have independent state. Dropping a request or model releases its owned
  resources without relying on mutable global model state.

Evidence: standalone consumer, dependency/source review, independent tokenizer and
prompt comparisons, request isolation and measured resource lifecycle checks.
