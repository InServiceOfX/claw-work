# claw-work

Public, reusable agent-authored tools and experiments. See [agent instructions](AGENTS.md)
and the [public/privacy boundary](PRIVACY.md).

## Projects

- [Lean agent toolkit](Rust/lean-agent/README.md): mathlib-enabled Docker environment,
  Rust-first isolated proof checking and mathematical code-review workflows.
  [Local setup for Claude Code, Codex, Grok Build, Hermes and Pi](Rust/lean-agent/docs/SETUP.md)
  includes native ARM64/x86-64 images, shared CLI tools and two portable skills.
- [llama.cpp server for macOS](Deployments/llama-cpp-server-macos/README.md): run
  local GGUF open-weight models natively on Apple Silicon (Metal), resolved from the
  HuggingFace hub cache, behind an OpenAI-compatible API for agent harnesses such as pi.

The Lean toolkit distinguishes proofs about mathematical models from verification
that source code implements those models. Read its verification status before relying
on any result. Development happens on feature branches; the repository owner merges.
