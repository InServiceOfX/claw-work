#!/usr/bin/env bash
set -euo pipefail
root=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
app="${PERSONAL_FILES_CACHE:-$HOME/Library/Caches/personal-files-macos}/Personal Files.app"
# Build explicitly to avoid launching a stale cached executable after edits.
"$root/build.sh"
exec "$app/Contents/MacOS/personal-files-macos" "$@"
