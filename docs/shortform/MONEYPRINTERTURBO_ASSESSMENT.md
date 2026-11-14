# MoneyPrinterTurbo → shortform-video: what we took, what we dropped, and why

Assessed 2026-09-02 against `workspace/repos/harry0703/MoneyPrinterTurbo`
(≈9.5k lines in the core services alone). Result of the assessment: **don't
adopt the app; port the algorithms.** The port lives at
`Rust/shortform-video/` (moved from Monoclaw to claw-work on 2026-11-14) (Rust + ffmpeg CLI, per the workspace stack
preference Rust > Python; TypeScript reserved for any future frontend).

## What MoneyPrinterTurbo is

A Python app (FastAPI backend + Streamlit WebUI) that assembles "faceless"
short videos: LLM writes a narration script and stock-search terms → TTS
narrates → subtitles are aligned → stock/AI b-roll is fetched → MoviePy/ffmpeg
compose a captioned 9:16/16:9 video, optionally cross-posted to social
platforms.

## Kept (ported into `Rust/shortform-video`)

| MoneyPrinterTurbo source | What it is | Where it went |
|---|---|---|
| `services/llm.py` script system prompt + terms prompt | battle-tested prompts (raw-script constraints; 1–3-word English terms as JSON array; ordered-terms mode) | `src/llm.rs` (verbatim prompts) |
| `llm.py` `<think>` stripping, code-fence strip, `\[.*\]` JSON recovery | makes local Qwen/DeepSeek reliable | `src/llm.rs` |
| `material.py` orientation matching, min-duration filter, URL dedup, duration budgeting (`min(clip, max_clip)` until audio covered) | the actual economics of stock sourcing | `src/material.rs` (Pexels + Pixabay) |
| `voice.py` edge cue aggregation (`_build_subtitle_items_from_edge_cues`) | word boundaries accumulated until they match a punctuation-split script line; keeps CJK captions readable and preserves real speech pauses | `src/subtitle.rs` `cues_from_word_boundaries` |
| `task.py` "measure the real audio file, not the last cue" fix | Edge TTS leaves a ~0.88 s tail after the final word boundary; under-counting starves material sourcing | `src/tts.rs` (ffprobe on written file) |
| `video.py` cover/contain fit math | exact-canvas normalize before concat | two ffmpeg one-liners in `src/video.rs::fit_filter` (scale+crop / scale+pad) |
| `video.py` `combine_videos` planning | subclip at `max_clip_duration`, sequential = one chunk per source, unique-source prioritization, loop pool until audio+0.1 s covered | `src/video.rs::plan_segments` (pure function, unit-tested) |
| `video.py` BGM handling | volume scale, 3 s fade-out, loop to video length | ffmpeg `amix`/`afade`/`-stream_loop` in `src/video.rs` |
| `llm.py` `SOCIAL_PLATFORMS` limits + social metadata prompt | per-platform title/caption/hashtag budgets | `src/llm.rs::PlatformLimits` |
| Pipeline shape with `stop_at` stages and per-task artifact dir | resumability + inspectability | CLI subcommands per stage + task dir in `src/pipeline.rs` |

## Dropped, and why

- **Streamlit WebUI + FastAPI backend + Redis state** — we're a CLI/agent
  shop; harnesses drive the pipeline directly. State machine, cross-post
  thread pools, Windows process liveness checks: all complexity that serves a
  hosted multi-user product we don't run.
- **China-market paid video-gen providers** (LoomLoom, VolcEngine Seedance,
  OFox, Metaso MiniMax, WaveSpeed, Sonilo BGM, SiliconFlow/MiniMax/Mimo TTS) —
  billing-coupled vendor glue, wrong market for us.
- **MoviePy** — replaced entirely with ffmpeg filter graphs. Fewer moving
  parts, no Python runtime, and the slim homebrew ffmpeg constraint (no
  libass/drawtext) is handled by rendering caption PNGs in Rust and using
  `overlay=enable='between(t,a,b)'` — the same trick claw-dj's
  `make_overlays.py` uses.
- **Whisper subtitle path** (`subtitle.py`) — only needed for
  bring-your-own-audio; deferred (whisper.cpp subprocess would be the port).
- **Upload-Post cross-posting** — auto-publishing is exactly the step that
  should stay human-gated per workspace safety rules. The `social` subcommand
  generates the metadata; a human posts.
- **edge-tts Python dependency** — replaced with the native Rust `msedge-tts`
  WebSocket client (word boundaries included), with macOS `say` as the
  offline fallback.

## How this complements what already existed

The 2026-09 setup had three distinct short-form modes. Since 2026-11-14 they
live in claw-work as the `short-form-strategy` skill plus the `claw-shorts`
production skill:

1. **Structure** — `skills/short-form-strategy/` (Noe
   Murillo hook/retention/engagement framework).
2. **Shoot** — `docs/shortform/obs-deck-pipeline/` (HTML deck + OBS single
   live take, for equation-heavy explainers with your own voice).
3. **Assemble** (new) — `Rust/shortform-video/` (fully automated faceless
   pipeline: TTS narration + stock b-roll + burned captions).

claw-dj's `agent/hermes-skill/scripts/ab-shorts/` remains the bespoke path for
mix-audio teasers; its xfade chain is the model for this crate's future
cross-transition support, and this crate's caption renderer can replace its
hand-made overlay PNGs.

## Verification status (2026-09-02)

- `cargo test`: 27/27; `cargo clippy`: clean.
- Offline E2E (lavfi materials + `say` + captions + BGM): 1080×1920 h264/aac,
  duration matches narration, captions render correctly (frame-inspected).
- Live Edge TTS: voice listing + synthesis verified; 24 word boundaries on the
  test script, SRT aligned to real speech pauses.
- Not yet exercised: Pexels/Pixabay search+download (needs API keys) and the
  LLM stages against a running local server (needs endpoint up). Both are
  direct ports of working logic; first live run should be watched.
