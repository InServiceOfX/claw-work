# Rust runtime decision: VoxCPM2 (bf16) on Apple Silicon

Status: **candle 0.11.1 + Metal recommended.** The candle findings below come from spikes that
were built and run. The mlx-rs, existing-ports and audio-I/O sections were **not verified**
because the session ended first. They are marked TODO with the next step for each.

Test machine: Apple M5, 32 GB, macOS 26.2, Rust 1.91.1, Command Line Tools only (no Xcode,
`xcrun metal` unavailable). **The machine was heavily loaded during every benchmark** (a GPU
game plus parallel `rustc` jobs, load average 8–15). Treat all timings as pessimistic, and
expect roughly 1.5–2x better on an idle machine.

## 1. Decision

| Option | Verdict | Why |
|---|---|---|
| **candle-core / candle-nn 0.11.1, `features=["metal"]`** | **Use** | Pure Rust/cargo. Kernels compile at runtime from embedded `.metal` source via `MTLDevice newLibraryWithSource`, so no Xcode is needed (verified). Every op VoxCPM2 needs runs in bf16 on Metal (matrix in section 3). Fused `sdpa` supports GQA. bf16 safetensors load via mmap. Memory is returned on drop. |
| mlx-rs (oxideai) | Not chosen (unverified) | mlx-sys builds MLX from source with CMake, and MLX's build compiles `.metal` → `.metallib` with the `metal` CLI, which needs Xcode. The other route is linking the prebuilt `libmlx.dylib` + `mlx.metallib` from the `mlx` pip wheel (0.32.4 ships `mlx/lib/libmlx.dylib`, 21 MB, and `mlx/lib/mlx.metallib`, 214 MB). That needs mlx-c built against it plus exact version matching with mlx-sys 0.7.0. Possible but brittle, so it is a fallback only. **TODO**: time-box a test of `MLX_BUILD_METAL`/prebuilt linking. |
| ort (ONNX Runtime) | Not chosen | No known VoxCPM2 ONNX export (**TODO**: verify). The CoreML EP does not do bf16 well, and the AR + diffusion loop would need several graphs. |
| Python mlx-audio / mlx | Last resort | Allowed only as the numerical reference/oracle. |

Main reasons: candle needs no toolchain beyond cargo, and on the same loaded machine its speed
is within about 1.2–2x of Python MLX (section 4). The remaining gaps are known and fixable in
our own crate (fusion, a custom depthwise kernel).

## 2. Exact versions that worked

```toml
candle-core = { version = "0.11.1", features = ["metal"] }   # pulls candle-metal-kernels 0.11.1, objc2-metal 0.3.2
candle-nn   = { version = "0.11.1", features = ["metal"] }
safetensors = "0.8"
half = "2"
memory-stats = "1.2"   # RSS only; use `footprint <pid>` for the real phys footprint
```

Latest versions on crates.io at time of writing (not all exercised): mlx-rs 0.34.0, mlx-sys 0.7.0,
symphonia 0.6.1, rubato 5.0.1, hound 3.5.1, tokenizers 0.23.2.

### Metal kernel compile cost (important for CLI UX)

- candle compiles each `.metal` source library on first use in a process. Cold first-use cost
  measured under load: mlx_gemm about 14 s, reduce/softmax about 14–19 s, sdpa_full about 6 s,
  binary about 2 s, gemv about 2 s. **A cold start totals 30–90 s.**
- **macOS caches the compiled result per executable path** (`$(getconf DARWIN_USER_CACHE_DIR)/com.apple.metal*`).
  The second run of the same binary took 2–10 ms per library, and the whole op suite took 0.4 s.
  Rebuilding at the same path kept the cache. **Copying the binary to a new path forced a full
  recompile** (matmul 14.5 s, reduce 19.1 s).
- Consequences: (a) `cargo install` to a stable path, then run a `voxcpm-tts warmup` subcommand
  once. (b) Document the slow first run. (c) Optionally add a `serve` mode that stays resident.
  No Xcode is needed for any of this. A later optimisation is an `MTLBinaryArchive` (via
  objc2-metal), which also needs no Xcode.

