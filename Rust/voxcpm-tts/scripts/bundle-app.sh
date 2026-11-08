#!/bin/bash
# Assemble only the executable and portable bundle metadata. No weights or venv.
set -euo pipefail
project_root=$(cd "$(dirname "$0")/.." && pwd)
app_dir="$project_root/dist"
binary=""
force=0
usage() {
  cat <<'HELP'
Usage: scripts/bundle-app.sh [--binary PATH] [--app-dir DIR] [--force]
Build a release GUI binary, then make an ad-hoc signed VoxCPM TTS.app.
--binary PATH   Package an existing executable instead of building.
--app-dir DIR   Destination directory (default: project dist/).
--force         Replace an existing bundle; otherwise refuse.
HELP
}
while [ "$#" -gt 0 ]; do
  case "$1" in
    --binary|--app-dir)
      [ "$#" -ge 2 ] || { echo "Missing value for $1" >&2; exit 2; }
      case "$1" in --binary) binary=$2 ;; --app-dir) app_dir=$2 ;; esac
      shift 2 ;;
    --force) force=1; shift ;;
    -h|--help) usage; exit 0 ;;
    *) echo "Unknown option: $1" >&2; usage >&2; exit 2 ;;
  esac
done
[ "$(uname -s)" = Darwin ] && [ "$(uname -m)" = arm64 ] || {
  echo "VoxCPM TTS requires Apple Silicon macOS." >&2; exit 1;
}
bundle="$app_dir/VoxCPM TTS.app"
if { [ -e "$bundle" ] || [ -L "$bundle" ]; } && [ "$force" -ne 1 ]; then
  echo "Refusing to overwrite $bundle; use --force." >&2; exit 1
fi
if [ -z "$binary" ]; then
  (cd "$project_root" && cargo build --locked --release --features gui --bin voxcpm-tts-app)
  target_dir=${CARGO_TARGET_DIR:-"$project_root/target"}
  case "$target_dir" in /*) ;; *) target_dir="$project_root/$target_dir" ;; esac
  binary="$target_dir/release/voxcpm-tts-app"
fi
[ -f "$binary" ] && [ -x "$binary" ] || { echo "Missing executable: $binary" >&2; exit 1; }
mkdir -p "$app_dir"
stage=$(mktemp -d "$app_dir/.voxcpm-bundle.XXXXXX")
trap 'rm -rf "$stage"' EXIT
new_bundle="$stage/VoxCPM TTS.app"
mkdir -p "$new_bundle/Contents/MacOS"
install -m 755 "$binary" "$new_bundle/Contents/MacOS/voxcpm-tts-app"
install -m 644 "$project_root/packaging/Info.plist" "$new_bundle/Contents/Info.plist"
plutil -lint "$new_bundle/Contents/Info.plist" >/dev/null
codesign --force --sign - "$new_bundle"
codesign --verify --deep --strict "$new_bundle"
if [ -e "$bundle" ] || [ -L "$bundle" ]; then
  mv "$bundle" "$stage/previous.app"
fi
if ! mv "$new_bundle" "$bundle"; then
  if [ -e "$stage/previous.app" ]; then mv "$stage/previous.app" "$bundle"; fi
  exit 1
fi
printf 'App: %s\n' "$bundle"
