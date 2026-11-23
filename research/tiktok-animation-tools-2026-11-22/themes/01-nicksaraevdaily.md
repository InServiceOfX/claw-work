# 01 — @nicksaraevdaily: "These 20 free tools give Claude an entire animation studio"

- **Source:** https://www.tiktok.com/@nicksaraevdaily/video/7708682252066016534
- **Creator:** @nicksaraevdaily (Nick Saraev, "Daily AI news")
- **Posted:** 2026-11-16
- **Duration:** 36 s
- **Stats at metadata fetch (2026-11-22, from info.json):** 27,100 views · 1,562 likes · 9 comments · 439 shares · 2,152 saves
- **Transcript:** [`../transcripts/nicksaraevdaily-7708682252066016534.txt`](../transcripts/nicksaraevdaily-7708682252066016534.txt)

### Caption (verbatim)

> These 20 free tools give Claude Opus 5.5 an entire animation studio. Remotion Skills teaches Opus how to make video, Motion Canvas turns an idea into an animated explainer, Manim puts math in motion, Blender handles rigging and character animation, and Matter.js adds real-time 2D physics. Follow for more AI news like this. #animation #claudeai #motiondesign #opensource #aitools

## Screenshots

| File | Time | What is readable |
|---|---|---|
| [`01-hook-20-free-tools-x-post.jpg`](../screenshots/nicksaraevdaily/01-hook-20-free-tools-x-post.jpg) | 0:03.3 | Hook card "Claude Opus 5.5 can run an animation studio" over a quoted X post by "eigen" (@eigenqq), with a "20 free tools" badge |
| [`02-x-post-tool-grid-crop.jpg`](../screenshots/nicksaraevdaily/02-x-post-tool-grid-crop.jpg) | 0:02.9 (crop) | The X post's tool grid: Remotion Skills, Anime.js, Motion, React Spring, mo.js, Flubber, Lottie Web, Rive Runtime, KUTE.js, AnimXYZ, Inochi2D, plus a Manim-style sine card |
| [`03-remotion-skills-claude-skills.jpg`](../screenshots/nicksaraevdaily/03-remotion-skills-claude-skills.jpg) | 0:10.5 | "The first: Remotion Skills" card, a `.claude/skills` folder, "Guidance for Opus · How to make video" |
| [`04-motion-canvas-prism-explainer.jpg`](../screenshots/nicksaraevdaily/04-motion-canvas-prism-explainer.jpg) | 0:18.3 | Motion Canvas demo: prompt "Explain how a prism splits light" → `scene.tsx` player with a prism dispersing a beam |
| [`05-manim-sine-wave.jpg`](../screenshots/nicksaraevdaily/05-manim-sine-wave.jpg) | 0:23.2 | Manim card: unit circle tracing y = sin(x − t) |
| [`06-blender-rig-keyframes.jpg`](../screenshots/nicksaraevdaily/06-blender-rig-keyframes.jpg) | 0:26.5 | Blender card: a stick-figure rig with a keyframe strip |
| [`07-matter-js-gravity.jpg`](../screenshots/nicksaraevdaily/07-matter-js-gravity.jpg) | 0:34.0 | Matter.js card: coloured balls fallen into a container, "↓ 9.81 m/s²" |

## Walkthrough

The caption and hook promise **20 tools**, but the video itself names only **five**, each on a "N of 5" card: Remotion Skills, Motion Canvas, Manim, Blender and Matter.js. The "20" comes from the grid of tool cards in a quoted X post by "eigen" (handle shown as @eigenqq) that sits behind the hook title for the first 4 seconds.

### The 20-tool grid: what can and can't be read

Readable on the grid (screenshot 02), in addition to the five named in the voice-over:

| Card | Subtitle on card |
|---|---|
| Remotion Skills | — |
| Anime.js | (partly hidden) |
| Motion | (partly hidden) |
| React Spring | (partly hidden) |
| mo.js | "Bursts and shape effects" |
| Flubber | "Smooth shape morphing" |
| Lottie Web | "Vector animation playback" |
| Rive Runtime | "Interactive state machines" |
| KUTE.js | "SVG tweening" |
| AnimXYZ | "CSS choreography" |
| Inochi2D | (cut off at the bottom) |
| (unnamed) | A sine curve card, "y = sin(t)" (Manim) and a path card "scene.tsx" (Motion Canvas) |

With Blender and Matter.js that makes **15 identifiable tools**. The other five cards are hidden behind the title card or cropped off the bottom, and web searches on 2026-11-22 did not find the original X post. We do not guess them.

### Transcript corrections

Whisper `small` misheard three phrases; the transcript file has the corrected text:

| Whisper | Corrected | Evidence |
|---|---|---|
| "The third is Madden" | "The third is Manim" | Caption and the "Pick Number Three — Manim" card |
| "So he knows how to build a video before he gets started" | "So it knows … before it gets started" | Subject is Opus |
| "Follow from AN News just like this" | "Follow for more AI news just like this" | On-screen "follow for more 'AI News'" |

### Format notes (for our own Shorts)

- **Hook card quoting an X post.** The first frame is someone else's post with a big serif-italic title over it and a "20 free tools" badge. Cheap to make and borrows the post's credibility.
- **Presenter in a lower frame, motion-graphic inserts above.** Each tool gets a short animated demo: a folder icon, a prism dispersing light, a sine wave being traced, a rig being keyframed, balls falling under gravity. The inserts are the faceless part, and they're what we can make without a presenter.
- **Word-by-word captions** that colour one keyword ("Opus", "move") and a small pixel-art mascot reacting to each beat (lightbulb, antenna, hard hat).
- **"N of 5" progress bar** at the top of each tool card.
- **Overclaim.** "20 free tools" in the hook vs five covered. Our own listicles should cover what the hook promises.

## Usefulness for our Shorts

See the verdict table in [`../README.md`](../README.md#evaluation). In short: Manim (math in motion) and a Matter.js-style physics sim are the two that create new faceless visuals we can't make today; the format lesson is "one tiny animated demo per item".
