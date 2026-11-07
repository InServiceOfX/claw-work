# Feasibility: WSP-Style Experiments on Apple Silicon

Generic hardware class: Apple Silicon laptop with 32 GiB unified memory.
Example viable model: small unquantised bf16 dense checkpoint (~0.6B params,
Apache-2.0). Quantised MLX/GGUF checkpoints are inference-only targets.

All numbers are arithmetic estimates, **not measurements**.

## Original proposal vs. implementable scope

| Element | Sparks post (S1/S2) | What is implementable locally |
|---|---|---|
| Editor | GPT-6 Astra, reasoning effort max, 8–20 h per run | Any frontier LLM or coding agent. Results depend on the editor |
| Target | Qwen3.5-0.8B (bf16 text part) | Any small unquantised bf16 dense checkpoint already cached |
| Tools | CPU Python kernel, `save_edit`, forward-pass server | Same pattern: numpy/safetensors for weights, metered forward pass |
| Eval | vLLM on H200, 32K tokens, multi-sample | transformers+MPS or mlx-lm with smaller token cap and synthetic tasks |
| Tested claim | "LLM reasoning can program weights" | (A) Toy WSP faithfully; (B) Agent-derived rank-1 edit vs decoding baselines |

## Recommended experiments

### Track A: Toy WSP (recommended first)

- **Task**: Hand-written transformer or MLP for bounded algorithmic tasks (parity
  ≤10 bits, 3-digit addition, sorting 5 items, counting).
- **Rules**: No optimizer, no pretrained weights, no side-channel computation.
- **Variant**: Plant a flaw in a hand-written net (silence an input, shift a
  threshold), ask the editor to audit and repair it blind.
- **Controls**: Exhaustive evaluation over bounded domain, fresh seed, simple
  per-unit statistic baseline, label/permutation control for read claims.
- **Cost**: Seconds of CPU per eval. Faithful to WSP and reproducible.

### Track B: Degeneration edit on copied 0.6B checkpoint

**Gate (measure first):**
1. With vendor-recommended sampling (temperature 0.6, top-p 0.95, top-k 20),
   generate on ~100–200 locally generated synthetic problems.
2. Measure accuracy, token-limit hit rate, and verbatim n-gram loop detector.
3. **If loops are rare, stop.** This loop-removal workflow has not demonstrated
   novel ability addition. No reliable benefit is expected without a measured
   target failure.

**Decoding baselines (required):**
- presence_penalty 1.5 (Qwen recommendation)
- repetition_penalty 1.1 (audit baseline)

**Diagnosis (no training):**
1. Build 4–16 contrastive pairs (productive vs looping continuation).
2. Take residual-stream activation differences per layer.
3. Pick direction and layer by pre-registered rule.

**Runtime steering arm:**
- Apply direction as activation intervention first. Reversible; shows whether
  direction matters before any weight change.

**Weight edit on a copy:**
1. Normalise direction u to unit L2 norm (reject if zero or non-finite).
2. Verify tensor orientation: formula `W' = W − α·û(ûᵀW)` assumes W is [d_out, d_in]
   with û in output residual space. Transpose appropriately if needed.
3. Rank-1 orthogonalisation on selected `mlp.down_proj` (optionally `o_proj`)
   columns of 1–2 layers. No generic slicing without concrete shape validation.
4. Compute in fp32; verify all values finite.
5. Cast to bf16; verify finite after cast.
6. Count entries that changed after rounding.
7. α sweep on practice split only, never held-out.
8. Never touch embeddings, LM head, or final norm.

**Evaluation:**
- Held-out synthetic test split.
- Paired base-vs-edit comparison (McNemar or bootstrap CI).
- Capability-regression mini-suite.

**Reporting:**
- All runs including failures.
- α sweep, edit norm ratio ‖ΔW‖/‖W‖.
- sha256 of base and edited tensors.

### Unsuitable or deferred

- **Quantised checkpoints** (MLX 4-bit/8-bit, GGUF Q4–Q8): Quantisation may
  distort small edits in ways not characterised here. This workflow uses
  unquantised checkpoints only.
- **Dense 4B+ edits**: No unquantised copy without download.
- **MoE models**: Routing layer adds complexity.
- **Cross-family merges**: Lineage unverified, architectures differ.
- **Capability addition**: S1 negative result (Gemma 3 270M) shows this workflow
  has not demonstrated novel ability addition.
- **Safety-behaviour removal**: Excluded by design.

## Memory arithmetic (estimates)

| Item | Estimate |
|---|---|
| 0.6B bf16 weights | ~1.5 GB on disk |
| fp32 copy of one MLP down_proj | ~12.6 MB if intermediate = 3072 |
| KV cache (28 layers, 8 heads, head_dim 128, bf16) | ~112 KiB/token/sequence |
| 8 sequences × 8K tokens | ~7 GiB KV cache |

## Software and licences

| Component | Licence |
|---|---|
| Small Qwen3 bf16 checkpoint | Apache-2.0 (verify LICENSE in snapshot) |
| Sparks logs/diffs | CC BY 4.0 / MIT per S3 card |
| Singh toy-WSP code | MIT |
| Audit code (S4) | No licence detected (read only) |
| MLX / mlx-lm | MIT |
| PyTorch | BSD-3-Clause |
| transformers, safetensors | Apache-2.0 |

## Rollback and reproducibility rules

1. Never write into HF cache or source directory.
2. Create fresh non-existing output directory; verify resolved path not under
   cache or source. No symlinks/hardlinks to source for writable files.
3. Store each edit as (tensor name, indices/rank-1 factors, α, dtype, base sha256
   for edited tensors, edited sha256). Do not claim full-checkpoint hashes unless
   actually computed.
4. Write atomic: temp file then rename.
5. Manifest all outputs with path and sha256.
6. Rollback = use original base (never modified). Retain artifacts.
7. No auto-delete unless every file verified in run manifest AND user approves.
8. Pre-register gate thresholds, direction-selection rule, α grid, eval seeds.
9. Report every run.
10. Synthetic, locally generated tasks only.
11. Ask before model downloads, package installs, heavy inference, or modifications.
