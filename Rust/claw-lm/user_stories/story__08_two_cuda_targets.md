# 08 — Use two explicit NVIDIA GPU targets without a framework

As a user, I want native CUDA C++ execution on RTX 30 series and the latest
GeForce generation so the runtime stays focused instead of accumulating broad
GPU and framework compatibility code.

Status: implemented; scoped evidence and limits in [VERIFICATION.md](../docs/VERIFICATION.md).

- Supported GPU families are RTX 30 series/Ampere sm_86 (including RTX 3060 and
  RTX 3070) and current RTX 50 series/Blackwell sm_120. The latest-family choice
  is pinned to 2026-11-08 using NVIDIA's public specifications.
- Compile native machine code only for sm_86 and sm_120. No project PTX fallback,
  runtime kernel/graph compiler, PyTorch, LibTorch, Transformers or Triton.
- Rust owns public APIs, model/generation state and serving. CUDA C++ owns GPU
  numerical kernels and direct NVIDIA matrix-library operations across a small
  checked C ABI. Use focused modules, namespaces, RAII and out-of-source builds.
- Explicitly reject unsupported devices, including RTX 40 and older generations,
  before allocating model memory. Never silently fall back to CPU or another GPU.
- Select a device explicitly when the machine has both supported and unsupported
  cards. Budget weights, KV and scratch memory before allocation and report
  insufficient VRAM clearly. Family support is not a claim that every member can
  fit every model/context: an 8 GB RTX 3070 is tighter than a 12 GB RTX 3060.
- Dropping Rust owners releases CUDA allocations, stream and matrix handles.
  Errors and cancellation do not leak state into later requests.

Evidence: negative device-policy cases, native-code image inspection, CPU/CUDA
numerical comparisons, resource lifecycle and real-model execution on available
hardware. A successful sm_120 compilation is not a Blackwell hardware test.
