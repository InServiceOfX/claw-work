# Short (version B): "Bug. Fix. Receipts." — HyperFrames

Same story and numbers as version A (`../2026-11-14-hyperframes/`), in our own
format instead of the borrowed "Fact or Hype?". Faceless, synthetic local voice
(VoxCPM2), disclosed on the end card. Not posted.

## Why a second version

Version A uses @itstundealao's "Fact or Hype?" phrase and verdict beat. When A
is posted, credit the format in its caption ("format: @itstundealao's Fact or
Hype"). Version B is ours, so it can become a series.

## The format: Bug → Fix → Receipts

A three-beat bug report, which suits a builder channel and how claw-work
already works (every number measured, nothing claimed without a log):

| Beat | What it holds | Spoken cue |
| --- | --- | --- |
| BUG | A real mistake we made, shown, in the first frame | (the hook itself) |
| FIX | The tool or change that caught or fixed it, with its source | "The fix is …" |
| RECEIPTS | Our measured numbers, printed as a receipt, plus the honest catch | "Now the receipts." |

Signature elements, repeated every episode so viewers learn them:

- A **step tracker** (BUG · FIX · RECEIPTS) stays on screen and lights the
  current beat: red, green, amber.
- A **printed receipt** is the payoff: line items with our numbers, a TOTAL,
  and a note for the catch.
- The **sign-off** "Bug. Fix. Receipts." lights the three steps in turn.

This is the recurring bit Noe's framework asks for (a signature mannerism
viewers wait for), in builder-safe form.

## Hook triple

| Layer | |
| --- | --- |
| Visual | Frame 0: a phone with the caption under the button rail, the overlap boxed in red; tracker on BUG |
| Verbal (T6) | "My captions were hiding under TikTok's buttons." |
| Text (6 words) | MY CAPTIONS WERE UNDER THE BUTTONS |

## Script

`narration.txt` is read by VoxCPM; `script.txt` is what the captions show.

| Beat | Spoken |
| --- | --- |
| BUG | My captions were hiding under TikTok's buttons. I never noticed. A free tool did. |
| FIX | The fix is HyperFrames. You write the video in HTML, and it renders on your own machine. |
| FIX | It checks every frame against TikTok, Reels and Shorts. That's how it caught mine. |
| RECEIPTS | Now the receipts. It timed every word of a twenty-five second voiceover in two and a half seconds. A twenty-seven second short rendered in under fourteen seconds. |
| RECEIPTS | It's free, Apache two, and local. One catch: turn telemetry off. |
| Sign-off | Bug. Fix. Receipts. The template is on our GitHub. Save this for your next short. |

Rhythm: no two sentences over 12 words back to back; short lines ("I never
noticed.", "Now the receipts.", "Bug. Fix. Receipts.") turn the story.

## Ending (both versions)

No comment-for-a-link ending: nothing replies to comments automatically yet.
The end card shows the URL itself (`github.com/InServiceOfX/claw-work`) and asks
for a save, which needs no follow-up. The URL only leads to the template once
the branch is merged and pushed.

## Posting caption

> My captions sat under TikTok's buttons. Bug: our safe-area rule. Fix: HyperFrames (free, Apache-2.0, runs locally) checks every frame against TikTok, Reels and Shorts. Receipts: 25 s of voice word-timed in 2.5 s; a 27 s short rendered in 13.5 s on an M4 Pro Mac mini (warm). Template: github.com/InServiceOfX/claw-work. AI voice (VoxCPM2, local). #hyperframes #shortformvideo #opensource #buildinpublic

## Self-score

| | Score | Why |
| --- | --- | --- |
| Hook | 4 | Same confession hook as A |
| Retention | 4 | Tracker shows progress; the receipt prints as the payoff |
| Engagement | 3 | Save prompt and a visible link; the series format can earn follows over episodes |

## Rebuild

As version A (`../2026-11-14-hyperframes/PLAN.md`), with this folder's
`index.html`, `compositions/`, `script.txt` and `narration.txt`, the extra
`steps.html` (tracker) and `receipt.html`, and captions highlighted with
`--highlight HyperFrames,14,27-second,2.5` (not the step words: the end card
colors BUG red).

## Measured (2026-11-14, M4 Pro Mac mini)

| | |
| --- | --- |
| Narration | 41.8 s, VoxCPM2 native, seed 7 |
| Transcription | Parakeet. Heard **"Book" for "Bug" at 33.4 s**: listen there (the voice may have said it wrong). "It's" before "free" and "That's" merged into neighbours |
| `hyperframes check` | 0 errors, 0 warnings after fixes (tracker moved out of TikTok's button column, unlit steps raised to readable contrast, the receipt window marked as intentional overflow, one hold lengthened) |
| Render | 21.9 s for 43.4 s |
| Finished | -14.13 LUFS, -1.23 dBTP, 1080x1920, 30 fps, full decode |
| SFX | 14 uisfx cues, placed one frame after their picture (not re-measured after render) |

Measured, not listened to.
