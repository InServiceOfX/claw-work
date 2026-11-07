---
name: weight-space-edit-experiment
description: >
  Plan and inspect a controlled, reversible weight-space programming (WSP) style
  experiment. Read-only inspection, compatibility check, experiment design with
  explicit user approval gates before any heavy operations. Use when asked to
  design a direction-removal fix or reproduce Sparks of Weight-Space Programming.
  Not for training, fine-tuning, merging across model families, or removing
  safety behaviour.
status: DRAFT — experimental workflow, not performance-proven
---

# Weight-space edit experiment (draft)

**This skill describes an experimental planning workflow. It is NOT performance-proven.**

Planning and inspection are safe defaults. Any model download, package install,
heavy inference, or weight modification requires explicit user approval.

## Honesty rules

1. This skill organises an *experiment*, not learning or improvement.
2. Report results only from the evaluation step, against the base model and
   decoding baselines.
3. Label work as **toy WSP** (weights written from a spec) or **adjacent weight
   editing** (direction removal on a pretrained model). Do not call the latter
   "the model learned".

## Safe defaults

- **Ask before heavy operations**: Model downloads, package installs, heavy
  inference runs, and weight modifications all require explicit user approval.
- **Planning and inspection are safe**: Reading documentation, inspecting config
  files, and designing experiments do not require approval.
- **No training**: No optimizer, autograd, backward pass, or least-squares fit.
- **Never modify source/cache**: Work only on explicit copies in fresh output
  directories.
- **Protected tensors**: Never edit embeddings, LM head, or final norm.
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

### Track A specifics

- Uses stdlib Python or numpy if available; no pretrained model loading required.
- No config.json, safetensors, or ML runtime needed.
- Hand-specify architecture and weights directly.

### Track B specifics

- Requires reading model config and safetensors metadata.
- Requires ML runtime (transformers, mlx-lm, etc.)—**ask before installing**.
- Requires model files—**ask before downloading**.

## Stage 1: Read-only inspection

### Track A

- Define architecture in code: layer sizes, activations, task.
- No external model files needed.

### Track B

1. Read `config.json` and safetensors header only (tensor names, shapes, dtypes).
   Do not load weights into a model yet.
2. Record:
   - architecture / model_type
   - num_hidden_layers, hidden_size, intermediate_size
   - num_attention_heads, num_key_value_heads, head_dim
   - vocab_size, `tie_word_embeddings`
   - dtype, licence file
3. **Reject** if: MoE, unknown licence, or tied embeddings with unclear edit scope.
4. **Quantised checkpoints** (MLX 4-bit/8-bit, GGUF, MXFP): This workflow does not
   support editing quantised weights. Quantisation may distort small edits in
   ways not characterised here. Use unquantised checkpoints only.

## Stage 2: Compatibility and gate (Track B only)

1. Confirm runtime supports the model_type. **Ask before installing** if missing.
2. Estimate memory:
   - Weights: ~2 bytes/param in bf16
   - KV cache: 2 × layers × kv_heads × head_dim × 2 bytes per token per sequence
   - Keep under ~60% of available RAM.
3. **Gate (measure failure mode first)**:
   - **Ask before running inference.** This is a heavy operation.
   - Generate on fixed, seeded synthetic set with vendor-recommended sampling.
   - Measure accuracy, token-limit hit rate, n-gram loop rate.
   - **If failure mode is rare, STOP.** This loop-removal workflow has not
     demonstrated novel ability addition. No reliable benefit is expected
     without a measured target failure.
4. Measure decoding baselines (presence_penalty, repetition_penalty) on practice
   split only—**never on held-out data**.

## Stage 3: Experiment design

### Track A

1. Write `write_weights(model, task)` in closed form.
2. Log every idea as a named, snapshotted program.

### Track B

**Ask before any weight modification.**

