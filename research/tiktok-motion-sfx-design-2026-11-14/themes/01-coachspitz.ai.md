# 01 — @coachspitz.ai: "Claude Motion is Team plan only — 4 free ways"

- **Source:** https://www.tiktok.com/@coachspitz.ai/video/7702553659275496718
- **Creator:** @coachspitz.ai (Coachspitz.ai)
- **Posted:** 2026-10-30
- **Duration:** 50 s
- **Stats at download time (2026-11-14, from info.json):** 2,898 views · 112 likes · 3 comments · 59 shares · 134 saves
- **Transcript:** [`../transcripts/coachspitz.ai-7702553659275496718.txt`](../transcripts/coachspitz.ai-7702553659275496718.txt)

## Screenshots

| File | What is readable |
|---|---|
| [`01-four-free-ways.jpg`](../screenshots/coachspitz.ai/01-four-free-ways.jpg) | "4 / $0 free ways" title card with the four tool icons |
| [`02-hyperframes-apache-2.jpg`](../screenshots/coachspitz.ai/02-hyperframes-apache-2.jpg) | HyperFrames card: "70.3k stars · by HeyGen · Apache 2.0 · you describe → Claude writes → real MP4" |
| [`03-huashu-motion-mit-signature-toggle.jpg`](../screenshots/coachspitz.ai/03-huashu-motion-mit-signature-toggle.jpg) | huashu motion card: "1.5k stars · free · MIT · 5 days old · I scanned it" and the "Made with huashu" tag toggle |
| [`04-remotion-free-if-just-you.jpg`](../screenshots/coachspitz.ai/04-remotion-free-if-just-you.jpg) | Remotion card: "66.5k stars · the original · free if it's just you · Claude already knows it" |
| [`05-gsap-skills-official-mit.jpg`](../screenshots/coachspitz.ai/05-gsap-skills-official-mit.jpg) | GSAP skills card: "17k stars · official · MIT · not a video tool · teaches Claude to animate" |

The star counts on screen are from when the video was made; live counts are in the table below.

## Walkthrough

The video argues that Anthropic's "Claude Motion" feature is limited to the Team plan, and that because it is "just code making video", four free open repos get the same result with Claude Code. The whole video is itself a faceless-style motion-graphic list (after a 6-second talking-head hook): one numbered card per tool, a GitHub repo screenshot, and pill-shaped labels timed to the voice-over. It ends with a save/follow CTA and says every link is in the caption.

### The creator's prompt (verbatim from the caption)

> "Here's my edited video with no captions or graphics yet. Transcribe it with word timestamps, then build a motion graphic for every line where I show or explain something, timed to the exact word. Vertical 1080x1920, my brand colors, real logos only, no placeholders. Check every scene frame by frame before you render."

The caption says to paste this into Claude Code with HyperFrames installed.

### Other caption tips (verbatim points)

- huashu motion "is brand new, so I scanned it before I installed it. Tip: install it from a git clone, not `npx skills add`."
- "Add `data-om-signature="off"` so it doesn't stamp 'Made with huashu' on your videos."
- Remotion is "free for individuals and very small teams".

## The 4 GitHub links (verified 2026-11-14)

