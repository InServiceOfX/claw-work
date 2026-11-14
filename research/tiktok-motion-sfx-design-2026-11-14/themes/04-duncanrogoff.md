# 04 — @duncanrogoff: "5 free repos that give Claude design skills"

- **Source:** https://www.tiktok.com/@duncanrogoff/video/7686228759833300237
- **Creator:** @duncanrogoff (Duncan | AI Automation)
- **Posted:** 2026-09-16
- **Duration:** 57 s
- **Stats at download time (2026-11-14, from info.json):** 95,900 views · 4,958 likes · 28 comments · 1,446 shares · 7,005 saves
- **Transcript:** [`../transcripts/duncanrogoff-7686228759833300237.txt`](../transcripts/duncanrogoff-7686228759833300237.txt)

## Screenshots

| File | What is readable |
|---|---|
| [`01-taste-skill.jpg`](../screenshots/duncanrogoff/01-taste-skill.jpg) | "Taste Skill" wordmark card |
| [`02-ui-ux-pro-max-repo-stats.jpg`](../screenshots/duncanrogoff/02-ui-ux-pro-max-repo-stats.jpg) | UI UX Pro Max GitHub sidebar at recording time: MIT license, 128.1k stars, 518 watching, 13.7k forks, v2.15.0, 41 releases, 97 contributors |
| [`03-getdesign-md-design-systems.jpg`](../screenshots/duncanrogoff/03-getdesign-md-design-systems.jpg) | getdesign.md site (home of the Awesome Claude Design collection) |
| [`04-kylezantos-design-motion-principles.jpg`](../screenshots/duncanrogoff/04-kylezantos-design-motion-principles.jpg) | Repo path "kylezantos/design-motion-principles" |
| [`05-design-md-chrome-extension.jpg`](../screenshots/duncanrogoff/05-design-md-chrome-extension.jpg) | "TypeUI DESIGN.md Extractor (Chrome Extension)" README header |

## Walkthrough

A talking-head video with B-roll of each repo. Duncan lists five free GitHub repos for making Claude-built front ends look less generic: Taste Skill (anti "AI-generated look"), UI UX Pro Max (layout, typography, color), Awesome Claude Design (pre-built design systems), Design Motion Principles (animation, transitions, easing), and Design.md Chrome (extract a design system from any website). The links are in a pinned comment that we cannot read. The closing CTA promotes his **Claude Code Club**, shown on screen as a Skool community (skool.com/claudecodeclub, "8k members", "JOIN $9/month"), which also offers "the repo link". We use the free public repos below instead and do not join or pay.

## The 5 GitHub repos (found by public search, verified 2026-11-14)

Each repo was identified by GitHub repository search on the spoken name and confirmed against what the video shows on screen.

