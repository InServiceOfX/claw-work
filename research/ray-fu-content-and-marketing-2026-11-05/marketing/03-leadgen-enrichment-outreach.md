# Lead-gen enrichment & outreach system

- **URL:** https://www.tiktok.com/@ray_fu/video/7675138380212063519
- **Views / duration:** ~5.0K / 92s (Aug 17, 2026)
- **Pipeline:** Research agent scrapes client site/case studies/LinkedIn/customer profiles → ICP (industry, size, tech, pain). Scout finds matching companies. Clay CLI waterfall enrichment for verifiable emails (pay-only-if-accurate). Personalization team visits site/LinkedIn/news → unique email per lead. Clay sequences + follow-ups. Sold as ~$5K system vs $10K/mo agencies.

## Zero-paid-SaaS enrichment path
1. ICP markdown from client materials (Claude).
2. Company discovery: public directories, GitHub, job boards, Product Hunt, conference lists, Google `site:` queries via Agent-Reach web.
3. Email: pattern generation (`first@domain`, `first.last@domain`) + **SMTP verify / free verification APIs with caps** — never blast unverified; prefer form-based or mutual intros when verification fails.
4. Personalization: Agent-Reach + Playwright read public pages → Claude draft.
5. Send: human-approved queue; self-hosted sequencer optional.

**Do not** depend on Clay paid waterfall or Apollo paid seats for the Ernest stack. Document Clay as what the video shows.
