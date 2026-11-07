# Claude Code ICP & buying-signal finder

- **URL:** https://www.tiktok.com/@ray_fu/video/7676271677218557214
- **Views / duration:** ~2.3K / 59s (Aug 20, 2026)
- **3 agents:**
  1. **ICP agent** — reverse-engineer past clients (industry, size, tech, stage, problem); watch buying signals (e.g. company posted 3 eng roles ⇒ needs help).
  2. **Data Flow agent** — MCP to Apollo / LinkedIn via Apify / “Connect Safety”; pull matches; score vs best past clients; `/schedule` daily 7am enrich.
  3. **Email agent** — personalized mail via Instantly mailbox (domain warm-up).

## Zero-paid-SaaS adaptation
- Buying-signal sources without Apollo: public job boards, GitHub org hiring READMEs, changelog/funding news (web), Product Hunt launches, government contract notices where relevant.
- Enrichment: same as lead-gen note (pattern + verify).
- Schedule: cron / Paperclip routine / Claude Code scheduled task.
- Email: human approve + SES/Workspace; skip Instantly paid if possible.

## Buying-signal library (seed)
- Spike in eng/sales job posts
- New funding announcement
- Tech-stack migration chatter
- Competitor complaint threads
- “Looking for agency/tool” posts on Reddit/X
