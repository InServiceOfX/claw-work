# claw-shorts HyperFrames mode

A third renderer for claw-shorts, next to the FFmpeg path (`Rust/claw-shorts render`)
and the canvas path (`canvas/`). [HyperFrames](https://github.com/heygen-com/hyperframes)
(Apache-2.0, HeyGen) renders HTML + GSAP to MP4 by seeking a paused timeline frame by
frame in headless Chrome. What it adds over our canvas path:

- **Word timings in seconds, locally**: `hyperframes transcribe` with NVIDIA Parakeet
  (about 640 MB, sha256-pinned, in `~/.cache/hyperframes`) transcribed 24.7 s of
  narration in 2.5 s on an M4 Pro.
- **A browser gate before rendering**: `hyperframes check` measures layout against
  TikTok, Reels and Shorts safe areas, text contrast, unreadably short holds, text
  hidden under other elements, and runtime errors. `snapshot --review` writes a
  contact sheet of every scene and a strip around every cut.
- **About 450 registry blocks** (`hyperframes catalog`): caption styles, transitions,
  charts, kinetic type, installed as editable source.
- **Placed audio**: `<audio>` clips are mixed at their `data-start`, so SFX land on
  the frame; `hyperframes audio` reports each file's onset and peak.

It does **not** normalize loudness to an absolute target or align captions to the
script's own spelling; `scripts/finish.sh` and `scripts/captions.py` add those.

## This folder

| Path | What |
| --- | --- |
| `frame.md` | Brand spec HyperFrames reads (palette from `Rust/claw-shorts/examples/style.json`, type, the safe area) |
| `index.html`, `compositions/*.html` | Example: a 27 s "3 free tools" listicle, one scene per idea, timed to its narration |
| `narration.txt`, `script.txt` | What the voice reads ("Gee-sap", so it is said right) and what the captions show ("GSAP") |
| `scripts/captions.py` | Caption track from the script's words + the transcript's timings (stdlib only) |
| `scripts/finish.sh` | Normalize the render to -14 LUFS / -1 dBTP, full decode, pass/fail |
| `scripts/sfx.sh` | Copy the six uisfx cues (CC0) from the pinned npm package into `assets/sfx/` |

`assets/` (narration, transcript, SFX) and renders are not committed.

## Run it

Node 22+ and FFmpeg. Work in a copy outside the repository so generated media
never lands in Git:

```bash
cp -R skills/claw-shorts/hyperframes "$WORK/short" && cd "$WORK/short"
export HYPERFRAMES_NO_TELEMETRY=1      # the CLI reports usage by default
npm ci                                 # hyperframes 0.9.50 + uisfx 0.4.0, project-local (lockfile)
npm run sfx

# 1. Narration (local, synthetic, disclosed on the end card).
voxcpm-tts say narration.txt -o assets/narration.wav --seed 7 --paragraph-gap 0.4

# 2. Word timings (one-time model install; never let it fall back to Homebrew).
npx hyperframes models install parakeet
npx hyperframes transcribe assets/narration.wav --engine parakeet --no-runtime-install

# 3. Captions with the script's spelling.
python3 scripts/captions.py --script script.txt --transcript assets/transcript.json \
  --out compositions/captions.html --highlight HyperFrames,GSAP,zero,Seventy-eight

# 4. Gate, look, render, finish.
npx hyperframes lint && npx hyperframes check
npx hyperframes snapshot --review --output review   # open review/review-beats.jpg
npx hyperframes render --output renders/raw.mp4
scripts/finish.sh renders/raw.mp4 out/short.mp4
```

For a new video, retime the scene hosts in `index.html` (`data-start`,
`data-duration`) and the anchor times in each scene from `assets/transcript.json`:
a scene starts on its first word, text lands on the word that says it. SFX
`data-start` = picture time + 1 frame - the cue's peak (`npx hyperframes audio
assets/sfx/<cue>.mp3 --json`).

## Safe area

`check` uses the platforms' own vertical-ad guides (HyperFrames
`packages/core/src/safeAreas.ts`, checked 2026-10-13). Scaled to 1080×1920, the
area clear on TikTok, Reels and Shorts at once is:

| Band | x |
| --- | --- |
| y 288-840 | 120-888 |
| y 840-1248 | 120-780 (TikTok's action buttons) |

That is tighter than the older claw-shorts rule (90 px sides, text above
1330 px). The example keeps scene content above 840 px, captions at 1090-1248 px
and every word between x 120 and 780 in the lower band.

## Measured (2026-11-14, Mac mini M4 Pro, 64 GB)

27 s, 1080×1920, 30 fps, narration + 8 SFX:

| Step | Result |
| --- | --- |
| `npm install hyperframes@0.9.50` | 10 s, 108 MB, uses the installed Chrome and Homebrew FFmpeg 8.0.1 |
| VoxCPM2 narration (native engine) | 24.8 s of audio in about 1 min 45 s |
| Parakeet transcription | 2.5 s |
| `check` | about 10 s; 0 errors, 0 warnings after fixes |
| Local render | 50 s first (cold) run, 13.5 s after (cause of the difference not isolated: font fetch and caches are likely); two runs byte-identical |
| `--docker` render | 170 s first run (builds a 3.4 GB `hyperframes-renderer:0.9.50-arm64` image), 20.6 s after; two runs byte-identical |
| Local vs Docker frames | not byte-identical, visually the same: SSIM 0.993, PSNR 34.5 dB |
| `finish.sh` | -20.0 → -14.2 LUFS, -1.37 dBTP, full decode, PASS |
| SFX placement | all 8 cues within 4 ms of plan by narration-subtracted onset analysis, except one cue whose loudest point is 2 frames after its reported peak |

Audio was measured, not listened to. Listen before posting.