## 3. Op support matrix: Metal, bf16 (also f16 and f32)

Method: each op ran on Metal and was compared against a CPU f32 reference. The table gives the
relative max error in bf16. **No op failed in bf16, f16 or f32.**

| Op | bf16 | Notes |
|---|---|---|
| matmul / broadcast_matmul, `Linear` (gemm + gemv 1×2048) | OK, ~3.5e-3 | |
| softmax_last_dim, softmax(dim) | OK, ~1e-2 | |
| `ops::rms_norm` (fused), `layer_norm` | OK, ~1e-2 | |
| `rotary_emb::rope` (non-interleaved), `rope_i` | OK, ~6e-3 | VoxCPM2 uses LongRoPE: precompute cos/sin with long/short factors on the CPU, then call `rope` |
| conv1d k7 / dilated (d=9) / depthwise g=64 / stride 2 | OK, ~3e-3 | **depthwise is very slow**, see section 4 |
| conv_transpose1d (s8 k16, s5 k10) | OK, ~3e-3 | **slow unless padding=0**, see section 4 |
| silu, gelu, gelu_erf, sigmoid, tanh, sin, cos, exp, log, sqrt, recip, powf, clamp | OK | |
| snake `x + 1/α·sin²(αx)` (composite) | OK, 8e-3 | |
| round (FSQ), argmax | OK | Bit-exact against the CPU when given the same bf16 inputs (verified separately) |
| sum/mean/max_keepdim, cumsum | OK | |
| Embedding / index_select (u32 and i64 ids), gather, where_cond(u8) | OK | |
| cat (KV append), narrow, transpose+contiguous, pad_with_zeros/same, slice_assign, GQA expand | OK | |
| `candle_nn::ops::sdpa` vector (q=1, kv 500 and 2048 two-pass, GQA 16/2) | OK, ~3e-3 | Decode path |
| `sdpa` full (causal and non-causal, GQA, q8×kv500, MHA b2 seq9) | OK, ~4e-3 | head_dim must be one of 32/64/72/80/96/128/256/512 (VoxCPM2 = 128) |
| randn / rand on Metal | OK | Generate in f32, then cast |
| `to_dtype` I32→BF16 on Metal | **Not implemented** | Only affects `audio_vae.decoder._sr_boundaries` (I32 [3]). Load it on the CPU as I32. |

### Checkpoint facts that affect the port (from safetensors headers)

- 820 tensors in two shards (4.29 GB + 0.67 GB). LM/DiT/LocEnc are **BF16**. **AudioVAE weights
  are F32** (242 tensors). Keep the VAE in f32 or cast it.
- Conv weights are in **MLX layout** and must be transposed for candle:
  - conv1d: MLX `(out, k, in/groups)` → candle `(out, in/groups, k)`.
  - conv_transpose: MLX `(out, k, in)`, e.g. `[1024,16,2048]` → candle `(in, out, k)`.
- VAE residual units are **depthwise k7** (`conv1 [C,7,1]`) plus **pointwise** (`conv2 [C,1,C]`).
  Decoder channels go 2048→1024→…→32 with rates 8,6,5,2,2,2 (25 Hz latent → 48 kHz). There are
  also `sr_cond_layers` scale/bias embeddings.
- The LocDiT/LocEnc attention is **GQA 16q/2kv** (k_proj `[256,1024]`, q_proj `[2048,1024]`) at
  hidden 1024. TSLM is 28L with h2048, i6144, GQA 16/2, hd128. RALM is 8 layers with no rope.

## 4. Benchmarks (bf16, Metal, batch 1, synthetic weights with real shapes)

