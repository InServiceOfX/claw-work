# Cross-platform hook & script scoring system

- **URL:** https://www.tiktok.com/@ray_fu/video/7622543611946749214
- **Views / duration:** ~4.6K / 67s (Mar 28, 2026)
- **System (3 steps):**
  1. **Research sweep:** YouTube top videos across ~15 keyword searches (time-filtered); Reddit pain-point quotes from viral threads; X high-engagement tweets with fight-in-comments.
  2. **Specialized agents in sequence** (not one AI doing everything): Hook agent writes 4 hook variations from a proven-hooks DB; Hook manager scores 5 dimensions — must be 10/10 to pass.
  3. **Line-level gates:** every line scored on **Venture novelty** (feels like a breakthrough) and **Copy intensity** (emotion, not just understanding); both 10/10 or rewrite.

## Zero-paid-SaaS adaptation
- Research: Agent-Reach (YouTube/Reddit/X) + yt-dlp + Reddit public JSON where available.
- Scoring: Claude/Codex/Grok as independent **checker** agents (do not let the writer grade itself — aligns with “independent checker” pattern shown in Agent-Reach research overlay).
- Store hooks DB as local markdown/JSON; no paid creative SaaS.
