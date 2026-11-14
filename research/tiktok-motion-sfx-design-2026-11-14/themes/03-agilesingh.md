# 03 — @agilesingh: "Claude can now sound design your videos" (Real SFX)

- **Source:** https://www.tiktok.com/@agilesingh/video/7702485321723743506
- **Creator:** @agilesingh (AgileSingh - AI)
- **Posted:** 2026-10-30
- **Duration:** 30 s
- **Stats at download time (2026-11-14, from info.json):** 4,143 views · 132 likes · 0 comments · 75 shares · 196 saves
- **Transcript:** [`../transcripts/agilesingh-7702485321723743506.txt`](../transcripts/agilesingh-7702485321723743506.txt)

## Screenshots

| File | What is readable |
|---|---|
| [`01-claude-sound-design-title.jpg`](../screenshots/agilesingh/01-claude-sound-design-title.jpg) | "Claude can now sound design" title over an editor timeline mock-up |
| [`02-sfx-placed-on-timeline.jpg`](../screenshots/agilesingh/02-sfx-placed-on-timeline.jpg) | Phone mock-up: Claude placing SFX clips on a timeline above "Logo slam" and music tracks |
| [`03-pitched-sounds-tuned-to-track.jpg`](../screenshots/agilesingh/03-pitched-sounds-tuned-to-track.jpg) | Cue grid over a piano keyboard: pitched sounds tuned to the track's notes |
| [`04-logo-slam-ui-clicks.jpg`](../screenshots/agilesingh/04-logo-slam-ui-clicks.jpg) | Logo-slam card with SFX markers on a timeline |
| [`05-real-sfx-name-card.jpg`](../screenshots/agilesingh/05-real-sfx-name-card.jpg) | "Real SFX" name card with its app-style icon |

## Walkthrough

The creator describes a Claude Code skill that watches a finished motion-graphics render, finds each moment that needs a sound (cuts, logo slams, UI clicks), pulls real recordings from your own sound library, and places each on the exact frame while leaving music and voice-over untouched. It tunes pitched sounds to the key of the music, matches loudness to the original sounds, checks the mix in mono for phone speakers, iterates until its own critique scores 8+, and then lists five moments for a human to check by ear because "Claude can't hear". The caption says it is "Free and MIT licensed. Needs Python and FFmpeg. Comes with 78 free UI sounds." The tool is named only at the very end: "Real SFX". The caption gives no link.

## Where Real SFX lives (the extra parsing)

