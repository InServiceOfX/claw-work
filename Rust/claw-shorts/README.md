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

Other commands: `narrate` (audio only), `kit` (posting text only), and the canvas
path below (`canvas`, `mux`).
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
| `clone` | Reference voice in, generated voice out (canvas path draws both waveforms) | `lines` (exactly 2 labels) |
| `endcard` | Lines and the visible source URL | `lines`, `url` |

Top level: `project`, `title`, `caption`, `hashtags`, `repo_url`, `claims`,
`voice` (`reference`, `transcript`, `seed`, `binary`), `style`, and the optional
`disclosure`, `credits`, `scoreboard` (up to 4 short real numbers) and
`engagement` (`device`, `token`, `prompt`, `pinned_comment`).

`disclosure` and `credits` are optional. When present they appear on every scene
and in the caption. When a synthetic voice is used without a disclosure,
`check` warns, and the posting kit's checklist reminds you.

## The canvas path (richer motion)

The FFmpeg path draws text, a terminal and badges. The canvas path adds what
FFmpeg cannot: a live waveform of the narration from the first frame, the
reference clip's waveform, a camera push-in with a scale punch on every cut, a
close-up on the terminal output, and word-by-word burned-in captions.

```bash
# 1. Narrate, measure loudness, and write a self-contained scene.html.
claw-shorts canvas --spec short.json --work work --output prep --strict
# 2. Word timings from the narration (local Whisper; see Setup below).
.venv/bin/python scripts/words.py prep/narration.wav words.json
# 3. Rebuild the scene with the captions data (narration is cached).
claw-shorts canvas --spec short.json --work work --output scene --words words.json --strict
# 4. Render frames with skills/canvas-explainer's renderer (856 frames took 2 min on an M5).
render.mjs scene/scene.html frames SECONDS 30
# 5. Join video and narration, verify, and write a contact sheet.
claw-shorts mux --video frames/video.mp4 --audio scene/narration.wav \
  --output short.mp4 --timeline scene/timeline.json
```

`canvas` prints the exact SECONDS for step 4. The scene file embeds all data and
loads nothing, so it satisfies the renderer's no-network rule.

**Setup.** Node and a pinned Playwright (`playwright@1.64.1`) in a project outside
the repo (`npm ci`), with `CANVAS_BROWSER_CHANNEL=chrome` to use the installed
Chrome instead of downloading one. For word timings, `uv venv --python 3.13 .venv`
and `uv pip install mlx-audio==0.6.4`; `scripts/words.py` then runs the locally
cached `mlx-community/whisper-large-v3-turbo-asr-fp16` offline.

## Rules taken from the short-form research

`check` turns the research into rules, so a draft cannot skip them:

| Lever | Rule |
| --- | --- |
| Hook: no greeting | The opening narration or on-screen text starting with "hey guys", "welcome back", "today we're going to" and similar is an **error**. |
| Hook: readable muted | An on-screen hook over 8 words is a warning. |
| Hook: motion in the first seconds | The canvas scene animates a waveform from frame 0. |
| Retention | A scene held over 10 s is a warning from `canvas`. |
| Close reel: one real number | `scoreboard` appears from about 1.4 s and returns on the end card. Use the same numbers in narration, terminal output and caption. |
| Engagement | No `engagement` device is a warning. A device must name a token (`like`, `comment`, `save`, `share`); its prompt appears on the end card and its pinned comment in the posting kit. |
| Fact-or-hype | A visible URL, a number, a verdict and a disclosure are checked as warnings; superlatives need evidence. |

Sources are credited by name, not linked: the hook, retention and engagement
framework follows Noe Murillo's two "Successful Content Creator" videos, the
scoreboard and close-reel shape follows an a13x_duke build-in-public reel, and the
claim-source-demo-number-verdict structure follows @itstundealao's "Fact or Hype?"
videos (see `research/tiktok-itstundealao-2026-11-06/`). Their performance figures
are their own claims, and the views-versus-format link is correlation.

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

The FFmpeg path needs no browser. The canvas path uses `canvas-explainer`'s own
renderer (Node and Playwright).

## Limits

- On the FFmpeg path, on-screen captions are the scene text and `captions.srt` has
  one cue per scene. Word-by-word captions come from the canvas path with a
  `--words` file; they show Whisper's spelling ("API", "21"), not the narration text.
- `verification.json` checks dimensions, duration, an audio stream and a full
  decode. It cannot judge speech quality or whether the video looks good. Listen
  to it and open `contact.png`.
- Layout uses fixed character-width ratios, so unusual fonts may need smaller
  text. Check the contact sheet.
- Reveal timing inside a scene is evenly spaced, not aligned to spoken words.
