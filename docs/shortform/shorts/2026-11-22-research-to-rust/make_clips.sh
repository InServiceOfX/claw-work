#!/usr/bin/env bash
# Render the footage this Short plays: two claw-motion clips and the Manim
# example. Deterministic, so re-running gives the same files. Run from anywhere.
set -euo pipefail
here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
out="$here/clips/out"
mkdir -p "$out"

cargo build --release --quiet --manifest-path "$repo/Rust/claw-motion/Cargo.toml"
motion="$repo/Rust/claw-motion/target/release/claw-motion"
"$motion" physics "$here/clips/galton.toml" -o "$out/galton.mp4" --force >/dev/null
"$motion" morph "$here/clips/morph.toml" -o "$out/morph.mp4" --force >/dev/null

rm -rf "$out/manim"
"$repo/skills/claw-shorts/manim/scripts/render.sh" \
  "$repo/skills/claw-shorts/manim/scenes/small_angle.py" SmallAngle "$out/manim" >/dev/null

ls -1 "$out"/galton.mp4 "$out"/morph.mp4 "$out"/manim/video.mp4