**Repository:** [github.com/Clipwing/skills](https://github.com/Clipwing/skills) → folder [`skills/real-sfx/`](https://github.com/Clipwing/skills/tree/main/skills/real-sfx)
**Catalog page:** [clipwing.pro/skills/real-sfx](https://clipwing.pro/skills/real-sfx)

How it was found: GitHub repository search for `realsfx` and `real-sfx` returned no repo by that name; a web search for "Real SFX" Claude skill pointed to the Clipwing catalog page and to independent write-ups, all citing `https://github.com/Clipwing/skills`. The repo README's catalog table lists "Real SFX — Sound-designs motion videos with real recorded sound effects… Ships with 78 CC0 UI sounds", which matches the video.

| Fact | Finding (checked 2026-11-14) |
|---|---|
| Owner | `Clipwing` (GitHub organization). Skill README: "Made by Alexander Korbanov at Clipwing." |
| Repo description | "Claude Skills for video editing" (also contains a HyperFrames-to-After-Effects skill) |
| Stars / forks | 123 / 6 |
| Created | 2026-10-09 |
| Last commit on default branch | `edbefcb` "feat: add real-sfx skill", 2026-10-21 (UTC); last push to the repo 2026-11-11 (UTC) |
| Repo-level LICENSE file | **None.** There is no LICENSE file at the repo root or in `skills/real-sfx/`, and GitHub detects no license for the repo. |
| Stated license for the scripts | The skill README's "Licences" section says: "Scripts: MIT. uisfx (bundled mp3): code MIT, sounds CC0. Your own sound libraries keep their own licences." |
| The 78 UI sounds | Come from the npm package **uisfx** v0.4.0 by Yuki Capital ([romainsimon/uisfx](https://github.com/romainsimon/uisfx), MIT, 940 stars, homepage uisfx.com). A vendored copy sits in `skills/real-sfx/vendor/node_modules/uisfx/` with its own `LICENSE` (MIT, code) and `LICENSE-AUDIO`: the audio files are dedicated to the public domain under **CC0 1.0**, commercial use allowed, attribution not required. "78" is the number of cues; each cue exists in 12 styles (arcade, cinematic, dreamy, glass, mechanical, minimal, organic, rubber, scifi, soft, studio, zen), so 936 mp3 files in total. |
| Dependencies | Python 3.9+ and FFmpeg with ffprobe on PATH. On macOS the README requires Homebrew `ffmpeg-full`, because the plain `ffmpeg` formula lacks the `drawtext` and `rubberband` filters it uses. Node.js/npm only for `setup.py init --uisfx` (runs `npm install uisfx`, ~9 MB). |
| Optional downloads | `listen.py install --yes` creates a venv and downloads onnxruntime + numpy from PyPI (~25 MB) and `yamnet.onnx` (16.1 MB) from [huggingface.co/audiomagic/yamnet-onnx](https://huggingface.co/audiomagic/yamnet-onnx) (Apache-2.0) for AudioSet tags. Never run implicitly. |
| Payment / sign-up / gated download | None found. The repo, the `.skill` package in `packages/`, and the catalog page are public; no API key, account, or paid tier is referenced in the README or setup script. The skill ships **without a sound library**: you bring your own (the README suggests Freesound with the CC0 filter, Sonniss GDC bundles, Mixkit, Pixabay — each with its own license). |
| Tests | `python tests/selftest.py`: 189 checks, about 5 minutes. |

### Verdict: is it really free, MIT and open source?

- **Free:** yes. No payment, account or gated download anywhere in the install path.
- **MIT:** **stated, not formally attached.** The README says "Scripts: MIT", but the repo has no LICENSE file, so GitHub shows no license and there is no MIT text with a copyright line for the Real SFX scripts themselves. The bundled uisfx package does carry a proper MIT LICENSE (code) and CC0 LICENSE-AUDIO (sounds).
- **Open source:** the full source is public and readable; the license grant rests on the README sentence. For a public claw-work skill that vendors or adapts the scripts, the safe move is to open an issue asking Clipwing to add a LICENSE file, or to use it as an installed dependency rather than copying code.
- **Sound assets:** the 78 UI cues are CC0 and safe for commercial shorts. Any other library you point it at keeps its own license.

## Usefulness for claw-dj short-form content and marketing

| Aspect | Verdict | Why |
|---|---|---|
| **Real SFX skill** (direct audio-for-video tool) | **Useful** | Faceless shorts depend on sound design; this places whooshes, hits and clicks on the exact frame of a rendered motion graphic, keeps the music, and outputs a separate SFX stem. It reads HyperFrames `data-start` timelines directly, so it chains after HyperFrames. Local Python + FFmpeg, no SaaS. |
| **"Keeps your music and voice untouched" + separate SFX WAV** | **Useful** | Critical for claw-dj: the AI mix is the product, so SFX must sit on top without altering it, and a separate stem lets us remix levels later. |
| **Tuning pitched sounds to the track's notes** | **Useful** | A risers/impacts-in-key feature is exactly what DJ-oriented content needs so transitions do not clash with the mix. |
| **Mono check for phone speakers** | **Useful** | Most Shorts/TikTok views are on phone speakers; catching channel cancellation is a cheap quality win. |
| **78 CC0 UI sounds** | **Maybe** | Safe to use commercially, but UI clicks fit app promos more than DJ content; we will still need CC0 whooshes, risers and impacts (Freesound CC0 filter or Openverse). |
| **"Five moments to check by ear" review page** | **Useful** | A short human listening pass on a phone fits a one-person faceless channel workflow. |
| **License status** | **Maybe** | Fine to install and use; avoid copying its scripts into claw-work until a LICENSE file exists. |
| **Video format** (presenter + UI mock-ups) | **Not useful** | Presenter-led; the clean product mock-up inserts are the only reusable visual idea. |
