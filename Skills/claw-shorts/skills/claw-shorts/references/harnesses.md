# Harness integration for claw-shorts

## Skill locations

| Harness | Skill root | Discovery |
|---------|------------|-----------|
| Claude Code | `~/.claude/skills/claw-shorts/` | `/claw-shorts`, `/skill:claw-shorts`, or natural language |
| Codex | `~/.agents/skills/claw-shorts/` | `$claw-shorts` or natural language |
| Grok Build | `~/.agents/skills/claw-shorts/` | `/claw-shorts` or `grok inspect` |
| Hermes Agent | `~/.hermes/skills/claw-shorts/` | `skills_list` then `skill_view claw-shorts` |
| Pi | `~/.agents/skills/claw-shorts/` | `/skill:claw-shorts` or `/reload` then ask |

## Installation

From the `Skills/claw-shorts/` directory:

```sh
./install-skill.sh
```

This copies `skills/claw-shorts/` to each harness location. After installation,
restart clients or reload skills (Pi: `/reload`).

## Manual installation

```sh
cp -r skills/claw-shorts ~/.claude/skills/
cp -r skills/claw-shorts ~/.agents/skills/
cp -r skills/claw-shorts ~/.hermes/skills/
```

## Dependencies

The skill expects these tools to be available:

| Tool | Purpose | Install |
|------|---------|---------|
| FFmpeg | Video encoding, muxing | `brew install ffmpeg` / `apt install ffmpeg` |
| Python 3.9+ | CLI and templates | System or `pyenv` |
| Pillow | Text rendering (optional) | `pip install pillow` |
| Whisper | Word timestamps | `pip install openai-whisper` |
| Playwright | Canvas animations | `npm i playwright && npx playwright install chromium` |

FFmpeg and Python are required; the others are optional depending on workflow.

## Example prompts by harness

### Claude Code

```
/claw-shorts I have a raw talking-head video at ~/Desktop/recording.mp4.
Edit it into a TikTok using the hook_reveal template. Hook: "You won't believe
this trick". CTA: "Follow for more". Use the last take of each line and cut
dead air.
```

### Codex

```
$claw-shorts Create a 15-second explainer animation about API rate limiting.
Use the canvas workflow, math/diagram style, vertical 9:16 format.
```

### Grok Build

```
/skill:claw-shorts Edit my screen recording at ./demo.mp4 into a step_breakdown
tutorial. Add numbered step overlays and burned-in captions.
```

### Hermes Agent

```
Use skill claw-shorts to create a DJ mix showcase video. Input audio is
track.mp3. Add waveform visualization and track info overlay. Include
rights info in the metadata sidecar.
```

### Pi

```
/skill:claw-shorts Render a hook_reveal video. Input: ./input/raw.mp4.
Hook text: "Stop scrolling". CTA: "Link in bio". Output to ./output/.
```

## Troubleshooting

### FFmpeg not found

Install FFmpeg and ensure it's in PATH:
- macOS: `brew install ffmpeg`
- Ubuntu/Debian: `sudo apt install ffmpeg`
- Check: `ffmpeg -version`

### Whisper not available

Whisper is optional; without it, captions use fixed timing rather than word-level
sync. To install: `pip install openai-whisper` (requires ~1GB for models).

### Playwright errors

For canvas workflow, install Chromium:
```sh
npm install playwright
npx playwright install chromium
```

### Output not 1080x1920

Check input resolution and template settings. The CLI scales/pads inputs to
1080×1920 by default. Override with `--width` and `--height` if needed.
