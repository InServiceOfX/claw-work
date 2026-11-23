# claw-shorts

The production skill for faceless vertical short-form video (TikTok, Reels,
Shorts), plus two tools that live with it: the canvas renderer and the Python
template CLI. Local/offline rendering only; never posts.

Since 2026-11-14 this folder holds what used to be four skills:

| Was | Now |
| --- | --- |
| `claw-shorts` (Python launch kit) | Launch kit retired (it emitted generic copy); templates kept below |
| `canvas-explainer` | `canvas/` (renderer) + `skills/claw-shorts/references/canvas*.md` |
| `transcript-video-edit` | `skills/claw-shorts/references/recording-cut*.md` (tool: `Rust/claw-edit`) |
| `short-form-video-assembly` (Monoclaw) | `skills/claw-shorts/references/stock-broll.md` (tool: `Rust/shortform-video`) |

Planning (hooks, retention, engagement, script rhythm) is the separate
[`short-form-strategy`](../short-form-strategy/SKILL.md) skill.

## Which tool

| Mode | Tool |
| --- | --- |
| Voiced explainer with validated copy and -14 LUFS audio | [`Rust/claw-shorts`](../../Rust/claw-shorts/README.md) |
| Canvas motion graphics | `Rust/claw-shorts canvas` + [`canvas/`](canvas/) |
| Designed motion graphics with word-timed captions (HyperFrames) | [`hyperframes/`](hyperframes/README.md) |
| Animated math: equations, graphs, derivations (Manim CE, used as is) | [`manim/`](manim/README.md) |
| Physics demos and shape morphs with collision-timed sound | [`Rust/claw-motion`](../../Rust/claw-motion/README.md) |
| Stock B-roll or photo videos (Ken Burns) | [`Rust/shortform-video`](../../Rust/shortform-video/README.md) |
| Transcript cut of a recording | [`Rust/claw-edit`](../../Rust/claw-edit/README.md) |
| Quick templates on raw clips | `scripts/claw_shorts.py` (below) |

The agent-facing guide is [`skills/claw-shorts/SKILL.md`](skills/claw-shorts/SKILL.md).

## Install as agent skills

```bash
skills/install-shortform-skills.sh            # from the repository root
```

Installs `claw-shorts` and `short-form-strategy` for Claude Code, Codex/Pi,
Grok and Hermes, and removes the retired skill names. Details:
[`harnesses.md`](skills/claw-shorts/references/harnesses.md).

## Canvas renderer

`canvas/scripts/render.mjs` captures `window.renderFrame(t)` from an HTML canvas
with Playwright and encodes it with FFmpeg; `canvas/assets/scene.html` is a
small starter. See
[`canvas-rendering.md`](skills/claw-shorts/references/canvas-rendering.md).

## Python templates

| Tool | Required | Install |
|------|----------|---------|
| FFmpeg | Yes | `brew install ffmpeg` / `apt install ffmpeg` |
| Python 3.9+ | Yes | `uv venv --python 3.13` |
| Playwright | `readme_scroll` only | pinned in a disposable project |
| agg | `terminal_typer` only | `cargo install --git https://github.com/asciinema/agg` |

```bash
python3 scripts/claw_shorts.py doctor
python3 scripts/claw_shorts.py templates
python3 scripts/claw_shorts.py info --input video.mp4
python3 scripts/claw_shorts.py render \
  --input <video|dir> --output <dir> --template <template> \
  [--title <text>] [--hook <text>] [--cta <text>] [--seed <int>]
```

| Template | Use | Extra flags |
| --- | --- | --- |
| `hook_reveal` | 2 s hook card, content, end CTA card | `--hook`, `--cta` |
| `step_breakdown` | Numbered step overlays | `--title`, `--steps "A,B,C"` |
| `demo_zoom` | Keyframed zoom/pan on a screen recording | `--zoom 1.5` |
| `before_after` | Sequential or side-by-side comparison | `--before`, `--after`, `--split` |
| `readme_scroll` | Scrolling capture of a URL | `--url`, `--scroll-duration` |
| `terminal_typer` | Asciinema cast to video | `--cast` |

Each render writes `<timestamp>_<hash>.mp4` (1080x1920, 30 fps, H.264/AAC) and
a `.json` sidecar with title, template, duration, suggested caption/hashtags
and an audio-rights reminder. Text stays inside safe zones (150 px top, 300 px
bottom, 50 px sides). The templates do not normalize loudness or check copy;
use a Rust mode for anything you will post.

## Principles

1. Never overwrite source files; outputs go to a separate directory.
2. Deterministic output names (timestamp + hash seed).
3. Offline only. No network calls to social platforms. Upload is manual.

## Tests

```bash
cd skills/claw-shorts
uv run --no-project --python 3.13 python -m unittest discover -s tests -p 'test_*.py'
```

Tests use synthetic video generated with FFmpeg. No binary video is committed.

## Structure

```
skills/claw-shorts/
├── README.md
├── canvas/                      # canvas renderer (was skills/canvas-explainer)
│   ├── assets/scene.html
│   ├── package.json
│   └── scripts/render.mjs
├── hyperframes/                 # HyperFrames mode: example project + helpers
│   ├── frame.md, index.html, compositions/
│   ├── narration.txt, script.txt, package.json
│   └── scripts/captions.py, finish.sh, sfx.sh
├── scripts/
│   ├── claw_shorts.py           # template CLI
│   └── requirements.txt
├── skills/claw-shorts/          # the installable skill
│   ├── SKILL.md
│   └── references/
│       ├── canvas.md, canvas-rendering.md, hyperframes.md
│       ├── recording-cut.md, recording-cut-rendering.md
│       ├── stock-broll.md
│       └── harnesses.md
└── tests/test_claw_shorts.py, test_hyperframes_captions.py
```

## License

MIT License. See repository LICENSE file.
