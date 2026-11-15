#!/usr/bin/env bash
# Bring a HyperFrames render to the claw-shorts loudness rule and verify it.
#
#   scripts/finish.sh renders/raw.mp4 out/short.mp4
#
# HyperFrames mixes narration and SFX but does not normalize the mix to an
# absolute target. This measures the render (EBU R128), applies one linear
# loudnorm pass to -14 LUFS integrated / -1 dBTP with the measured values,
# copies the video stream untouched, fully decodes the result and fails when
# it misses by more than 1 LU or its true peak is above -1 dBTP. Writes <output>.loudness.txt.
set -euo pipefail

if [ "$#" -ne 2 ]; then
  echo "usage: $0 INPUT.mp4 OUTPUT.mp4" >&2
  exit 2
fi
in="$1"
out="$2"
if [ -e "$out" ]; then
  echo "refusing to overwrite $out" >&2
  exit 1
fi
target_i=-14
target_tp=-1.5 # the gate is -1 dBTP; AAC encoding overshoots the ceiling by a few tenths
target_lra=11

measure() {
  ffmpeg -hide_banner -nostats -i "$1" -af "loudnorm=I=$target_i:TP=$target_tp:LRA=$target_lra:print_format=json" \
    -f null - 2>&1 | sed -n '/^{/,/^}/p'
}
field() { python3 -c 'import json,sys; print(json.loads(sys.stdin.read())[sys.argv[1]])' "$1"; }

m=$(measure "$in")
mi=$(echo "$m" | field input_i)
mtp=$(echo "$m" | field input_tp)
mlra=$(echo "$m" | field input_lra)
mth=$(echo "$m" | field input_thresh)
off=$(echo "$m" | field target_offset)
echo "input:  $mi LUFS, $mtp dBTP"

ffmpeg -hide_banner -v error -i "$in" -map 0:v -map 0:a -c:v copy \
  -af "loudnorm=I=$target_i:TP=$target_tp:LRA=$target_lra:measured_I=$mi:measured_TP=$mtp:measured_LRA=$mlra:measured_thresh=$mth:offset=$off:linear=true,aresample=48000" \
  -c:a aac -b:a 192k -movflags +faststart "$out"

ffmpeg -hide_banner -v error -i "$out" -f null - # full decode
r=$(measure "$out")
oi=$(echo "$r" | field input_i)
otp=$(echo "$r" | field input_tp)
dims=$(ffprobe -v error -select_streams v:0 -show_entries stream=width,height,r_frame_rate -of csv=p=0 "$out")
dur=$(ffprobe -v error -show_entries format=duration -of csv=p=0 "$out")
report="output: $oi LUFS, $otp dBTP; video $dims, ${dur}s; full decode ok"
echo "$report"
printf 'input: %s LUFS, %s dBTP\n%s\n' "$mi" "$mtp" "$report" > "$out.loudness.txt"

python3 - "$oi" "$otp" <<'EOF'
import sys
i, tp = float(sys.argv[1]), float(sys.argv[2])
problems = []
if abs(i - (-14.0)) > 1.0:
    problems.append(f"integrated {i} LUFS is more than 1 LU from -14 (linear mode cannot raise a peaky mix: add a limiter or lower the SFX)")
if tp > -1.0:
    problems.append(f"true peak {tp} dBTP is above -1")
if problems:
    print("FAIL: " + "; ".join(problems), file=sys.stderr)
    sys.exit(1)
print("PASS: -14 LUFS +/-1, true peak <= -1 dBTP")
EOF