| # | Name in video | Repo | Resolves | License (LICENSE file) | Stars (live) | Last push (UTC date) | How it was matched |
|---|---|---|---|---|---|---|---|
| 1 | TasteSkill | [github.com/Leonxlnx/taste-skill](https://github.com/Leonxlnx/taste-skill) | 200 | MIT | 115,484 | 2026-11-08 | Top search result for `taste-skill`; README wordmark "Taste Skill" matches screenshot 01; site tasteskill.dev |
| 2 | UI UX Pro Max | [github.com/nextlevelbuilder/ui-ux-pro-max-skill](https://github.com/nextlevelbuilder/ui-ux-pro-max-skill) | 200 | MIT | 149,399 | 2026-11-14 | Sidebar in screenshot 02 (MIT, v2.15.0, 41 releases) matches this repo |
| 3 | Awesome Claude Design | [github.com/VoltAgent/awesome-claude-design](https://github.com/VoltAgent/awesome-claude-design) | 200 | MIT | 4,545 | 2026-06-20 | Repo description "68 ready-to-use design system inspirations in DESIGN.md format"; homepage getdesign.md matches screenshot 03 |
| 4 | Design Motion Principles | [github.com/kylezantos/design-motion-principles](https://github.com/kylezantos/design-motion-principles) | 200 | MIT | 1,569 | 2026-05-30 | Repo path shown in screenshot 04 |
| 5 | Design.md Chrome | [github.com/bergside/design-md-chrome](https://github.com/bergside/design-md-chrome) | 200 | MIT | 3,180 | 2026-05-25 | README title "TypeUI DESIGN.md Extractor (Chrome Extension)" matches screenshot 05 |

### Claimed numbers vs live repos

| Claim in video | Check | Status |
|---|---|---|
| UI UX Pro Max "128,000 stars" | Screenshot shows 128.1k at recording; live count 149,399 on 2026-11-14 | **Verified** (accurate when recorded, since grown) |
| Awesome Claude Design "68 pre-built design systems" | README badge "DESIGN.md count 68" and repo description say 68. The repo itself holds only README.md and LICENSE; the DESIGN.md files are linked out to getdesign.md | **Verified as listed** (files are hosted on getdesign.md, not in the repo) |
| "Five free repos" | All five are public with MIT LICENSE files | **Verified** |
| Claude Code Club "almost eight thousand of us" | On-screen card reads "8k members"; not independently checked | **Creator's claim** |

### Paid tiers, extras, and notes

- **Taste Skill** — MIT. The README promotes a companion desktop app, TasteCode ("free and open source"), and lists sponsors.
- **UI UX Pro Max** — MIT; Python 3.x; also published as the npm CLI `ui-ux-pro-max-cli`. README badges: 192 reasoning rules, 80 searchable UI styles. Has a PayPal donate link; no paywall.
- **Awesome Claude Design** — MIT list, aimed at Anthropic's Claude Design workspace (a Claude product feature) but the DESIGN.md files work as plain context for any agent. The linked site getdesign.md sells extras: "Video Templates: Get all editable template files for $29, one time" and a $29 "launchkit". The DESIGN.md files themselves are free to read.
- **Design Motion Principles** — MIT. Two modes: *create* (components with motion) and *audit* (anti-AI-slop motion checklist, HTML report). Distilled from the published work of Emil Kowalski, Jakub Krehel and Jhey Tompkins. Install: `npx skills add kylezantos/design-motion-principles`.
- **Design.md Chrome** — MIT; load unpacked in `chrome://extensions` or install from the Chrome Web Store listing "DESIGN.md Style Extractor". Outputs `DESIGN.md` or `SKILL.md` in the open TypeUI DESIGN.md format.

## Usefulness for claw-dj short-form content and marketing

All five are **web-UI / front-end design** resources. Their value to claw-dj is indirect: a sharper landing page, and HTML-authored motion graphics rendered to MP4 via HyperFrames (whose `frame.md` layer explicitly translates a brand `design.md` into video rules) or our `canvas-explainer`.

| Aspect | Verdict | Why |
|---|---|---|
| **Taste Skill** | **Maybe** | Helps avoid the generic AI look on a claw-dj site or HTML-rendered cards; not a video tool. Worth it once we build a landing page. |
| **UI UX Pro Max** | **Maybe** | Strong typography/color guidance for a brand style sheet, but web-UI oriented (responsive breakpoints, components). One-time use to settle claw-dj palette and type. |
| **Awesome Claude Design / DESIGN.md format** | **Useful** | A single `DESIGN.md` for claw-dj (lobster red, type scale, motion rules) can drive both the website and HyperFrames videos via `frame.md`, keeping every short on-brand. The list gives good starting templates; skip the paid getdesign.md extras. |
| **Design Motion Principles** | **Useful** | Easing, timing and an anti-slop motion checklist apply directly to motion graphics in shorts (the "audit" mode can review a HyperFrames composition). |
| **Design.md Chrome** | **Maybe** | Handy for extracting a reference aesthetic from a site we admire into DESIGN.md; low effort, but only a starting point. |
| **Claude Code Club CTA** ($9/month Skool community) | **Not useful** | Paid community; the repos are public and free without it. |
| **Video format** (presenter + repo B-roll) | **Not useful** | Presenter-led; the faceless "N repos" list style from video 01 is the better template for claw-dj. |
