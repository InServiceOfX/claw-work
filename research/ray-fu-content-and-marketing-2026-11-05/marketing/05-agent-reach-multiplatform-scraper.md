# Multi-platform research scraper ("Agent Reach")

- **URL:** https://www.tiktok.com/@ray_fu/video/7681484756407586078
- **Views / duration:** ~12.9K / 43s (Sep 3, 2026)
- **Repo (verified):** https://github.com/Panniantong/Agent-Reach
- **Ray guide:** https://www.raycfu.com/guides/scrape-platforms-claude-code-free
- **Install pattern:** `pip install agent-reach && agent-reach install --env=auto` then `agent-reach doctor`

## Platforms (read-only CLI)
GitHub, YouTube, Reddit, X/Twitter, LinkedIn (jobs/optional), Bilibili, XiaoHongShu, web/Exa, etc. Examples in video: scrape 50 B2B founder profiles; extract screenshots/hooks from top 10 YT videos; monitor competitor complaints on X.

## Safety / zero-paid notes
- Default channels need no paid API — fits constraints.
- Logged-in channels use **cookies/session** — Ray’s guide warns: use spare accounts; LinkedIn scraping risks primary account. Prefer public-web backends for Ernest workflows.
- On-screen research overlay cites twitter-cli, gh CLI, Exa, bili-cli (June 2026 agent landscape).
