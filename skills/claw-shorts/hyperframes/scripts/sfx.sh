#!/bin/sh
# Copy the example's six uisfx cues (CC0 1.0, see node_modules/uisfx/LICENSE-AUDIO)
# from the pinned npm package into assets/sfx/. Run after `npm ci`.
set -eu
src=node_modules/uisfx/sounds
[ -d "$src" ] || { echo "run npm ci first ($src missing)" >&2; exit 1; }
mkdir -p assets/sfx
for pair in cinematic/swipe:swipe-cinematic studio/swipe:swipe-studio soft/swipe:swipe-soft \
            studio/snap:snap-studio studio/badge:badge-studio cinematic/complete:complete-cinematic; do
  cp "$src/${pair%%:*}.mp3" "assets/sfx/${pair##*:}.mp3"
done
echo "6 cues -> assets/sfx/"
