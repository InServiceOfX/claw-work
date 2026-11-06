# Build in Public (@build_in_public) — 6,337 cold-email stats breakdown

- Video: https://www.tiktok.com/@build_in_public/video/7695962167941532961 (66s)
- Auto-captions: [`../transcripts/build_in_public-7695962167941532961.md`](../transcripts/build_in_public-7695962167941532961.md)
- Screenshots: [`../screenshots/build_in_public/`](../screenshots/build_in_public/)
- Theme: cold-email dataset breakdown (Reddit/Whali study)
- CTA: none (content-only breakdown)
- Source: https://whali.com/blog/cold-email-data-study-students-2026

**Note:** `@build_in_public` is a **different account** from `@buildingnpublic` (ICM clip `7703729143749397774` in `tiktok-mcp-and-skills-2026-11-06/`).

## Key stats (on-screen / spoken; treat as directional)

Screen-recorded Reddit/Whali cold-email study walkthrough:

| Rule | Figure shown |
|---|---|
| Body < 100 words | **11.9%** reply |
| Body 100–200 words | **1.9%** reply |
| Body > 200 words | **0.3%** reply |
| Subject < 30 chars | **~83%** open (on-screen highlight **83.7%**) |
| Longer subjects | **~69%** open (69.5% in source) |
| Best send days | **Tuesday, Wednesday** |
| 0 follow-ups | **4.1%** reply |
| +1 follow-up | **6.6%** reply |
| >2 follow-ups | spam / diminishing (source ceiling ~6.9% at 2) |
| Creator add-on | one-word subjects; personalize; short; get to the point |

Creator caveat (visible on screen / in source post): Apple Mail Privacy inflates opens ~30–40%; reply tracking under-counts — trust **relative** patterns more than absolute %.

## Outreach automation encoding

For cold-email agents, encode these hard gates:

```yaml
cold_email_gates:
  body_words_max: 90
  body_words_sweet_spot: 60-90
  subject_chars_max: 30
  subject_one_word_preferred: true
  send_days: ["Tuesday", "Wednesday"]
  max_followups: 2
  personalize_first_line: true
  human_approve_send: true
```

## Faceless short-form analogue

Content format: "data finger-cam / screen highlight" is easily faceless:

1. Scrape a stats card (Remotion or HTML screenshot)
2. Ken-Burns over a blurred doc background
3. TTS voice-over reading the key stats
4. No face required

Script gate: same length discipline on hooks — first caption ≤ ~12 words; VO beats under ~20 words before a visual change.

## Screenshot index

- `01-hook.jpg` (~0:01) — "one of the most useful business posts"
- `02-under-100-words.jpg` (~0:09) — 11.9% highlight (key stat)
- `03-subject-under-30.jpg` (~0:24) — 83.7% + Tue/Wed + Apple MPP caveat
- `04-followups.jpg` (~0:43) — 6.6% follow-up reply rate
