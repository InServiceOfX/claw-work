# Verification — 2026-11-08

Scope: the public Nemotron-Labs-Diffusion-3B checkpoint pinned in MODEL_PORT.md,
Linux x86_64, CUDA Toolkit 13.0, Rust 1.97.1, and one RTX 3060 12GB. Measurements use
public/synthetic inputs. Raw model dumps and harness event logs remain private.

## Evidence

| Layer | Result | What it establishes |
| --- | --- | --- |
| Local assets | Full SHA256 equals public LFS pointer;237 BF16 tensors validated | Integrity, names, shapes, dtype, offsets and EOF |
| Rust behavior |18 ordinary tests; CPU-only build also checked | Generation modes, EOS statistics, corruption, request/cache isolation, malformed tools, call IDs, Unicode stops, HTTP/SSE and cancellation |
| Native CUDA reference | Max absolute logit difference 2.98e-8; tolerance1e-4 | Synthetic full-model causal/refinement comparison and cache rollback |
| Independent NumPy, full real forward | RMSE 0.0284079, max error0.1526694, cosine 0.9999871; same next token | Mixed-precision implementation agrees with independently written equations on one prompt of 27 tokens |
| Rust/Jinja rendering |3exact-text comparisons pass | Unicode, reasoning-history truncation and typed tool history |
| Real HTTP | All three modes, matching SSE/non-stream text,two longer AR/self-spec comparisons and tool round trip | Actual real-model server behavior rather than mocks |
| External Rust consumer | Real self-speculation produced the expected arithmetic answer | Library use independently of HTTP/server dependencies |
| Actual Pi | All three profiles sent verified mode values and received responses; one add(12,30) returned42 and final answer42 | Real harness chat, dispatch, execution, result history and follow-up |
| GPU code images | Only sm_86 and sm_120 cubins; no project PTX images | Ahead-of-time target restriction |
| Device rejection | Actual older GPU rejected before model allocation | Explicit supported-family boundary |
| Shutdown | Controlled server shutdown released over 8 GiB of GPU memory | Whole-process resource lifecycle; not an isolated allocator benchmark |
| Rust quality | fmt and strict Clippy pass | Formatting and static diagnostics in the implemented workspace |
| PDD contracts |five prompts, zero warnings/errors | Deterministic prompt contract lint; no semantic generation/sync claim |

The NumPy tolerances were max absolute error<0.75, RMSE<0.15, cosine>0.999 and
matching next-token argmax. They were retained while fixing numerical drift.
The original all-BF16 activation prototype failed that RMSE check. Native state
was changed to FP32, while retaining BF16 weights and GEMM inputs. There is one
production precision policy; the failed prototype is not a compatibility branch.
Full-forward checks compare the same declared mixed-precision equations, rather
than claiming bit-identical behavior to a different framework's BF16 execution.

The real HTTP suite uses an arithmetic prompt in all three modes, prime-number
and exact-text prompts for AR/self-speculation agreement, and a two-turn synthetic
addition tool task. Function calling uses explicit single-call guidance. We also
observed repeated calls with looser instructions and unreliable behavior on one
made-up tool task. A functioning transport does not guarantee model tool judgment.

## Pi output-budget follow-up

A normal Pi greeting initially returned `finish_reason=length` after one token,
with no visible answer. A payload capture reproduced `max_tokens=1`: Pi's fixed
4096-token safety margin exhausted the estimated room in the advertised
8192-token window. The portable profiles now explicitly set
`samplingParams.max_tokens=512`. A second capture confirmed 512 in the request;
server context/output validation remains unchanged.

An actual Pi check with workspace instructions loaded (5658 input tokens),
normal registered profiles, external extensions/MCP disabled and tool execution
blocked completed this request in all three modes:

> Say hello in one short sentence. Do not call tools.

AR and self-speculation returned the same short greeting (11 output tokens);
diffusion returned a shorter greeting (4 tokens). All ended with `stop`, no
error and no tool execution. A separate AR exact-word request also completed,
although it added punctuation. These are bounded smoke checks, not broad
instruction-following or full extension/tool integration evidence.

An unconstrained greeting with the full tool declarations also exposed a model
quality limitation: it copied the template's example function name, which was
correctly rejected as unknown. An independently rendered request with identical
native prompt token IDs reproduced this without executing tools. Explicit
no-tool guidance worked for the greeting above. The budget correction does not
claim to fix model tool judgment. Raw context and diagnostic captures stay
outside the public repository.

## Named-mode switching and context recovery follow-up

The original Pi entries shared one model ID, so its status could not distinguish
modes. The named IDs now end in `-autoregressive`, `-block-diffusion`, and
`-self-speculation`; the legacy base alias remains supported. Both JSON and all
SSE frames echo the requested ID. Conflicting mode options are rejected. The
server still shares one engine and resident weight allocation.

The running RTX 3060 server and portable profiles now agree on 16384 context
tokens. Its measured GPU allocation is about 10863 MiB. A real CUDA request
successfully prefilled 9020 prompt tokens and generated with an intentionally
one-token cap, proving execution beyond the former 8192-token boundary. This
is a capacity check, not a long-context answer-quality benchmark.

