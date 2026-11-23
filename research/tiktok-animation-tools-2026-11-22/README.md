# Research: animation tools for agent-made Shorts (2026-11-22)

This batch covers one TikTok and one Notion link page about tools that give Claude "an animation studio" or "design skills". For each source there is a transcript or captured text, readable screenshots, verified links and a verdict on what helps our headless, faceless Shorts. Those Shorts are made by Claude Code, Codex, pi or Grok Build with the `skills/claw-shorts` modes, and nothing is posted automatically.

All links, licenses and star counts were checked on 2026-11-22.

## Index

| # | Source | Creator | Length | Views at fetch | Topic | Notes |
|---|---|---|---|---|---|---|
| 01 | [TikTok 7708682252066016534](https://www.tiktok.com/@nicksaraevdaily/video/7708682252066016534) | @nicksaraevdaily | 36 s | 27,100 | "20 free tools" for animation: Remotion Skills, Motion Canvas, Manim, Blender, Matter.js (+ a 20-card grid from a quoted X post) | [themes/01-nicksaraevdaily.md](themes/01-nicksaraevdaily.md) |
| 02 | [Notion page](https://noocap.notion.site/5-Free-Repos-That-Give-Claude-Real-Design-Skills-Every-Link-3dd508e99dda81ebade8ed0f108c895f) | Claude Code Club | — | — | Link page for the five design-skill repos already covered on 2026-11-14 | [themes/02-notion-5-design-repos.md](themes/02-notion-5-design-repos.md) |

## Layout

```text
README.md          this index and the evaluation
BIBLIOGRAPHY.md    every verified link with its license, stars and last push
themes/            one write-up per source
transcripts/       cleaned spoken text of the TikTok
screenshots/       readable stills (720 px wide)
```

## The "20 tools": what we could identify

The video names five tools. The caption's "20" refers to a grid of cards in a quoted X post, and 15 of those cards are identifiable:

- **Named in the voice-over (5):** Remotion Skills, Motion Canvas, Manim, Blender, Matter.js
- **Readable on the grid (10 more):** Anime.js, Motion, React Spring, mo.js, Flubber, Lottie Web, Rive Runtime, KUTE.js, AnimXYZ, Inochi2D

The other five are hidden behind the title card, and the original post could not be found, so they are not listed.

## Evaluation

**The test.** Can an agent make the tool produce a frame-exact video with no person and no GUI? A renderer that is seekable or deterministic can be driven frame by frame, in the same way HyperFrames seeks every frame and our canvas mode calls `window.renderFrame(t)`. A tool that runs on wall-clock time or only renders from an editor can't. The second question is whether the tool adds a visual we can't make already.

Our modes today (`skills/claw-shorts`): voiced explainer (Rust FFmpeg), canvas (`renderFrame(t)` in headless Chrome), HyperFrames (seek-based HTML with GSAP/CSS/WAAPI/Lottie/anime.js adapters), stock B-roll (`Rust/shortform-video`), recording cut (`Rust/claw-edit`).

| Tool | Headless & frame-exact? | Adds something new? | Verdict |
|---|---|---|---|
| **Manim** (Community, MIT) | Yes. A CLI renders a Python scene straight to MP4; output is deterministic and LaTeX is already installed here | **Yes.** Equations, graphs and geometric proofs in motion; nothing in our stack draws math | **Use as is** via `uv run --python 3.13 --with manim` (pip package; no clone). New claw-shorts **math** mode at vertical 1080×1920, then narration, captions and loudness from our Rust tools |
| **Matter.js** (MIT) | Only when stepped at a fixed `delta`. HyperFrames seeks each frame independently, so a JS sim would have to be simulated ahead and replayed | **Yes.** Physics shorts (balls falling, colliding, piling up) are a popular faceless genre, and collisions give exact times for sound effects | **Write our own in Rust** (`Rust/claw-motion`): deterministic fixed-step 2D circle physics, offline frames, collision-timed SFX. Credit Matter.js as the reference. Rapier (Apache-2.0) is the heavyweight alternative if we ever need polygons or joints |
| **Flubber** (MIT) | Yes; `interpolate(a, b)(t)` is a pure function of t | **Yes, small.** Smooth shape morphs (icon to icon, logo reveals) for kinetic cards | **Write our own in Rust** in the same crate: resample both outlines, align start points, interpolate. Flubber has been dormant since 2022, so there's no upstream to track |
| Remotion / Remotion Skills | Yes (`remotion render`) | No; HyperFrames already covers React-free HTML video and the agent workflow | **Skip.** Remotion License: Company License above 3 employees, and the skills repo has no LICENSE file of its own |
| Motion Canvas (MIT) | **No.** Rendering starts from its editor; issue #415 (headless render) open since 2023 | No; canvas mode and HyperFrames already make animated explainers | **Skip.** If the generator-style API is ever wanted, its fork Revideo (MIT) renders headless |
| Blender (GPL) | Yes, `blender -b -P script.py` | 3D and character rigs: not needed by any current format | **Closed (2026-11-22).** No planned Short needs 3D, and Manim's `ThreeDScene` covers 3D math (surfaces, vectors, orbits). Reopen only for a concrete 3D format Manim can't draw; then it's used as is (`blender -b -P`, GPL is fine for a tool), not rewritten |
| Anime.js (MIT) | Yes; v4 `autoplay: false` plus `.seek()`, and **HyperFrames has an adapter** | Little; GSAP already fills this role in HyperFrames | **Allowed inside HyperFrames** when existing anime.js code is reused; no new install |
| Motion / motion.dev (MIT) | Partly; controls can be paused and `time` set, but there's no HyperFrames adapter | No | **Skip for video.** A web-UI library |
| React Spring (MIT) | No; springs integrate per animation frame and there's no seek-to-time | No | **Skip** |
| mo.js (MIT) | Manual `setProgress()` only; no adapter; last release 2023 | Bursts and shape effects; GSAP or our Rust crate can do these | **Skip** |
| KUTE.js (MIT) | Tween engine with no adapter | No; GSAP covers SVG tweening | **Skip** |
| AnimXYZ (MIT) | CSS keyframes, which HyperFrames picks up, but it's built around Vue/React enter/leave transitions | No | **Skip** |
| Lottie Web (MIT) | Yes, through HyperFrames' Lottie/dotLottie adapter | Maybe: free After Effects-made animations (LottieFiles) as overlays | **Closed (2026-11-22): nothing to build.** HyperFrames already plays Lottie/dotLottie, so a downloaded animation can be dropped into a composition today (check each file's license). `velato` (Rust, Apache-2.0 OR MIT) is the path if a Rust renderer is ever needed. lottie-web itself hasn't been pushed since 2025-09 |
| Rive Runtime (MIT) | The runtime yes; authoring needs the Rive editor (hosted) | Interactive state machines, which is an app feature, not a video one | **Skip** |
| Inochi2D (BSD-2) | Realtime puppet app | A VTuber-style avatar | **Skip, final.** Our channels do not use avatar personas |
| Notion's five design repos | — | Already handled on 2026-11-14 | **Nothing new** (see theme 02). The page misidentifies Design Motion Principles |

### Did anything contradict the planned verdicts?

No. Three findings refined them:

1. **HyperFrames already supports anime.js and Lottie.** It ships runtime adapters for both, so neither needs a mode of its own. That moves anime.js from "skip" to "allowed inside HyperFrames".
2. **Motion Canvas's headless gap is confirmed.** Issue #415 is still open three years on. Revideo is the headless fork.
3. **Remotion Skills has no LICENSE file**, so the Remotion License from the monorepo it is generated from applies. That's one more reason to keep using HyperFrames.

### Format lessons from video 01

- Give each list item a **tiny animated demo** that shows the item doing its job: a prism splitting light, a sine wave tracing, balls falling. A logo card alone isn't enough. The new math mode and physics crate exist to make exactly these demos.
- An **"N of 5" progress bar** tells viewers how long the list is, which helps retention.
- **Cover what the hook promises.** The video hooks on "20 tools" and covers 5.

## Next

[EVALUATION-2026-11-22.md](EVALUATION-2026-11-22.md) records what ran and what was adopted: the Manim math mode and `Rust/claw-motion` (physics and morph).
