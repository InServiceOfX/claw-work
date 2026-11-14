# 02 — @jackroberts____: "Top 3 websites for Claude users" (motion.dev, anime.js, shadcn charts)

- **Source:** https://www.tiktok.com/@jackroberts____/video/7706188024962420001
- **Creator:** @jackroberts____
- **Posted:** 2026-11-09
- **Duration:** 35 s
- **Stats at download time (2026-11-14, from info.json):** 44,400 views · 2,700 likes · 11 comments · 657 shares · 4,083 saves
- **Transcript:** [`../transcripts/jackroberts____-7706188024962420001.txt`](../transcripts/jackroberts____-7706188024962420001.txt)

## Screenshots

| File | What is readable |
|---|---|
| [`01-motion-dev.jpg`](../screenshots/jackroberts____/01-motion-dev.jpg) | motion.dev home page in a browser tab: "The animation library loved by humans and agents alike" |
| [`02-animejs.jpg`](../screenshots/jackroberts____/02-animejs.jpg) | animejs.com tab with the "All-in-one animation engine" hero |
| [`03-shadcn-charts.jpg`](../screenshots/jackroberts____/03-shadcn-charts.jpg) | ui.shadcn.com chart gallery (donut, bar, line charts) |
| [`04-chart-matching-design.jpg`](../screenshots/jackroberts____/04-chart-matching-design.jpg) | Demo travel site ("Kyoto", "Trips booked", "By month", "1,260") with charts styled to match |

## Walkthrough

A short talking-head-plus-screen-recording video. The creator names three sites he keeps open while building websites with Claude: motion.dev for transitions ("written so that AI can use them"), anime.js for custom motion (springs, timelines, morphing shapes), and shadcn charts for data visualizations that match the site's design. The demo is a travel-booking front end that gains transitions, eased curves, and themed charts. The caption sums it up: "Same builder, better finish."

## The 3 website links (verified 2026-11-14)

| # | Link | Resolves | Underlying repo | License | Stars (live) |
|---|---|---|---|---|---|
| 1 | [motion.dev](https://motion.dev) | 200 | [motiondivision/motion](https://github.com/motiondivision/motion) | MIT | 35,144 |
| 2 | [animejs.com](https://animejs.com) | 200 | [juliangarnier/anime](https://github.com/juliangarnier/anime) | MIT | 74,641 |
| 3 | [ui.shadcn.com/docs/components/chart](https://ui.shadcn.com/docs/components/chart) | 200 (redirects to `/docs/components/base/chart`) | [shadcn-ui/ui](https://github.com/shadcn-ui/ui) (charts are built on [Recharts](https://github.com/recharts/recharts)) | MIT (both) | 128,179 / 27,712 |

### Paid tiers and limits

- **motion.dev:** the library is MIT and free. The site also sells **Motion+**, a paid add-on ("unlocks the full animation workflow"); it is not needed to use the library.
- **anime.js:** MIT; the site asks for sponsorship but has no paywall.
- **shadcn charts:** MIT copy-paste components on top of Recharts; no account needed.

## Usefulness for claw-dj short-form content and marketing

All three are **web front-end** tools. Their value to claw-dj is indirect: a better landing page, or motion graphics that are authored as HTML and rendered to MP4 (HyperFrames or our claw-shorts canvas mode).

| Aspect | Verdict | Why |
|---|---|---|
| **motion.dev** (web animation library) | **Maybe** | Good for a claw-dj landing page or link-in-bio site. For video it is less direct than GSAP, which HyperFrames already uses for seekable timelines; Motion's React-centric, interaction-driven animations do not map as cleanly to frame-by-frame rendering. |
| **anime.js** (web animation engine) | **Maybe** | Timelines, springs and SVG morphing can be scrubbed per frame, so it could animate a lobster mascot or waveform inside an HTML scene rendered to video. Overlaps with GSAP; adopt only if a scene needs its morphing features. |
| **shadcn charts** (data-viz components) | **Maybe** | Useful for an "AI mix stats" card (BPM curve, energy per minute, key changes) in an HTML-rendered short or on the site. Charts are a small part of DJ content, so this is situational. |
| **Video format** (talking head + screen recording) | **Not useful** | Relies on an on-camera presenter; claw-dj is faceless. |
| **"Give Claude a better toolbox" idea** | **Useful** | The underlying lesson — point the agent at a named, AI-documented library instead of letting it invent animation code — applies equally to our video skills (GSAP skills, HyperFrames registry). |
