# Intent: Native Rust Nemotron diffusion library and server

<!-- pdd-intent-id: native-rust-nemotron-diffusion-library-and-serve-c9e7b8ef -->
<!-- pdd-intent-sha256: c9e7b8efdfe0a0a5002b992087980262611425a1575ccfa12954ec7a5dd274ee -->

## Record

- Intent ID: `native-rust-nemotron-diffusion-library-and-serve-c9e7b8ef`
- Kind: `add`
- Supersedes: none
- Approval ID: `native-rust-nemotron-diffusion-library-and-serve-c9e7b8ef`
- Source kind: `file`
- Source reference: `docs/IMPLEMENTATION_REQUEST.md`
- Request SHA-256: `c9e7b8efdfe0a0a5002b992087980262611425a1575ccfa12954ec7a5dd274ee`
- Project scope: `subproject`
- Adoption scenario: `new_subproject_design`

- Technology: `Rust library and server, CUDA C++ kernels compiled ahead of time for sm_86 and sm_120 only; no PyTorch, Transformers or Candle`

## Original Request

> # Native Rust Nemotron diffusion library and server
>
> Build a readable Rust library and a separate Rust server for the existing local
> nvidia/Nemotron-Labs-Diffusion-3B assets. The runtime MUST NOT depend on or invoke
> Hugging Face Transformers, PyTorch, LibTorch, or a Python model worker. Implement weight access,
> model math, attention, cache ownership and generation directly, without Candle
> or a general framework spanning model families. Narrow utility crates, including
> Rust tokenizers for tokenizer.json, memory mapping, serialization, BF16 conversion,
> CPU matrix multiplication and direct CUDA bindings, are allowed.
>
> The library MUST load explicit local assets without downloads or executing model
> Python and MUST support all three decoding modes: autoregressive, prefix-cached block
> diffusion, and hybrid linear self-speculation with diffusion drafting and causal
> verification. All three MUST be available through the library and server.
> Start with a CPU correctness reference, then native NVIDIA GPU deployment.
> GPU support MUST be restricted to two targets: GeForce RTX 30 series Ampere
> (sm_86, including RTX 3060 and RTX 3070), and current GeForce RTX 50 series
> Blackwell (sm_120). The latter is the date-pinned interpretation of "the latest"
> on 2026-11-08, not a promise to support untested future architectures. No older
> GPU, RTX 40 series, datacenter GPU or non-NVIDIA compatibility is required.
> An explicitly requested unsupported GPU MUST fail clearly without CPU fallback.
> Use Rust for the reusable library, decoding control and server. Use CUDA C++ as
> much as practical for GPU operations, including direct NVIDIA cuBLAS/cuBLASLt
> calls. Build native sm_86 and sm_120 machine code ahead of time with nvcc;
> MUST NOT require PyTorch, TorchScript, torch.compile, Triton, NVRTC, runtime
> computation-graph compilation or PTX JIT for project kernels. Follow the user's
> established CUDA C++ organizational conventions: focused numerical modules,
> namespaces, RAII resource ownership, checked calls and out-of-source builds.
> Do not copy private code or its history into this public repository.
>
> The separate server MUST provide loopback-default health, model listing and
> OpenAI-compatible chat completions, both non-streaming and SSE streaming of
> finalized contiguous text. Preserve chat history and the model template, translate
> tool calls and tool results, enforce context/token limits and bounded admission,
> isolate request state, support cancellation and clean shutdown. Verify real-model
> chat and a harmless synthetic tool round trip using at least one installed Pi or
> Hermes harness; label each harness independently.
>
> Use deterministic greedy decoding as the first numerical acceptance evidence.
> Defer optional LoRA, quantization, other model families, multimodality and
> broad llama-server API parity. Reject unsupported runtime options explicitly.
> For example, a final output budget smaller than a diffusion block must be honored
> without emitting unresolved mask tokens or exceeding the requested token limit.
>
> Maintain independent user stories, versioned PDD prompt sources, numerical and
> behavioral checks, source provenance, measured results and Markdown continuation
> notes. Preserve unrelated monorepo work. MUST NOT publish private paths, raw
> conversations, model weights, caches or diagnostics. Research, synthetic tests,
> real-model execution and actual harness evidence must be reported separately.
>
> This is the portable approved implementation scope, including the user
> follow-up approving the prior proposal and adding all three modes, two GPU
> families and the explicit no-PyTorch/CUDA C++ requirements. The exact original request is retained privately; this file
> intentionally omits machine-specific locations. Scope: Rust/claw-lm only.

## Must Stay Unchanged

- The runtime MUST NOT depend on or invoke
- model math, attention, cache ownership and generation directly, without Candle
- The library MUST load explicit local assets without downloads or executing model
- Python and MUST support all three decoding modes: autoregressive, prefix-cached block
- All three MUST be available through the library and server.

## Examples

- For example, a final output budget smaller than a diffusion block must be honored

## Candidate Product Areas

- None stated.
