# Native transformer port

Status: implemented and numerically verified on Apple M5 Metal, 2026-11-07.
This is the continuation record for the shared transformer, patch encoder,
diffusion transformer, scalar quantizer and Euler sampler.

## Implemented components

- `src/native/minicpm.rs`: MiniCPM RMSNorm, LongRoPE, GQA, causal and
  bidirectional attention, SwiGLU and final RMSNorm. Per-layer KV caches store
  post-RoPE keys and raw values, validate position offsets and support a cached
  suffix with multiple tokens. Metal uses fused SDPA; CPU uses explicit
  attention as a portable correctness path. Candle requires explicit mask
  broadcasting to batch and query-head dimensions.
- `src/native/fsq.rs`: learned projections around scalar quantization. Exact
  half-integer ties use ties-to-even to match PyTorch/MLX; Metal's native
  `round` alone uses a different tie rule.
- `src/native/locenc.rs`: projected latent patches prefixed by the learned
  CLS token, bidirectional local transformer, CLS output.
- `src/native/locdit.rs`: two conditioning tokens, sinusoidal timestep MLP,
  cached zero delta-time embedding, projected previous/noisy patches,
  bidirectional local transformer and velocity projection.
- `src/native/cfm.rs`: descending sway schedule, Euler integration,
  configurable CFG, batched positive/negative velocities and CFG-Zero* with
  the upstream initial-zero-step policy.

The existing interfaces used by `model.rs` were preserved. All public
transformer outputs are f32; stack compute precision is configurable, with
**f32 verified and recommended**. The VAE is independently kept in f32.

## Reproduction

From this package, with public checkpoint weights already in the HF cache:

```sh
cargo test --offline --lib t2_t14_transformer_golden -- --nocapture --test-threads=1
cargo test --offline --lib causal_cache_matches_full_sequence_and_cannot_see_future
cargo test --offline --lib invalid_config_and_cache_are_rejected
cargo test --offline --lib quantization_ties_use_even_levels
cargo test --offline --lib sway_schedule_matches_reference
cargo clippy --offline --lib --tests -- -D warnings
```

The golden test requires real Metal access. A sandbox that hides Metal cannot
perform that check. Checkpoint or golden-file absence produces a skip note,
not numerical verification. `VOXCPM_TTS_GOLDEN` can select another golden
folder, and `VOXCPM_TTS_CPU=1` selects the much slower CPU path.

The verified golden set was regenerated with `MLX_ENABLE_TF32=0`. Its metadata
records that setting, 30 autoregressive patches and bit-identical agreement
between the instrumented dump and upstream `model.generate()`. The original
TF32-default dump was unsuitable for tight f32 comparisons on M5; see the VAE
port record for the isolated numerical investigation.

## Measured golden comparisons

Single serialized transformer test: **passed, 4.89 seconds** on a warm Metal
kernel cache. All comparisons below use f32 compute and the actual public
checkpoint weights; no synthetic substitute weights were used.

| Spec point | Comparison | Measured error | Enforced threshold |
|---|---|---:|---:|
| T2 | LongRoPE cosine/sine, 57 positions | max abs 3.666e-6 / 3.740e-6 | max abs 5e-6 |
| T6 | LocEnc output for all 57 prefill patches | rel 7.429e-7 | rel 1e-5 |
| T7 | TSLM final normalized prefill | rel 1.693e-6 | rel 1e-4 |
| T7 | Layer-0 post-RoPE K / raw V cache | rel 6.927e-7 / 7.598e-7 | rel 1e-4 |
| T8 | FSQ on golden prefill activations | exact | rel 1e-4 |
| T9 | RALM final normalized prefill | rel 1.181e-6 | rel 1e-4 |
| T10 | First evaluated DiT velocity, CFG batch 2 | rel 9.271e-7 | rel 1e-5 |
| T11 | CFM generated patch 0 / patch 1 | rel 1.131e-6 / 1.186e-6 | rel 1e-4 |
| T12 | LocEnc + projection for generated patch 0 | rel 3.030e-6 | rel 1e-5 |
| T14 | All 29 teacher-forced TSLM steps | max rel 1.199e-6 | rel 1e-4 each |
| T14 | All 29 teacher-forced RALM steps | max rel 4.898e-7 | rel 1e-4 each |

`rel` is the L2 norm of the difference divided by the golden L2 norm.
The teacher-forced RALM test supplies the golden next quantized LM hidden
and current patch embedding, isolating the residual stack from FSQ flips.

T2 deliberately uses a documented 5e-6 absolute threshold instead of the
specification's suggested 1e-6. Scalar f32 `powf` versus MLX Metal's power
approximation changes frequencies by a few ulps; multiplying by the position
magnifies the table difference. Independent NumPy recomputation also differs
from MLX by approximately 3.7e-6. No model-output threshold was loosened:
prefill, cache, teacher-forced decoding and CFM all pass their original tighter
relative tolerances. This observation does not establish a table bound for
arbitrarily long positions.

The golden test checks final stack outputs and the layer-0 cache. It does not
individually assert every intermediate layer, timestep embedding or Euler
velocity listed in the suggested porting matrix. Those remain useful extra
diagnostic points if a future modification fails the final comparisons.

## Independent CPU behavior checks

- Changing future tokens leaves earlier causal outputs exactly unchanged.
- A cached prefix plus a two-token suffix matches one-shot causal attention
  (relative error 3.441e-8), including offset RoPE and GQA.
- Bidirectional outputs change when later tokens change.
- Invalid head counts and an incorrect cache position are rejected.
- Positive and negative half-integer quantization ties round to even levels.
- The ten-step sway grid matches independently rounded upstream reference
  numbers and decreases strictly from one to zero.

These small tests run without downloading weights or allocating a large model.

## Remaining limits

- BF16 inference is implemented through the common dtype interface but has
  not been numerically or audibly accepted. Keep f32 as the default.
- Whole-utterance autoregressive replay, stop-head behavior, generated WAV
  checks, cancellation and memory release are integration responsibilities;
  passing these component tests alone does not prove them.
- No Mac mini M4 verification was performed by this component task.
- QKV and gate/up projection fusion and timestep-MLP batching are future
  performance work. Preserve the teacher-forced golden checks before tuning.

## Full-model integration follow-up

The parent integration run also passed T18 with 30 generated patches and the
exact reference stop count. The first five latent-patch relative errors were
3.61e-6, 1.56e-6, 2.75e-6, 5.38e-6 and 5.66e-6; decoded waveform length was
correct. The test took 11.18 seconds including load and decode. Integration
exposed a strided CLS-view projection problem on Metal: the model projection
helper now makes inputs contiguous, and LocEnc also returns a dense CLS tensor.
The T6 regression checks the full 57-patch projection, including its layout.
