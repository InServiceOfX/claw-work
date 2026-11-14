# PROGRESS: docs/shortform/tiktok

## 2026-10-30: first TikTok ingest batch (7 videos)

- Added per-video notes, transcripts, and screenshots for 7 marketing,
  outreach, and short-form videos, plus the `PLAYBOOK.md` synthesis.
- Original branch `bots/tiktok-ingest-marketing` (yeung-assistant); merged into `master` via PR #3. This historical ingest record is retained alongside the complementary research pass below.
- Known limits: transcripts are whisper `small` output (minor errors are
  possible and corrected in the notes where obvious). On-screen text was read
  by eye, not OCR.
- Next: if more TikToks arrive, reuse the same layout
  (`<creator>-<videoid>/README.md`, `transcript.md`, `screenshots/`) and
  update the index and the PLAYBOOK.

## 2026-10-31: first YouTube Short

- Added `../youtube/wanderlustalley-cY6GojPFOYc/` (Wanderlust Alley, "12
  streams of income" plus a Printify print-shop ad), and a new top-level index
  `docs/shortform/README.md` that links TikTok and YouTube and records the
  screenshot success criterion (on-screen lists and key data must be legible).
- yt-dlp downloaded the Short without extra extractor args. The list
  is on screen for only about 1s (0:00.4 to 0:01.4), so it was found with a frame-by-frame
  check of the first 1.6s. All 12 items are transcribed in the notes.
- PLAYBOOK gained section 5 (income streams, story-wrapped ads) and two formats.
- Same branch and PR (`bots/tiktok-ingest-marketing`, PR #3).

## 2026-10-31: ingest tooling versioned

- The ad-hoc ingest scripts are now a reusable tool,
  [`Scripts/shortform-ingest/`](../../../Scripts/shortform-ingest/README.md)
  (yt-dlp download, faster-whisper transcript, scene-change + dense early-frame
  candidates with labelled contact sheets, grab, README/transcript scaffold).
  Use it for the next batch.
## Complementary caption-backed research — 2026-10-30

The following records the separate `bots/tiktok-learning-intake` pass over the
same seven marketing videos and three DJ references. Its caption/audio limits
are pass-specific; the folder-based ingest above separately produced Whisper
transcripts. Preserve both evidence trails rather than replacing one with the other.

### Completed
- 2026-10-30: seven Monoclaw source notes; public media downloaded; eight caption tracks across the ten-video intake; selected semantic keyframes inspected; source URLs/hashes/gaps recorded.
- Three DJ references routed to claw-dj rather than mixed into general marketing advice.
- @ginyboi and @themekhicarter added as user-preferred creators to revisit.

### Pending
- Reconcile caption-backed notes with the folder-based Whisper transcripts and check discrepancies against audio. The caption-backed pass did not independently transcribe audio; two DJ clips had no retrieved caption track.
- Independent validation of performance claims; structured-data documentation check before implementation.
- Grok interoperability: awaiting identification of local versus hosted bot. No connector, account/session inspection, or automatic messaging configured.

### Branch / review
`bots/tiktok-learning-intake`; documentation-only changes committed on request. No push. Existing unrelated untracked macOS project preserved. All ten downloaded MP4s passed full FFmpeg decode; all ten source notes contain source URLs, SHA-256 and explicit evidence status.

### Rebase resolution
Combined the add/add README and PROGRESS conflicts with the latest `master`, retaining the folder-based ingest, PLAYBOOK/YouTube/tooling context, caption-backed source notes, watchlist, provenance and pending verification. Existing equivalent commits were skipped by Git; no source content was intentionally dropped.
