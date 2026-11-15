# Short: "My captions were hiding under TikTok's buttons" (HyperFrames, fact or hype)

Planned with `skills/short-form-strategy`, rendered with the claw-shorts
HyperFrames mode (`skills/claw-shorts/hyperframes/`). Faceless, synthetic local
voice (VoxCPM2), disclosed on the end card. Not posted.

Every number in it is ours, measured on 2026-11-14 (see
`research/tiktok-motion-sfx-design-2026-11-14/EVALUATION-2026-11-14.md`).

## Constraints (step 0)

| | |
| --- | --- |
| Platform | TikTok first; Reels and Shorts cross-post |
| Niche / persona | Builder tooling, brand-safe; no personal bait |
| Length | about 30 s |
| Goal | Brand-safe reach; follows from a reusable skill (the template) |
| Format | Faceless motion graphics + VO; "Fact or Hype?" (claim, source, demo, number, verdict) with a build-in-public confession as the hook |

## Logline

A free, open-source tool caught our captions sitting under TikTok's buttons,
and here is what else it does, with our own numbers and a verdict.

## Hook triple

| Layer | |
| --- | --- |
| Visual | Frame 0: a phone with a caption running under the right-hand button rail, the overlap outlined in red |
| Verbal (T6, blunt one-line take) | "My captions were hiding under TikTok's buttons." |
| Text (6 words) | MY CAPTIONS WERE UNDER THE BUTTONS |

Seconds 3-8 pay the hook: "I never noticed. A free tool did." names the turn,
then the tool.

## Script

`narration.txt` is what VoxCPM reads; `script.txt` is what the captions show
(digits, "HyperFrames").

| # | Beat | Spoken | Words | Purpose |
| --- | --- | --- | --- | --- |
| 1 | Hook | My captions were hiding under TikTok's buttons. | 7 | Confession + specific stakes |
| 2 | Turn | I never noticed. A free tool did. | 3 + 4 | Short lines carry the twist |
| 3 | Source | It's called HyperFrames. You write the video in HTML, and it renders on your own machine. | 3 + 13 | Name + mechanism |
| 4 | Question | Fact or hype? | 3 | Format signature, short after long |
| 5 | Demo 1 | It checks every frame against TikTok, Reels and Shorts. That's how it caught mine. | 9 + 5 | Live proof (real `check` output) |
| 6 | Demo 2 + number | It times every caption to the word. A twenty-seven second short renders in under fourteen seconds. | 7 + 11 | Measured number (13.5 s, warm) |
| 7 | Verdict | Verdict: fact. Free, Apache two, and local. One catch: turn telemetry off. | 2 + 5 + 5 | Verdict with an honest caveat |
| 8 | CTA | Comment frames, and I'll pin our template. | 7 | Engagement device |

Rhythm: no two sentences over 12 words back to back; every long line is
followed by one of 5 words or fewer. No greeting open.

## Shot list

| Scene | Picture | Change |
| --- | --- | --- |
| Hook | Phone mock-up, button rail, caption under it, red overlap box; hook text above | Overlap box flashes on "buttons"; a `check` warning line types in on "A free tool did" |
| Source | `<html>` code card → arrow → MP4 card; chips "HTML in", "MP4 out", "your machine" | Code lines arrive on "HTML", MP4 on "renders"; FACT OR HYPE? stamp on the question |
| Safe area | The same phone with the stepped safe area drawn in green; TikTok / Reels / Shorts chips; the caption slides left into the green | Chips land on each platform name; caption moves on "caught mine" |
| Timing | Waveform bars with words lighting on their time; then a render bar filling to 13.5 s | Words light word by word; bar fills on "renders" |
| Verdict | FACT stamp; chips "free", "Apache-2.0", "local"; amber caveat card `HYPERFRAMES_NO_TELEMETRY=1` | Stamp on "fact"; caveat on "catch" |
| CTA | COMMENT "FRAMES"; credits and voice disclosure | Holds about 2.5 s after the last word |

## Sound

