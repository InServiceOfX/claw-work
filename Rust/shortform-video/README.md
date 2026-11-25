# shortform-video

Faceless short-form video assembly in Rust: subject → LLM script → TTS
narration → script-aligned captions → stock b-roll → captioned 9:16 (or 16:9,
1:1) render. The composition layer is pure ffmpeg CLI — no MoviePy, no Python
runtime.

Core algorithms (duration budgeting, cover/contain fitting, word-cue → script
line subtitle aggregation, search-term and script prompts) are ported from
[MoneyPrinterTurbo](https://github.com/harry0703/MoneyPrinterTurbo) (MIT). See
`docs/shortform/MONEYPRINTERTURBO_ASSESSMENT.md` for what was kept,
dropped, and why.

## Build

```bash
cargo build --release        # binary at target/release/shortform-video
cargo test                   # pure unit tests, no network needed
```

Requires `ffmpeg`/`ffprobe` on PATH (a slim build is fine — no libass or
drawtext needed; captions are PNG overlays rendered in-process).

## Quick start

```bash
# Fully offline smoke test: your own script, macOS `say` voice, local clips
# and stills (.mp4/.mov/.m4v/.webm/.mkv and .jpg/.jpeg/.png/.webp)
SHORTFORM_TTS_BACKEND=say shortform-video run \
  --subject "money as a tool" \
  --script-file script.txt \
  --source /path/to/dir-of-clips-and-photos \
  --aspect portrait --transition fade --task-dir out

# Full pipeline (needs an LLM endpoint + a Pexels key + network for Edge TTS)
export SHORTFORM_LLM_BASE_URL=http://127.0.0.1:8080/v1   # llama-server / mlx / LiteLLM
export SHORTFORM_LLM_MODEL=qwen3.5-9b
export SHORTFORM_PEXELS_API_KEY=...
shortform-video run --subject "why the ocean glows at night" --aspect portrait

# Individual stages
shortform-video script --subject "..." [--language zh-CN] [--paragraphs 2]
shortform-video terms --subject "..." --script-file script.txt [--ordered]
shortform-video tts --script-file script.txt --output narration.mp3 --srt
shortform-video voices --locale en-US
shortform-video social --subject "..." --script-file script.txt --platform tiktok
```

## Configuration

`~/.config/shortform-video/config.toml` (all sections optional), overridden by
`SHORTFORM_*` environment variables (`PEXELS_API_KEY`/`PIXABAY_API_KEY` also
work):

```toml
[llm]
base_url = "http://127.0.0.1:8080/v1"  # any OpenAI-compatible server
api_key = ""
model = "default"

[materials]
pexels_api_key = ""
pixabay_api_key = ""

[tts]
backend = "edge"            # "edge" (network, word-level timing) or "say" (offline)
voice = "en-US-JennyNeural" # `shortform-video voices` lists Edge voices
rate = 1.0

[render]
font_path = ""              # default: probes Arial Bold (macOS), then Liberation/DejaVu (Linux)
font_size = 64
fps = 30
crf = 20
voice_volume = 1.0
bgm_volume = 0.2
sfx_volume = 0.25           # shutter clicks (--still-sfx)
frame_background = "#0f141b" # --still-layout frame: brand background
frame_accent = "#f5b83d"     # --still-layout frame: card border
```

`<think>…</think>` blocks are stripped from LLM responses automatically, so
local reasoning models (Qwen, DeepSeek) work out of the box.

## Task artifacts

Each `run` writes into a task directory (default `./shortform-tasks/<ts>/`):
`script.txt`, `terms.json`, `narration.mp3`, `subtitle.srt`, `materials/`,
`materials.json` (attribution/source pages), `captions/*.png`, `segments/`,
`combined.mp4`, `final.mp4`, `loudness.json` (measured on `final.mp4`).

## Design notes

- **Stills move (Ken Burns).** A photo in a local `--source` folder holds for
  one clip (`--max-clip-duration`) with an 8% zoom and a sideways drift;
  consecutive stills alternate zoom in/out and drift left/right. The still is
  cover-fitted at twice the canvas size first, so zoompan's whole-pixel steps
  do not jitter. Lesson from @ginyboi (see
  `skills/short-form-strategy/references/creator-lessons.md`).
- **B-roll space (`--still-layout frame`).** Instead of cropping a photo to
  fill 9:16, show the whole image as a bordered card on the brand background
  (`render.frame_background`, `render.frame_accent`), in the band from 12% to
  70% of the height so it stays clear of the platform UI and the captions.
  Use it for app screenshots and wide images. Ken Burns still applies.
- **Shutter clicks (`--still-sfx`).** A camera-shutter click on every cut to a
  photo, synthesized in `src/sfx.rs` (no audio file, nothing to license),
  placed at the measured start of each still segment and mixed at
  `render.sfx_volume`. Eye-line alignment for fast portrait cuts is not built:
  it needs a face-detection model, and these explainers rarely cut portraits.
- **Segments are limited-range yuv420p.** JPEGs decode as full-range
  yuvj420p; joined by stream copy with limited-range segments, the colors
  after a JPEG were misread. Every segment is now converted.
- **Audio renders in its own ffmpeg run**, then is muxed with the captioned
  video. In one shared filter graph with the caption overlays, identical
  inputs gave a different audio track on each run (sometimes ~50 ms early,
  dropping a click); the audio-only graph is byte-for-byte repeatable.
- **Loudness is -14 LUFS / -1 dBTP on every render**, using
  `claw-shorts`' loudness module (path dependency): measure the voice+BGM mix,
  boost into a limiter only when linear gain would clip, then linear
  `loudnorm`. The run fails if the finished file is more than 1 LU off or
  clips. `voice_volume`/`bgm_volume` still set the balance; the final level is
  fixed.
- **Captions are pre-rendered PNGs** (ab_glyph + image) overlaid with
  `overlay=…:enable='between(t,start,end)'`. This works with slim ffmpeg
  builds and gives full control of wrap, outline, and the rounded background.
- **Subtitle timing** uses Edge TTS word boundaries aggregated to script
  lines (real speech pauses land in the timeline); when boundaries are
  unavailable or don't line up, timing falls back to per-line proportional
  allocation over the measured audio duration.
- **Narration duration is measured from the audio file**, not the last word
  boundary — Edge TTS leaves a ~0.9 s tail that would otherwise truncate
  material sourcing and the final cut.
- Publishing is deliberately out of scope: posting to TikTok/YouTube stays a
  human-gated step (see the workspace skill for the workflow).
