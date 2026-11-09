# Continuation checkpoint — native claw-lm

Updated 2026-11-08. The user approved the design, added all three modes and RTX 30/current
GeForce targets, prohibited PyTorch, required CUDA C++, and explicitly said to
continue until done. Do not ask again for the same scope approval.

## Delivered implementation

- Branch: `feat/claw-lm-nemotron-diffusion`; starting HEAD `edc327c`.
- Rust workspace: reusable `claw-lm` library plus independent `claw-lm-server`.
- Native numerical code: `crates/claw-lm/native/Source/{Runtime,Numerics,Model}`;
  checked C ABI in `native/Include/claw_lm.h`, owning Rust wrapper in `src/cuda.rs`.
- Local mapped BF16 safetensors loader, configuration validation, scalar reference,
  all three greedy decoding modes, chat/tokenizer/tool adapter and HTTP/SSE server.
- GPU weights/matrix inputs are BF16; activations, residuals, logits and KV are
  FP32. This replaced an all-BF16 activation prototype that failed a strict
  independent check. The retained numerical limit now passes; see VERIFICATION.md.
- Only native sm_86 and sm_120 images are built; project PTX/JIT and older/middle
  GPU compatibility paths are absent. Exact-family policy rejects unsupported
  cards before model allocation. CUDA device indices can differ from nvidia-smi.
- Model mapping is released after GPU upload; server weights remain resident
  across isolated requests. Cancellation, bounded admission and clean shutdown
  are implemented. Source remains model-specific and traceable.
- No Transformers, PyTorch, LibTorch, Candle, Triton or NVRTC runtime/verification.
  Narrow Rust tokenizer/template/serialization/mapping utilities are used.
- Portable build/run, direct library example and selectable three-mode Pi profiles
  are in README.md and examples/pi/. Global harness configuration was not changed.

## Verification

See [VERIFICATION.md](VERIFICATION.md) for scope and commands.18 ordinary tests,
explicit GPU numerical/cache test, CPU-only builds, fmt, strict Clippy, three
exact Jinja comparisons, full real NumPy forward, external library consumer and
real HTTP/SSE/Pi checks passed. Real NumPy RMSE 0.0284079 against 0.15; cosine 0.999987.
Native synthetic max logit error 2.98e-8 against 1e-4. Native image inspection found
only sm_86/sm_120 cubins and no project PTX. Five prompt contracts lint cleanly.

All three modes run on the actual checkpoint on RTX 3060. Longer AR/self-speculation
smoke outputs agree. Actual Pi profiles send their requested modes. A real Pi
addition tool ran once and the model answered 42 from its result. Explicit
single-call guidance matters: loose instructions caused repeated calls. The
server translates checked calls; the harness executes them. Tool result IDs are
validated and concurrent completion order is normalized before rendering.

Publication scan: the full candidate-index privacy gate passed. The actual Git
index was unchanged; the candidate included only this project and its root README
entry. Final delivery repeats the scan after this receipt update.
No commit, push, merge, model download, model conversion or global installation
was performed. No temporary Candle or llama.cpp clone exists; both were inspected
online. Build outputs and private evidence are outside this public repository.

## Running instance and private state

At delivery a loopback server is intended to remain available at
`http://127.0.0.1:8093`, legacy alias `nemotron-labs-diffusion-3b`, context 16384 on the
explicitly selected supported device. Confirm health before reusing it. Its
process ID, CUDA ordinal, memory footprint, binary path and safe stop utility are
in the main workspace's private `memory/projects/claw-lm/running-server.json`
and RESUME.md. Never kill a process by name without verifying ownership/arguments.

The verified binary and Cargo build cache are temporary local artifacts; rebuild
from source using README if they have been removed. Global Pi/Hermes settings
were left intact. Private isolated Pi profiles/event logs, all numerical captures,
source requests and original planner outputs stay in that private record.

## PDD provenance

Approved portable intent:
`native-rust-nemotron-diffusion-library-and-serve-c9e7b8ef`.
`intent apply --no-sync --no-story` wrote the durable intent event but timed out
in architecture step 1 after 90 seconds. No child agent remained. The active agent
then wrote the scoped .pddrc, architecture graph and five operative prompt groups,
maintained independent stories and implemented/tested directly under the approved
meaning. Operational state may still say applying; that is not a successful
PDD apply/sync. No generated story contracts have been fabricated.

