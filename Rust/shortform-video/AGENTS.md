# AGENTS.md — Rust/shortform-video

## What this is

Faceless short-form video assembly pipeline (see README.md). Ported/refactored
from MoneyPrinterTurbo's Python services into idiomatic Rust + ffmpeg-CLI
composition. Not PDD-managed.

## Commands

```bash
cargo build --release
cargo test          # unit tests, offline
cargo clippy        # keep it at zero warnings
```

Offline end-to-end verification (no keys, no network) — generate synthetic
materials and run with `say`:

```bash
ffmpeg -f lavfi -i "testsrc2=size=1920x1080:rate=30:duration=8" -pix_fmt yuv420p /tmp/m/clip.mp4
printf "One sentence. Another sentence." > /tmp/script.txt
SHORTFORM_TTS_BACKEND=say ./target/release/shortform-video run \
  --subject test --script-file /tmp/script.txt --source /tmp/m \
  --aspect portrait --task-dir /tmp/sfv-task
# verify: ffprobe /tmp/sfv-task/final.mp4 → 1080x1920 h264 + aac, duration ≈ narration
```

## Conventions

- Module boundaries mirror pipeline stages (`llm`, `material`, `tts`,
  `subtitle`, `caption`, `video`, `pipeline`). Keep filter-graph string
  builders pure functions so they stay unit-testable.
- ffmpeg is always invoked as a subprocess with `-y -hide_banner -loglevel
  error`; never add a video-processing crate dependency for what an ffmpeg
  filter can do.
- The slim homebrew ffmpeg on this machine has **no libass/drawtext**; caption
  burn-in must stay on the PNG-overlay path.
- Loudness comes from `../claw-shorts` (`claw_shorts::loudness`). Change the
  target or the limiter step there, once, for both renderers.
- rustls: both reqwest and msedge-tts are in the tree; `main.rs` installs the
  ring provider at startup. Don't remove that line.
- Git scope follows the repository `BOTS.md` and `AGENTS.md`.

## Known limitations / next steps

See PROGRESS.md. Biggest: no xfade cross-transitions between segments (only
per-segment fade-in), no Whisper subtitle path for bring-your-own audio, no
BGM library management (pass `--bgm <file>` explicitly).
