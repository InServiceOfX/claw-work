---
name: claw-shorts
description: Create and edit 9:16 vertical short-form videos (TikTok, Reels, Shorts) from raw takes, screen recordings, HTML/Canvas animations, or audio. Non-destructive workflow with Whisper timestamps, FFmpeg filtergraphs, paced captions, and safe-zone margins. Local/offline rendering only; never auto-posts to social platforms.
---

# Vertical short-form video creation

Tools: `claw-shorts` (CLI), FFmpeg, Python 3.9+, and optionally Whisper (word
timestamps) or Playwright (HTML canvas animations). Verify FFmpeg is available
with `ffmpeg -version`. This skill is local/offline; it never auto-posts to or
makes network calls to social platforms.

## Non-destructive principles

1. **Never overwrite source files.** Copy inputs to a work directory and write
   outputs to a dedicated output directory. Use deterministic/seeded filenames
   (timestamp + template hash) so consecutive renders are not near-duplicates.
2. **All renders are offline.** No network calls to TikTok, YouTube, Instagram,
   or any social platform. Uploading is a deliberate manual step by the user.
3. **Respect music rights.** Track audio licensing in the output metadata. Only
   use royalty-free or properly licensed audio for monetized uploads.

## Workflow overview

### 1. Prepare inputs

Collect source files in a single input directory:

- Raw video takes (MP4/MOV) — record multiple takes per line in one file.
- Screen recordings or demo captures.
- Audio tracks (voice-over, music).
- Brand assets (logo, color palette JSON).

### 2. Generate a cut list from transcription

For talking-head edits with multiple takes, use Whisper to get word timestamps:

```sh
whisper input.mp4 --output_format json --output_dir ./work/
```

The JSON contains word-level timestamps. Use these to:

- Identify the last (best) take of each spoken line.
- Find pauses longer than 0.5s to trim as dead air.
- Build a cut list for FFmpeg.

### 3. Choose a template

Built-in templates:

| Template | Use case | Structure |
|----------|----------|-----------|
| `hook_reveal` | Hook + reveal format | 2s hook card, content body, end CTA card |
| `step_breakdown` | Tutorial/explainer | Numbered steps with paced captions |

Custom templates: provide a JSON spec with timing, text, and asset references.

### 4. Render with safe zones

TikTok, Reels, and Shorts overlay UI elements on the video. Reserve margins:

- **Top safe zone**: 150px (status bar, search icons)
- **Bottom safe zone**: 300px (captions, action buttons, usernames)
- **Side margins**: 50px left/right

All burned-in captions and hook cards stay within these bounds.

### 5. Run the render

```sh
claw-shorts render \
  --input ./input/ \
  --output ./output/ \
  --template hook_reveal \
  --title "How to build X" \
  --hook "Stop scrolling if you want to learn X" \
  --cta "Follow for more"
```

Output:
- `output/<timestamp>_<hash>.mp4` — 1080×1920 30fps H.264/AAC
- `output/<timestamp>_<hash>.json` — metadata sidecar (title, caption, hashtags,
  duration, audio rights checklist)

## Template details

### hook_reveal

1. **Hook card (0–2s)**: Full-screen text overlay on a solid/gradient background
   or first frame. Bold sans-serif, centered in safe zone.
2. **Content (2s–end-3s)**: Main video with optional burned-in captions (word-
   paced from Whisper timestamps or fixed timing). Captions use a simple sans-
   serif font, positioned in lower safe zone.
3. **CTA card (last 3s)**: End screen with call-to-action text, optional logo.

### step_breakdown

1. **Title card (0–2s)**: "5 steps to X" or similar.
2. **Step cards**: Each step is introduced with a numbered overlay ("Step 1:")
   that fades in before the spoken content.
3. **Summary/CTA (last 3s)**: Recap or follow prompt.

### Audio/DJ showcase (music mode)

For tracks or mixes:
1. Waveform or spectrum visualization (FFmpeg showwaves/showspectrum).
2. Track title, BPM, and key overlay (if provided).
3. Transition reasoning card (optional, for DJ educational content).
4. Rights checklist in JSON sidecar: license type, source, attribution.

## Procedural HTML canvas animations

For motion graphics (explainers, branded animations):

1. Author a `frame(t)` function in JavaScript that draws to an HTML canvas.
2. Use Playwright to capture frames at 30fps.
3. Mux frames with FFmpeg.
4. Generate a contact sheet for visual QA before final render.

Example workflow:

```sh
claw-shorts canvas \
  --script explainer.js \
  --duration 15 \
  --fps 30 \
  --output ./output/
```

See [references/canvas-workflow.md](references/canvas-workflow.md) for the
JavaScript template and Playwright setup.

## Integration with HyperFrames

If HyperFrames (from HeyGen) is installed (`npx skills add heygen-com/hyperframes`),
you can use it as an alternative renderer for HTML-based animations. The claw-shorts
workflow is compatible: generate the edit plan locally, then invoke HyperFrames for
rendering if preferred.

## Quality guidelines (anti-slop)

1. **No duplicates**: Seeded filenames prevent near-duplicate outputs. Delete or
   archive old renders before publishing.
2. **Caption pacing**: Match caption timing to speech (use Whisper word stamps).
   Do not display 10 words instantly.
3. **Audio levels**: Normalize audio; human voice and AI voice on separate tracks
   if applicable.
4. **Visual clarity**: Fix dark/grainy footage or reshoot. Do not publish
   artifacts that are hard to read.
5. **Music rights**: Never use unlicensed copyrighted music for monetized content.
   Document the license in the metadata sidecar.

## Harness integration

| Harness | Skill location | Invocation |
|---------|----------------|------------|
| Claude Code | `~/.claude/skills/claw-shorts/` | `/claw-shorts` or ask for vertical video help |
| Codex | `~/.agents/skills/claw-shorts/` | `$claw-shorts` or ask |
| Grok Build | `~/.agents/skills/claw-shorts/` | `/claw-shorts` or `/skill:claw-shorts` |
| Hermes Agent | `~/.hermes/skills/claw-shorts/` | `skills_list` then `skill_view` |
| Pi | `~/.agents/skills/claw-shorts/` | `/skill:claw-shorts` |

Install the skill by copying the `skills/claw-shorts/` folder to the appropriate
location, or use the install script:

```sh
./install-skill.sh
```

## Example requests

> Use claw-shorts to edit my talking-head recording into a TikTok. Use the
> hook_reveal template with hook "This changed everything" and CTA "Follow for
> more tips". My input is in ~/Desktop/raw-video.mp4.

> Create a 15-second explainer animation for how DNS works using the canvas
> workflow. Output 9:16 vertical format.

> Edit this screen recording into a step_breakdown tutorial. Add numbered step
> overlays and burned-in captions.

## Safety reminders

- Review rendered output before uploading anywhere.
- Do not include personal information, private repo names, or credentials in
  captions or overlays.
- Music rights are your responsibility; the sidecar is a reminder, not legal advice.
