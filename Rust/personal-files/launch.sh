#!/usr/bin/env bash
set -euo pipefail
project_root=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
cache_root=${XDG_CACHE_HOME:-$HOME/.cache}/personal-files
binary=${CARGO_TARGET_DIR:-$cache_root/target}/release/personal-files
if [[ ! -x "$binary" ]]; then
  "$project_root/build.sh"
fi
exec "$binary" "$@"
