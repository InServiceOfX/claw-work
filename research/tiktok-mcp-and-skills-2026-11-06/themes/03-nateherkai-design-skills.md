# Nate Herk (@nateherkai) — UI taste & design skill stack

- Video: https://www.tiktok.com/@nateherkai/video/7684323553537232142
- Auto-captions: [`../transcripts/nateherkai-7684323553537232142.md`](../transcripts/nateherkai-7684323553537232142.md)
- Screenshots: [`../screenshots/nateherkai/`](../screenshots/nateherkai/)
- Theme: four skills to stop generic AI UI + Figma/Playwright verification loop
- CTA: Instagram DM keyword "DIE" / TikTok link-in-bio (gated)

## The stack

Nate demos a four-skill stack to make Claude produce "professionally designed" websites instead of generic AI slop:

### 1. Emil Kowalski design skills

Typography, spacing, layouts, and animation rules. The foundational design principles.

- Repo: https://github.com/emilkowalski/skills
- Landing: https://emilkowal.ski/skill
- Install: `npx skills@latest add emilkowalski/skills`

### 2. Impeccable (Paul Bakaus)

"Design fluency for AI harnesses" — a design vocabulary/command set that tells the AI how to talk about and execute design decisions.

- Repo: https://github.com/pbakaus/impeccable
- Docs: https://impeccable.style/
- Install: `npx impeccable install`

### 3. Taste Skill

Reference great websites instead of producing generic output. Teaches Claude to look up and learn from real, well-designed sites.

- Repo: https://github.com/Leonxlnx/taste-skill (~7.2k★ at filming)
- Docs: https://www.tasteskill.dev/docs

### 4. Figma MCP + Playwright

Connect Figma for design handoff and Playwright for visual verification. Claude can design, build, test, and screenshot what it creates in a loop.

- Figma MCP guide: https://github.com/figma/mcp-server-guide
- Playwright: standard installation

## Workflow

1. Add Emil Kowalski design + Impeccable skills to Claude.
2. Add the Taste Skill so Claude references great sites.
3. Connect Figma MCP and Playwright.
4. Describe the website in plain English.
5. Claude designs, builds, tests, and screenshots in a loop.
6. Iterate until satisfied.

## Demo sites shown

The video shows demo outputs: Pegasus, AEON, and aicm — all with polished, non-generic UI.

## Screenshot index

- `01-demo-site-pegasus.jpg` — Pegasus demo site (polished AI UI output)
- `02-impeccable-card.jpg` — Impeccable skill card showing "Design fluency for AI harnesses"
- `03-taste-skill-readme.jpg` — Taste Skill GitHub README (~7.2k stars)
- `04-demo-site-aicm.jpg` — AICM demo site output
