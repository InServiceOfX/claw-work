# TikTok research notes: marketing, outreach, and short-form content

Notes ingested from TikTok shorts for faceless short-form content and
app marketing (getting the word out about an app).
Each folder holds a `README.md` (metadata, summary, takeaways, verbatim
phrases, on-screen text, screenshots, how to apply), a timestamped
`transcript.md`, and `screenshots/` (720px JPGs). Raw videos are **not**
committed.

**Start here:** [PLAYBOOK.md](PLAYBOOK.md) is the synthesis of all seven TikToks and the YouTube Shorts.
YouTube Shorts and the screenshot success criterion are indexed in [../README.md](../README.md).

| Video | Creator | Uploaded | Topic |
|---|---|---|---|
| [danidocumented-7698978039249521933](danidocumented-7698978039249521933/README.md) | Danielle Lynch | 2026-10-21 | 6 closing and persuasion lines (verbatim) |
| [danidocumented-7698982436352445709](danidocumented-7698982436352445709/README.md) | Danielle Lynch | 2026-10-21 | Follow-ups: never "just following up" |
| [danidocumented-7699623985130081549](danidocumented-7699623985130081549/README.md) | Danielle Lynch | 2026-10-23 | 20 sales-psychology tactics |
| [jamout.ai-7700225527377857822](jamout.ai-7700225527377857822/README.md) | Jam AI & Marketing | 2026-10-24 | Entity map for AI SEO / LLM recommendations |
| [ronan0437-7688325930997075214](ronan0437-7688325930997075214/README.md) | Ronan | 2026-09-22 | Framing: text hook, eye line, captions |
| [themekhicarter-7696704990320004366](themekhicarter-7696704990320004366/README.md) | themekhicarter | 2026-10-15 | Building a following: value over virality |
| [ginyboi-7678511193039981854](ginyboi-7678511193039981854/README.md) | ginyboi | 2026-08-27 | 5-step short-form editing pipeline |

## How the folder-based ingest was made

1. `yt-dlp --write-info-json` for the video and metadata.
2. faster-whisper `small` (CPU, int8) transcription with segment timestamps.
   A VAD pass was used to fill gaps where the plain pass dropped speech.
3. Key frames picked from transcript cue points plus ffmpeg scene-change
   detection (`select='gt(scene,0.3)'`), reviewed by eye. Redundant frames
   were dropped. Tesseract OCR was not available (apt mirror errors), so the
   on-screen text was read visually from full-resolution crops.
4. Screenshots are kept unedited, including creator payout figures, since they're useful data points.
5. Success criterion: frames that show on-screen lists or key data must be captured legibly (see [../README.md](../README.md#screenshot-selection-success-criterion)).

DJ-transition videos from the same batch live in `InServiceOfX/claw-dj`
under `docs/research/tiktok-dj-transitions/`.
## Complementary caption-backed research pass — 2026-10-30

Both research passes are retained: the creator-named folders above contain the
Whisper transcripts and screenshots from the ingest branch; the video-ID Markdown
files below contain the separate caption-backed analysis, media hashes, caveats,
and application hypotheses from `bots/tiktok-learning-intake`. They cover the
same seven marketing videos, not seven additional videos. Evidence statements
apply to their respective pass; neither machine captions nor machine transcription
should be treated as error-free or as independent validation of creator claims.

### Method and status

Ten user-supplied videos downloaded across Monoclaw (seven) and claw-dj (three). This directory owns marketing/content research, not DJ product behavior. This complementary pass combines retrieved TikTok captions with inspected, meaning-selected frames. Its captions were not independently verified against audio. Its raw MP4s, metadata, VTTs, frame JPEGs and evidence manifest live outside Git in workspace `Data/Public/Videos/TikTok/research-intake-2026-10-30/`. These notes include source URLs, hashes, timestamps, observed findings and explicit gaps; this pass adds no generated images/media to Git. The folder-based ingest above separately retains reviewed screenshots.

### Caption-backed source notes

- [@danidocumented: six closes / phrase bank](7698978039249521933.md)
- [@danidocumented: follow-up messages](7698982436352445709.md)
- [@danidocumented: twenty persuasion tactics, critically assessed](7699623985130081549.md)
- [@jamout.ai: entity map for application discovery](7700225527377857822.md)
- [@ronan0437: visual framing and caption placement](7688325930997075214.md)
- [@themekhicarter: follower conversion, value and consistency](7696704990320004366.md)
- [@ginyboi: five-stage editing workflow](7678511193039981854.md)

DJ references: sibling repo `claw-dj/docs/research/tiktok/README.md`.

## Working synthesis (agent interpretation, not validated business results)

1. Choose a repeatable audience payoff, not merely a viral topic.
2. Write narration/script → tighten basic cut → illustrate meaning → readable captions → restrained sound/effects.
3. Preview layouts against platform controls. Avoid covering the demonstration or pushing captions into bottom UI.
4. Measure return viewers, follows per 1,000 views, qualified clicks and conversions. Reach alone is not a business outcome.
5. For outreach, bring a real useful observation, honestly qualify fit and propose a clear optional next step. Preserve source phrases without treating pressure tactics as an approved messaging policy.
6. For app discovery, connect accurate product/organization/use-case information to real evidence. An entity-map JSON file is not automatically valid Schema.org markup or a guarantee of AI recommendations.

## Preferred creators to revisit

The current watchlist is in [../REFERENCE_CREATORS.md](../REFERENCE_CREATORS.md) (@ginyboi was retired on 2026-11-14). Revisit occasionally during relevant research or heartbeat opportunities; prioritize substantive demonstrations and novel techniques. No automatic follow/like/comment/message, paid subscription, publication, or perpetual agent loop is authorized. No fixed scheduled job has been installed.

## Next evidence pass

- Compare caption-backed notes with the folder-based Whisper transcripts; independently check audio where necessary to resolve differences. The original caption-pass audio-verification gaps remain provenance, not a claim that no transcription exists anywhere in the combined archive.
- Closely inspect any small UI text needed for implementation; contact sheets do not validate schema syntax.
- Check authoritative platform/structured-data documentation before deploying advice.
- Turn selected ideas into bounded experiments only when product scope is clear; research alone changes no software or campaigns.