| # | Repo | Resolves | License (from LICENSE file) | Stars (live) | Last push (UTC date) |
|---|---|---|---|---|---|
| 1 | [github.com/heygen-com/hyperframes](https://github.com/heygen-com/hyperframes) | 200 | Apache-2.0 | 84,227 | 2026-11-14 |
| 2 | [github.com/alchaincyf/huashu-motion](https://github.com/alchaincyf/huashu-motion) | 200 | MIT (bundles GSAP under the GSAP Standard License and fonts under SIL OFL 1.1, per its README / THIRD_PARTY.md) | 2,701 | 2026-11-10 |
| 3 | [github.com/remotion-dev/remotion](https://github.com/remotion-dev/remotion) | 200 | Remotion License (custom, source-available; GitHub reports NOASSERTION) | 73,148 | 2026-11-14 |
| 4 | [github.com/greensock/gsap-skills](https://github.com/greensock/gsap-skills) | 200 | MIT | 18,439 | 2026-07-29 |

### License and requirement details

- **HyperFrames** — "Write HTML. Render video. Built for agents." Requires Node.js 22+ and FFmpeg. Renders by seeking each frame in headless Chrome and encoding with FFmpeg, locally or in Docker (a cloud render option also exists; local rendering needs no account). Ships ~450 registry blocks (transitions, captions, charts, kinetic type) and a `frame.md` layer that turns a brand `design.md` into video rules.
- **huashu-motion** — Chinese-language README; "commercial-grade launch films by your coding agent". Requires Node 22+, Chrome and FFmpeg. On films of 4 s or longer it adds a small "Made with huashu-motion" signature in a free corner for the last 1.2 s. The repo's own code confirms the switch: `cli/lib/render-om.js` prints `data-om-signature="off" on the root removes it`, and `cli/lib/check.js` also accepts a corner value such as `data-om-signature="top-left"`. The README documents both `npx skills add alchaincyf/huashu-motion` and `git clone`; the creator recommends the clone.
- **Remotion** — Free License covers individuals, for-profit organizations with **up to 3 employees**, non-profits, and evaluation. Larger for-profit organizations need a paid **Company License** (pricing at remotion.dev/docs/license/pricing). Copying or modifying Remotion to sell your own derivative is disallowed. The LICENSE notes the license changes slightly in Remotion 5.0.
- **GSAP skills** — official GreenSock agent skills (core, timelines, ScrollTrigger, plugins, React/Vue/Svelte). The README states GSAP itself is now 100% free including every former Club plugin (SplitText, MorphSVG…) after Webflow's acquisition. Note: the *skills repo* is MIT; the *GSAP library* is under GreenSock's own "Standard License", not MIT.

## Usefulness for claw-dj short-form content and marketing

Context: claw-dj = "AI mixes" brand, lobster mascot, faceless YouTube Shorts and TikTok, local/open-source tools, no paid SaaS voices, no AI influencer personas. claw-work already has a working canvas pipeline (HTML canvas → headless browser → FFmpeg): the canvas mode of `skills/claw-shorts`, formerly `skills/canvas-explainer`.

| Aspect | Verdict | Why |
|---|---|---|
| **HyperFrames** (direct video tool) | **Useful** | HTML-to-MP4 with deterministic headless-Chrome rendering is the same model as our claw-shorts canvas mode, but with ~450 ready blocks (captions, kinetic type, transitions, charts). Apache-2.0 and local, so it fits a lobster-mascot explainer or a "track A → track B" transition card with no SaaS. |
| **The caption prompt** (word-timed motion graphics) | **Useful** | Transcribe → one graphic per line → timed to the word → vertical 1080×1920 → frame-by-frame check is exactly a faceless Shorts recipe. Swap "my edited video" for a claw-dj mix clip plus local Whisper word timings and "real logos only" for the lobster mascot assets. |
| **huashu-motion** (direct video tool) | **Maybe** | MIT, local, and it can write its own soundtrack, but it is brand new, Chinese-only docs, and stamps "Made with huashu-motion" unless `data-om-signature="off"` is set. Worth a sandboxed trial for launch-film style promos; the bundled GSAP library carries its own license. |
| **Remotion** (direct video tool) | **Maybe** | Mature React-to-video with strong agent knowledge, and free for an individual or a company of up to 3 employees. Not open source in the OSI sense; if claw-dj ever grows past that threshold it needs a paid Company License, so HyperFrames is the lower-risk default. |
| **GSAP skills** (animation knowledge for agents) | **Useful** | Not a video tool, but HyperFrames and huashu both animate with GSAP, so these skills directly improve the easing and timing of every rendered short. MIT skills; GSAP itself is free including plugins. |
| **"Claude Motion is Team plan only" framing** | **Not useful** | Product-plan news, not a technique; nothing to adopt. |
| **Video format itself** (numbered tool cards, repo screenshot, timed pills) | **Useful** | A faceless, repeatable "N free tools" list format that claw-dj can copy for "3 free tools behind this AI mix" shorts, rendered with HyperFrames or the claw-shorts canvas mode. |
