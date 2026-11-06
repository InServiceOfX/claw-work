# Faceless YouTube channel system (Claude + ElevenLabs → OSS TTS)

- **URL:** https://www.tiktok.com/@ray_fu/video/7639961828809837855
- **Views / duration:** ~525.6K / 88s (May 14, 2026)
- **What the video claims (accurate to transcript):** Build a faceless YT channel with Claude + ElevenLabs; one operator hit ~$8K/mo in 4 months at ~3 hrs/week. Niche selection beats passion niches. Finance RPM math: 500K views/mo × $20 RPM ≈ $10K AdSense; entertainment only $3–8 RPM. Pipeline: Claude scripts → ElevenLabs VO → CapCut/Camtasia + Pexels stock → Claude titles/descriptions/tags/chapters → TubeBuddy/vidIQ keywords → 1–2 posts/week then 3–4 → affiliates/sponsors by month 4. Mentions Nano Banana Pro for thumbnails.

## On-screen / prompt frameworks
1. **Niche scoring signals** (diagram ~24s): Search volume + volume growth; audience size / who’s searching; competition difficulty → data model → ranked niches. Optimize for content repeatability, audience size, monetization beyond AdSense.
2. **Script system prompt** (spoken): Pattern-interrupt hook in first 30s; open loops; spoken English (not essay); pause markers + visual tags per scene; soft CTA; also emit thumbnail + title options.
3. **RPM examples kept unredacted:** $20 RPM finance / $18 RPM at 600K views → $10,800 AdSense.

## Zero-paid-SaaS adaptation (our stack)
| Video uses | Our replacement |
|---|---|
| ElevenLabs VO | Local OSS TTS: [Kokoro](https://github.com/hexgrad/kokoro), [Piper](https://github.com/rhasspy/piper), or [F5-TTS](https://github.com/SWivid/F5-TTS); Claude/Codex write SSML-ish pause markers |
| CapCut / Camtasia | `ffmpeg` + Remotion (`remotion-dev/remotion`) or headless Playwright frame loop |
| Pexels stock | Pexels free API / local B-roll library / Remotion programmatic scenes |
| TubeBuddy / vidIQ | YouTube Data API (free quota) + Claude keyword clustering; or Agent-Reach YouTube search |
| Nano Banana Pro thumbs | Remotion thumb templates or local SD/Flux if available; otherwise Canva-free / HTML→PNG via Playwright |
| Claude (kept) | Claude / Codex / Grok for niche scoring, scripts, metadata |

## Implementation sketch
- `niche_score(signals) -> ranked niches`
- `script_system(topic, rpm_niche) -> {hook, body, cta, pause_markers, visual_tags, titles[], thumbs[]}`
- `render_faceless(script, voice_wav, broll) -> mp4` via ffmpeg/Remotion
- Do **not** automate ToS-violating mass uploads; keep human approve-before-publish gate.
