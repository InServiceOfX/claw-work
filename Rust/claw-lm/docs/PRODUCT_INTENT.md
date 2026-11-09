# Native Rust language-model library and server

Status: meaning approved by the user on 2026-11-08, with the explicit additions
below. Do not ask again for the same approval. PDD application and implementation
evidence are tracked independently in HANDOFF.md.

## User intent

Use the locally cloned public model nvidia/Nemotron-Labs-Diffusion-3B through a
readable Rust library and a separate Rust server that a harness such as Pi or
Hermes can connect to in the same general manner as llama-server. Build the
runtime from the ground up, avoiding the complexity of a framework serving every
model. Use upstream Python as a behavioral reference. Keep user stories and
periodic Markdown handoffs so another session or harness can continue.

The exact original request and machine-specific model location are retained
privately outside this repository. Public examples use `MODEL_DIR`.

## Durable requirements

- R-001 MUST implement inference in Rust. MUST NOT depend on or invoke Hugging
  Face Transformers, PyTorch or LibTorch, including through workers/subprocesses.
- R-002 MUST expose a reusable library independently of HTTP and any AI harness.
- R-003 MUST implement autoregressive, block diffusion and hybrid linear
  self-speculation for this model, selectable in both library and server.
- R-004 MUST load existing local model assets without automatic downloads,
  executing model-repository Python, or conversion through Transformers.
- R-005 MUST keep model computations, weight layouts and state ownership explicit
  and traceable, with ordinary Rust composition and useful errors.
- R-006 MUST provide a separate Rust server usable from Pi or Hermes. Protocol
  tests and actual harness compatibility require separate evidence.
- R-007 MUST keep independent acceptance stories and current handoffs containing
  source references, verification results, remaining gaps and restart commands.
- R-008 MUST preserve unrelated projects and keep weights, private paths, raw
  conversations, caches and private diagnostics out of public Git.

## Approved decisions

1. One Cargo workspace: `claw-lm` library and `claw-lm-server` application, plus a
   small direct-inference example. No Candle dependency, autograd, generic model
   registry or inheritance-style model hierarchy.
2. Own weight access, model math, attention, caching and generation. Narrow utility
   dependencies may include serde, mmap, BF16 conversion, CPU matrix multiplication,
   direct CUDA bindings, and Rust `tokenizers` for the existing tokenizer JSON.
   Rust tokenizers is distinct from Python Transformers; its use is explicit here.
3. CPU correctness path first, followed by all three modes and direct NVIDIA GPU
   deployment. GPU support targets RTX 30 series sm_86 and current RTX 50 series
   sm_120 only; CUDA remains an optional Cargo build feature for CPU-only checks.
   CPU correctness alone does not establish interactive speed or GPU support.
4. Server v1: loopback default, health, model listing, OpenAI-compatible chat
   completions, non-streaming and SSE streaming of committed text, bounded
   admission, cancellation, clean shutdown and independent request state.
5. Preserve system/user/assistant/tool history and the model's chat template.
   Translate model tool calls to OpenAI tool calls. Verify a harmless synthetic
   tool round trip with at least one installed target harness before claiming
   that harness works. The server translates calls; the harness executes them.
6. Deterministic greedy decoding is the first numerical evidence. Temperature
   sampling follows once independently tested. Reject unsupported options
   explicitly. Optional LoRA, quantization, other model families,
   multimodality and broad llama-server API parity are deferred.

## Prompt groups and evidence

| Group | Responsibility | Stories |
| --- | --- | --- |
| assets | configuration, inventory, bounded BF16 reads | 01, 02 |
| compute | CPU reference, CUDA C++ operations, YaRN, GQA | 02, 03, 08 |
| generation | AR, diffusion, self-speculation, caches, limits | 03, 04, 07 |
| chat | tokenizer, template, tool mapping | 02, 05 |
| server | HTTP, SSE, admission, cancellation | 04, 05 |

Establish `.pddrc`, `architecture.json` and versioned
`.prompt` sources before implementation. Do not fabricate generated contracts.

Delivery sequence: malformed-asset tests and real header validation; independent
synthetic numerical and tokenizer/template comparisons; real-weight intermediate
activation/logit checks; AR and diffusion generation; synthetic protocol tests
then real-model HTTP tests; actual harness chat/tool cycle; measured memory and
latency. Report each layer separately and refresh the handoff at every milestone.

## Approved additions from the follow-up

- R-009 MUST expose all three modes, including hybrid self-speculation, through
  library and server. Self-speculation drafts bidirectionally, verifies causally,
  accepts a contiguous valid prefix with its bonus token, and crops rejected KV.
- R-010 GPU deployment MUST target only RTX 30 series/Ampere sm_86 and current
  RTX 50 series/Blackwell sm_120. Unsupported devices fail before allocation with
  an actionable error. CPU is an explicit development reference, never a silent
  fallback. Architecture support does not promise that every card has enough VRAM.
- R-011 MUST NOT use PyTorch, LibTorch, TorchScript, Triton, NVRTC or runtime
  graph compilation in runtime or verification. Use independent scalar/NumPy
  calculations for reference evidence. Inspecting public Python source is allowed.
- R-012 MUST favor CUDA C++ for GPU kernels and direct NVIDIA matrix libraries,
  with a small C ABI to Rust and native sm_86/sm_120 code built by nvcc ahead of
  time. Do not include PTX as a compatibility fallback for project kernels.

NVIDIA's [GPU table](https://developer.nvidia.com/cuda/gpus) and
[GeForce comparison](https://www.nvidia.com/en-us/geforce/graphics-cards/compare/)
identify the two target families as of 2026-11-08. Future generations require an
explicit, tested target update. The user requested less compatibility code, not
an unmeasured speed claim. No support is promised for RTX 40, older GPUs, other
vendors or datacenter-specific variants.

<!-- pdd-intent-entry:native-rust-nemotron-diffusion-library-and-serve-c9e7b8ef:start -->
## Native Rust Nemotron diffusion library and server

- Intent event: [`docs/intents/intent__native-rust-nemotron-diffusion-library-and-serve-c9e7b8ef.md`](intents/intent__native-rust-nemotron-diffusion-library-and-serve-c9e7b8ef.md)
- Change kind: `add`
- Supersedes: none
- Scope: `new_subproject_design`
- Technology: `Rust library and server, CUDA C++ kernels compiled ahead of time for sm_86 and sm_120 only; no PyTorch, Transformers or Candle`

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
<!-- pdd-intent-entry:native-rust-nemotron-diffusion-library-and-serve-c9e7b8ef:end -->

## Harness integration correction

The user requested visible decoding-mode identities and continued conversation
when switching models in Pi. The server exposes one named model ID per mode
while sharing weights, retains its legacy alias and returns standard recognizable
context errors. Recommended Pi profiles match the running context and apply
compaction budgets only to these models. Acceptance includes one persistent
harness session; model answer quality remains distinct from integration behavior.
