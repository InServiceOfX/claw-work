# AudioVAE V2 native port

Verified on Apple M5 with candle 0.11.1 / Metal, 2026-11-07. Implementation:
`src/native/vae.rs`. The encoder and decoder use the checkpoint's original f32
weights, without quantization or Python at runtime.

## Interface and causal behavior

- `AudioVae::load(vb.pp("audio_vae"), config)` requires an f32 builder.
- `encode` accepts mono f32 16 kHz samples, padded to a positive multiple of
  640, and returns `[frames, 64]` latents. The application pads reference clips
  to a multiple of 2560 for four-frame patches.
- `decode` accepts `[frames, 64]` latents and returns exactly `frames * 1920`
  mono f32 samples at 48 kHz. The decoder applies learned sample-rate
  conditioning, the output Snake/convolution, then tanh.
- Default encode/decode chunks are 64/50 latent frames. Each causal convolution
  carries its left receptive field; transposed convolutions carry one input
  frame and emit only the current chunk. State tensors are copied so they do
  not keep the entire activation buffer alive.
- `DecodeStream` plus `decode_stream` exposes the same causal decoder for
  arbitrary consecutive latent chunks. Different chunk sizes may change f32
  rounding, but do not change the signal alignment or output sample count.

Depthwise kernels use seven shifted multiply/add operations per channel,
avoiding candle's per-channel grouped-convolution loop. Pointwise convolutions
use matrix multiplication. Transposed convolutions use the fast zero-padding
col2im path, followed by causal trimming. The odd encoder stride uses the
original PyTorch left padding of five, not mlx-audio's six. The original
PyTorch decoder length is retained; the extra eight samples from mlx-audio's
odd-stride handling are excluded from comparisons.

Chunking bounds intermediate activation sizes. Final returned waveforms and
the allocator's cached buffers still consume memory; this is not a claim of
constant total process memory or a measured peak-memory limit.

## Precision correction to the numerical oracle

The inherited golden dump used MLX's default reduced-precision f32 convolution
on M5. T15 failed at the first transposed convolution: relative error
`6.944e-4`, maximum absolute error `1.911e-3`. Direct and streamed candle output
were identical; independent scalar f64 sums agreed with candle within about
`5e-7` on sampled outputs. The discrepancy came from reference arithmetic.

[MLX's precision documentation](https://ml-explore.github.io/mlx/build/html/usage/precision.html)
documents the `MLX_ENABLE_TF32=0` launch setting for full-f32 arithmetic.
`reference/dump_golden.py` now sets that default **before importing MLX** and
records its effective value as `mlx_enable_tf32` in `meta.json`. An explicitly
set environment value is preserved for deliberate precision experiments.

All local golden tensors were regenerated together with this setting. The
25.01-second dump produced 30 AR steps / 4.800 seconds and matched
`model.generate()` bit-for-bit (`generate_check_maxabs_diff = 0`). No numerical
tolerance was relaxed to obtain passing VAE tests.

## Executed evidence

Commands run from this crate:

```sh
cargo test --offline --lib native::vae::tests::causal_transpose_matches_scalar_and_streams
cargo test --offline --lib native::vae::tests::depthwise_matches_scalar_and_streams
cargo test --offline --lib native::vae::tests::t3_encoder_unit_trace -- --nocapture --test-threads=1
cargo test --offline --lib native::vae::tests::t4_encoder_full -- --nocapture --test-threads=1
cargo test --offline --lib native::vae::tests::t15_decoder_unit_trace -- --nocapture --test-threads=1
cargo test --offline --lib native::vae::tests::t16_t17_decode_full_and_streaming -- --nocapture --test-threads=1
```

The two scalar-oracle tests use authored synthetic CPU tensors and need no
checkpoint or GPU. They independently cover odd-stride alignment, overlap,
absence of a future tail, dilation, separate channels, and chunks smaller than
the receptive field. Both passed.

| Test | Result |
|---|---|
| T3 encoder unit | worst intermediate relative error `1.432e-6`; final μ relative error `8.626e-7`; chunked vs one-shot max error `0` |
| T4 full reference encode | relative error `1.365e-6`, max error `2.024e-5`; one-shot relative error `1.365e-6` |
| T15 decoder unit, corrected oracle | worst intermediate relative error `1.427e-6`; waveform max error `8.978e-7` |
| T16 full 120-frame decode | max error `1.516e-6`, relative error `2.382e-6`; default 50-frame chunks also max error `1.516e-6` |
| T17 streaming vs one-shot | four-frame chunks exact; seven-frame max error `9.779e-9`; single-frame `8.941e-7`; 50-frame exact; uneven explicit stream `1.919e-7` |

T3/T4 were initially run before the reference precision correction; decoder
tests were run after it. Any later rerun results supersede the initial encoder
measurements. T4 measured 852.1 ms for 5.44 seconds of reference audio on this
machine; this single observation is not a cross-device performance guarantee.

The restricted execution sandbox hid Metal devices and candle panicked during
device creation. Real GPU tests succeeded when run with permitted native GPU
access. A missing GPU must not be confused with a failed model calculation.

## Remaining scope

The codec's numerical and streaming checks pass. Full-model end-to-end
generation, user-facing audio quality, application memory release, installation,
and Mac mini M4 validation belong to integration evidence in `docs/HANDOFF.md`.
The codec alone does not establish those outcomes.
