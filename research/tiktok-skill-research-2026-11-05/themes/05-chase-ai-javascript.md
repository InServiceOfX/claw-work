# Chase AI (@chase_ai_) — Claude writes JavaScript frames

- Video: https://www.tiktok.com/@chase_ai_/video/7693602525081111821 (about 14 minutes; title: "claude now does videos (the javascript hack)")
- Auto-captions: [`../transcripts/chase_ai_-7693602525081111821.txt`](../transcripts/chase_ai_-7693602525081111821.txt)
- Screenshots: [`../screenshots/chase_ai/`](../screenshots/chase_ai/)
- Theme: procedural motion graphics (HTML canvas + Playwright + FFmpeg)
- CTA: paid Chase AI+ Skool community and Claude Code/Codex masterclasses; link in the pinned comment. The workflow and GitHub plugin are shown in the video and do not require that purchase. Free core: https://github.com/cb9191/animate (MIT), shown on screen.
- Inspiration mentions: skilry.dev (browse free; paid prompt library $9.99/mo, $79/yr, or $169 lifetime — use free thumbnails as visual refs instead of buying prompts); @KevinNgo on X ("Kevin Ngo - Creative coding with AI"); elevenlabs.io for TTS (paid; swap for open-source TTS if you want a free stack).

This is not generative "AI video" (no Runway/Sora-style model) and he says you do not need Remotion or After Effects. Claude writes deterministic JavaScript; a headless browser draws frames; FFmpeg stitches them.

## Walkthrough (from the watch)

- **0:00–0:36** Hook: high-quality motion graphics from Claude + JavaScript + code, for free, without Remotion / AE / AI video generators.
- **0:36–3:42** Mechanism: Claude writes a `frame(t)` function (timestamp in, one canvas frame out). About 60 frames per second. Playwright (headless Chromium) draws each frame; FFmpeg muxes an MP4. Claude is told to render a contact sheet, visually inspect its own frames (cut-off text, layout bugs), fix the JS, and re-render.
- **3:42–4:07** Sponsor block (Chase AI+ / masterclasses; pinned comment).
- **4:07–5:58** Baseline prompt for a dynamic ~15s motion-design showreel at Low / High / Ultra effort (he cites roughly 20 min / 30 min / 8 hours). Ultra looked best; Low and High were still usable.
- **5:58–7:19** Storyboard first: ask Claude for 5–7 static keyframes (e.g. t=1, t=5, t=10) before committing to a full render. User approves beats, then full 60fps Playwright render.
- **7:19–9:56** Level 2 master prompt (on-screen ~7:19 and ~8:36). See below.
- **9:56–11:43** Level 3: pass reference images or reference videos so Claude copies composition/aesthetic. Example shown: a coded recreation of a traditional motion-graphics timeline of currency/communication from 9000 BC to 1969.
- **11:43+** Level 4 skill packaging: the `animate` skill with 7 built-in styles (cut-paper, crosshatch, riso, sketchbook, and others). README title on screen: "Animate - procedural animation in any style, for Claude Code". Automates intake, storyboard, generation, and self-correction. Then the `cb9191/animate` README and install commands.

Public writeup covering the same method and Level 1/2 prompt text: https://visions.joaoqueiros.com/blog/claude-javascript-video-canvas-playwright-ffmpeg-animate-skill/

## Level 2 master prompt (verbatim, on-screen ~7:19 and ~8:36)

> Make a 30-second animated explainer of how [TOPIC] works, 16:9. Build it in JavaScript on a canvas, with a soundtrack generated in code, and render it to an MP4 with Playwright and FFmpeg.

Optional audio block:

> Here's my voice-over: voice.mp3. Time the visuals to it: get word timestamps with whisper and land each change on the words.

Close:

> Before you call it done, use FFmpeg to pull a contact sheet, look at it, fix what's wrong, re-render, and repeat.

## On-screen GitHub install (screenshot `06-install-commands.jpg`, README of `cb9191/animate`, MIT)

Requirements shown: Claude Code, Node 18+, Playwright with Chromium, and ffmpeg on PATH. Fonts are local system fonts with Windows/macOS/Linux fallbacks. Optional voice-overs: Python `faster-whisper` for word timing, and optionally the ElevenLabs connector in Claude (Settings → Connectors).

```
npm i -g playwright && npx playwright install chromium
```

As a plugin:

```
/plugin marketplace add cb9191/animate
/plugin install animate@animate
```

Or copy `plugins/animate/skills/animate/` to `~/.claude/skills/animate/`.

Example `/animate` invocations shown:

- `/animate a 45-second history of the bicycle, cut paper`
- `/animate how a hash map works, in the math style`
- `/animate a short about our API in the style of these screenshots: ./refs/`
- `/animate a 20s launch reel for https://my-product.com, cut to ./audio/song.mp3, vertical and square`
- `/animate a narrated 45s explainer of how DNS works, math style, 16:9, voice from ElevenLabs`

## Skill / workflow to reuse

Tightened loop:

1. **Intake / audio prep.** Describe the video (duration, aspect ratio, style). Optionally attach a voice-over (`voice.mp3`).
2. **Storyboard keyframes.** Ask Claude for 5–7 static keyframes (t=1, t=5, t=10, …). Approve beats before full render.
3. **Playwright 60fps frame dump.** Claude implements `frame(t)` on a canvas; Playwright captures every frame.
4. **FFmpeg contact-sheet vision QA.** Pull a contact sheet, Claude visually inspects it, fixes JS, re-renders if needed.
5. **FFmpeg mux MP4 + original audio.** Stitch frames into MP4 and layer the voice-over or generated soundtrack.
6. Optional: use `faster-whisper` word timings to land visual changes on specific words.

For claw-dj marketing this is useful for explainer motion graphics (how AI mixes work, lobster brand boards), not for cutting live Mixxx recordings.

## Screenshot index

- `01-architecture.jpg` (~2:20)
- `02-storyboard.jpg` (~6:40)
- `03-level2-prompt.jpg` (~8:00)
- `04-reference-compare.jpg` (~10:40)
- `05-github-readme.jpg` (~12:00)
- `06-install-commands.jpg` (~13:15)
