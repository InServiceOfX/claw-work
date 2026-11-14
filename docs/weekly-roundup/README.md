# Weekly reading roundup

Every week, turn that week's creator research in `research/` into two things:

1. **A summary** of the TikTok and YouTube videos we studied, ranked by the
   audience-interest signals we recorded. It also names what each video pointed to
   that is worth reading.
2. **One vertical short** (TikTok + YouTube Shorts, ~50 s) in the "N things I'm
   reading this week" format. Each item sends viewers to a primary source they
   can read, such as a repo, paper or study, rather than to the video that
   mentioned it.

Editions are named by ISO week: `YYYY-Www.md` (for example `2026-W45.md` for
Monday 2026-11-02 to Sunday 2026-11-08). Start from [TEMPLATE.md](TEMPLATE.md).
The format notes and outside examples are in [FORMAT.md](FORMAT.md).

## Editions

| Week | Research covered | Short's #1 | File |
| --- | --- | --- | --- |
| 2026-W45 (Nov 2–8) | 5 research folders, 2 YouTube analyses, ~120 videos transcribed | Sparks of Weight-Space Programming + independent audit | [2026-W45.md](2026-W45.md) |

## Weekly procedure

1. **Collect.** List every `research/*` folder added or updated during the week.
   Copy each video's recorded stats (views, likes, comments or saves) **with the
   date they were captured**. Don't re-scrape to make numbers bigger. If a YouTube
   video has no recorded stats, fetch public metadata once and write down the
   date you fetched it.
2. **Rank by interest.** Sort by views. Then compute rates:
   - **like rate** = likes / views. This is the general approval signal.
   - **save rate** = saves / views, where recorded. It is the best proxy for "I want
     to come back to this", and it is the signal that matters most for a reading list.
   - **comment rate**, discounted when the video gates a resource behind a comment
     keyword ("comment EDIT"). Those comments are requests, not discussion.
   - Rates across platforms aren't comparable: YouTube like rates run much lower
     than TikTok's. Rank each platform separately.
3. **Pick the readable sources.** For each high-interest video, name the primary
   source it points to. Skip videos whose only "source" is a paid course, a
   DM/comment-gated drop, or excluded content (impersonation, IP or legal risk; see
   the research folders' exclusion notes).
4. **Verify.** Check that every link in the short returns 200. Record each repo's
   license, and say whether claims were tested or are only the creator's.
5. **Choose 5 items.** Order them #5 → #2 by audience signal and make #1 the
   editorial pick: the most surprising or most worth reading, with any caveat it
   needs. Say in the edition how #1 was chosen.
6. **Write the short** from the template: hook ≤ 12 words naming the count and
   teasing #1, then ~6–8 s per item, with #1 longest. Every claim on screen must
   appear in the edition's evidence table.
7. **Produce locally, faceless,** with the
   [claw-shorts](../../skills/claw-shorts/README.md) voiced explainer
   ([`Rust/claw-shorts`](../../Rust/claw-shorts/README.md)). It validates the
   spec's copy, narrates each scene with `voxcpm-tts`, sets each scene's length from its
   narration, and writes captions and a -14 LUFS mix. Disclose the synthetic
   voice. Then publish by hand. The
   [YouTube CLI](../../Tools/social-publish/README.md) only ever uploads as private,
   and publishing is a separate confirmed step. Nothing auto-posts.

## Status and next steps (2026-11-14)

- **Done:** process, format research, template, and the 2026-W45 edition
  (summary + script). The W45 short has **not** been rendered or posted.
- **Ready on the Mac mini:** `voxcpm-tts` is installed and verified (see
  `Rust/voxcpm-tts/docs/HANDOFF.md`), and `Rust/claw-shorts` calls it from `PATH`.
- **Next:**
  1. Turn the W45 script table into a `Rust/claw-shorts` spec (one scene per
     row; see its `examples/voxcpm-tts.short.json`), then run
     `check --strict` and render. Expect a warning: the W45 narration is
     ~50 s and `check` warns above ~45 s. Trim #2's "saved" line first.
  2. Decide the voice. The claw-shorts skill says faceless narration should be
     synthetic and disclosed, and it should not suggest recording the user's own
     voice. Cloning Ernest's voice with VoxCPM is his call. The public-sample
     voice or another voice he's authorized to use works without that decision.
  3. Write **2026-W46** (Nov 9–15) from that week's `research/` additions
     using [TEMPLATE.md](TEMPLATE.md).

## Honesty and public-safety rules

- "This week" means **what we read or watched this week**, not "posted this week".
  Say so when a featured video is older.
- Creator figures (cost, timing, revenue, benchmark scores) stay attributed as
  claims. Single runs are labelled as single runs.
- Credit the creator of each source video on screen (`via @handle`) and link the
  primary source, not a mirror.
- No paid-course contents, no gated materials, no faces or voices of other
  creators in our render. Use our own screen recordings of public pages.
- The repo is public: the privacy gate runs on every edition (see
  [PRIVACY.md](../../PRIVACY.md)). No personal paths or account details.
