# claw-shorts

Create and edit 9:16 vertical short-form videos (TikTok, Reels, Shorts) from raw
takes, screen recordings, HTML/Canvas animations, or audio. Non-destructive workflow
with paced captions, safe-zone margins, and deterministic output naming.

**Launch Kit**: Point a coding agent at a GitHub repo URL, get 5 captioned launch
Shorts plus a posting kit with titles, captions, hashtags, and posting order.

Local/offline rendering only; never auto-posts to social platforms.

> **Voiced explainer Shorts: use the Rust tool.** [`Rust/claw-shorts`](../../Rust/claw-shorts)
> validates the copy (no placeholders, no unsupported claims), narrates each scene
> with a local TTS such as `voxcpm-tts`, renders the video, and writes a contact
> sheet and posting kit. The generic `launch` plan below emits template text
> ("Tired of the old way?") that must be rewritten by hand; prefer the Rust
> `brief` then `check` flow for anything you will post. The Python templates
> (`demo_zoom`, `before_after`, `readme_scroll`, `terminal_typer`) are not ported
> and remain here.

## Quick Start (60 seconds)

```bash
# 1. Verify FFmpeg is installed
ffmpeg -version

# 2. Check all dependencies
python3 scripts/claw_shorts.py doctor

# 3. Generate launch Shorts from a GitHub repo
python3 scripts/claw_shorts.py launch \
  --repo https://github.com/user/project \
  --output ./output/ \
  --render

# Or render a single video using a template
python3 scripts/claw_shorts.py render \
  --input ~/Desktop/my-raw-video.mp4 \
  --output ./output/ \
  --template hook_reveal \
  --title "How to build X" \
  --hook "Stop scrolling if you want to learn X" \
  --cta "Follow for more"
```

## Launch Kit

Generate 5 launch Shorts from any GitHub repository:

```bash
# Generate plan only (review before rendering)
python3 scripts/claw_shorts.py launch \
  --repo https://github.com/user/project \
  --output ./output/

# Generate and render with a screen recording
python3 scripts/claw_shorts.py launch \
  --repo https://github.com/user/project \
  --output ./output/ \
  --recording ~/demo.mp4 \
  --render

# From a local repo with sponsored disclosure
python3 scripts/claw_shorts.py launch \
  --repo ./my-local-project \
  --output ./output/ \
  --sponsored \
  --render
```

### Output

The launch command produces:

1. `launch_plan.json` - Storyboard for 5 clips with hooks, beats, and CTAs
2. `posting_kit.json` - Titles, captions, hashtags, and posting order
3. `clips/` - Rendered MP4 files (when using `--render`)

### The 5 Launch Clips

| # | Template | Purpose |
|---|----------|---------|
| 1 | hook_reveal | Pain point hook to grab attention |
| 2 | demo_zoom | Screen recording with zoom effect |
| 3 | step_breakdown | 3-step "how to use" explainer |
| 4 | before_after | Before/after comparison |
| 5 | hook_reveal | Social proof (stars, usage) |

### Rights-Safe

- No copyrighted music (silent by default, add your own track)
- No AI-generated faces or personas
- Disclosure snippets for sponsored/affiliate use
- Never auto-posts to any platform

## Requirements

| Tool | Required | Install |
|------|----------|---------|
| FFmpeg | Yes | `brew install ffmpeg` / `apt install ffmpeg` |
| Python 3.9+ | Yes | System or pyenv |
| Playwright | Optional | `pip install playwright && npx playwright install chromium` |
| agg | Optional | `cargo install --git https://github.com/asciinema/agg` |
| Whisper | Optional | `pip install openai-whisper` |
| Piper TTS | Optional | https://github.com/rhasspy/piper |

Check all dependencies:

```bash
python3 scripts/claw_shorts.py doctor
```

## Templates

### hook_reveal

Hook card → content → CTA card format for attention-grabbing reveals.

```bash
python3 scripts/claw_shorts.py render \
  --input video.mp4 \
  --output ./output/ \
  --template hook_reveal \
  --hook "This changed everything" \
  --cta "Link in bio"
```

### step_breakdown

Numbered steps with title card for tutorials and how-tos.

```bash
python3 scripts/claw_shorts.py render \
  --input tutorial.mp4 \
  --output ./output/ \
  --template step_breakdown \
  --title "Build an API" \
  --steps "Setup,Code,Deploy" \
  --cta "Subscribe for more"
```

### demo_zoom

Keyframed zoom/pan effect for screen recordings.

```bash
python3 scripts/claw_shorts.py render \
  --input screen-recording.mp4 \
  --output ./output/ \
  --template demo_zoom \
  --hook "Watch this in action" \
  --zoom 1.5
```

### before_after