| Component (per 160 ms LM step) | candle 0.11.1 | Python MLX 0.32.4 (same load, same shapes) |
|---|---|---|
| TSLM decode, 28L, kv 500 | 35–56 ms (18–28 tok/s) | 46 ms |
| RALM decode, 8L | 12–38 ms (noisy) | 15 ms |
| LocDiT, 10 Euler steps × CFG batch 2, seq 9, 12L | 190–217 ms | 111 ms |
| LocEnc, 12L, seq 5 | 15–32 ms | 13 ms |
| **Total per 160 ms of audio** | **254–338 ms → RTF 1.6–2.1** | 185 ms → RTF 1.16 |
| TSLM prefill, 128 tokens | 121–165 ms (~800–1050 tok/s) | |
| AudioVAE-like decode, 10 s → 48 kHz, naive conv | 14.2 s (RTF 1.4) | |
| AudioVAE-like decode with fixes below | **1.73 s bf16 / 2.1 s f32 (RTF 0.17–0.21)** | |

Interpretation:

- TSLM performance is weight-bandwidth bound: 2.64 GB of bf16 weights gives a floor of about
  17 ms on an idle M5. Candle roughly matches MLX here.
- **LocDiT is launch-overhead bound.** That is about 3,000 tiny kernels per LM step at
  roughly 60 µs each under load. It is the largest gap to MLX, which uses a lazy graph.
- Fixes, in order:
  1. Fuse QKV and gate/up weights at load time (tested: RALM/LocEnc improved about 30%).
  2. Avoid `transpose().contiguous()` in the DiT (seq ≤ 9).
  3. Precompute time embeddings for all 10 steps.
  4. Try `CANDLE_METAL_COMPUTE_PER_BUFFER` > 50. Untested: my run passed it incorrectly.
  5. Write a custom fused Metal kernel via candle `CustomOp` + `new_library_with_source`
     (runtime-compiled, no Xcode).
- **RTF estimate:** about 1.6–2.1 as measured on the loaded machine, and an estimated
  **0.7–1.2 idle with the fusions above**. The VAE adds about 0.17–0.2. Plan for roughly real
  time, not much faster. Fewer diffusion steps (VoxCPM allows tuning `inference_timesteps`) gives
  a near-linear LocDiT speedup.

### Two mandatory candle workarounds for the AudioVAE (measured)

1. **Grouped conv1d loops over groups** (`Tensor::chunk(groups)` → one conv per channel), which
   means thousands of launches. Implement depthwise conv as K shifted multiply-adds:

```rust
fn depthwise_conv1d(x: &Tensor, w: &Tensor, pad: usize, dil: usize) -> candle_core::Result<Tensor> {
    let (_b, _c, l) = x.dims3()?;               // w: (C, 1, K) candle layout
    let k = w.dim(2)?;
    let xp = x.pad_with_zeros(2, pad, pad)?;     // causal variant: pad left only
    let lout = l + 2 * pad - dil * (k - 1);
    let mut acc: Option<Tensor> = None;
    for i in 0..k {
        let wi = w.narrow(2, i, 1)?.reshape((1, w.dim(0)?, 1))?;
        let t = xp.narrow(2, i * dil, lout)?.broadcast_mul(&wi)?;
        acc = Some(match acc { None => t, Some(a) => (a + t)? });
    }
    Ok(acc.unwrap())
}
// pointwise conv (k=1): w.squeeze(2)?.matmul(&x.squeeze(0)?)?.unsqueeze(0)
```

   Even this is about 75% of the VAE time, so a fused snake+depthwise `CustomOp` Metal kernel is
   the next step.

2. **conv_transpose1d is 100–190x slower when `padding != 0` or `output_padding != 0`**. Only
   `padding=0` takes the matmul + col2im path. Always call it with padding 0 and crop with
   `narrow` afterwards. This was verified equal to the padded kernel within bf16 eps. Timings:
   1111 ms → 7 ms for 2048→1024 s8, and 1683 ms → 20 ms for 512→256 s5.

## 5. Memory release (measured on the real 4.29 GB shard `model-00001.safetensors`)

