# Fast app landing-page build for launches

- **URL:** https://www.tiktok.com/@ray_fu/video/7569035495896780062
- **Views / duration:** ~2.7K / 50s (Nov 4, 2025)
- **Flow:** Claude Code on web → attach GitHub → new repo → prompt with product purpose + color scheme → download zip → run Claude’s local commands → skeleton landing page with auth placeholder → customize.

## Zero-paid-SaaS adaptation
Already Claude+GitHub. Use local Claude Code / Codex / Grok instead of “Claude Code on web credits.” Templates: Next.js + Tailwind static marketing page; auth optional (Clerk free tier or skip). Remotion not required. Good launch companion for Entrepreneur product drops.

## Prompt skeleton
```
Build a conversion-focused landing page for {product}.
Sections: hero with clear value prop, social proof, feature grid,
how-it-works, pricing stub, FAQ, final CTA.
Color scheme: {colors}. Mobile-first. Accessible. No stock AI-slop gradients.
Output a minimal Node/static site I can run locally.
```
