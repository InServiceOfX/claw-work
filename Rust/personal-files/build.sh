#!/usr/bin/env bash
set -euo pipefail
project_root=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
cache_root=${XDG_CACHE_HOME:-$HOME/.cache}/personal-files
if ! pkg-config --exists gtk+-3.0; then
  native_pc=$cache_root/gtk-dev/usr/lib/$(gcc -print-multiarch)/pkgconfig
  if [[ -f "$native_pc/gtk+-3.0.pc" ]]; then
    export PKG_CONFIG_PATH="$native_pc${PKG_CONFIG_PATH:+:$PKG_CONFIG_PATH}"
  else
    printf '%s\n' 'GTK development files are needed to build. On Ubuntu: sudo apt install libgtk-3-dev pkg-config build-essential' >&2
    exit 1
  fi
fi
export CARGO_TARGET_DIR=${CARGO_TARGET_DIR:-$cache_root/target}
cargo build --locked --release --manifest-path "$project_root/Cargo.toml"
printf 'Built: %s/release/personal-files\n' "$CARGO_TARGET_DIR"
