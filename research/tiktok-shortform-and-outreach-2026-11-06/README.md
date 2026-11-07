# TikTok short-form workflow and outreach research (2026-11-06)

Parsed three public TikToks: one motion-graphics "Pulse" workflow, one inbox-folder + style-interview editing skill, and one cold-email stats breakdown. Nothing here is a paid course dump. Stats are from yt-dlp at download time (2026-11-06).

Source request: document reusable workflow patterns for faceless Remotion/Playwright/FFmpeg pipelines and outreach automation without paid SaaS dependencies.

## Videos

| Creator | Handle | URL | Length | Views / likes / comments (download) | Theme | Notes file |
| --- | --- | --- | --- | --- | --- | --- |
| Joshua Blatch | [@joshuablatch](https://www.tiktok.com/@joshuablatch) | [7695819667444714765](https://www.tiktok.com/@joshuablatch/video/7695819667444714765) | 81s | 41,800 / 2,400 / 39 | Pulse motion-opportunities ledger | [themes/01-joshuablatch-pulse.md](themes/01-joshuablatch-pulse.md) |
| Sanam | [@systemswithsanam](https://www.tiktok.com/@systemswithsanam) | [7703550492319780110](https://www.tiktok.com/@systemswithsanam/video/7703550492319780110) | 76s | 13,900 / 938 / 65 | Inbox folder + style interview skill | [themes/02-systemswithsanam-style-skill.md](themes/02-systemswithsanam-style-skill.md) |
| Build in Public | [@build_in_public](https://www.tiktok.com/@build_in_public) | [7695962167941532961](https://www.tiktok.com/@build_in_public/video/7695962167941532961) | 66s | 280,100 / 24,900 / 68 | 6,337 cold-email stats breakdown | [themes/03-build-in-public-cold-email.md](themes/03-build-in-public-cold-email.md) |

Auto-captions (cleaned spoken text) live in `transcripts/`. Chosen stills (720px wide, readable UI) live in `screenshots/<creator>/`.

## Theme map

1. **Pulse motion-opportunities ledger (Josh Blatch).** Claude edits a video while a strict "ideas only" mod scans the transcript + catalog, pitches missing motion-graphic modules, and waits for human Approve/Pass before spending render tokens. After approval, 2–3 still directions appear before any animation. Generalizes to sales decks, email templates, ad creative. Public prompts at https://www.signalai.agency/pulse.html; OSS analog at https://github.com/charlie947/motion-graphics-skills.

2. **Inbox folder + style interview → skill (Sanam).** Desktop `CapCut/` folder as a single-file inbox; Claude Code picks it up, runs a one-time style interview (cuts, fonts, colors, pacing), iterates, then saves as a reusable skill. First video ~3 hours / 6–16 iterations; later 1–2 iterations. Skill updates weekly. Faceless analog: skip CapCut GUI, use Remotion or FFmpeg + local TTS.

3. **Cold-email stats (Build in Public).** 6,337-email dataset from Whali: `<100` words = 11.9% reply, `100–200` = 1.9%, `>200` = 0.3%; subject `<30` chars ≈ 83% open (Apple MPP caveat); Tue/Wed; 1–2 follow-ups max. Faceless "data-card hook" short-form template: scrape stats → Ken-Burns over blurred doc → TTS VO.

## Suggested uses

- **Motion catalog ledger.** Encode `10. Motion/OPPORTUNITIES.md` + `ledger.csv` pattern in `Skills/claw-shorts` so each batch run pitches missing templates before building.
- **Style interview prompt.** Add `style.md` skill (cuts, fonts, colors, pacing) to any editing workflow—CapCut-free via Remotion or transcript-driven FFmpeg.
- **Outreach gates.** Enforce `body_words <= 90`, `subject_chars < 30`, Tue/Wed schedule, max 2 follow-ups in cold-email agent prompts.

## Bibliography

See [BIBLIOGRAPHY.md](BIBLIOGRAPHY.md).