| Point | `footprint` (phys) | RSS | candle `current_allocated_size()` |
|---|---|---|---|
| before load | 4 MB | 13 MB | 1 MB |
| after `VarBuilder::from_mmaped_safetensors` (lazy) | 5 MB | 14 MB | 1 MB |
| after materialising all 753 tensors as bf16 (1.2–6 s) | 4452 MB | 8726 MB | 4107 MB |
| after `drop(tensors)` + `device.synchronize()` | 635 MB | 4815 MB | 1 MB |
| after `drop(VarBuilder)` | 495–532 MB | 529 MB | 1 MB |
| reload 300 MB tensor → drop again | 783 → 495 MB | | 301 → 1 MB |

- **Memory is returned.** candle pools freed Metal buffers and releases them in
  `drop_unused_buffers()`, which runs on `device.synchronize()`. **Call `synchronize()` after
  dropping a model.**
- RSS double-counts the mmap'd file pages, so use `footprint <pid>` instead.
- A residual ~0.5 GB stays after the first load: compiled pipelines and allocator arenas. It
  does not grow on reload.
- Peak is about 2x the weight size while the mmap and the device copies coexist. Use
  `MmapedSafetensors` and drop the VarBuilder right after construction.
- Full model in memory: about 4.1 GB with the VAE cast to bf16, about 4.3 GB with the VAE in f32.

## 6. Audio I/O stack (**TODO: not spike-verified in this session**)

Recommended recipe, unverified:

- `symphonia` 0.6 with features `["wav","pcm","mp3"]`: probe → decode → `SampleBuffer<f32>`
  → downmix to mono. Note that 0.6 is a fresh major release and its API differs from 0.5.
- `rubato` 5.x for resampling to 16 kHz (FFT-based fixed-ratio resampler, sinc for HQ).
  Alternative: shell out to the Homebrew `ffmpeg` `-af aresample=resampler=soxr` as an HQ
  cross-check.
- `hound` 3.5.1 for writing 48 kHz WAV, as `SampleFormat::Int` 16-bit or `SampleFormat::Float`
  32-bit.
- For chunk joins, use an equal-power crossfade (`cos`/`sin` ramps over 10–50 ms) and zero
  padding.
- `tokenizers` 0.23 with default features (`onig` builds via the cc crate, no system deps
  expected). Load the model's `tokenizer.json` (3.7 MB; already in the HF cache from another
  agent).

Next step: add an `audio_spike` crate that does each item above and asserts round-trip sample
counts and RMS.

## 7. Existing ports (**TODO: search not completed**)

Not searched. Candidates to check: the openbmb VoxCPM repo (any ONNX/TensorRT export), mlx-audio's
`voxcpm` model (Python reference for the MLX weight layout used here), candle-transformers
(`minicpm`? — not present as of 0.11 to my knowledge, unverified), and crates.io/GitHub "voxcpm rust".

## 8. Risks

1. **LocDiT launch overhead** decides whether we beat real time. Mitigations: fusion, custom
   kernels, fewer steps.
2. **Cold Metal compile of 30–90 s** on first run per binary path. Mitigation: a warmup command
   and a stable install path.
3. Layout bugs from MLX-layout conv weights and the LongRoPE factors. Validate each module
   against the Python MLX reference with saved intermediate tensors.
4. **bf16 accumulation drift** over long AR sequences. Option: keep the residual stream / norms
   in f32 if parity tests drift. The f32 path works for all ops.
5. candle Metal is moving fast (0.11.1 shipped one week before this test). Pin exact versions.
6. The benchmarks were taken under heavy contention. Re-run `candle_spike bench bf16` on an idle
   machine before committing to an RTF target.

## Reproduce

The spike crate (not committed) is a single `src/main.rs` with these subcommands:

- `ops` (op matrix)
- `compile` (per-library cold/warm compile time)
- `roundcheck`
- `bench bf16|f32` (env `FUSE=1`, `VAE_FAST=1`, `ONLY_VAE=1`)
- `vaeprof`
- `mem <shard.safetensors>`

Rebuild from the descriptions above if needed.
