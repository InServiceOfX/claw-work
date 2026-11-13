# claw-shorts

A Rust tool that turns a reviewed spec into a voiced vertical Short for an
open-source project, using local FFmpeg and a local TTS engine such as
[`voxcpm-tts`](../voxcpm-tts). It validates the copy, narrates each scene,
renders the video, verifies it, and writes a contact sheet and a posting kit.

It **never uploads or posts anything**. Posting is a separate, manual step.

This is the Rust successor to the planning half of the Python
`Skills/claw-shorts` launch kit. The Python kit's other templates
(`demo_zoom`, `before_after`, `readme_scroll`, and so on) are not ported and
keep working.

## Why it exists: lessons from the Python launch kit

| Problem seen | What this tool does |
| --- | --- |
| The old plan emitted generic placeholders ("Tired of the old way?", a literal `{name}`, "Developers love X" for a repo with 0 stars) | `brief` writes a template of `TODO` text. `check` and `render` reject `TODO`, unfilled braces and the known generic phrases. |
| Superlatives with nothing behind them ("killer", "best", "beats") | A claim term in any narration, caption or on-screen text is an error unless the spec's `claims` has an entry with non-empty `evidence`. |
| Silent videos | Each scene is narrated by the local TTS and its length sets the scene length. |
| Text rendered as boxes (a newline or a symbol the font lacks) | Each string is drawn from its own text file, and on-screen glyphs the font cannot draw are rejected up front. |
| No proof on screen | `brief` collects measured results from the project's own docs as *candidates*. `check` warns when there is no visible URL, no number, no verdict, or no disclosure for a synthetic voice. |

## Build

Requirements: Rust 1.85+, FFmpeg with libx264 and `drawtext` (libfreetype), and
`ffprobe` on `PATH`. The default fonts are macOS system fonts; set `style.font`
and `style.mono_font` elsewhere. Narration needs `voxcpm-tts` on `PATH` and a
reference recording, unless every scene supplies its own `audio` file.

```bash
cargo build --release
cargo test
```

## Use

```bash
# 1. Collect facts and measured-result candidates from a project.
claw-shorts brief --repo ../voxcpm-tts --output /tmp/brief
#    -> short.template.json (cannot render: it is all TODO) and evidence.md

# 2. Write real copy from the evidence, then validate it.
claw-shorts check --spec short.json --strict

# 3. Narrate and render. Both directories are created; output must not exist.
claw-shorts render --spec short.json --work /tmp/work --output /tmp/out --strict
```

`render` writes `short.mp4`, `contact.png` (one frame per scene), `captions.srt`,
`timeline.json`, `verification.json`, `posting_kit.md` and `posting_kit.json`.
Narration is cached in the work directory by a hash of the text, voice and seed,
so re-rendering after a visual change does not repeat TTS; `--force-tts` does.

Other commands: `narrate` (audio only) and `kit` (posting text only).
See [`examples/voxcpm-tts.short.json`](examples/voxcpm-tts.short.json) for a
complete spec.

## The spec

Scenes play back to back. Each needs a `narration`, which is spoken and sets the
scene's length plus `pad_ms` of silence.

| `kind` | Shows | Fields |
| --- | --- | --- |
| `title` | Centered lines revealed one by one, last line in the accent color | `lines` (1-6) |
| `terminal` | A panel where `command` is typed, then `output` lines appear | `command`, `output` (up to 8) |
| `verdict` | Colored badges with a sentence each | `rows` of `mark`, `tone` (`pass`, `warn`, `fail`), `text` |
| `endcard` | Lines and the visible source URL | `lines`, `url` |

Top level: `project`, `title`, `caption`, `hashtags`, `repo_url`, `claims`,
`voice` (`reference`, `transcript`, `seed`, `binary`), `style`, and the optional
`disclosure` and `credits`.

`disclosure` and `credits` are optional. When present they appear on every scene
and in the caption. When a synthetic voice is used without a disclosure,
`check` warns, and the posting kit's checklist reminds you.

## Style recipe

Colors, fonts, size and pacing live in the spec's `style` or in a standalone
file passed with `--style`, so every Short from a project looks the same.
[`examples/style.json`](examples/style.json) is the built-in default; a test keeps
it in sync. The recipe is a plain file: save one only from choices you accepted
in a real edit.

## Design notes

The layout follows the checklist in `skills/canvas-explainer`: one idea per
beat, a limited palette, safe margins (content stays 90 px from the sides and
clear of the bottom 420 px where platform buttons sit), a contact sheet to
inspect, and timing taken from measured audio. The copy structure follows
`research/tiktok-itstundealao-2026-11-06/themes/03-fact-or-hype-format.md`:
a testable claim, a visible source, a live run, a measured result, a verdict.
That research notes correlation, not causation, with views.

`canvas-explainer`'s own renderer (Node and Playwright) was not used because
Playwright is not installed here; this tool draws with FFmpeg instead.

## Limits

- Captions on screen are the scene text, not a word-by-word transcript. The
  `captions.srt` has one cue per scene. Word timing would need a local Whisper
  model, which is not bundled.
- `verification.json` checks dimensions, duration, an audio stream and a full
  decode. It cannot judge speech quality or whether the video looks good. Listen
  to it and open `contact.png`.
- Layout uses fixed character-width ratios, so unusual fonts may need smaller
  text. Check the contact sheet.
- Reveal timing inside a scene is evenly spaced, not aligned to spoken words.
