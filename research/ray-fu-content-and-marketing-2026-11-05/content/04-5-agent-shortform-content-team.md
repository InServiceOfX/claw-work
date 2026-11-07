# 5-agent short-form content team

- **URL:** https://www.tiktok.com/@ray_fu/video/7655493506986757407
- **Views / duration:** ~20.2K / 77s (Jun 25, 2026)
- **Agents (markdown files in `.claude/agents/`):**
  1. **Strategist** — topic+audience → angles, hooks, 3 subpoints, what to avoid; never writes the piece; decides if worth making.
  2. **Researcher** — 5 primary sources, 3 key stats, 3 contrarian datapoints; **current date** mandatory.
  3. **Writer** — full draft from brief + length/structure.
  4. **Editor** — cut ~30%, sharpen opening hook, rewrite closer (screenshot-worthy).
  5. **Publisher** — format for LinkedIn / X / YT script / Substack / IG.

## Critical design rule
Each agent may only consume the **previous agent’s output** (no role contamination). Fire in sequence: Strategist → Researcher → Writer → Editor → Publisher.

## Zero-paid-SaaS adaptation
Pure Claude/Codex/Grok markdown agents + web/GitHub research via Agent-Reach / gh / yt-dlp. No paid SaaS required. Map 1:1 onto Paperclip org roles or plain folder ICM-style stages.