Narration plus uisfx CC0 cues, one frame after their picture: swipes on cuts
(three takes rotating), `warning` on the check line, `success` on the fix,
`snap` on chips, one `complete` on the FACT stamp. No music bed (none licensed
for this). Finished at -14 LUFS / -1 dBTP with `finish.sh`.

## Engagement device

Comment token: "Comment frames, and I'll pin our template." The pinned comment
carries the link. Secondary: the honest caveat (telemetry) invites replies from
people who care about privacy.

Pinned comment (only after the branch is merged and pushed):

> Template + captions script: github.com/InServiceOfX/claw-work/tree/master/skills/claw-shorts/hyperframes
> HyperFrames: github.com/heygen-com/hyperframes (Apache-2.0). Set HYPERFRAMES_NO_TELEMETRY=1.

## Posting caption

> My captions sat under TikTok's buttons. HyperFrames (free, Apache-2.0, runs locally) flagged it. Fact or hype? Our numbers: 27 s short rendered in 13.5 s on an M4 Pro Mac mini (warm), 25 s of voice word-timed in 2.5 s. AI voice (VoxCPM2, local). #hyperframes #shortformvideo #opensource #buildinpublic #claudecode

The hook is literal: `hyperframes check` flagged our first draft's captions as
"outside the safe area: TikTok (right edge)", and our old 90 px margin rule
allowed it.

## Self-score

| | Score | Why |
| --- | --- | --- |
| Hook | 4 | Specific, visual, a confession; not a promise of money |
| Retention | 4 | A new picture every 3-6 s, a question at 8 s, the verdict as payoff |
| Engagement | 3 | Comment token + caveat; no debate bait |

Upgrade for more reach: open on the red overlap box zooming in with the line
"This is why your captions get cut off", and put the before/after side by side.

## Rebuild

Media is not committed. From a work directory outside the repository:

```bash
cp -R skills/claw-shorts/hyperframes "$WORK/hf-short" && cd "$WORK/hf-short"
cp -R <claw-work>/docs/shortform/shorts/2026-11-14-hyperframes/{index.html,compositions,script.txt,narration.txt} .
export HYPERFRAMES_NO_TELEMETRY=1
npm ci
for p in cinematic/swipe:swipe-cinematic studio/swipe:swipe-studio soft/swipe:swipe-soft studio/snap:snap-studio \
         studio/warning:warning-studio studio/success:success-studio cinematic/complete:complete-cinematic; do
  mkdir -p assets/sfx && cp node_modules/uisfx/sounds/${p%%:*}.mp3 assets/sfx/${p##*:}.mp3; done
voxcpm-tts say narration.txt -o assets/narration.wav --seed 7 --paragraph-gap 0.4
npx hyperframes transcribe assets/narration.wav --engine parakeet --no-runtime-install
python3 scripts/captions.py --script script.txt --transcript assets/transcript.json \
  --out compositions/captions.html --highlight HyperFrames,fact,FRAMES,14,27-second
npx hyperframes check && npx hyperframes render --output renders/raw.mp4
scripts/finish.sh renders/raw.mp4 out/short.mp4
```

A new voice take changes every time: re-read `assets/transcript.json` and move the
scene slots, anchor times and SFX starts to the new words.

## Measured (2026-11-14, M4 Pro Mac mini)

| | |
| --- | --- |
| Narration | 35.8 s, VoxCPM2 native, seed 7, generated in 49 s |
| Transcription | Parakeet; 75 of 81 script words matched. Heard "Factor hype", "pen", and no separate "HyperFrames" after "It's called" (inside a 1.1 s "called"): **listen at 10.0-11.1 s** |
| `hyperframes check` | 0 errors, 0 warnings after fixes (type under 28 px, lines held too briefly to read, glyphs past the left edge) |
| Render | 18.8 s for 38.4 s |
| Finished | -14.2 LUFS, -1.28 dBTP, 1080x1920, 30 fps, full decode |
| SFX | 11 cues; 10 within 4 ms of plan, the stamp cue fixed after it measured 0.18 s late |

Measured, not listened to.
