# Weight-space programming: primary-source audit and RTX 3060 experiment

Research dates: 2026-11-08–09. This extends the survey merged in PR #8. The Rust
implementation, exact checkpoint reconstruction and approved GPU comparison are
complete. See the [verification record](../../Rust/weight-space/docs/VERIFICATION.md)
for measured results and limits.

## Findings

Directly changing a small open model is feasible. The strongest first experiment
is **reconstructing an existing, precisely specified edit**, rather than expecting
an agent to invent an improvement from arbitrary weights. The proposed target is
Qwen3.5-0.8B, Apache-2.0, with unquantized BF16 weights. An upstream BF16 GGUF is
about 1.56 GB; this is a plausible RTX 3060 12 GiB inference workload, subject to
measuring runtime allocations. Model size alone does not prove context capacity.
[Vendor configuration](https://huggingface.co/Qwen/Qwen3.5-0.8B/blob/2fc06364715b967f1860aea9cf38778875588b17/config.json),
[upstream GGUF](https://huggingface.co/ggml-org/Qwen3.5-0.8B-GGUF/tree/807dfd9a6fadbf703207aac3bfb0b2b3040d077c).

The original announcement describes agent-guided weight editing with tools and
budgeted inference for its language-model experiment. It reports reduced
reasoning loops and higher math scores, while its Gemma experiment did not show
a comparable gain. This motivates testing recovery from a specific failure, not
claiming that sparse weight edits generally add reasoning ability. Our proposed
run uses a different inference backend and a small synthetic suite, so it cannot
replicate the published benchmark scores.
[Original announcement](https://rikiyat.github.io/blog/weight-space-programming/).

There are three distinct questions:

1. Can software modify a checkpoint correctly? Exact hashes and an audit of
   unchanged bytes answer this.
2. Does a specified edit change behavior usefully? Paired inference and decoding
   controls answer this for the tested workload.
3. Can an agent discover a new useful edit? That requires a separate discovery
   experiment with hidden evaluation data. Replaying a published patch does not
   answer it.

## Following Wikipedia's references

[Wikipedia](https://en.wikipedia.org/wiki/Weight-space_programming) was used as a
discovery index. Method claims below were checked against primary papers or the
authors' released artifacts. The reception links are commentary, not an
independent experimental result.

### Eilertsen et al., Classifying the Classifier (2020)

[arXiv:2002.05688](https://arxiv.org/abs/2002.05688), especially §§3–6 and the
limitations in §6.1. The authors train meta-classifiers to infer training setup
from neural-network parameters, using a large collection of small CNNs. This
demonstrates that weights contain statistical evidence about training; it does
not demonstrate that an untuned language model understands a network's complete
algorithm. Their experiments are explicitly limited to small vanilla CNNs.

**Consequence:** include simple weight statistics as a baseline for any future
claim that the editor can infer behavior. Never equate recognition of training
provenance with tracing the computation.

### Unterthiner et al., Predicting Neural Network Accuracy from Weights (2020/2021)

[arXiv:2002.11448v4](https://arxiv.org/abs/2002.11448v4), §§2, 4.2 and 5. The
predictors are trained on collections of networks and use raw parameters or
summary statistics. Generalization depends on how training and evaluation
networks are split; sharing hyperparameter configurations can make prediction
look easier than it is. Their reported transfer results are empirical, not a
universal way to certify any model's accuracy from its weights.

**Consequence:** a weight-only score is a diagnostic, never a replacement for
running held-out cases. Split discovery data by problem family where possible,
not only by random seed.

### Lindner et al., Tracr (2023)

[arXiv:2301.05062v5](https://arxiv.org/abs/2301.05062v5), §§2–4 and Appendix A.2.
Tracr compiles a restricted program language into decoder-transformer weights,
providing networks with known mechanisms for interpretability research. Its
expressivity and efficiency restrictions matter: a compiled algorithmic model
is not automatically representative of a trained language model.

**Consequence:** a small Rust network whose weights implement an explicit
algorithm is a useful independent positive control. Exhaustive testing over its
declared finite input domain is stronger than a handful of successful examples.
It must be reported separately from pretrained-model editing.

### Meng et al., ROME (2022)

[arXiv:2202.05262v5](https://arxiv.org/abs/2202.05262v5), §3.1, Appendix A and E.5.
ROME interprets an MLP projection as an associative memory and uses a constrained
rank-one update to install a key/value association while approximately preserving
other associations. An important boundary: the final matrix update is closed
form, but the original pipeline **optimizes the desired value vector**. It is
not an entirely gradient-free procedure just because its last step is algebraic.

**Consequence:** do not claim a faithful gradient-free ROME implementation unless
the value-selection step is also specified and tested. For the first experiment,
replay the published loop edit instead of silently implementing a different
fact-writing method.

### The primary WSP release

The original experiment's code, manifest and sparse patch were inspected at
Git commit `4d09ba6d6a7e891dd8063974dfe81b3d78e3494f`. This is artifact inspection,
not a local reproduction of its benchmark. The release separates practice
problems from evaluation and forbids editing embeddings, the output head and
final normalization. Its larger-model run used extensive tool access; the
released patch can be applied without that discovery process.
[Experiment README and artifact](https://github.com/RikiyaT/lm_understand-logs/tree/4d09ba6d6a7e891dd8063974dfe81b3d78e3494f/exp8_math).

## Related papers and what they change in the design

| Paper | Mechanism and limit | Experimental consequence |
|---|---|---|
| [Contrastive Activation Addition, 2312.06681v4](https://arxiv.org/abs/2312.06681v4), §3 | Mean activation differences from paired examples; runtime intervention, not a permanent weight edit. Prompt/answer-token confounds require care. | A future independently derived direction needs matched pairs and a runtime intervention control before permanent editing. |
| [Refusal direction, 2406.11717v3](https://arxiv.org/abs/2406.11717v3), §4.1 | Removing one residual direction can be encoded in residual-writing weights. It establishes the operator family, not the semantic meaning of every candidate direction. | Use the linear algebra for the loop experiment; its original application is outside this experiment's scope. |
| [ThinkEdit, 2503.22048v4](https://arxiv.org/abs/2503.22048v4), §§3–4 | Identifies heads associated with **short** reasoning and projects their output weights. Removing a direction can increase useful reasoning length. | Do not assume shorter output is always better. Score final answers and termination separately. |
| [Reasoning steering vectors, 2506.18167v4](https://arxiv.org/abs/2506.18167v4), §§3–4 | Causal steering of behaviors such as backtracking; behavior selection and activation location matter. | Do not label all backtracking or uncertainty as pathological repetition. |
| [MEMIT, 2210.07229v2](https://arxiv.org/abs/2210.07229v2), §§4.2–4.3 | Derives a covariance-regularized batch update and distributes it across causally relevant MLP layers. Its target hidden states are optimized, and downstream keys are recollected after updates. | A closed-form matrix update does not make the entire pipeline gradient-free. Defer batch fact editing until independent factual/locality evaluation exists. |
| [Unified model-editing framework / EMMET, 2403.14236v2](https://arxiv.org/abs/2403.14236v2), §§3–7 | Connects equality-constrained and least-squares preservation/memorization objectives. Closed-form updates still inherit representation and distribution assumptions. | Separate exact local algebra from claims of preserved global behavior. |
| [AlphaEdit, 2410.02355v3](https://arxiv.org/abs/2410.02355v3), §§3.1–3.3 | Projects an edit into a preservation subspace, using a thresholded covariance eigenspace in practice. Exact null-space algebra does not imply exact behavioral preservation with approximate statistics. | An unchanged-weight audit proves edit locality in parameter space; a regression suite is still necessary. |
| [Task Arithmetic, 2212.04089v3](https://arxiv.org/abs/2212.04089v3), §2 | Manipulates deltas from compatible fine-tunes. It needs trained task vectors and compatible lineage. | Not a substitute for direct gradient-free discovery, and no arbitrary cross-family merging. |
| [Git Re-Basin, 2209.04836v6](https://arxiv.org/abs/2209.04836v6), §§2–3 | Hidden-unit permutation symmetry makes raw coordinate comparisons ambiguous across independent training runs. | Never transfer sparse indices or a direction to a different checkpoint just because shapes match. |
| [Clinical network audit, 2611.05341v1](https://arxiv.org/abs/2611.05341v1), §§II–IV | Small-network repairs can work; simple statistics detect many of the same defects, and healthy-network/trigger results prevent certification claims. | Require strong negative controls and exhaustive tests for the toy track; this is not medical deployment evidence. |
| [Skeptical weight-reading control, 2611.08070v1](https://arxiv.org/abs/2611.08070v1), §§4–7 | Label permutation exposes semantic claims driven by dataset priors in the tested model and setup. | A future weight-reading experiment needs relabel/permutation controls; the first experiment makes no semantic weight-reading claim. |

Reading scope: methods and limitations were inspected for the Wikipedia
foundations, CAA, ThinkEdit, reasoning steering, AlphaEdit, EMMET, and the two
November follow-ups. Task Arithmetic and Git Re-Basin were inspected for their
compatibility boundary. MEMIT's single-layer derivation and multi-layer procedure
were inspected in the continuation; no MEMIT implementation was run.

### Additional November follow-ups

[WeightLoop, 2611.07319v1](https://arxiv.org/abs/2611.07319v1), §§2–3, separates
one-time agent construction of an editing program from repeated numerical
execution using measured feedback. The neural application changes a small
functional readout while preserving the encoder. Candidate changes are selected
using support training/validation, and each accepted patch is reconstructed from
the original parameters rather than accumulated. Its experiments are offline
replays, not new laboratory interventions or general language-model improvements.
This is useful prior art for a future bounded discovery tool, but it still uses
supervised evidence and candidate selection; absence of gradients does not mean
absence of optimization or feedback fitting.

[Restoration diffusion conjectures, 2611.04268v1](https://arxiv.org/abs/2611.04268v1),
§§6–8, offers budgeted editing pseudocode and discusses seed-dependent metrics,
perceptual tradeoffs and out-of-distribution regressions. It explicitly presents
its numerical propositions as hypotheses. Its illustrative quality thresholds,
speed claims and recursive-improvement discussion must not be treated as measured
evidence that diffusion weights can already be improved this way. We therefore
do not substitute this proposal for a reproduced language-model experiment.

## Why a sparse projection changes generation

For an MLP output matrix in the convention `W: [d_out, d_in]`, its contribution
to the residual stream is `y = W h`. Let `u` be a unit vector in output space and
let diagonal matrix `M` select input neurons. A targeted edit is

```text
W' = W - alpha * u * (u^T W M)
y' = y - alpha * u * (u^T W M h)
```

Thus the intervention depends on the current activation `h`; it is not a fixed
logit bias or an instruction added to the prompt. At `alpha = 1`, selected
columns have zero component along `u` in exact arithmetic. BF16 rounding leaves
a residual. Neither orthogonality nor a small parameter count proves a useful
semantic effect. For a row-vector weight convention, transpose the operation.
The projection family is documented in
[ThinkEdit §4.3](https://arxiv.org/html/2503.22048v4#S4.SS3).

The proposed first implementation replays exact published BF16 values, rather
than attempting to reconstruct them from an approximate direction. A reusable
Rust FP32 projection routine can be tested separately for future discovery work.

## Exact artifact compatibility

Base source revision: `Qwen/Qwen3.5-0.8B` at
`2fc06364715b967f1860aea9cf38778875588b17`. The upstream BF16 GGUF repository at
`807dfd9a6fadbf703207aac3bfb0b2b3040d077c` names that same source in `.src_sha`.
Its conversion log reports the two target matrices as BF16 with GGUF dimensions
`[3584, 1024]`; the original tensor's shape is `[1024, 3584]`. The fastest index
is the 3584-dimensional input axis in both layouts. **This mapping must still
pass the complete base-tensor hashes before any edit.**
[Source provenance](https://huggingface.co/ggml-org/Qwen3.5-0.8B-GGUF/blob/807dfd9a6fadbf703207aac3bfb0b2b3040d077c/.src_sha),
[conversion log](https://huggingface.co/ggml-org/Qwen3.5-0.8B-GGUF/blob/807dfd9a6fadbf703207aac3bfb0b2b3040d077c/convert.log).

| Layer | GGUF tensor | Base tensor SHA-256 | Expected edited SHA-256 |
|---|---|---|---|
| 15 | `blk.15.ffn_down.weight` | `ef60d2c687bab19464dca9c8bfd7777e46ad226df0c9e25643b0bab04b5a1cca` | `82e2d733ea64feaeb4af93a1e065641d3df98b494a8b7576ef2f52e90fe2ee53` |
| 19 | `blk.19.ffn_down.weight` | `638eedeb701b94aa861f4438e615c07ec6c60ee703db1bfee2de577bb6498c3e` | `8ef3c61bc36af113ad58e89dbb0a4a14dcba5a45cb750c15ab515c82ddd12eb2` |

The public `p50c/edit/edit.npz` contains four one-dimensional little-endian NPY
arrays: signed 32-bit indices and unsigned 16-bit BF16 replacement bits for each
tensor. Layer 15 changes 65,260 entries; layer 19 changes 64,928. The archive is
approximately 379 KiB. The patch data is CC BY 4.0; retain attribution if it is
redistributed. Do not copy the surrounding agent transcripts into this repo.

## Proposed implementation and evidence

New Rust project: `Rust/weight-space`. Inputs are explicit local paths. Proposed
modules: bounded GGUF inspection, bounded NPY/NPZ patch reader, hash-checked
copy-only edit and audit, projection algebra, and local evaluation CLI. CUDA
inference uses the existing llama.cpp backend, not a new Rust inference engine.
No Python, PyTorch, Transformers, optimizer or autograd is needed by the proposed
runtime pipeline.

Tests must cover malformed/truncated files, overflow and bounds, wrong hashes,
unsupported tensor types, invalid sparse indices, non-finite BF16 values,
existing outputs and source aliases, source preservation, and exact changes.
The independent file audit must reject every byte difference outside the
approved tensor entries, including header/tokenizer changes.

The GPU experiment will use a pinned existing runtime, an isolated loopback port,
and one model at a time. It must leave other servers and application model
settings alone. Record exact runtime/image and model hashes, sampling, seeds,
token cap, prompt template, elapsed time, tokens, termination reason, final
answer accuracy and a precisely defined repetition measure. Keep raw local run
data and model files outside the Git tree.

Pre-register base and edited arms with the same vendor-style sampling, plus a
base repetition-penalty arm. Include non-thinking as a diagnostic baseline if
the runtime exposes it consistently. Use fixed synthetic math and unrelated
instruction probes. Do not tune the published patch on the held-out cases.
Report paired wins/losses and uncertainty, and retain failures. A short-token
pilot cannot establish the effect at the original 32,768-token budget.

**Measured outcome:** the RTX 3060 ran the unquantized BF16 model at about
2.1 GiB observed allocation. At an 8,192-token cap, the original completed 11/24
synthetic cases correctly, the published weight edit 22/24, and the decoding
control 22/24. The edit recovered 11 answers without regressions; the control
recovered 12 and regressed one. On the fixed six-case 32,768-token confirmation,
scores were 4/6, 6/6 and 6/6. Both base failures remained unfinished at the larger
cap. All 90 outputs and matching requests passed independent validation.

This establishes a reproducible direct-weight intervention and completion
recovery on the tested workload. It does not establish a weight-editing advantage
over the decoding control, independent edit discovery, or the original paper's
MATH-500/GSM8K gains. The source model remained byte-identical and experiment
servers were stopped. Detailed checks, uncertainty and limitations are in the
verification record.
