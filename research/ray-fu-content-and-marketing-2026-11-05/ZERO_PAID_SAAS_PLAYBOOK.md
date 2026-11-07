# Zero-paid-SaaS engineering and marketing playbooks

Actionable playbooks extracted from 14 [@ray_fu](https://www.tiktok.com/@ray_fu) TikToks (7 content + 7 marketing). Every workflow works with Claude / Codex / Grok basic plans, local `ffmpeg`, Playwright, Remotion, yt-dlp, faster-whisper, and OSS TTS (Kokoro / Piper / F5-TTS). **No ElevenLabs, paid Apollo, paid Clay, paid Higgsfield, or paid Arcads seats required.**

---

## 1. ICP → warm outreach loop

**Sources:** videos on sales prospecting, lead-gen enrichment, and ICP + buying signals.

1. Interview yourself (5–10 questions) → produce `icp.md` + `buying_signals.md`.
2. Discover companies via public web / GitHub / job boards / news (Agent-Reach or manual research). **Never scrape LinkedIn with a primary account's cookies** (ToS risk).
3. Score prospects against your past best clients.
4. Research each target (site, news, public posts) → personalized draft.
5. **Human approve-and-send**; use SES / Google Workspace. Skip Instantly / Apollo if unpaid.

### Buying signals to watch

- Hiring spikes (engineering, sales)
- Funding announcements
- Stack migrations
- "Looking for tool / agency" posts on Reddit / X
- Competitor complaint threads

---

## 2. Competitor ad hook mining

**Source:** Meta Ad Library hook-cloner video.

1. Meta Ad Library → filter niche, sort by **longest-running** ads (30+ days → likely profitable).
2. Capture first-3-second hooks + structure (not logos / trademarks).
3. Claude `/clone_hook` style rewrite for your product → 10 copy variants.
4. Render static / UGC-style creatives with Remotion / ffmpeg (**no AI influencer actors**).
5. Human launches in Ads Manager; agent proposes scale / kill after ~3 days CAC read.

### OSS tooling

- https://github.com/jacopocappelli1989/facebook-library-ads-mcp
- https://github.com/the-baweja/competitor-ad-spy

---

## 3. Always-on marketing org (Paperclip)

**Source:** zero-budget Paperclip marketing team video.

Self-host Paperclip: `npx paperclipai onboard --yes`

Roles:
- **Content Strategist** — trends via Verlo or web sweeps, content calendar
- **Video Editor** — Remotion skill for UGC / shortform
- **Social Media Manager** — schedule TikTok / IG / LinkedIn / X
- **QA** — review before publish

Routines:
- **Daily:** strategist viral check → brief → editor + social → QA → draft schedule
- **Weekly:** strategist reviews performance → next-week plan
- **On release:** auto-draft Discord / Slack / email / X update

Repos:
- https://github.com/paperclipai/paperclip
- https://github.com/paperclipai/companies

---

## 4. Free multi-platform research (Agent-Reach)

**Source:** Agent-Reach multi-platform scraper video.

Install: `pip install agent-reach && agent-reach install --env=auto && agent-reach doctor`

Read-only CLI for: GitHub, YouTube, Reddit, X / Twitter, LinkedIn (jobs/optional), Bilibili, XiaoHongShu, web / Exa.

**Safety:** default channels need no paid API. Logged-in channels use cookies/session. Use spare accounts; LinkedIn scraping risks primary account.

Repo: https://github.com/Panniantong/Agent-Reach

---

## 5. Launch landing page in < 1 hour

**Source:** fast app landing-page build video.

Claude / Codex / Grok + GitHub → conversion landing page with:
- Hero with clear value prop
- Social proof section
- Feature grid
- How-it-works
- Pricing stub
- FAQ
- Final CTA

**Mobile-first, no AI-slop gradients.**

### Prompt skeleton

```
Build a conversion-focused landing page for {product}.
Sections: hero with clear value prop, social proof, feature grid,
how-it-works, pricing stub, FAQ, final CTA.
Color scheme: {colors}. Mobile-first. Accessible.
Output a minimal Node/static site I can run locally.
```

---

## 6. Content growth engine

**Sources:** faceless YouTube niche/RPM, 5-agent content team, 7-agent YouTube factory, cross-platform hook scoring, computer-use YouTube pipeline videos.

### Niche selection

Niche by **RPM + repeatability**, not passion:
- Finance explainers: $15–50 CPM
- AI / tech explainers: $10–25 CPM
- Gaming: ~$2 CPM

### 5-agent markdown team

Fire in sequence (each agent may only consume the previous agent's output):
1. **Strategist** — topic + audience → angles, hooks, subpoints, what to avoid
2. **Researcher** — 5 primary sources, 3 key stats, 3 contrarian datapoints; **current date mandatory**
3. **Writer** — full draft from brief + length / structure
4. **Editor** — cut ~30%, sharpen opening hook, rewrite closer
5. **Publisher** — format for LinkedIn / X / YT script / Substack / IG

### Hook scoring system

1. **Research sweep:** YouTube top videos (time-filtered), Reddit pain-point quotes, X high-engagement tweets.
2. **Specialized agents in sequence:** Hook agent writes 4 hook variations; Hook manager scores 5 dimensions (must be 10/10 to pass).
3. **Line-level gates:** every line scored on **Venture novelty** (feels like a breakthrough) and **Copy intensity** (emotion); both 10/10 or rewrite.

### Rendering stack (zero paid SaaS)

| Video uses | OSS replacement |
|---|---|
| ElevenLabs VO | Kokoro, Piper, or F5-TTS; Claude/Codex write SSML-ish pause markers |
| CapCut / Camtasia | `ffmpeg` + Remotion or headless Playwright frame loop |
| Pexels stock | Pexels free API / local B-roll / Remotion programmatic scenes |
| TubeBuddy / vidIQ | YouTube Data API (free quota) + Claude keyword clustering; or Agent-Reach YouTube search |

### Optional factories

- `darkzOGx/youtube-automation-agent` with local TTS swap: https://github.com/darkzOGx/youtube-automation-agent
- Clip long-form with `zhouxiaoka/autoclip_mvp`: https://github.com/zhouxiaoka/autoclip_mvp

### Headless motion graphics pipeline

Claude writes code that paints every frame. Studio = Node + Playwright (Chrome captures frames) + ffmpeg (stitch MP4).

Rules:
- Ban generic all-caps-on-gradient "AI slop"
- Require real assets + closed-form spring physics + synthesized audio locked to beat
- Self-critique loop: Claude grades own frames, hunts dead frames, re-renders until ≥8/10

---

## Explicit non-goals

- AI influencer personas / PersonaLive livestream avatars (documented in repos roundup video; **not adapted**)
- Any workflow that **requires** ElevenLabs, Apollo paid, Clay paid, Higgsfield, Arcads subscription
- Unattended mass cold-email without verification + human send gate
- LinkedIn automation on primary accounts

---

## GitHub / OSS repositories (verified)

### Content

| Repo | URL |
|---|---|
| AutoClip MVP | https://github.com/zhouxiaoka/autoclip_mvp |
| Remotion | https://github.com/remotion-dev/remotion |
| OpenHands | https://github.com/OpenHands/OpenHands |
| PersonaLive (documented; NOT adapted) | https://github.com/GVCLab/PersonaLive |
| YouTube Automation Agent (7 agents) | https://github.com/darkzOGx/youtube-automation-agent |
| Kokoro TTS | https://github.com/hexgrad/kokoro |
| Piper TTS | https://github.com/rhasspy/piper |
| F5-TTS | https://github.com/SWivid/F5-TTS |

### Marketing

| Repo | URL |
|---|---|
| Agent Reach | https://github.com/Panniantong/Agent-Reach |
| Paperclip | https://github.com/paperclipai/paperclip |
| Paperclip companies / templates | https://github.com/paperclipai/companies |
| Facebook Library Ads MCP | https://github.com/jacopocappelli1989/facebook-library-ads-mcp |
| competitor-ad-spy | https://github.com/the-baweja/competitor-ad-spy |

---

## Source TikToks (14)

Creator: https://www.tiktok.com/@ray_fu

See `content/` and `marketing/` folders for per-video breakdowns.