1. Build 4–16 contrastive pairs (productive vs degenerate continuation).
2. Derive per-layer difference-in-means direction u.
3. **Normalise u to unit L2 norm.** Reject if zero or non-finite.
4. **Test as runtime steering first** (activation addition). This is reversible.
5. Weight edit formula (apply only to a copy, with user approval):

   ```
   W' = W − α · û (ûᵀ W)
   ```

   where û is the unit-norm direction vector.

   **Architecture verification required:**
   - Confirm tensor orientation: formula assumes W has shape [d_out, d_in] with
     û in the output (residual-stream) space.
   - If W is [d_in, d_out], transpose appropriately: `W' = W − α · W (û ûᵀ)`.
   - No generic column slicing without concrete tensor shape, orientation, and
     mask validation for the specific architecture.

6. Compute in fp32.
7. **Finite checks**: Verify all values finite before and after dtype cast.
8. Cast to original dtype.
9. **Count entries that changed after cast** (small edits can vanish under
   bf16/fp16 rounding).
10. α sweep on **practice split only**, never held-out.

### Copy safety rules

1. **Fresh output directory**: Create a new, non-existing directory for output.
   Verify the resolved path is not under any cache, home alias, or source tree.
2. **No symlinks/hardlinks to source**: Writable output files must be true copies,
   not links that could modify the original.
3. **Atomic writes**: Write checkpoint to a temporary file, then rename. Never
   write in-place to a file that might be read.
4. **Manifest all outputs**: Record every file created by the run with its path
   and sha256.
5. **No automatic deletion**: Do not auto-delete any file unless:
   - Every file is verified to be in the run manifest (generated by this run), AND
   - User explicitly requests cleanup.
6. **Default rollback**: Simply use the original base checkpoint. Retain all
   artifacts for inspection.
7. **Hash verification**: Record sha256 of edited tensors. Do not claim to have
   hashed multi-GB base files unless actually computed and verified.

### Manifest contents

Write manifest as JSON:
- Tensor names, shapes
- α value (if applicable)
- Unit direction vector û (or how to recompute it)
- Indices/mask if sparse edit
- Base tensor sha256 (for edited tensors only, not full checkpoint)
- Edited tensor sha256
- Edit-norm ratio ‖ΔW‖ / ‖W‖

Cap forward-pass budget (e.g. ≤25 batches) and log every use.

## Stage 4: Evaluation, reporting, rollback

**Ask before running evaluation inference.**

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

1. Use the original base checkpoint (never modified).
2. Retain all experiment artifacts for inspection.
3. Only delete output files if manifest-verified and user-approved.

## What this workflow does NOT support

- **Quantised checkpoint editing**: Quantisation may distort small edits; not
  characterised in this workflow.
- **Cross-architecture grafting**: Lineage unverified, tensor layouts differ.
- **Capability addition**: This loop-removal workflow has not demonstrated novel
  ability addition.
- **Safety-behaviour removal**: Excluded by design.
- **Recursive self-improvement**: Not demonstrated.
- **Any claim not backed by held-out evaluation**.

## References

- [Sparks of Weight-Space Programming](https://rikiyat.github.io/blog/weight-space-programming/)
  (Takehi et al., 2026 blog)
- [Exp 8 logs](https://github.com/RikiyaT/lm_understand-logs/tree/main/exp8_math)
- [Independent audit](https://achintyakj.tech/blog/weight-space-audit)
  (Achintya K J, 2026-11-04)
- [arXiv:2406.11717](https://arxiv.org/abs/2406.11717) — Refusal direction /
  weight orthogonalisation (Arditi et al.)
- [arXiv:2503.22048](https://arxiv.org/abs/2503.22048) — ThinkEdit: reasoning-length
  edits (Sun, Yan, Weng)
- [arXiv:2312.06681](https://arxiv.org/abs/2312.06681) — Contrastive activation
  addition (Panickssery et al.)
- [arXiv:2611.05341](https://arxiv.org/abs/2611.05341) — Clinical audit/repair
  protocol (Nusrat & Nusrat)
- [arXiv:2611.08070](https://arxiv.org/abs/2611.08070) — Skeptical control for
  reading claims (Ahmed)
- [arXiv:2301.05062](https://arxiv.org/abs/2301.05062) — Tracr: compiler baseline
  (Lindner et al.)
- [arXiv:2610.22891](https://arxiv.org/abs/2610.22891) — Exhaustive evaluation
  requirement (Lee, Oozeer, Quirke)
