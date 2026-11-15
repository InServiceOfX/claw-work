#!/usr/bin/env bash
set -euo pipefail
project_root=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
"$project_root/build.sh"
cache_root=${XDG_CACHE_HOME:-$HOME/.cache}/personal-files
data_root=${XDG_DATA_HOME:-$HOME/.local/share}
app_root=$HOME/.local/lib/personal-files
binary=${CARGO_TARGET_DIR:-$cache_root/target}/release/personal-files
icon_name=io.monoclaw.PersonalFiles
icon_root=$data_root/icons/hicolor
mkdir -p -- "$app_root" "$HOME/.local/bin" "$data_root/applications" "$icon_root/scalable/apps"
for size in 16 24 32 48 64 128 256 512; do
  mkdir -p -- "$icon_root/${size}x${size}/apps"
  install -m 644 -- "$project_root/assets/icons/${size}.png" "$icon_root/${size}x${size}/apps/${icon_name}.png"
done
temporary_binary=$(mktemp "$app_root/.personal-files.XXXXXX")
trap 'rm -f -- "$temporary_binary"' EXIT
install -m 755 -- "$binary" "$temporary_binary"
mv -f -- "$temporary_binary" "$app_root/personal-files"
trap - EXIT
install -m 644 -- "$project_root/assets/personal-files.svg" "$icon_root/scalable/apps/${icon_name}.svg"
# A themed name alone is drawn as Yaru's generic gear until the shell rescans icons.
icon_file=$icon_root/512x512/apps/${icon_name}.png
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
Icon=$icon_file
Terminal=false
Categories=System;FileTools;FileManager;
StartupNotify=true
EOF
chmod 644 -- "$data_root/applications/io.monoclaw.PersonalFiles.desktop"
if command -v update-desktop-database >/dev/null; then
  update-desktop-database "$data_root/applications"
fi
printf '%s\n' 'Installed. Open Personal Files from the Ubuntu applications menu, or run ~/.local/bin/personal-files.'