Sequential or side-by-side comparison.

```bash
python3 scripts/claw_shorts.py render \
  --output ./output/ \
  --template before_after \
  --before old-way.mp4 \
  --after new-way.mp4 \
  --split sequential
```

### readme_scroll (requires Playwright)

Scrolling README page capture.

```bash
python3 scripts/claw_shorts.py render \
  --output ./output/ \
  --template readme_scroll \
  --url "https://github.com/user/repo" \
  --scroll-duration 10
```

### terminal_typer (requires agg)

Asciinema cast rendered to video.

```bash
python3 scripts/claw_shorts.py render \
  --output ./output/ \
  --template terminal_typer \
  --cast demo.cast \
  --hook "Watch this CLI magic"
```

## CLI Reference

```bash
# Check dependencies
python3 scripts/claw_shorts.py doctor

# List available templates
python3 scripts/claw_shorts.py templates

# Get video info (duration, resolution, codec)
python3 scripts/claw_shorts.py info --input video.mp4

# Generate launch Shorts from a repo
python3 scripts/claw_shorts.py launch \
  --repo <url|path> \
  --output <dir> \
  [--recording <video>] \
  [--render] \
  [--sponsored | --affiliate]

# Render with any template
python3 scripts/claw_shorts.py render \
  --input <video|dir> \
  --output <dir> \
  --template <template> \
  [--title <text>] \
  [--hook <text>] \
  [--cta <text>] \
  [--seed <int>]
```

## Output

Each render produces two files:

1. **Video**: `<timestamp>_<hash>.mp4` (1080×1920, 30fps, H.264/AAC)
2. **Metadata**: `<timestamp>_<hash>.json`

```json
{
  "title": "How to build X",
  "template": "hook_reveal",
  "duration_seconds": 15.5,
  "resolution": "1080x1920",
  "fps": 30,
  "codec": "H.264/AAC",
  "suggested_caption": "How to build X Stop scrolling...",
  "suggested_hashtags": ["#shorts", "#tutorial"],
  "audio_rights": {
    "status": "check_required",
    "notes": "Verify audio licensing before monetized upload"
  }
}
```

## Safe Zones

TikTok, Reels, and Shorts overlay UI elements. The templates keep text within:

```
┌─────────────────────────────┐
│░░░░░░░ TOP: 150px ░░░░░░░░░│
├────┬───────────────────┬────┤
│    │                   │    │
│50px│   SAFE CONTENT    │50px│
│    │      AREA         │    │
│    │                   │    │
├────┴───────────────────┴────┤
│░░░░░░ BOTTOM: 300px ░░░░░░░│
└─────────────────────────────┘
```

## Install as Agent Skill

```bash
./install-skill.sh
```

Installs to:
- `~/.claude/skills/claw-shorts/` (Claude Code)
- `~/.agents/skills/claw-shorts/` (Codex, Grok Build, Pi)
- `~/.hermes/skills/claw-shorts/` (Hermes Agent)

### Harness Usage

| Harness | Invocation |
|---------|------------|
| Claude Code | `/claw-shorts` or ask for vertical video help |
| Codex | `$claw-shorts` or ask |
| Grok Build | `/claw-shorts` or `/skill:claw-shorts` |
| Hermes Agent | `skills_list` → `skill_view claw-shorts` |
| Pi | `/skill:claw-shorts` |
| Cursor | Read the skill file, ask for launch kit or video help |

## Non-Destructive Principles

1. **Never overwrite source files.** Inputs are read-only; outputs go to a
   separate directory.
2. **Deterministic naming.** Output filenames include timestamp + hash seed to
   prevent near-duplicate renders.
3. **Offline only.** No network calls to social platforms. Upload is manual.

## Running Tests

```bash
cd Skills/claw-shorts
python3 -m unittest discover -s tests -p 'test_*.py' -v
```

Tests use synthetic video generated with FFmpeg. No binary video files are
committed to the repository.

## Structure

```
Skills/claw-shorts/
├── README.md                           # This file
├── install-skill.sh                    # Install skill to harness locations
├── fixtures/
│   └── sample-repo/                    # Sample repo for offline testing
├── scripts/
│   ├── claw_shorts.py                  # CLI tool
│   ├── launch_kit.py                   # Launch Kit module
│   └── requirements.txt                # Python dependencies
├── skills/claw-shorts/
│   ├── SKILL.md                        # Agent skill definition
│   └── references/
│       ├── canvas-workflow.md          # HTML canvas animation guide
│       └── harnesses.md                # Harness integration details
└── tests/
    ├── test_claw_shorts.py             # Core CLI tests
    └── test_launch_kit.py              # Launch Kit tests
```

## License

MIT License. See repository LICENSE file.
