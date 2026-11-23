#!/usr/bin/env bash
# Merge narration, footage, clip sound and cut ticks into the finished Short.
#
#   ./render.sh            # writes out-YYYYmmdd-HHMMSS/short.mp4 plus captions,
#                          # contact sheet, timeline and posting kit
#
# Needs clips/out/ (./make_clips.sh) and narration/scene_*.wav (./voice.sh).
# Scene lengths come from the narration, so re-voicing re-times the video.
set -euo pipefail
here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
for n in "$here"/narration/scene_*.txt; do
  [[ -f "${n%.txt}.wav" ]] || { echo "missing ${n%.txt}.wav; run ./voice.sh first" >&2; exit 1; }
done
[[ -f "$here/clips/out/galton.mp4" ]] || { echo "missing clips; run ./make_clips.sh first" >&2; exit 1; }

cargo build --release --quiet --manifest-path "$repo/Rust/claw-shorts/Cargo.toml"
shorts="$repo/Rust/claw-shorts/target/release/claw-shorts"
"$shorts" check --spec "$here/short.json"
"$shorts" render --spec "$here/short.json" --work "$here/work" --output "$here/out-$(date +%Y%m%d-%H%M%S)"
