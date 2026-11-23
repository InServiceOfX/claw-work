#!/usr/bin/env bash
# Render one Manim scene to a silent vertical MP4 (1080x1920, 30 fps).
#
# Usage:
#   scripts/render.sh SCENE_FILE SCENE_CLASS OUT_DIR [--timeline timeline.json] [--words words.json]
#
# Writes OUT_DIR/video.mp4 (OUT_DIR/video.mp4 must not exist yet) and keeps
# Manim's working files in OUT_DIR/manim-media. --timeline takes the timeline.json
# from `claw-shorts canvas`, so each beat ends where its narration scene ends;
# --words takes words.json from Rust/claw-shorts/scripts/words.py. Then:
#   claw-shorts mux --video OUT_DIR/video.mp4 --audio narration.wav --output short.mp4
set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

usage() { sed -n '2,11p' "${BASH_SOURCE[0]}" | sed 's/^# \{0,1\}//'; exit 2; }
[[ $# -ge 3 ]] || usage
scene_file="$(cd "$(dirname "$1")" && pwd)/$(basename "$1")"
scene_class="$2"
out_dir="$3"
shift 3
timeline="" words=""
while [[ $# -gt 0 ]]; do
  case "$1" in
    --timeline) timeline="$(cd "$(dirname "$2")" && pwd)/$(basename "$2")"; shift 2 ;;
    --words) words="$(cd "$(dirname "$2")" && pwd)/$(basename "$2")"; shift 2 ;;
    *) usage ;;
  esac
done

[[ -f "$scene_file" ]] || { echo "no such scene file: $scene_file" >&2; exit 1; }
command -v ffprobe >/dev/null || { echo "ffprobe (FFmpeg) is required" >&2; exit 1; }
command -v uv >/dev/null || { echo "uv is required (https://docs.astral.sh/uv/)" >&2; exit 1; }
# MathTex needs a LaTeX install (MacTeX/BasicTeX on macOS, texlive on Linux).
if ! command -v latex >/dev/null && [[ -x /Library/TeX/texbin/latex ]]; then
  export PATH="/Library/TeX/texbin:$PATH"
fi
command -v latex >/dev/null || echo "warning: no latex on PATH; MathTex/Tex will fail (Text still works)" >&2

mkdir -p "$out_dir"
out_dir="$(cd "$out_dir" && pwd)"
[[ ! -e "$out_dir/video.mp4" ]] || { echo "$out_dir/video.mp4 already exists" >&2; exit 1; }

start=$(date +%s)
CLAW_TIMELINE="$timeline" CLAW_WORDS="$words" \
  uv run --quiet --python 3.13 --project "$here" \
  manim render -v WARNING --config_file "$here/vertical.cfg" --media_dir "$out_dir/manim-media" \
  -o video "$scene_file" "$scene_class"
elapsed=$(( $(date +%s) - start ))

stem="$(basename "$scene_file" .py)"
rendered="$out_dir/manim-media/videos/$stem/1920p30/video.mp4"
[[ -f "$rendered" ]] || { echo "manim finished but $rendered is missing" >&2; exit 1; }
cp "$rendered" "$out_dir/video.mp4"

duration=$(ffprobe -v error -show_entries format=duration -of csv=p=0 "$out_dir/video.mp4")
size=$(ffprobe -v error -select_streams v:0 -show_entries stream=width,height -of csv=s=x:p=0 "$out_dir/video.mp4")
sha=$(shasum -a 256 "$out_dir/video.mp4" | cut -d' ' -f1)
echo "video: $out_dir/video.mp4"
echo "  ${size}, ${duration} s, rendered in ${elapsed} s, sha256 ${sha}"
echo "next: claw-shorts mux --video $out_dir/video.mp4 --audio NARRATION.wav --output short.mp4"
