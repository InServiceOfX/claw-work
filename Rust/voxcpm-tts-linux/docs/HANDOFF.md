# Handoff: voxcpm-tts-linux

Linux/CUDA sibling of `../voxcpm-tts`. The macOS crate is untouched; this crate is a
copy of its engine-independent layer (`text`, `audio`, `voice`, `eta`, `job`) and the
native candle port (`native/`), adapted as follows.

| Change | Why |
|---|---|
| candle `metal` → optional `cuda` feature (default on) | No Metal on Linux |
| removed `objc2` autorelease pools, `candle-metal-kernels`, Metal pool timing | Apple-only |
| removed GUI, `.app` packaging, Python mlx-audio worker, `hub.rs`/`fetch` | No GUI wanted; MLX is Apple-only; no hub cache |
| `config.rs`: `Settings` (`model_dir`, `device`, `cuda_device`, `dtype`) from flag > env > `config.toml` | Weights are a user-cloned directory, not `~/.cache/huggingface/hub` |
| completeness check rejects git-lfs pointer files | A clone without git-lfs "looks" complete but is not |
| XDG paths for config/data | `~/Library/Application Support` is macOS |

## Status

- [x] `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings` (CUDA and `--no-default-features`), 65 unit tests pass.
- [x] `voxcpm-tts doctor` opens a CUDA device; missing/bad `model_dir` gives an actionable error.
- [ ] **Not yet verified:** a real synthesis. This needs the weights cloned locally. The numerical golden
      tests (`VOXCPM_TTS_GOLDEN`) only ran as skips, and the `dtype = "bf16"` path has no golden check.
- [ ] Measure GPU memory and real-time factor on the target card; confirm which `cuda_device` ordinal is the intended GPU.

## Next steps

1. Clone the weights, write `~/.config/voxcpm-tts/config.toml`, run `voxcpm-tts doctor`.
2. `voxcpm-tts say --text 'Hello.' --voice <clip> --seed 42 -o out.wav`; watch `nvidia-smi`.
3. If f32 runs out of memory, set `dtype = "bf16"` and listen for quality differences.
