# Creator lessons, distilled

What other short-form creators taught that changes how we plan or render a
Short. Each lesson names its source and where it now lives. Creator view counts
and income figures are their claims, and format-versus-views links are
correlation. Hook, retention and engagement fundamentals are Noe Murillo's, in
`noe-murillo-pt1-framework.md` and `noe-murillo-pt2-framework.md`; they are not
repeated here.

"Enforced" means `claw-shorts check` (or the renderer) applies it, so a draft
cannot skip it. "Applied" means the renderer does it for you. "Plan" means it
is a planning rule for you or the agent.

## Script

| Lesson | Source | Status |
| --- | --- | --- |
| Vary sentence length; never run two long sentences back to back. House style (2026-11-22): full sentences of about 6-12 words by default; a line of 5 words or fewer only rarely, to land one point or for the final summary. The source's long-short-long-short cadence is too choppy for us | @ginyboi, crediting @kallaway (2026-10-23); adapted | Enforced: warning on two sentences over 12 words back to back, or 4+ sentences with no short line at all |
| A human writes the script, or edits the LLM draft hard. LLM scripts read cheesy and repeat themselves as they get longer; keep them short. Automate cuts, captions and motion, not the words. | @ginyboi, a month testing AI as an editor (2026-10-28) | Plan |
| Value earns follows; views alone do not. Give a reason to come back: a specific skill or knowledge most people lack. | @themekhicarter (2026-10) | Plan |
| Claim, visible source, live run, measured number, verdict. | @itstundealao "Fact or Hype?" | Enforced as warnings (URL, number, verdict, disclosure) |
| A close reel needs one real number, the same in the open, the narration and the end card. | @a13x_duke, see `build-in-public-close-reel.md` | Enforced: `scoreboard` |

## Picture

| Lesson | Source | Status |
| --- | --- | --- |
| Change the picture every 3-5 seconds. No footage? Build a branded "B-roll space" frame for screenshots, icons and assets. | @ginyboi 5-step pipeline (2026-08) | Applied when asked: `shortform-video --still-layout frame` (bordered card on brand colors). Scene holds over 10 s warn in `claw-shorts canvas` |
| Stills need motion: slow zoom or sideways pan (4-8% over 3-5 s) so momentum never dies. | @ginyboi, editing without B-roll (2026-08-25) | Applied: Ken Burns on local stills in `shortform-video` |
| A camera-shutter sound on each cut to a photo reads as "looking through memories" (storytelling, travel, finance). | same | Applied when asked: `shortform-video --still-sfx` (click synthesized, no SFX file to license) |
| When cutting fast between portraits, keep the eyes on one line so viewers don't tire. | same | Plan only: automating it needs a face-detection model, and faceless explainers rarely cut between portraits |
| Never leave a visible jump at a cut: overlap the audio across it and cover both sides with B-roll. | @ginyboi jump cuts (2026-08-05) | Applied by design: narration is one continuous track under the visuals |
| Keep effects light; too many steal from the message. | @ginyboi 5-step pipeline | Plan |

## Captions

| Lesson | Source | Status |
| --- | --- | --- |
| Captions are part of the storytelling. Pick the style for the audience: short, bouncy captions for young viewers; clean sans-serif and simple layouts for professionals. Finish the edit, then choose the caption style; don't default to generic TikTok captions. | @ginyboi (2026-11-01, and step 4 of the 5-step pipeline) | Plan; style comes from the `claw-shorts` style recipe |
| Spell-check burned-in text (his own fix shipped "Pheonix"). | @ginyboi jump cuts | Plan; `claw-shorts` captions use the narration's own spelling |

## Sound

| Lesson | Source | Status |
| --- | --- | --- |
| Measure, don't trust your ears: finish at -14 LUFS integrated, -1 dBTP. A hard limiter before the loudness step lets a peaky voice get loud without clipping. | @ginyboi audio settings (2026-10-25) | Applied: every `claw-shorts` and `shortform-video` render; the run fails if it misses by more than 1 LU or clips |
| Processing beats gear: a phone mic plus EQ and a limiter is enough. | same | Plan (no recording on camera in this workflow) |

## Cadence

| Lesson | Source | Status |
| --- | --- | --- |
| Cap creation at about an hour a day; batch writing on one day; outsource or automate the edit when your hour is worth more. | @ginyboi posting every day (2026-11-04) | Plan: write a week of specs in one sitting, render in batches of 3-5 |
| Consistency: be unavoidable on the timeline. | @themekhicarter | Plan |

## Sources

- @ginyboi: claw-work `docs/shortform/tiktok/ginyboi-7678511193039981854/` and
  `research/tiktok-shortform-checkin-2026-10-31/`,
  `research/tiktok-faceless-analogues-2026-11-06/`,
  `research/tiktok-ginyboi-checkin-2026-11-07/`. Checks retired 2026-11-14; see
  `docs/shortform/REFERENCE_CREATORS.md`.
- @themekhicarter: `docs/shortform/tiktok/themekhicarter-7696704990320004366/`.
- @itstundealao: `research/tiktok-itstundealao-2026-11-06/themes/03-fact-or-hype-format.md`.
- @a13x_duke: `build-in-public-close-reel.md`.
