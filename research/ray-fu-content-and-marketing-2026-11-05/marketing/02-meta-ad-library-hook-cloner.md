# Meta Ad Library competitor-hook cloner

- **URL:** https://www.tiktok.com/@ray_fu/video/7695017848497818910
- **Views / duration:** ~7.9K / 70s (Oct 10, 2026)
- **System:** Spy agent (Arcads MCP + “spy competitor ads”) finds top 10 **longest-running** Meta ads in niche (30+ days ⇒ likely profitable); extracts first-3-second hooks. Design agent: `/clone_hook` adapts hooks + AI actor → 10 ad variants (~$30 claimed vs $5K agency). Optimizer agent (Meta Ads MCP) launches, checks CAC after 3 days, scales winners, loops spy+design when winners fatigue.

## Zero-paid-SaaS adaptation
| Video | Replacement |
|---|---|
| Arcads paid creatives / AI actors | **Public Meta Ad Library** manual/Playwright research + Claude rewrite hooks; generate static/UGC-style creatives with Remotion + local assets (no AI influencer actors). Skills ref: https://github.com/arcads-ai/skills (document what video shows; do not require Arcads subscription) |
| Meta Ads MCP auto-launch spend | Keep **human** campaign launch in Ads Manager; agent only proposes creatives + copy |
| Open-source Ad Library helpers | https://github.com/jacopocappelli1989/facebook-library-ads-mcp ; https://github.com/the-baweja/competitor-ad-spy |

## Reusable rule
Prefer longest-running ads as proxies for profitable hooks; clone **structure** (problem/agitation/proof/CTA timing), not trademarked creative assets.
