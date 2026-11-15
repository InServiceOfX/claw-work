#!/usr/bin/env bash
set -euo pipefail
root=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
cd "$root"
export CARGO_TARGET_DIR=${CARGO_TARGET_DIR:-$HOME/Library/Caches/personal-files-macos/target}
cargo fmt --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked
cargo test --locked --test desktop native_new_windows_story -- --ignored --nocapture
cargo test --locked --test desktop native_saved_session_story -- --ignored --nocapture
cargo test --locked --test desktop native_desktop_story -- --ignored --nocapture
"$root/build.sh"
cache="${PERSONAL_FILES_CACHE:-$HOME/Library/Caches/personal-files-macos}"
plutil -lint "$cache/Personal Files.app/Contents/Info.plist"
codesign --verify --deep --strict "$cache/Personal Files.app"