Two additional server regression tests cover named-model dispatch/identity,
legacy compatibility, contradictory selection, exact context boundaries and
recognizable overflow before admission, with streamed/non-streamed cases.
All 18 ordinary tests, strict Clippy and formatting pass. The extended real HTTP
suite passes all three named modes, matching SSE text/identity, legacy AR versus
self-speculation agreement, a genuine oversized request and the addition-tool
round trip. Earlier numerical evidence is unchanged; CUDA math was not modified.

An actual Pi RPC process switched through all three registered modes in one
conversation, retaining one, two and three user turns respectively. Every model
returned a short greeting with the selected response ID. Workspace instructions
were loaded; external extensions/MCP were disabled and tool execution blocked.
This verifies conversation continuity through model switches, not arbitrary
extension behavior or model answer quality.

A controlled one-shot oversized request then exercised real HTTP rejection,
Pi's overflow classification, actual model summarization and one successful
compact-and-retry. The final response completed normally. That recovery fixture
used `keepRecentTokens=32` in its private test profile to make the short history
compactable; the installed and portable normal profiles use 2048. Normal
`reserveTokens=2048` gives a 14336-token threshold in the 16384-token window.
The server uses `context_length_exceeded` and a recognized maximum-context-length
message with exact prompt/output/limit counts. It never silently drops history.
Natural long-running compaction with arbitrary tools and the model's summary
quality are not certified by this bounded recovery test.

The normal Pi model listing confirms all three distinct IDs with a 16.4K window.
The user's original conversation was not edited. Reload Pi and reselect a named
mode once; the conversation can be retained. Private backups, raw RPC events and
current process/hash receipts remain outside this public repository.

## Reproduce

From the project root, with MODEL_DIR and CARGO_TARGET_DIR selected as in README:

```bash
cargo fmt --all --check
cargo test --workspace --features cuda
cargo test --workspace --no-default-features
cargo clippy --workspace --all-targets --features cuda -- -D warnings
CLAW_LM_TEST_DEVICE=0 cargo test -p claw-lm --features cuda \
  --test cuda_reference -- --ignored --nocapture
PDD_LOCAL_ONLY=1 PDD_SKIP_UPDATE_CHECK=1 pdd --no-core-dump contracts check prompts
```

For real-model checks, use an explicit directory outside this public repository:

```bash
mkdir -p "$EVIDENCE_DIR/intermediates"
CLAW_LM_TRACE_DIR="$EVIDENCE_DIR/intermediates" \
  "$CARGO_TARGET_DIR/release/examples/infer" --model-dir "$MODEL_DIR" --device 0 \
  --max-tokens 3 --dump-logits "$EVIDENCE_DIR/logits.json"
OPENBLAS_NUM_THREADS=4 CLAW_LM_TRACE_DIR="$EVIDENCE_DIR/intermediates" \
  python3 verification/numpy_reference.py "$EVIDENCE_DIR/logits.json" "$EVIDENCE_DIR/numerics.json"
python3 verification/template_reference.py "$CARGO_TARGET_DIR/release/examples/infer"
python3 verification/http_smoke.py http://127.0.0.1:8093 "$EVIDENCE_DIR/http.json"
```

The first-forward `.f32` captures are opt-in diagnostics. They never belong in
Git. The NumPy verifier needs NumPy; the template verifier needs Jinja2. Neither
imports Transformers, PyTorch or LibTorch. The runtime requires neither Python
nor those development-only packages.

## Limits

The independent numerical oracle currently covers one short real prompt.
Longer instruction-following and code-generation outputs are not yet validated
against an upstream implementation. Observed answer errors must not be treated
as proven checkpoint limitations until broader reference checks distinguish
model behavior from possible porting errors.


- Actual GPU execution was tested on RTX 3060 only. sm_120 compiles; Blackwell
  hardware validation remains outstanding. RTX 3070-family support shares sm_86
  but its lower VRAM can require a shorter context.
- The numerical gate covers one real prompt and reduced synthetic cases, not
  every prompt, context position or shape. Wider model-quality evaluation remains
  a future extension. Short smoke timings are not a throughput benchmark or a
  claim that diffusion/self-speculation is faster than AR on every request.
- Initial Pi tool-cycle checks used isolated settings and a short explicit
  system prompt. The later budget smoke check also covers loaded workspace
  instructions, with extension/MCP limits described above. Hermes-specific
  integration is unverified; unsupported sampling options fail explicitly.
- Optional LoRA, quantization, multimodal input and nonzero-temperature sampling
  remain outside the accepted initial milestone.
- PDD's automated architecture application timed out after durable intent was
  recorded. Prompt sources and implementation were maintained directly; no
  successful automated sync or generated story-contract claim is made.

Before publication, run the repository's privacy checker against the full
candidate index and inspect the candidate diff. A scanner alone does not replace
review. The delivery handoff records the actual scan result.
