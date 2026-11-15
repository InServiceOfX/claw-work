#!/usr/bin/env bash
set -euo pipefail
[[ $(uname -s) == Darwin ]] || { echo 'macOS is required.' >&2; exit 1; }
root=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
cache="${PERSONAL_FILES_CACHE:-$HOME/Library/Caches/personal-files-macos}"
mkdir -p "$cache"
cache=$(cd -- "$cache" && pwd)
export CARGO_TARGET_DIR=${CARGO_TARGET_DIR:-$cache/target}
cd "$root"
host=$(rustc -vV | awk '/^host:/ {print $2}')
# Explicit native target overrides CARGO_BUILD_TARGET and Cargo build.target.
export MACOSX_DEPLOYMENT_TARGET=${MACOSX_DEPLOYMENT_TARGET:-11.0}
cargo build --locked --release --target "$host" --manifest-path "$root/Cargo.toml"
app="$cache/Personal Files.app"
publisher="$CARGO_TARGET_DIR/$host/release/personal-files-macos"
stage=$(mktemp -d "$cache/.bundle.XXXXXX")
staged_app="$stage/Personal Files.app"
published=false
cleanup() {
  local status=$?
  if ! $published && [[ -e "$stage/previous.app" || -L "$stage/previous.app" ]]; then
    if ! "$publisher" --install-bundle "$stage/previous.app" "$app"; then
      printf 'Previous cached app retained for recovery: %s\n' "$stage/previous.app" >&2
      return "$status"
    fi
  fi
  rm -rf -- "$stage"
  return "$status"
}
trap cleanup EXIT
trap 'exit 130' INT
trap 'exit 143' TERM
mkdir -p "$staged_app/Contents/MacOS" "$staged_app/Contents/Resources"
cp "$publisher" "$staged_app/Contents/MacOS/"
cp "$root/assets/Info.plist" "$staged_app/Contents/Info.plist"
icon=/System/Library/CoreServices/CoreTypes.bundle/Contents/Resources/GenericFolderIcon.icns
if [[ -f "$icon" ]]; then cp "$icon" "$staged_app/Contents/Resources/PersonalFiles.icns"; fi
plutil -lint "$staged_app/Contents/Info.plist"
codesign --force --sign - "$staged_app"
codesign --verify --deep --strict "$staged_app"
if [[ -e "$app" || -L "$app" ]]; then
  "$publisher" --install-bundle "$app" "$stage/previous.app"
fi
"$publisher" --install-bundle "$staged_app" "$app"
published=true
printf 'Built: %s\n' "$app"
