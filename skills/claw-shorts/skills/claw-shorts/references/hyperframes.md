# HyperFrames mode

Use when a Short needs designed motion graphics timed to narration: listicles
("3 free tools"), explainers with diagrams, charts, kinetic captions. The
example project, helpers and measurements are in claw-work
`skills/claw-shorts/hyperframes/` (read its README first).

HyperFrames (heygen-com/hyperframes, Apache-2.0) renders HTML + a paused GSAP
timeline to MP4 in headless Chrome, locally. Pin the CLI in the project
(`hyperframes@0.9.50`); do not install it globally.

## Pipeline

1. Script with `short-form-strategy`; run its copy rules (`claw-shorts check`
   applies to the same words). Keep `narration.txt` (spelled for the voice) and
   `script.txt` (spelled for the screen) separate when they differ.
2. Narrate locally (`voxcpm-tts say`). Disclose the synthetic voice on the end card.
3. `npx hyperframes transcribe <wav> --engine parakeet --no-runtime-install`.
   Read the transcript: misheard names are normal and are fixed in step 4, but a
   missing word can mean the voice skipped it — listen to that spot.
4. `scripts/captions.py` builds the caption scene with the script's spelling and
   the transcript's timing.
5. One scene per idea in `compositions/`; a scene starts on its first word; text
   lands on the word that says it and then holds still. Vary entrances
   (whip, blur swap, rise, dip cut); never fade-and-slide everything.
6. SFX: few and exact, each with a visible cause (cut, chip landing, payoff).
   `start = picture time + 1 frame - peak`. Rotate takes of a repeated sound.
   uisfx cues are CC0 but synthesized UI sounds: use them for interface-like
   moments, not as a substitute for recorded foley.
7. `npx hyperframes lint` and `check` must both be clean; then
   `snapshot --review` and look at `review-beats.jpg` and the seam strips.
8. `npx hyperframes render`, then `scripts/finish.sh` (-14 LUFS, -1 dBTP, full
   decode). Report what was measured and that audio was not listened to.

## Safe area

Every word inside y 288-1248 px; x 120-888 px above y 840, x 120-780 px below it
(TikTok's buttons). Captions sit at about y 1090-1248. `check` enforces this.

## Local vs Docker

Render locally on macOS. For the 27 s example, warm renders took 13.5 s locally
and 20.6 s with `--docker`. The first runs took 50 s (cold caches, cause not isolated) and 170 s
(building a 3.4 GB image). Each path was byte-identical across its own runs, and
the two paths match visually (SSIM 0.993) but not byte for byte. `--docker` pins
Chromium, fonts and FFmpeg: use it for CI or a Linux box where the same bytes on
every machine matter. On Apple Silicon the image is linux/arm64,
which the HyperFrames docs say is not byte-identical to amd64 renders anyway.

## Cautions

- **Telemetry is on by default.** Set `HYPERFRAMES_NO_TELEMETRY=1`. The
  `/hyperframes` skill also asks the agent to run `npx hyperframes events`
  (a usage beacon); skip it.
- **`transcribe` may install software.** Without `--no-runtime-install` it can
  run `brew install whisper-cpp` or build whisper.cpp from source.
- **`hyperframes add` copies to the clipboard** (`clipboardCopied: true`).
- **Fonts and GSAP load from CDNs** at render time (Google Fonts beyond the
  bundled weights; jsDelivr for GSAP) unless vendored into `assets/`.
- Registry blocks are reference source, often 1920×1080: adapt them to the
  vertical frame and the safe area; do not mount them as they are.
- Never run `npx skills add ... --all` from inside a HyperFrames checkout (it
  overwrites the checkout's `skills/`).
