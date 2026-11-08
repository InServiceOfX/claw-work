#!/usr/bin/env bash
set -euo pipefail
project_root=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
"$project_root/build.sh"
cache_root=${XDG_CACHE_HOME:-$HOME/.cache}/personal-files
data_root=${XDG_DATA_HOME:-$HOME/.local/share}
app_root=$HOME/.local/lib/personal-files
binary=${CARGO_TARGET_DIR:-$cache_root/target}/release/personal-files
mkdir -p -- "$app_root" "$HOME/.local/bin" "$data_root/applications" "$data_root/icons/hicolor/scalable/apps"
temporary_binary=$(mktemp "$app_root/.personal-files.XXXXXX")
trap 'rm -f -- "$temporary_binary"' EXIT
install -m 755 -- "$binary" "$temporary_binary"
mv -f -- "$temporary_binary" "$app_root/personal-files"
trap - EXIT
install -m 644 -- "$project_root/assets/personal-files.svg" "$data_root/icons/hicolor/scalable/apps/io.monoclaw.PersonalFiles.svg"
if [[ -e "$HOME/.local/bin/personal-files" && ! -L "$HOME/.local/bin/personal-files" ]]; then
  printf '%s\n' 'Existing ~/.local/bin/personal-files was preserved; launch from the applications menu.' >&2
else
  ln -sfn -- "$app_root/personal-files" "$HOME/.local/bin/personal-files"
fi
# Desktop Entry quoting: Exec is parsed by the desktop, never by a shell.
desktop_exec=$app_root/personal-files
desktop_exec=${desktop_exec//\\/\\\\}
desktop_exec=${desktop_exec//\"/\\\"}
desktop_exec=${desktop_exec//\$/\\\$}
desktop_exec=${desktop_exec//\`/\\\`}
desktop_exec=${desktop_exec//%/%%}
cat > "$data_root/applications/io.monoclaw.PersonalFiles.desktop" <<EOF
[Desktop Entry]
Type=Application
Version=1.0
Name=Personal Files
Comment=Your favorite, recent, and frequent folders, one click away
Exec="$desktop_exec" %f
Icon=io.monoclaw.PersonalFiles
Terminal=false
Categories=System;FileTools;FileManager;
StartupNotify=true
EOF
chmod 644 -- "$data_root/applications/io.monoclaw.PersonalFiles.desktop"
if command -v update-desktop-database >/dev/null; then
  update-desktop-database "$data_root/applications"
fi
printf '%s\n' 'Installed. Open Personal Files from the Ubuntu applications menu, or run ~/.local/bin/personal-files.'
