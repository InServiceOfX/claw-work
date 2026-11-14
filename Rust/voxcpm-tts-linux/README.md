# VoxCPM TTS for Ubuntu / Linux

Command-line voice cloning for [`mlx-community/VoxCPM2-bf16`](https://huggingface.co/mlx-community/VoxCPM2-bf16).
This is the Linux sibling of [`../voxcpm-tts`](../voxcpm-tts) (macOS, candle/Metal), which it
does not modify. Same CLI (`say`, `session`, `plan`, `doctor`), same chunking, retry, stitching and ETA
code, but the engine runs on **candle + CUDA** (or CPU) and the weights are **a directory you
cloned**, not the Hugging Face hub cache. There is no GUI and no Python engine here.

The MLX checkpoint is used as-is: the Rust port reads its safetensors directly through candle and
needs no MLX. (The upstream `openbmb/VoxCPM2` weights use a different tensor layout and would need conversion.)

## 1. Get the weights (about 5 GB)

```sh
git lfs install
git clone https://huggingface.co/mlx-community/VoxCPM2-bf16 /your/absolute/path/VoxCPM2-bf16
```

If `git lfs` was missing, the `.safetensors` files are tiny pointer files; run `git lfs pull` inside the clone.

## 2. Point the tool at them

Settings come from, highest priority first: `--model-dir`, `VOXCPM_TTS_MODEL_DIR`, then
the config file `~/.config/voxcpm-tts/config.toml` (override the location with `VOXCPM_TTS_CONFIG`).

```sh
mkdir -p ~/.config/voxcpm-tts
cp config.toml.example ~/.config/voxcpm-tts/config.toml
$EDITOR ~/.config/voxcpm-tts/config.toml     # set model_dir to your absolute path
```

Only `config.toml.example` is committed; `config.toml` is git-ignored because it contains your paths.
Other keys: `device` (`cuda`|`cpu`), `cuda_device`, `dtype` (`f32`|`bf16`), `data_dir`.
Environment overrides: `VOXCPM_TTS_DEVICE`, `VOXCPM_TTS_CUDA_DEVICE`, `VOXCPM_TTS_DTYPE`, `VOXCPM_TTS_HOME`.

## 3. Build and install

Needs Rust 1.88+ and, for CUDA, the CUDA toolkit (`nvcc`).

```sh
./scripts/install.sh --compute-cap 86     # e.g. 86 = RTX 30xx; see `nvidia-smi --query-gpu=name,compute_cap --format=csv`
./scripts/install.sh --cpu-only           # no CUDA toolkit needed (slow)
voxcpm-tts doctor
```

`--compute-cap` matters on mixed-GPU machines: nvcc must target the GPU you run on, and a newer CUDA
toolkit cannot target older architectures.

## Use

```sh
voxcpm-tts say /path/to/script.txt --voice /path/to/speaker.wav -o narration.wav
voxcpm-tts say --text 'Welcome to the harbor.' --voice speaker.mp3 -o welcome.wav
voxcpm-tts session --voice speaker.wav --out-dir narrations/
voxcpm-tts plan /path/to/script.txt --verbose     # chunking + estimate, no model load
```

Options are the same as the macOS tool (`--transcript`, `--timesteps`, `--cfg`, `--seed`, `--max-chars`,
`--float`, `--keep-parts`, `--report`, `--force`). Reference audio: 2–30 s of one speaker you are authorized to clone.

## Memory

`dtype = "f32"` (default) keeps the verified numerics and needs about 10 GB of GPU memory for
weights plus scratch space, which is tight on a 12 GB card. `dtype = "bf16"` needs about 5 GB. The AudioVAE always runs in f32.

## Development

```sh
CUDA_COMPUTE_CAP=86 cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test --release
```

Numerical golden-tensor tests skip unless `VOXCPM_TTS_GOLDEN` points at dumps from the macOS crate's `reference/dump_golden.py`.
See [docs/HANDOFF.md](docs/HANDOFF.md) for what is verified and what is not.
