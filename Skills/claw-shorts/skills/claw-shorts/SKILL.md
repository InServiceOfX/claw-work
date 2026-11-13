---
name: claw-shorts
description: Create and edit 9:16 vertical short-form videos (TikTok, Reels, Shorts) from raw takes, screen recordings, HTML/Canvas animations, or audio. Includes Launch Kit for generating 5 captioned launch Shorts from a GitHub repo. Non-destructive workflow with paced captions and safe-zone margins. Local/offline rendering only; never auto-posts to social platforms.
---

# Vertical short-form video creation

Tools: `claw-shorts` (CLI), FFmpeg, Python 3.9+, and optionally Whisper (word
timestamps) or Playwright (HTML canvas animations). Verify FFmpeg is available
with `ffmpeg -version`. This skill is local/offline; it never auto-posts to or
makes network calls to social platforms.

## Launch Kit (for dev tools and indie apps)

Point your coding agent at a GitHub repo URL or product URL, optionally with a
screen recording, and get 5 captioned vertical Shorts plus a posting kit.

```sh
# Generate launch plan from a GitHub repo
claw-shorts launch --repo https://github.com/user/project --output ./output/

# Generate and render clips immediately
claw-shorts launch --repo https://github.com/user/project \
  --output ./output/ \
  --recording ~/demo.mp4 \
  --render

# With disclosure for sponsored content
claw-shorts launch --repo ./my-local-repo \
  --output ./output/ \
  --sponsored
```

The launch command:
1. Fetches README, package metadata, and screenshots from the repo
2. Generates `launch_plan.json` with hooks, beats, and CTAs for 5 clips
3. Creates `posting_kit.json` with titles, captions, hashtags, and posting order
4. Optionally renders the clips with `--render`

### Launch Kit Templates

| Clip | Template | Purpose |
|------|----------|---------|
| 1 | `hook_reveal` | Pain point hook to grab attention |
| 2 | `demo_zoom` | Screen recording with zoom effect |
| 3 | `step_breakdown` | 3-step "how to use" explainer |
| 4 | `before_after` | Before/after comparison |
| 5 | `hook_reveal` | Social proof (stars, usage) |

### Disclosure

For sponsored or affiliate content, the launch kit includes on-screen and spoken
disclosure text. Use `--sponsored` or `--affiliate` flags.

## Non-destructive principles

1. **Never overwrite source files.** Copy inputs to a work directory and write
   outputs to a dedicated output directory. Use deterministic/seeded filenames
   (timestamp + template hash) so consecutive renders are not near-duplicates.
2. **All renders are offline.** No network calls to TikTok, YouTube, Instagram,
   or any social platform. Uploading is a deliberate manual step by the user.
3. **Respect music rights.** Track audio licensing in the output metadata. Only
   use royalty-free or properly licensed audio for monetized uploads. No music
   is added by default.
4. **No AI faces or personas.** The kit does not generate AI avatars or UGC-style
   talking heads.

## Check dependencies

```sh
claw-shorts doctor
```

This checks for FFmpeg (required), Playwright (optional), agg (optional), 
Whisper (optional), and Piper TTS (optional).

## Templates

| Template | Use case | Structure |
|----------|----------|-----------|
| `hook_reveal` | Hook + reveal format | 2s hook card, content body, end CTA card |
| `step_breakdown` | Tutorial/explainer | Numbered steps with paced captions |
| `demo_zoom` | Screen recording highlight | Keyframed zoom/pan effect |
| `before_after` | Transformation showcase | Sequential or side-by-side comparison |
| `readme_scroll` | README walkthrough | Scrolling capture (requires Playwright) |
| `terminal_typer` | CLI demo | Asciinema cast rendered to video (requires agg) |

### hook_reveal

```sh
claw-shorts render \
  --input ./video.mp4 \
  --output ./output/ \
  --template hook_reveal \
  --hook "Stop scrolling if you want to learn X" \
  --cta "Follow for more"
```

### step_breakdown

```sh
claw-shorts render \
  --input ./video.mp4 \
  --output ./output/ \
  --template step_breakdown \
  --title "Build an API" \
  --steps "Setup,Code,Deploy"
```

### demo_zoom

```sh
claw-shorts render \
  --input ./screen-recording.mp4 \
  --output ./output/ \
  --template demo_zoom \
  --hook "Watch this in action" \
  --zoom 1.5
```

### before_after

```sh
claw-shorts render \
  --output ./output/ \
  --template before_after \
  --before ./old-way.mp4 \
  --after ./new-way.mp4 \
  --split sequential  # or side_by_side
```

### readme_scroll (requires Playwright)

```sh
claw-shorts render \
  --output ./output/ \
  --template readme_scroll \
  --url "https://github.com/user/repo" \
  --scroll-duration 10
```

### terminal_typer (requires agg)

```sh
claw-shorts render \
  --output ./output/ \
  --template terminal_typer \
  --cast ./demo.cast \
  --hook "Watch this CLI magic"
```

## Safe zones

TikTok, Reels, and Shorts overlay UI elements on the video. Reserve margins:

- **Top safe zone**: 150px (status bar, search icons)
- **Bottom safe zone**: 300px (captions, action buttons, usernames)
- **Side margins**: 50px left/right

## Harness integration

| Harness | Skill location | Invocation |
|---------|----------------|------------|
| Claude Code | `~/.claude/skills/claw-shorts/` | `/claw-shorts` or ask for vertical video help |
| Codex | `~/.agents/skills/claw-shorts/` | `$claw-shorts` or ask |
| Grok Build | `~/.agents/skills/claw-shorts/` | `/claw-shorts` or `/skill:claw-shorts` |
| Hermes Agent | `~/.hermes/skills/claw-shorts/` | `skills_list` then `skill_view` |
| Pi | `~/.agents/skills/claw-shorts/` | `/skill:claw-shorts` |
| Cursor | Read this skill file | Ask for launch kit or video help |

Install the skill by copying the `skills/claw-shorts/` folder to the appropriate
location, or use the install script:

```sh
./install-skill.sh
```

## Example requests

> Generate launch Shorts for my CLI tool at https://github.com/me/mytool.
> I have a screen recording at ~/demo.mp4.

> Use claw-shorts to edit my talking-head recording into a TikTok. Use the
> hook_reveal template with hook "This changed everything" and CTA "Follow for
> more tips".

> Create a before/after comparison video showing the old workflow vs the new one.

> Edit this screen recording into a step_breakdown tutorial with 3 steps.

## Safety reminders

- Review rendered output before uploading anywhere.
- Do not include personal information, private repo names, or credentials in
  captions or overlays.
- Music rights are your responsibility; the sidecar is a reminder, not legal advice.
- For sponsored/affiliate content, always include proper disclosure.
