#!/bin/bash
# Build and install the Linux CLI. Nothing is downloaded except Cargo crates;
# model weights are a directory you clone yourself (see README.md).
set -euo pipefail
project_root=$(cd "$(dirname "$0")/.." && pwd)
prefix="$HOME/.local"
cpu_only=0
force=0
compute_cap=${CUDA_COMPUTE_CAP:-}
usage() {
  cat <<'HELP'
Usage: scripts/install.sh [OPTIONS]
  --prefix DIR        CLI goes into DIR/bin (default: ~/.local).
  --cpu-only          Build without CUDA (no nvcc needed; slow inference).
  --compute-cap NN    GPU compute capability for nvcc, e.g. 86 for an RTX 3060
                      (default: $CUDA_COMPUTE_CAP). Required with CUDA when several
                      different GPUs are present, because nvcc must target the one you use.
  --force             Replace an existing installed binary.
  -h, --help          Show this help.
HELP
}
while [ "$#" -gt 0 ]; do
  case "$1" in
    --prefix|--compute-cap)
      [ "$#" -ge 2 ] || { echo "Missing value for $1" >&2; exit 2; }
      case "$1" in --prefix) prefix=$2 ;; --compute-cap) compute_cap=$2 ;; esac
      shift 2 ;;
    --cpu-only) cpu_only=1; shift ;;
    --force) force=1; shift ;;
    -h|--help) usage; exit 0 ;;
    *) echo "Unknown option: $1" >&2; usage >&2; exit 2 ;;
  esac
done
[ "$(uname -s)" = Linux ] || { echo "This installer is for Linux; use ../voxcpm-tts on macOS." >&2; exit 1; }
dest="$prefix/bin/voxcpm-tts"
if [ -e "$dest" ] && [ "$force" -ne 1 ]; then
  echo "$dest exists; use --force to replace it." >&2; exit 1
fi
cd "$project_root"
if [ "$cpu_only" -eq 1 ]; then
  cargo build --release --no-default-features
else
  command -v nvcc >/dev/null || { echo "nvcc not found; install the CUDA toolkit or use --cpu-only." >&2; exit 1; }
  if [ -z "$compute_cap" ]; then
    echo "Set --compute-cap (e.g. 86 for RTX 30xx) or CUDA_COMPUTE_CAP. GPUs here:" >&2
    nvidia-smi --query-gpu=index,name,compute_cap --format=csv >&2 || true
    exit 1
  fi
  CUDA_COMPUTE_CAP="$compute_cap" cargo build --release
fi
mkdir -p "$prefix/bin"
install -m 0755 target/release/voxcpm-tts "$dest"
echo "Installed $dest"
cfg=${XDG_CONFIG_HOME:-$HOME/.config}/voxcpm-tts/config.toml
if [ ! -e "$cfg" ]; then
  echo "Next: mkdir -p $(dirname "$cfg") && cp config.toml.example $cfg   # then edit model_dir"
fi
echo "Then run: voxcpm-tts doctor"
