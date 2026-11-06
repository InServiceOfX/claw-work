# Headless Node + Playwright + FFmpeg motion graphics

- **URL:** https://www.tiktok.com/@ray_fu/video/7695761144589356319
- **Views / duration:** ~146.2K / 54s (Oct 12, 2026)
- **Core insight:** Claude does not “make a video”; it writes code that paints every frame. Studio = Node + Playwright (Chrome captures frames) + ffmpeg (stitch MP4). Ban generic all-caps-on-gradient “AI slop”; require real assets + closed-form spring physics + synthesized audio locked to beat. Self-critique loop: Claude grades own frames, hunts dead frames, re-renders until ≥8/10.

## Stack (already zero-paid-SaaS)
- Node.js, Playwright, ffmpeg, Claude Opus (or Codex/Grok) — matches Ernest’s tools.
- Optional: Remotion for React-based compositions instead of raw Playwright HTML.

## Prompt / loop pattern
1. Generate scene HTML/CSS/JS with spring physics (no basic CSS transitions only).
2. Playwright screenshot each frame at target FPS.
3. ffmpeg encode + mux generated audio.
4. Vision-critique pass on sampled frames → patch code → re-render deltas.
5. Gate: every sampled frame ≥ 8/10.

## Fit
Primary implementation target for Monoclaw `Projects/short-form-video-pipeline` / `Rust/shortform-video`.
