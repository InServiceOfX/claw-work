#!/usr/bin/env bash
# Voice every scene of this Short with the local TTS.
#
#   ./voice.sh path/to/reference.wav   # voxcpm-tts, the voice to publish with
#   ./voice.sh --say                   # macOS `say`, a rough timing preview only
#
# Writes narration/scene_NN.wav from narration/scene_NN.txt. The reference must
# be one speaker you are authorized to use (for example a CC0 LibriVox reader).
# Set VOXCPM_SEED to change the take; the default keeps takes repeatable.
set -euo pipefail
here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
[[ $# -eq 1 ]] || { sed -n '2,9p' "${BASH_SOURCE[0]}" | sed 's/^# \{0,1\}//'; exit 2; }

for txt in "$here"/narration/scene_*.txt; do
  wav="${txt%.txt}.wav"
  if [[ "$1" == "--say" ]]; then
    say -o "${wav%.wav}.aiff" -f "$txt"
    ffmpeg -loglevel error -y -i "${wav%.wav}.aiff" -ar 48000 -ac 1 "$wav"
    rm -f "${wav%.wav}.aiff"
  else
    [[ -f "$1" ]] || { echo "no such reference: $1" >&2; exit 1; }
    voxcpm-tts say "$txt" -o "$wav" --voice "$1" --seed "${VOXCPM_SEED:-42}" --force
  fi
  printf '%s  %ss\n' "$(basename "$wav")" "$(ffprobe -v error -show_entries format=duration -of csv=p=0 "$wav")"
done