## Next-session operation

1. Read repository AGENTS/BOTS/PRIVACY instructions, README, PRODUCT_INTENT,
   IMPLEMENTATION_REQUEST, VERIFICATION and versioned prompts before changing code.
2. Inspect the branch/status, preserve the uncommitted work and confirm the
   running server/version. Do not repeat meaning approval or provider setup.
3. For corrections, update the operative prompt and independent acceptance mold,
   then change/test the smallest affected component. Keep GPU targets explicit.
4. Rerun relevant numerical checks when precision/operations/cache behavior changes;
   rerun actual HTTP/Pi checks when chat, tool or streaming behavior changes.
5. Retain the hardware-testing limit: RTX 50is compiled, not hardware-verified;
   Hermes is unverified. Optional LoRA/quantization/temperature/other families are
   follow-up scope, not hidden incomplete requirements of this milestone.
6. Before any publication, run the full-index privacy gate and inspect candidate
   blobs/diffs. Do not bulk-stage, push or merge on the strength of passing tests.

Mounted repository writes required authorized escalated execution in this harness;
ordinary reads work. That sandbox condition does not imply a damaged checkout.

## Pi follow-up — picker and one-token output budget

The normal Pi model picker was subsequently registered at the user's request;
private config locations/backups are recorded outside this repository. All three
portable profiles now explicitly set `samplingParams.max_tokens=512` to prevent
Pi's fixed 4096-token context margin from reducing replies to a single token.
The server's actual 8192-token prompt-plus-output guard is retained. Reload Pi
after editing its models config. All modes completed a short explicit no-tool
greeting with 5658 input tokens and workspace instructions loaded; see
VERIFICATION.md for precise isolation and tool-judgment limitations. No server
code/binary, GPU allocation or default provider was changed for this correction.

## Current Pi mode IDs and context recovery — supersedes earlier profile notes

The three Pi entries now use distinct IDs ending in `-autoregressive`,
`-block-diffusion` and `-self-speculation`, so Pi's model status identifies the
mode. The server advertises those plus the legacy base alias and echoes the
requested model in JSON/SSE/usage. They share one engine and weight allocation.
Conflicting explicit modes are rejected. Server/profile context is now 16384;
RTX 3060 allocation is about 10863 MiB, and real 9020-token prefill passed.

Portable examples include both models.json and settings.json. Merge only the
three providers and their compaction.modelOverrides into an existing Pi setup.
Each normal entry uses reserveTokens=2048, keepRecentTokens=2048 and an explicit
512-token output budget. Defaults/auth/other providers were preserved during
the requested normal-profile repair. `/reload`, then `/model nemotron` and
reselect one named entry; no fresh conversation is required.

A true context overflow now returns code context_length_exceeded and exact
counts in wording Pi recognizes. Verified: 18 ordinary Rust tests, strict Clippy,
formatting, real named-mode HTTP/SSE, legacy/tool regressions, one persistent Pi
session switching all modes, and a controlled overflow followed by real
summarization and one successful retry. That short recovery test used a private
keepRecentTokens=32 fixture; normal installed settings remain 2048. See the
precise limits in VERIFICATION.md. The original user conversation was untouched.

The current server PID/hash/session are in the private running-server.json;
older PID/context/hash receipts describe past instances. This correction changed
Rust server routing/error handling and profiles, not CUDA math. No push/commit,
model conversion or automatic PDD sync was performed. The exact correction's
read-only intent plan identified the server prompt; approved harness repair
meaning was maintained directly in the existing prompt and independent stories.

## Runtime hand-over to the llama.cpp deployment

The native server was later shut down gracefully to release the shared GPU for
local model comparison. Its source and weights were retained. The active Pi
selection now belongs to the separate
[CUDA llama.cpp deployment](../../../Deployments/Scripts/llama-cpp-server/README.md).
The native Pi examples remain available for a future restart; do not assume the
old endpoint is currently running. Stop the active runtime before loading
another model onto the same device. Machine-local current process/container
receipts remain private.
