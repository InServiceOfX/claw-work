# AI sales prospecting & cold outreach agent

- **URL:** https://www.tiktok.com/@ray_fu/video/7687023907030961438
- **Views / duration:** ~95.9K / 62s (Sep 18, 2026)
- **3 steps in video:** (1) Hermes agent interviews you 5–10 Qs to lock ICP (e.g. ecommerce brands, 10–50 employees, actively running ads) + Apollo for contacts; (2) daily research loop on website/activity/growth → prioritize + draft personalized outreach; (3) deploy on Bluehost agent hosting for 24/7 persistent memory.

## Paid pieces → zero-paid-SaaS
| Video | Replacement |
|---|---|
| Apollo contact DB | Free/public sources: company websites, LinkedIn public pages (manual/careful), GitHub org members, BuiltWith-style free tech detection, Hunter-style pattern guess **only with verification**, Clearbit-free alternatives, CSV imports from conferences; **Agent-Reach** for public research; never scrape LinkedIn with primary account cookies (ToS/risk — Ray’s own guide warns) |
| Hermes agent | Claude Code / Codex / Grok scheduled agent with local markdown CRM |
| Bluehost agent hosting | Local always-on box, cron, or self-hosted Paperclip; no paid host required for MVP |
| Instantly (mentioned in related ICP video) | Self-hosted SMTP / Amazon SES free tier / Google Workspace with warm-up discipline + human send approval |

## Prompt skeleton (ICP interview)
Ask 5–10 questions: offer, past best customers, industry, company size, tech stack, geography, buying triggers, disqualifiers. Emit `icp.md` + `buying_signals.md`. Morning job: pull prospects → research → score → draft email → **queue for human send**.
