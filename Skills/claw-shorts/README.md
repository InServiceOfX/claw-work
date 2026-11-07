# claw-shorts

Create and edit 9:16 vertical short-form videos (TikTok, Reels, Shorts) from raw
takes, screen recordings, HTML/Canvas animations, or audio. Non-destructive workflow
with paced captions, safe-zone margins, and deterministic output naming.

Local/offline rendering only; never auto-posts to social platforms.

## Quick Start (60 seconds)

```bash
# 1. Verify FFmpeg is installed
ffmpeg -version

# 2. Render a video using the hook_reveal template
python3 scripts/claw_shorts.py render \
  --input ~/Desktop/my-raw-video.mp4 \
  --output ./output/ \
  --template hook_reveal \
  --title "How to build X" \
  --hook "Stop scrolling if you want to learn X" \
  --cta "Follow for more"

# Output: ./output/<timestamp>_<hash>.mp4 + .json sidecar
```

## Requirements

| Tool | Required | Install |
|------|----------|---------|
| FFmpeg | Yes | `brew install ffmpeg` / `apt install ffmpeg` |
| Python 3.9+ | Yes | System or pyenv |
| Pillow | Optional | `pip install pillow` |
| Whisper | Optional | `pip install openai-whisper` |
| Playwright | Optional | `npm i playwright && npx playwright install chromium` |

## Templates

### hook_reveal

Hook card → content → CTA card format for attention-grabbing reveals.

```
┌──────────────────┐   ┌──────────────────┐   ┌──────────────────┐
│                  │   │                  │   │                  │
│   "Stop         │   │                  │   │   "Follow for    │
│    scrolling"   │   │   Main content   │   │    more"         │
│                  │   │                  │   │                  │
│    (0-2s)       │   │   (2s - end-3s)  │   │   (last 3s)      │
└──────────────────┘   └──────────────────┘   └──────────────────┘
```

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

## CLI Reference

```bash
# List available templates
python3 scripts/claw_shorts.py templates

# Get video info (duration, resolution, codec)
python3 scripts/claw_shorts.py info --input video.mp4

# Render with hook_reveal
python3 scripts/claw_shorts.py render \
  --input ./input/ \
  --output ./output/ \
  --template hook_reveal \
  --title "Title" \
  --hook "Hook text" \
  --cta "CTA text" \
  --seed 12345  # deterministic naming

# Render with step_breakdown
python3 scripts/claw_shorts.py render \
  --input ./input/ \
  --output ./output/ \
  --template step_breakdown \
  --title "Tutorial" \
  --steps "First,Second,Third"
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

## Non-Destructive Principles

1. **Never overwrite source files.** Inputs are read-only; outputs go to a
   separate directory.
2. **Deterministic naming.** Output filenames include timestamp + hash seed to
   prevent near-duplicate renders.
3. **Offline only.** No network calls to social platforms. Upload is manual.

## Quality Guidelines (Anti-Slop)

1. **Caption pacing**: Match caption timing to speech. Use Whisper word
   timestamps for precise sync.
2. **Audio levels**: Normalize audio. Keep human voice and background music
   balanced.
3. **Visual clarity**: Fix dark/grainy footage or reshoot. Do not publish
   artifacts that are hard to read.
4. **No duplicates**: Use seeded filenames. Delete old renders before publishing.

## Music Rights Checklist

Before monetizing content with music:

- [ ] Royalty-free license confirmed
- [ ] Attribution requirements met (if any)
- [ ] Platform-specific rules checked (TikTok Commercial Music Library, etc.)
- [ ] License documented in metadata sidecar

The `audio_rights` field in the JSON sidecar is a reminder, not legal advice.

## Canvas Animations (Advanced)

For procedural motion graphics (explainers, branded animations):

1. Write a `frame(t)` JavaScript function that draws to HTML canvas
2. Capture frames with Playwright at 30fps
3. Mux with FFmpeg

See `skills/claw-shorts/references/canvas-workflow.md` for the full workflow.

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
├── scripts/
│   ├── claw_shorts.py                  # CLI tool
│   └── requirements.txt                # Python dependencies
├── skills/claw-shorts/
│   ├── SKILL.md                        # Agent skill definition
│   └── references/
│       ├── canvas-workflow.md          # HTML canvas animation guide
│       └── harnesses.md                # Harness integration details
└── tests/
    └── test_claw_shorts.py             # Unit and integration tests
```

## License

MIT License. See repository LICENSE file.
