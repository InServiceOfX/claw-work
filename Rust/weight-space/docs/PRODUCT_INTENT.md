# Rust weight-space experiment — approved intent

Status: approved 2026-11-09. The user approved Rust editing/evaluation tools with
the existing llama.cpp CUDA backend, source preservation and paired controls.
Implementation/evidence status is tracked in VERIFICATION.md.

Continue the research merged in PR #8, follow Wikipedia's primary references and
related arXiv papers, then implement and run a reproducible direct-weight edit
on an open model suitable for an RTX 3060 with 12 GiB VRAM.

The first experiment will reconstruct the published Qwen3.5-0.8B loop-suppression
patch using Rust. It will compare the original and modified model, and a
decoding-only control, on a fixed synthetic suite. The new editing, auditing,
projection and evaluation tools will be Rust. The existing llama.cpp CUDA backend
will provide inference; this is not a claim of Rust-native model computation.

The experiment MUST preserve original model files, reject a mismatched source,
verify exact edited tensor hashes, and independently audit all file differences.
It MUST retain failures and state measured limits. It MUST NOT claim general
capability improvement or new edit discovery from a patch-replay experiment.

Models and raw evidence stay outside Git. Changes remain confined to this new
project, its research documentation, and explicit repository integration links.
The concurrent personal-files checkout remains untouched. No push or merge is
part of this request.

Approved software components:

1. GGUF metadata/tensor inspection with bounded parsing.
2. Sparse NPZ patch import with dtype, bounds and finite-value validation.
3. Copy-only edit and full-file audit, with SHA-256 receipts.
4. Validated FP32 rank-one projection for future experiments.
5. Local inference evaluation and paired reporting.

Independent acceptance stories will cover source preservation/rejection,
exact reconstruction and fair paired evaluation. The mathematical projection
gets numerical property tests. The experiment is allowed to conclude that no
benefit was established under the tested conditions.

Research and the exact proposed artifact mapping:
[`rtx3060-research.md`](../../../research/weight-space-programming/rtx3060-research.md).

<!-- pdd-intent-entry:rust-weight-space-research-and-rtx-3060-experime-1926821a:start -->
## Rust weight-space research and RTX 3060 experiment

- Intent event: [`docs/intents/intent__rust-weight-space-research-and-rtx-3060-experime-1926821a.md`](intents/intent__rust-weight-space-research-and-rtx-3060-experime-1926821a.md)
- Change kind: `add`
- Supersedes: none
- Scope: `new_subproject_design`
- Technology: `Rust library and CLI; existing llama.cpp CUDA backend for RTX 3060 inference; no Python or training runtime`

> Now I merged https://github.com/InServiceOfX/claw-work/pull/8 this in, but I want you to keep doing deep research into weight space programming, and read related arxiv papers related to it, also consider following the references from wikipedia page: https://en.wikipedia.org/wiki/Weight-space_programming look there, continue doing deep research, and then if we can actually take a current open source model that a 3060 our 3060 can run, and then modify the weights directly for some improvement or desired effect, let's implement in Rust all the code necessary for this and try it out
<!-- pdd-intent-entry:rust-weight-space-research-and-rtx-3060-experime-1926821a:end -->
