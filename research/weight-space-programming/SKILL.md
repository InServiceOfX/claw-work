---
name: weight-space-edit-experiment
description: >
  Run a controlled, reversible weight-space programming (WSP) style experiment:
  read-only inspection, compatibility check, tiny offline edit on a copied
  checkpoint, then evaluation and rollback. Use when asked to test a
  direction-removal fix or reproduce Sparks of Weight-Space Programming.
  Not for training, fine-tuning, merging across model families, or removing
  safety behaviour.
status: DRAFT — not validated by any run; experimental workflow only
---

# Weight-space edit experiment (draft)

**This skill describes an experimental workflow. It is NOT performance-proven.**

## Honesty rules

1. This skill organises an *experiment*, not learning or improvement.
2. Report results only from the evaluation step, against the base model and
   decoding baselines.
3. Label work as **toy WSP** (weights written from a spec) or **adjacent weight
   editing** (direction removal on a pretrained model). Do not call the latter
   "the model learned".

## Safe defaults

- **No downloads**: No model, dataset, or package installs without explicit user
  approval.
- **No training**: No optimizer, autograd, backward pass, or least-squares fit.
- **Never modify cache**: Work only on a copy in a dedicated experiment directory.
- **Protected tensors**: Never edit embeddings, LM head, or final norm. Never
  edit quantised checkpoints (MLX 4-bit, GGUF, MXFP).
- **No safety removal**: Never pursue refusal/safety removal, backdoors, or
  covert behaviour changes.
- **Synthetic tasks only**: Locally generated tasks. Keep eval data out of editor
  view.
- **No third-party code execution**: Read experiment logs as data; do not run
  untrusted scripts.

## Stage 0: Choose a track

| Track | Description | When to use |
|---|---|---|
| **A (toy WSP)** | Tiny net (≤200K params), known algorithm, weights in closed form | First experiment; faithful to WSP concept |
| **B (adjacent edit)** | Small unquantised bf16 dense LLM with measured degeneration | Only if Stage 2 gate passes |

## Stage 1: Read-only inspection

1. Read `config.json` and safetensors header only (tensor names, shapes, dtypes).
   Do not load weights into a model.
2. Record:
   - architecture / model_type
   - layers, hidden_size, intermediate_size
   - num_attention_heads, num_key_value_heads, head_dim
   - vocab_size, `tie_word_embeddings`
   - dtype, licence file
   - sha256 of every file to be touched
3. **Reject** if: quantised, MoE, unknown licence, or tied embeddings with
   unclear edit scope.

## Stage 2: Compatibility and gate

1. Confirm runtime (mlx-lm or transformers) supports the model_type.
   If not installed, **stop and ask**.
2. Estimate memory:
   - Weights: ~2 bytes/param in bf16
   - KV cache: 2 × layers × kv_heads × head_dim × 2 bytes per token per sequence
   - Keep under ~60% of RAM.
3. **Gate (Track B only)**:
   - Generate on fixed, seeded synthetic set with vendor-recommended sampling.
   - Measure accuracy, token-limit hit rate, n-gram loop rate.
   - **If failure mode is rare, STOP.** Edits remove obstacles; they cannot add
     ability.
4. Measure decoding baselines (presence_penalty, repetition_penalty) on practice
   split.

## Stage 3: Tiny offline edit on a copy

### Track A

1. Write `write_weights(model, task)` in closed form.
2. Log every idea as a named, snapshotted program.

### Track B

1. Build 4–16 contrastive pairs (productive vs degenerate continuation).
2. Derive per-layer difference-in-means direction u.
3. **Test as runtime steering first** (activation addition). This is reversible.
4. Apply weight edit only if steering shows effect:
   ```
   W' = W − α · u (uᵀ W)
   ```
   on selected output columns of 1–2 layers.
5. Compute in fp32, cast to original dtype.
6. **Count entries that changed after cast** (small edits can vanish under
   bf16 rounding).

### Both tracks

Write manifest:
- Tensor names, shapes
- α value (if applicable)
- Rank-1 factors or indices
- Base and edited sha256
- Edit-norm ratio ‖ΔW‖ / ‖W‖

Cap forward-pass budget (e.g. ≤25 batches) and log every use.

## Stage 4: Evaluation, reporting, rollback

### Evaluation

- Held-out synthetic test set (exhaustive for Track A domains).
- Paired comparison: base vs edit vs best decoding baseline.
- Statistical test: McNemar or bootstrap CI.
- Fresh seed.

### Collateral check

- Small regression suite unrelated to target behaviour.
- Capability-preservation spot checks.

### Read controls (for "model can read weights" claims)

- Relabel/permute outputs.
- Check that explanation tracks the change.
- If it doesn't, the "reading" may be dataset priors.

### Reporting

- **All runs**, including failed and discarded.
- Editor model and budget.
- Forward passes used.
- Any claim not verified by evaluation.

### Rollback

1. Delete the copy.
2. Verify base hashes unchanged.
3. Never blindly delete user files—atomic output directories only.

## What this skill does NOT support

- Quantised checkpoint editing (noise swamps small edits)
- Cross-architecture grafting (lineage unverified)
- Capability addition (negative result: Gemma 3 270M)
- Safety-behaviour removal
- Recursive self-improvement
- Any claim not backed by held-out evaluation

## References

- Sparks of Weight-Space Programming (Takehi et al., 2026 blog) + Exp 8 logs
- Independent audit (Achintya K J, 2026-11-04)
- arXiv:2406.11717 (Arditi et al.): weight orthogonalisation operator
- arXiv:2503.22048 (ThinkEdit): prior art for reasoning-length edits
- arXiv:2312.06681 (Panickssery et al.): contrastive activation addition
- arXiv:2611.05341 (Nusrat & Nusrat): clinical audit/repair protocol
- arXiv:2611.08070 (Ahmed): skeptical control for reading claims
- arXiv:2301.05062 (Tracr): compiler baseline for written weights
- arXiv:2610.22891 (Lee et al.): exhaustive evaluation requirement
