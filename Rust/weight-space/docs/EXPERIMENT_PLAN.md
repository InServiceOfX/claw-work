# Experiment protocol

Scope approved 2026-11-09. Primary protocol frozen after the practice pilot and
before primary inference. The fixed published patch is never selected or tuned
using the evaluation cases. See VERIFICATION.md for execution status.

## Question

Does the published `p50c` Qwen3.5-0.8B BF16 edit reduce repetitive or unfinished
reasoning and recover correct answers on a small synthetic suite using a local
RTX 3060 and llama.cpp? This tests portability and practical effect of a known
edit. It does not test independent edit discovery.

## Order and gates

1. Unit/acceptance tests establish parsing, exact patch reconstruction, source
   preservation, output collision rejection and full-file difference audit.
2. Verify the model licence and revision; fetch pinned BF16 GGUF and patch into
   a fresh local data directory outside Git. Record whole-file SHA-256 values.
3. Verify the two base tensor hashes. On mismatch, stop without generating an
   edited checkpoint. Do not relax hashes to accommodate a different model.
4. Create the edited copy and verify published edited hashes. Independently
   compare source and output and reject any unexpected byte difference.
5. Confirm the pinned existing runtime loads the base on the RTX 3060, with all
   intended layers offloaded. Use an isolated server and explicit loopback port.
6. Run a small practice pilot to establish throughput and viable token/context
   bounds. Do not select or alter edit values based on the pilot.
7. Freeze the measured token cap and prompt list, then execute all evaluation
   arms. An infrastructure failure is retained as a failed run, not scored as a
   wrong model answer or silently retried until favorable.

## Arms

| Arm | Weights | Sampling |
|---|---|---|
| Base | Original | temperature 0.6, top-p 0.95, top-k 20, presence penalty 1.5 |
| Edit | Published sparse patch | Same as Base |
| Decoding control | Original | Same as Base plus repetition penalty 1.1 |

Use identical problem order and per-problem seeds for every arm. Identical seeds
do not imply identical sampled token draws across changed distributions. Run one
server at a time. Confirm the runtime actually accepts and reports the sampling
parameters rather than ignoring unknown API keys. Keep thinking enabled in all
three primary arms. A non-thinking arm may be reported separately as a diagnostic.

Start with a 4,096-token pilot. Prefer 8,192 tokens for the main small run if
throughput permits. If the pilot shows truncation dominates, include a separately
labeled 32,768-token confirmation on a fixed subset. Report each budget as a
different experiment. Never compare base at a shorter cap to edit at a longer
cap, or present these small runs as MATH-500/GSM8K replications.

## Inputs and grading

Use deterministic synthetic integer arithmetic, two-step word problems and
linear equations, plus unrelated instruction-following probes. The expected
answer is computed from the generator's input values, not from model output.
Generate practice and evaluation cases from distinct seeds, with no duplicate
problems. Freeze the case file and hash before primary inference.

Ask math prompts to put their final answer inside `\boxed{}`. Score the last
complete boxed answer in the final answer section after `</think>`. A correct
number appearing only inside reasoning does not count. Incomplete thinking,
missing final answers and token-cap truncation must remain visible. Restrict the
generator to integer answers so an explicit strict integer parser is sufficient;
do not pretend it is a general symbolic-math verifier.

Report accuracy and termination as separate outcomes. Define the repetition
heuristic before running it, using generated token IDs where available; record
its n-gram length, threshold and minimum output length. It is a diagnostic rather
than ground truth. Long, useful reasoning is not automatically a loop.

For unrelated probes, use explicit machine-checkable instructions such as exact
strings or bounded JSON fields. Retain raw outputs for qualitative inspection
outside Git. Small probe counts cannot prove broad capability preservation.

## Report

Record every attempted arm and case, model/patch/runtime hashes, request settings,
prompt and case-file hashes, tokens, finish reason, time and errors. Summarize
paired rescued/regressed answers, accuracy, cap hits, thinking completion and
repetition diagnostics. Provide uncertainty suitable for paired binary outcomes;
do not infer significance from percentages alone on a tiny suite.

Parameter locality is established by byte audit. Behavioral preservation is
estimated only by the evaluated probes. The experiment may legitimately end
with no demonstrated benefit under the chosen workload or token budget.

Rollback means selecting the untouched base. Retain artifacts; do not alter
normal assistant/provider settings or replace an existing server.

## Frozen run choices (2026-11-09)

- Practice seed 1313, two cases, cap 4,096. Corrected pilot: one finished correct
  answer, one cap hit, no infrastructure errors. Its outputs are excluded from
  primary results. An earlier pilot had two rejected HTTP requests because this
  backend rejects a negative history-window sentinel; that failure is retained.
- Primary seed 78323, 24 cases: 20 integer-math cases and four exact-string
  instruction probes. Per-case inference seed is 20261109 plus case position.
- Primary cap 8,192, context 40,960, thinking enabled, arms base/edit/decoding.
- Fixed confirmation: first six cases from seed 78323, same per-case inference
  seeds and sampling, cap 32,768, context 40,960. Run all three arms; report
  separately from the primary budget. This subset is fixed before primary runs.
- Case-file SHA-256: primary
  `ac6100a22c720bd481996e5df395a49b45e592b1224939faa4cee12c552bf51b`;
  confirmation `ed6a4f226409f311eee20ca4fd7e86b0f9c5aa5c81a6cc7dd179dace9fdaa3c4`.
- Rust binary SHA-256:
  `253ae397532a07019d914ed6501cd647557cb367e302a795cdfc3c84d0c48fbf`.
- Existing llama.cpp CUDA image:
  `sha256:68347eb344cc9a0c40267f1d07f7ad4313fba66af16df01d11a84a85b0e94df4`,
  build 10603, commit `c060ca974`. RTX 3060 12 GiB selected explicitly.
- Repetition/presence history is an explicit nonnegative 40,960-token window;
  the returned window, sampling parameters and seed are verified. This replaces
  the rejected sentinel in the failed pilot. Native backend penalty semantics
  may include prompt history and are not asserted identical to vLLM.
- One isolated server/model at a time. Exact synthetic-case generation and
  strict grading rules are versioned in the Rust source and tests.
