---
name: claw-shorts
description: Produce faceless vertical short-form videos (TikTok, Reels, Shorts) locally, in every production mode - voiced explainer Shorts with validated copy, canvas motion graphics, animated math (Manim), physics demos and shape morphs with collision-timed sound, automated stock B-roll or photo videos with TTS and burned captions, transcript-based cuts of screen recordings, and template edits of raw clips. Use when the user asks to "make a short", "generate a faceless video", "turn this script/topic into a video", "add captions + voiceover to b-roll", "animate an explainer", "animate this equation", "make a physics/Galton board video", "cut this recording into a short", or runs /claw-shorts. Enforces script, hook and loudness rules; renders offline; never posts. For hooks, retention and what to say, load short-form-strategy first.
---

# claw-shorts: produce the Short

This is the production skill. Planning (hook, retention, engagement, script
rhythm) is the `short-form-strategy` skill: when reach matters, load it first,
write the script with it, then come back here to render.

All tools live in a claw-work checkout (`InServiceOfX/claw-work`). Build what
you need there; nothing here installs global tools on its own.

## Ground rules (every mode)

- **Faceless.** Synthetic, disclosed narration plus screens, motion graphics,
  stock or photos. Do not suggest recording the user's face or own voice.
- **Never post.** Rendering is local and offline. Uploading is a separate,
  explicit human step. Review the video and listen before anything is posted.
- **Loudness is fixed at -14 LUFS integrated, -1 dBTP.** `Rust/claw-shorts`
  and `Rust/shortform-video` measure, normalize (with a limiter step when the
  voice is peaky) and fail a render that misses by more than 1 LU or clips.
  Do not hand-tune volume to "sound right"; read `verification.json` or
  `loudness.json`.
- **Safe zones.** On a 1080x1920 frame every word sits inside y 288-1248 px,
  x 120-888 px above y 840 and x 120-780 px below it (TikTok's action buttons).
  This is the area clear on TikTok, Reels and Shorts at once, from the
  platforms' vertical-ad guides as encoded in HyperFrames `check`.
- **Disclosure and rights.** Disclose synthetic voices and sponsorships; use
  only licensed or royalty-free music and credit stock where its license asks.
- **Originals are read-only.** Write to new work and output directories.

## Pick a mode

| You have | Mode | Tool | Read |
| --- | --- | --- | --- |
| A claim about a project or tool to explain with narration | **Voiced explainer** (default for anything you will post) | `Rust/claw-shorts`: `brief`, `check --strict`, `render` | `Rust/claw-shorts/README.md` |
| The same, but want richer motion (live waveform, push-ins, word-by-word captions) | **Canvas** | `claw-shorts canvas`, then `skills/claw-shorts/canvas/scripts/render.mjs`, then `claw-shorts mux` | [canvas.md](references/canvas.md), [canvas-rendering.md](references/canvas-rendering.md) |
| A listicle or explainer that needs designed motion graphics, charts or kinetic captions timed to the narration, plus a safe-area/contrast gate | **HyperFrames** | `skills/claw-shorts/hyperframes/` (pinned `hyperframes` CLI, `captions.py`, `finish.sh`) | [hyperframes.md](references/hyperframes.md) |
| An equation, derivation or plot to animate for a STEM explainer (typeset math that transforms, graphs that draw) | **Math (Manim)** | `skills/claw-shorts/manim/scripts/render.sh` (pinned `manim==0.22.1`, safe-area gate, beats from `timeline.json`), then `claw-shorts mux` | [manim.md](references/manim.md) |
| A physical demo (balls through a Galton board, Plinko, a pile-up) or a shape-to-shape morph, with sounds on every collision | **Physics / morph** | `Rust/claw-motion` (`physics`, `morph`, `--still` preview, `--narration`) | [motion.md](references/motion.md) |
| A topic or script, and stock footage or a folder of clips and photos | **Stock B-roll** | `Rust/shortform-video run` | [stock-broll.md](references/stock-broll.md) |
| A screen recording or demo with spoken takes to tighten | **Recording cut** | transcript, reviewed cut table, `Rust/claw-edit` | [recording-cut.md](references/recording-cut.md), [recording-cut-rendering.md](references/recording-cut-rendering.md) |
| A raw clip that needs a quick template (zoom, before/after, README scroll, terminal cast) | **Templates** | `skills/claw-shorts/scripts/claw_shorts.py render` | below |
| An equation-heavy deck to record yourself | not this skill | claw-work `docs/shortform/obs-deck-pipeline/` | |

## Voiced explainer (default)

```bash
cd Rust/claw-shorts && cargo build --release
claw-shorts brief --repo ../some-project --output /tmp/brief   # facts + evidence candidates, a TODO template
claw-shorts check --spec short.json --strict                   # copy rules, see below
claw-shorts render --spec short.json --work /tmp/work --output /tmp/out --strict
```

`check` turns the research into rules so a draft cannot skip them:

- **Errors:** placeholder copy, unfilled `{braces}`, superlatives ("best",
  "beats", "killer") without recorded evidence, glyphs the font cannot draw, a
  greeting opener ("hey guys", "welcome back").
- **Warnings:** an on-screen hook over 8 words; no engagement device; no
  visible source URL, number, verdict or synthetic-voice disclosure; narration
  over ~45 s; **script rhythm** (two sentences over 12 words back to back, or
  4+ sentences with none of 5 words or fewer).

`render` writes `short.mp4`, `contact.png`, `captions.srt`, `timeline.json`,
`verification.json` (dimensions, duration, full decode, narration and output
loudness) and a posting kit. Look at the contact sheet and listen before
calling it done; a contact sheet cannot judge audio.

## Templates (Python, raw clips)

The older Python CLI keeps templates the Rust tool does not have. Its repo
"launch kit" was retired on 2026-11-14 because it emitted generic copy; use
`claw-shorts brief` instead.

```bash
python3 skills/claw-shorts/scripts/claw_shorts.py doctor
python3 skills/claw-shorts/scripts/claw_shorts.py render --input demo.mp4 --output out/ \
  --template demo_zoom --hook "Watch this in action" --zoom 1.5
```

| Template | Use |
| --- | --- |
| `hook_reveal` | 2 s hook card, the clip, end CTA card |
| `step_breakdown` | Numbered step overlays (`--steps "Setup,Code,Deploy"`) |
| `demo_zoom` | Keyframed zoom/pan on a screen recording |
| `before_after` | `--before`/`--after`, `--split sequential` or `side_by_side` |
| `readme_scroll` | Scrolling capture of a URL (needs Playwright) |
| `terminal_typer` | An asciinema cast rendered to video (needs `agg`) |

These templates do not normalize loudness or check copy. For anything you
will post, prefer a Rust mode, or check the result's loudness with
`ffmpeg -i out.mp4 -af loudnorm=print_format=summary -f null -`.

## Deliver

Report the output paths, what was actually checked (probe, decode, loudness
numbers, frames you looked at) and what was not (audio listened to or not).
Posting needs its own authorization.

## Install

`skills/install-shortform-skills.sh` in claw-work copies this skill and
`short-form-strategy` into Claude Code, Codex/Grok/Pi (`~/.agents/skills`) and
Hermes. See [harnesses.md](references/harnesses.md).

- Math mode: `uv`, FFmpeg, LaTeX (MacTeX/BasicTeX or TeX Live) and, on macOS,
  `brew install cairo pkgconf`. The first `skills/claw-shorts/manim/scripts/render.sh`
  builds `.venv` from `uv.lock`.
- Physics / morph mode: Rust and FFmpeg;
  `cargo build --release --manifest-path Rust/claw-motion/Cargo.toml`.
