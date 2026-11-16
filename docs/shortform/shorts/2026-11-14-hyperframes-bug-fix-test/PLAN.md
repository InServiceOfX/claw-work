# Short (version B): "Bug. Fix. Test." — HyperFrames

Same story and numbers as version A (`../2026-11-14-hyperframes/`), in our own
format instead of the borrowed "Fact or Hype?". Faceless, synthetic local voice
(VoxCPM2), disclosed on the end card. Not posted.

## Why a second version

Version A uses @itstundealao's "Fact or Hype?" phrase and verdict beat. When A
is posted, credit the format in its caption ("format: @itstundealao's Fact or
Hype"). Version B is ours, so it can become a series.

Plain words only: no contemporary slang in the script, on screen or in the
caption.

## The format: Bug → Fix → Test

A three-step bug report in the order engineers already work, which suits a
builder channel and how claw-work operates (every number measured, nothing
claimed without a log):

| Step | What it holds | Spoken cue |
| --- | --- | --- |
| BUG | A real mistake we made, shown, in the first frame | (the hook itself) |
| FIX | The tool or change that caught or fixed it, with its source | "The fix is …" |
| TEST | Our measured numbers, printed as a test report, plus the honest catch | "Then I tested it." |

Signature elements, repeated every episode so viewers learn them:

- A **step tracker** (BUG · FIX · TEST) stays on screen and lights the
  current step: red, green, amber.
- A **printed test report** is the payoff: line items with our numbers, the
  COST, and a note for the catch.
- The **sign-off** "Find the bug. Fix it. Test it." lights the three steps in
  turn on the end card (BUG. FIX. TEST.).

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

| Step | Spoken |
| --- | --- |
| BUG | My captions were hiding under TikTok's buttons. I never noticed. A free tool did. |
| FIX | The fix is HyperFrames. You write the video in HTML, and it renders on your own machine. |
| FIX | It checks every frame against TikTok, Reels and Shorts. That's how it caught mine. |
| TEST | Then I tested it. It timed every word of a twenty-five second voiceover in two and a half seconds. A twenty-seven second short rendered in under fourteen seconds. |
| TEST | It's free, Apache two, and local. One catch: turn telemetry off. |
| Sign-off | Find the bug. Fix it. Test it. The template is on our GitHub. Save this for your next short. |

Rhythm: no two sentences over 12 words back to back; short lines ("I never
noticed.", "Then I tested it.", "Find the bug. Fix it. Test it.") turn the story.

Why the spoken sign-off is longer than the card: VoxCPM2 (seed 7) read a bare
"Bug." as "Book" in three takes ("Bug.", "Bugg.", "Bug, fix, test."); "Bug!"
came out doubled. "Find the bug." transcribed cleanly.

## Ending (both versions)

No comment-for-a-link ending: nothing replies to comments automatically yet.
The end card shows the URL itself (`github.com/InServiceOfX/claw-work`) and asks
for a save, which needs no follow-up.

## Posting caption

> My captions sat under TikTok's buttons. Bug: our safe-area rule. Fix: HyperFrames (free, Apache-2.0, runs locally) checks every frame against TikTok, Reels and Shorts. Test: 25 s of voice word-timed in 2.5 s; a 27 s short rendered in 13.5 s on an M4 Pro Mac mini (warm). Template: github.com/InServiceOfX/claw-work. AI voice (VoxCPM2, local). #hyperframes #shortformvideo #opensource #buildinpublic

## Self-score

| | Score | Why |
| --- | --- | --- |
| Hook | 4 | Same confession hook as A |
| Retention | 4 | Tracker shows progress; the test report prints as the payoff |
| Engagement | 3 | Save prompt and a visible link; the series format can earn follows over episodes |

## Rebuild

As version A (`../2026-11-14-hyperframes/PLAN.md`), with this folder's
`index.html`, `compositions/`, `script.txt` and `narration.txt`, the extra
`steps.html` (tracker) and `report.html`, and captions highlighted with
`--highlight HyperFrames,14,27-second,2.5` (not the step words: the end card
colors BUG red).

One extra step: VoxCPM normalizes the voice to a -1 dBFS peak, and this take
needs about +4 dB to reach -14 LUFS, so `finish.sh`'s linear pass cannot hold
the -1 dBTP ceiling. Limit the narration before rendering:

```sh
ffmpeg -i narration.wav -af "alimiter=limit=0.5:attack=5:release=50:level=false" \
  -c:a pcm_s16le assets/narration.wav
```

## Measured (2026-11-15, M4 Pro Mac mini)

| | |
| --- | --- |
| Narration | 39.5 s, VoxCPM2 native, seed 7; peaks limited to -6 dBFS |
| Transcription | Parakeet; 95 of 98 script words matched, every word of the sign-off among them |
| `hyperframes check` | 0 errors, 0 warnings (unlit word-timing words raised to #69717d for contrast) |
| Render | 19.0 s for 41.2 s |
| Finished | -14.06 LUFS, -1.22 dBTP, 1080x1920, 30 fps, full decode |
| SFX | 14 uisfx cues, retimed to this take, one frame after their picture (not re-measured after render) |

Measured, not listened to.
