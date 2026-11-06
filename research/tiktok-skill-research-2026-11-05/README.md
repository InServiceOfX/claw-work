# TikTok skill research (2026-11-05)

Parsed six public TikToks for workflow notes, auto-generated captions, readable screenshots, and theme tags. Nothing here is a paid course dump. Stats below are from yt-dlp at download time (2026-11-05), not invented.

Source request: document the Claude / AI-edit skills the creators teach, plus one DJ technique for Mixxx (Phil Harris). When a video said "link in bio" or "comment X for the setup," extras were found from public pages and writeups rather than paid DMs or comment-gated drops. See `PUBLIC-EXTRAS.md`.

## Videos

| Creator | Handle | URL | Length | Views / likes / comments (download) | Theme | Notes file |
| --- | --- | --- | --- | --- | --- | --- |
| Brian Scully | [@bridotai](https://www.tiktok.com/@bridotai) | [7701679351661743391](https://www.tiktok.com/@bridotai/video/7701679351661743391) | 78s | 26,100 / 2,211 / 168 | Talking-head edit via HyperFrames | [themes/01-bridotai-hyperframes.md](themes/01-bridotai-hyperframes.md) |
| Sarah \| Content Marketing BFF | [@abloommedia](https://www.tiktok.com/@abloommedia) | [7703263566719192350](https://www.tiktok.com/@abloommedia/video/7703263566719192350) | 64s | 15,100 / 1,031 / 456 | Claude → CapCut plan-and-edit | [themes/02-abloommedia-capcut.md](themes/02-abloommedia-capcut.md) |
| Jam \| AI & Marketing | [@jamout.ai](https://www.tiktok.com/@jamout.ai) | [7695100487216975118](https://www.tiktok.com/@jamout.ai/video/7695100487216975118) | 241s | 432,400 / 26,000 / 906 | Audio-prompt "video editor" skill + HyperFrames | [themes/03-jamout-video-editor.md](themes/03-jamout-video-editor.md) |
| Adam Stewart | [@adamstewartmarketing](https://www.tiktok.com/@adamstewartmarketing) | [7696038654904503560](https://www.tiktok.com/@adamstewartmarketing/video/7696038654904503560) | 33s | 190,300 / 7,529 / 2,298 | Claude Code talking-head edit + HyperFrames | [themes/04-adamstewart-claude-code.md](themes/04-adamstewart-claude-code.md) |
| Chase AI | [@chase_ai_](https://www.tiktok.com/@chase_ai_) | [7693602525081111821](https://www.tiktok.com/@chase_ai_/video/7693602525081111821) | 849s | 253,700 / 18,100 / 123 | Procedural JS canvas video (not Remotion-required) | [themes/05-chase-ai-javascript.md](themes/05-chase-ai-javascript.md) |
| Phil Harris | [@djphilharris](https://www.tiktok.com/@djphilharris) | [7694719261549317379](https://www.tiktok.com/@djphilharris/video/7694719261549317379) | 61s | 14,400 / 401 / 7 | DJ tonal play (Ableton/CDJ/sampler → Mixxx analog) | [themes/06-djphilharris-tonal-play.md](themes/06-djphilharris-tonal-play.md) |

Auto-captions (cleaned spoken text) live in `transcripts/`. Chosen stills (720px wide, readable UI, not raw high-res dumps) live in `screenshots/<creator>/`.

## Theme map

1. **Talking-head AI edit (HyperFrames + last-take cutting).** Brian, Adam, and Jam all describe: film multiple takes in one file, keep the best/final take of each line, cut pauses, add captions, iterate in chat. The actual renderer is not Claude itself. Claude (or ChatGPT) plans an edit; HyperFrames / FFmpeg / a wrapper renders it.
2. **Branded CapCut execution.** Sarah's on-screen Claude UI is motion graphics, not the stock Claude app. She claims a Claude plan (storyboard + clarifying questions + effects library) then execution inside CapCut. The "comment edit" CTA leads to a paid course; a public CapCut-CLI analog is cited in extras, not copied.
3. **Skill packaging / audio prompt injection.** Jam talks to the viewer's AI through the phone mic and dictates a `video editor` skill (9 rules, FFmpeg, Whisper, Node.js, HyperFrames, symlink `.agents/skills` → `.claude/skills`).
4. **Procedural motion graphics in JavaScript.** Chase: Claude writes `frame(t)` on HTML canvas, Playwright captures frames, FFmpeg muxes an MP4. Self-critique via contact sheets. Open plugin: `cb9191/animate` (MIT).
5. **Comment / bio lead magnets.** Sarah ("edit"), Adam ("CLAUDE"), Jam ("JAM" / link in bio) gate extras. Brian does not. Chase pitches a paid community in a pinned comment but the method and GitHub skill are shown in-video. Free substitutes are listed in `PUBLIC-EXTRAS.md`.
6. **DJ tonal play (claw-dj / Mixxx).** Phil shows Ableton pitch-duplication + Pioneer CDJ hot cues, or a Pioneer DJS-1000 in Scale Mode. Mixxx workarounds: pre-produced hot-cue track, or multiple samplers at different pitches. This folder is the Mixxx-facing note; product code stays in InServiceOfX/claw-dj.

## Follow-through

See [production workflows and per-video breakdown](../../docs/video-production/README.md)
for original reusable skills, the local Rust cut-list helper, verification and
limits. DJ follow-through is documented in claw-dj's
`docs/TONE_PLAY_AGENT_HANDOFF.md`; it reuses that repository's existing
original-source Python/Rust experiment. The suggestions below record the
original research PR's proposals, not evidence they were implemented there.

## Original suggested next uses (not done in the research PR)

- Marketing: turn Brian/Adam/Jam last-take + caption rules into a claw-dj short-form edit prompt that points at HyperFrames docs (open source).
- Mixxx: optionally implement sampler pitch maps or a hot-cue tonal-play helper in claw-dj (separate repo/PR).
- Do not buy or paste Sarah's paid CapCut course files.

## Bibliography

See [BIBLIOGRAPHY.md](BIBLIOGRAPHY.md).
