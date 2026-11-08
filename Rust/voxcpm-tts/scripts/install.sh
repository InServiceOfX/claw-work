#!/bin/bash
# Local, reviewable installer. Network/model/Python operations are opt-in.
set -euo pipefail
project_root=$(cd "$(dirname "$0")/.." && pwd)
prefix="$HOME/.local"
app_dir="$HOME/Applications"
support_dir=${VOXCPM_TTS_HOME:-"$HOME/Library/Application Support/voxcpm-tts"}
profile=release
with_app=1
with_python=0
fetch=0
force=0
skip_build=0
offline=0
usage() {
  cat <<'HELP'
Usage: scripts/install.sh [OPTIONS]
Install the Rust CLI and an ad-hoc signed native macOS app.
  --prefix DIR            CLI goes into DIR/bin (default: ~/.local).
  --app-dir DIR           App destination (default: ~/Applications).
  --cli-only              Skip the GUI build and app bundle.
  --with-python-engine    Install optional pinned MLX fallback with uv.
  --support-dir DIR       Optional worker support directory; set
                          VOXCPM_TTS_HOME to this directory at runtime.
  --fetch                 Fetch model weights after installation.
  --offline               Prevent cargo/uv/fetch network access.
  --force                 Replace existing selected installation artifacts.
  --debug                 Use debug binaries (packaging smoke tests only).
  --skip-build            Package existing binaries in the selected profile.
  -h, --help              Show this help.
No Python, model weights, or virtual environment is bundled in the .app.
HELP
}
while [ "$#" -gt 0 ]; do
  case "$1" in
    --prefix|--app-dir|--support-dir)
      [ "$#" -ge 2 ] || { echo "Missing value for $1" >&2; exit 2; }
      case "$1" in
        --prefix) prefix=$2 ;; --app-dir) app_dir=$2 ;; --support-dir) support_dir=$2 ;;
      esac
      shift 2 ;;
    --cli-only) with_app=0; shift ;;
    --with-python-engine) with_python=1; shift ;;
    --fetch) fetch=1; shift ;;
    --offline) offline=1; shift ;;
    --force) force=1; shift ;;
    --debug) profile=debug; shift ;;
    --skip-build) skip_build=1; shift ;;
    -h|--help) usage; exit 0 ;;
    *) echo "Unknown option: $1" >&2; usage >&2; exit 2 ;;
  esac
done
if [ "$offline" -eq 1 ] && [ "$fetch" -eq 1 ]; then
  echo "--offline and --fetch cannot be combined; fetching model files can need network access." >&2
  exit 2
fi
[ "$(uname -s)" = Darwin ] && [ "$(uname -m)" = arm64 ] || {
  echo "VoxCPM TTS requires Apple Silicon macOS." >&2; exit 1;
}
cli="$prefix/bin/voxcpm-tts"
app="$app_dir/VoxCPM TTS.app"
worker_dir="$support_dir/reference"
check_destination() {
  if { [ -e "$1" ] || [ -L "$1" ]; } && [ "$force" -ne 1 ]; then
    echo "Refusing to overwrite $1; use --force." >&2; exit 1
  fi
}
check_destination "$cli"
if [ "$with_app" -eq 1 ]; then check_destination "$app"; fi
if [ "$with_python" -eq 1 ]; then
  check_destination "$worker_dir"
  command -v uv >/dev/null || { echo "Optional Python engine requires uv." >&2; exit 1; }
fi
if [ "$skip_build" -ne 1 ]; then
  cargo_args=(build --locked --bin voxcpm-tts)
  if [ "$profile" = release ]; then cargo_args+=(--release); fi
  if [ "$with_app" -eq 1 ]; then cargo_args+=(--features gui --bin voxcpm-tts-app); fi
  if [ "$offline" -eq 1 ]; then cargo_args+=(--offline); fi
  (cd "$project_root" && cargo "${cargo_args[@]}")
fi
target_dir=${CARGO_TARGET_DIR:-"$project_root/target"}
case "$target_dir" in /*) ;; *) target_dir="$project_root/$target_dir" ;; esac
artifacts="$target_dir/$profile"
[ -x "$artifacts/voxcpm-tts" ] || { echo "Missing CLI binary in $artifacts" >&2; exit 1; }
if [ "$with_app" -eq 1 ] && [ ! -x "$artifacts/voxcpm-tts-app" ]; then
  echo "Missing GUI binary in $artifacts" >&2; exit 1
fi
cli_stage=""
python_stage=""
cleanup() {
  if [ -n "$cli_stage" ]; then rm -rf "$cli_stage"; fi
  if [ -n "$python_stage" ]; then rm -rf "$python_stage"; fi
}
trap cleanup EXIT
if [ "$with_python" -eq 1 ]; then
  mkdir -p "$support_dir"
  python_stage=$(mktemp -d "$support_dir/.voxcpm-reference.XXXXXX")
  mkdir -p "$python_stage/reference"
  install -m 644 "$project_root/reference/voxcpm_worker.py" "$python_stage/reference/voxcpm_worker.py"
  install -m 644 "$project_root/reference/requirements.txt" "$python_stage/reference/requirements.txt"
  uv_args=()
  if [ "$offline" -eq 1 ]; then uv_args+=(--offline); fi
  uv venv "${uv_args[@]}" --python 3.13 --relocatable "$python_stage/reference/.venv"
  uv pip install "${uv_args[@]}" --python "$python_stage/reference/.venv/bin/python" \
    -r "$python_stage/reference/requirements.txt"
  if [ -e "$worker_dir" ] || [ -L "$worker_dir" ]; then
    mv "$worker_dir" "$python_stage/previous-reference"
  fi
  if ! mv "$python_stage/reference" "$worker_dir"; then
    if [ -e "$python_stage/previous-reference" ]; then mv "$python_stage/previous-reference" "$worker_dir"; fi
    exit 1
  fi
fi
mkdir -p "$prefix/bin"
cli_stage=$(mktemp -d "$prefix/bin/.voxcpm-install.XXXXXX")
install -m 755 "$artifacts/voxcpm-tts" "$cli_stage/voxcpm-tts"
mv -f "$cli_stage/voxcpm-tts" "$cli"
if [ "$with_app" -eq 1 ]; then
  bundle_args=(--binary "$artifacts/voxcpm-tts-app" --app-dir "$app_dir")
  if [ "$force" -eq 1 ]; then bundle_args+=(--force); fi
  "$project_root/scripts/bundle-app.sh" "${bundle_args[@]}"
fi
printf 'CLI: %s\n' "$cli"
if [ "$with_python" -eq 1 ]; then
  printf 'Python fallback: %s\nSet VOXCPM_TTS_HOME=%q for a custom support location.\n' "$worker_dir" "$support_dir"
fi
if [ "$fetch" -eq 1 ]; then
  "$cli" fetch
fi
