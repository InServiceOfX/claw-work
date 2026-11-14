# Stock B-roll mode

Formerly the `short-form-video-assembly` skill in Monoclaw (merged 2026-11-14).

Drive the `shortform-video` Rust CLI (claw-work `Rust/shortform-video`) to turn a subject or a
finished script into a rendered, captioned vertical video. The pipeline is:
script → search terms → TTS narration → subtitles → stock b-roll → ffmpeg
composition. Ported from MoneyPrinterTurbo; see
claw-work `docs/shortform/MONEYPRINTERTURBO_ASSESSMENT.md` for provenance.

Use this mode for a fully automated faceless video from stock footage or a
local folder of clips and photos. Plan the script with the `short-form-strategy`
skill first when reach matters, then pass it with `--script-file`.

## Prerequisites

- Binary: `Rust/shortform-video/target/release/shortform-video` in a
  claw-work checkout (build with `cargo build --release` in that directory;
  it depends on `../claw-shorts` for loudness).
- `ffmpeg`/`ffprobe` on PATH. Slim builds are fine (captions don't need
  libass).
- LLM stages need an OpenAI-compatible endpoint (`SHORTFORM_LLM_BASE_URL`,
  `SHORTFORM_LLM_MODEL`; local llama-server/mlx/LiteLLM works; `<think>`
  blocks are stripped automatically).
- Stock sourcing needs `SHORTFORM_PEXELS_API_KEY` or
  `SHORTFORM_PIXABAY_API_KEY`. A local directory works with no key:
  `--source /path/to/folder`. Videos (.mp4/.mov/.m4v/.webm/.mkv) are cut;
  photos (.jpg/.jpeg/.png/.webp) get a slow Ken Burns zoom and drift for one
  clip's length (`--max-clip-duration`), alternating direction.
- Edge TTS needs network; `SHORTFORM_TTS_BACKEND=say` is the offline macOS
  fallback (subtitle timing degrades from word-accurate to proportional).
- Config file (optional): `~/.config/shortform-video/config.toml` — see the
  crate README for the full schema.

## Procedure

1. **Structure first when reach matters.** Load `short-form-strategy`,
   draft the script with a cold-open hook (no "welcome to this video" — the
   LLM prompt already forbids it, but hook quality is on you), and save it to
   a file.
2. **Assemble.**
   - From a finished script:
     `shortform-video run --subject "<subject>" --script-file script.txt --aspect portrait`
   - From just a subject (LLM writes the script):
     `shortform-video run --subject "<subject>" --aspect portrait`
   - Useful flags: `--source pexels|pixabay|<local-dir>`, `--concat
     sequential` (visuals follow script order; search terms are generated in
     narrative order), `--transition fade`, `--bgm music.mp3`,
     `--language zh-CN`, `--paragraphs 2`, `--task-dir out/`.
3. **QC before showing the human.** Play `final.mp4`. Check: captions match
   narration and fit on screen; audio present and voice louder than BGM
   (defaults: voice 1.0, bgm 0.2); aspect correct; no black tail at the end.
   Loudness is fixed at -14 LUFS / -1 dBTP and measured into `loudness.json`;
   the run fails if it misses.
   Artifacts (script.txt, subtitle.srt, materials.json with attribution
   pages) are all in the task dir for inspection.
4. **Publishing metadata.**
   `shortform-video social --subject "..." --script-file script.txt --platform tiktok`
   gives title/caption/hashtags within per-platform limits.
5. **Publishing is human-gated. Never auto-post.** Present the video and
   metadata; the human uploads. Stock attribution pages are recorded in
   `materials.json` — surface them when a platform or license requires
   credit.

## Per-stage commands (for partial workflows)

```bash
shortform-video script --subject "..."            # script only
shortform-video terms --subject "..." --script-file s.txt --ordered
shortform-video tts --script-file s.txt --output narration.mp3 --srt
shortform-video voices --locale en-US             # list Edge voices
```

## Failure modes

- `edge TTS connection failed` → network/endpoint issue; retry or fall back:
  `SHORTFORM_TTS_BACKEND=say`.
- `no usable video materials` → missing/invalid stock API key, or terms too
  niche; retry with broader `--terms-amount`, the other provider, or a local
  clips directory.
- `no subtitle font found` → set `SHORTFORM_FONT_PATH` to any .ttf.
- LLM errors → check the endpoint is up (for local mlx: `./serve.sh` in
  `Deployments/Scripts/mlx` in Monoclaw, or claw-work
  `Deployments/llama-cpp-server-macos`).
