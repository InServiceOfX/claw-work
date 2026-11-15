#!/usr/bin/env bash
set -euo pipefail
root=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
destination="$HOME/Applications/Personal Files.app"
if [[ -e "$destination" ]]; then
  echo "Already installed: $destination. Move it aside before installing a new version; nothing was overwritten." >&2
  exit 1
fi
"$root/build.sh"
mkdir -p "$HOME/Applications"
staging=$(mktemp -d "$HOME/Applications/.personal-files-install.XXXXXX")
trap 'if [[ -d "$staging" ]]; then printf "Unpublished staging retained: %s\\n" "$staging" >&2; fi' EXIT
app="${PERSONAL_FILES_CACHE:-$HOME/Library/Caches/personal-files-macos}/Personal Files.app"
ditto "$app" "$staging/Personal Files.app"
"$app/Contents/MacOS/personal-files-macos" --install-bundle "$staging/Personal Files.app" "$destination"
rmdir "$staging"
printf 'Installed: %s\nLaunch with: open "%s"\n' "$destination" "$destination"
