# TikTok research: motion graphics, sound design, and design skills (2026-11-14)

Four TikToks about giving coding agents better motion, sound and design tools, parsed into transcripts, readable screenshots, verified links and a usefulness review for claw-dj short-form content ("AI mixes", lobster mascot, faceless YouTube Shorts and TikTok, local/open-source tools, no paid SaaS voices, no AI influencer personas).

All links, licenses and star counts were checked on 2026-11-14. Video stats are as of download on the same day.

## Index

| # | Video | Creator | Length | Views at download | Topic | Notes |
|---|---|---|---|---|---|---|
| 01 | [7702553659275496718](https://www.tiktok.com/@coachspitz.ai/video/7702553659275496718) | @coachspitz.ai | 50 s | 2,898 | 4 free alternatives to Claude Motion: HyperFrames, huashu-motion, Remotion, GSAP skills | [themes/01-coachspitz.ai.md](themes/01-coachspitz.ai.md) |
| 02 | [7706188024962420001](https://www.tiktok.com/@jackroberts____/video/7706188024962420001) | @jackroberts____ | 35 s | 44,400 | motion.dev, anime.js, shadcn charts for Claude-built sites | [themes/02-jackroberts____.md](themes/02-jackroberts____.md) |
| 03 | [7702485321723743506](https://www.tiktok.com/@agilesingh/video/7702485321723743506) | @agilesingh | 30 s | 4,143 | Real SFX: Claude places real recorded SFX on the exact frame | [themes/03-agilesingh.md](themes/03-agilesingh.md) |
| 04 | [7686228759833300237](https://www.tiktok.com/@duncanrogoff/video/7686228759833300237) | @duncanrogoff | 57 s | 95,900 | 5 design-skill repos: Taste Skill, UI UX Pro Max, Awesome Claude Design, Design Motion Principles, Design.md Chrome | [themes/04-duncanrogoff.md](themes/04-duncanrogoff.md) |

## Layout

```text
README.md          this index
LINKS.md           only the extra links requested per video, verified
BIBLIOGRAPHY.md    every verified link with its license
EVALUATION-2026-11-14.md  hands-on results: what ran, what was adopted, corrections
themes/            one write-up per video: walkthrough, links, licenses, verdicts
transcripts/       cleaned spoken text from each video's captions
screenshots/       3–5 readable stills per video (720 px wide)
```

## Theme map

| Theme | Direct video/audio tools | Indirect (web UI / design) tools |
|---|---|---|
| Rendering motion graphics from code | HyperFrames (01), huashu-motion (01), Remotion (01) | — |
| Animation quality and timing | GSAP skills (01), Design Motion Principles (04) | motion.dev (02), anime.js (02) |
| Sound design for rendered video | Real SFX + uisfx CC0 sounds (03) | — |
| Brand system / visual consistency | HyperFrames `frame.md` (01) | Awesome Claude Design / DESIGN.md (04), Design.md Chrome (04), UI UX Pro Max (04), Taste Skill (04) |
| Data visuals | HyperFrames chart blocks (01) | shadcn charts (02) |
| Prompts | Word-timed motion-graphic prompt (01) | — |

## Overall verdict: what to adopt first for claw-dj shorts

1. **HyperFrames + the coachspitz prompt (01)** — Apache-2.0, local HTML-to-MP4 with ~450 blocks; the prompt (transcribe with word timestamps → one graphic per line → vertical 1080×1920 → frame-by-frame check) is a ready faceless Shorts recipe. Complements our existing canvas mode (`skills/claw-shorts/canvas/`, formerly `skills/canvas-explainer`).
2. **GSAP skills (01)** — MIT skills that improve the motion in every HyperFrames render; GSAP itself is free including plugins.
3. **Real SFX (03)** — local frame-accurate sound design that keeps the AI mix untouched, tunes pitched hits to the track, checks mono, and outputs a separate SFX stem. Free; README says "Scripts: MIT" but the repo has no LICENSE file, so install and use it rather than copying its code. The 78 bundled UI cues are CC0.
4. **A claw-dj `DESIGN.md` (04: Awesome Claude Design format) + Design Motion Principles (04)** — one brand file feeding both site and videos, plus an anti-slop motion audit.
5. **Maybe later:** Remotion (free for individuals and companies up to 3 employees; Company License above that), huashu-motion (new; set `data-om-signature="off"` to remove the "Made with huashu-motion" signature), motion.dev / anime.js / shadcn charts / UI UX Pro Max / Taste Skill / Design.md Chrome for a landing page.
6. **Skip:** the Claude Code Club paid community CTA (04), and the presenter-led video formats (02, 03, 04).

## After installing them

[EVALUATION-2026-11-14.md](EVALUATION-2026-11-14.md) records the hands-on test: HyperFrames was adopted as a claw-shorts mode (`skills/claw-shorts/hyperframes/`, local install preferred over Docker), uisfx is pinned for interface-like cues, and the DESIGN.md files turned out to live in `VoltAgent/awesome-design-md`.
