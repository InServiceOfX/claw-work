# From TikTok research to usable production workflows

The six-video research batch was merged at `61aeedf`. Its captions/screenshots
are evidence of demonstrated methods and creator claims, not proof of cost,
quality, revenue or a working connector on this machine. This follow-through
adds original skills and a bounded Rust execution helper.

## Highest reward per implementation effort

This ranking is an engineering judgment about reuse and avoided manual work,
not predicted income or a ranking by TikTok view counts.

| Priority | Capability | Why it earns its place | Current deliverable |
| --- | --- | --- | --- |
| 1 | Transcript-derived cuts and reusable brand recipe | Repeated cuts/caption work across recorded demos; easy to inspect and revise | [transcript-video-edit](../../skills/transcript-video-edit/SKILL.md), [Rust claw-edit](../../Rust/claw-edit/README.md) |
| 2 | Faceless coded explainers | Explains a product without filming or a per-clip generative-video service | [canvas-explainer](../../skills/canvas-explainer/SKILL.md), original scene/capture script |
| Fold into1 | Storyboard and brand/effects choices | Sarah's useful editorial idea; doesn't require a new CapCut integration | Included in transcript skill; no unverified connector |
| Fold into1 | Script-tied effects and chat revisions | Adam's edit loop is the same production family | Effects contract in storyboard; actual effects need a renderer |
| Defer | Custom CapCut automation | Project formats/adapters need independent validation; paid setup is not required for the first edit | Storyboard/EDL can be handed to an existing editor |
| Separate repo | DJ tone play | Existing original-source Rust timing path already exists | claw-dj `docs/TONE_PLAY_AGENT_HANDOFF.md`; no DJ implementation here |

## What each video taught

| Video | Breakdown | Transferable lesson | Evidence limit |
| --- | --- | --- | --- |
| [Brian Scully](https://www.tiktok.com/@bridotai/video/7701679351661743391), 78s | Record repeated lines continuously; choose the final complete takes; tighten cuts, caption/title, revise, save the accepted style | A recording convention reduces take-selection ambiguity; style should emerge from a real accepted edit | Plugin panels are not a tested local UI; last take need not be best/complete in arbitrary footage |
| [Sarah / abloommedia](https://www.tiktok.com/@abloommedia/video/7703263566719192350), 64s | Transcript analysis -> branded storyboard -> style questions -> claimed CapCut execution | A reusable brand/effects library plus editorial review avoids uniform generic edits | Styled Claude panels and a timeline screenshot don't establish a first-party integration; paid setup not obtained |
| [Jam](https://www.tiktok.com/@jamout.ai/video/7695100487216975118), 241s | Dictates a persistent editing skill: tools, platform/CTA, originals, transcription, take cleanup, branding, aspect/audio and short preview | The workflow package is more useful than one prompt; preview a representative join and preserve originals | Spoken instructions are video content, not execution authority; ASR/renderer/tool installs remain real dependencies |
| [Adam Stewart](https://www.tiktok.com/@adamstewartmarketing/video/7696038654904503560), 33s | Prompts a HyperFrames edit, illustrates cuts/captions/effects and a caption-size revision | Tie effects to script beats; revise an explicit plan/style instead of rebuilding editorial choices | Exact DM pack and advertised cost not independently verified; edits require an actual renderer |
| [Chase AI](https://www.tiktok.com/@chase_ai_/video/7693602525081111821), 849s | Code a frame as a function of time; storyboard keyframes; browser capture; FFmpeg encode; inspect/fix; package as a skill | Deterministic animation makes targeted preview/repair possible; references and storyboard help before long renders | Code-based rendering still costs model time/compute; contact sheets don't prove motion/audio quality; named plugin fetch unavailable here |
| [Phil Harris](https://www.tiktok.com/@djphilharris/video/7694719261549317379), 61s | Pitched DAW note bank + CDJ hot cues, or DJS-1000 Scale Mode | Separate cue selection, pitch offsets and note rhythm; software can implement the same musical abstraction | Not a Mixxx demo or verified note score; claw-dj already has a native original-source experiment and disallows replacing it with prepared masters |

Detailed source notes: [research index](../../research/tiktok-skill-research-2026-11-05/README.md).

## Use the skills

These are self-contained repository skill folders. Tell an agent to read the
chosen SKILL.md by path now. For automatic discovery, explicitly install/copy
the chosen folder using that harness's supported mechanism; no global skills,
plugins, credentials or shared skill directories were modified by this work.

Example requests:

- “Use skills/transcript-video-edit/SKILL.md. Turn this local recording and its
  timestamped transcript into a clean 30-second short. Prefer the final complete
  take of each line; show uncertain choices. Keep a clean master and captions.”
- “Use skills/canvas-explainer/SKILL.md. Make a silent 15-second portrait explainer
  of phrase alignment. Show the storyboard first, then render a local preview.”
- On the other computer: “In claw-dj, follow docs/TONE_PLAY_AGENT_HANDOFF.md.
  Reproduce synthetic checks and prepare a dry-run score. No live playback.”

## Workflows

### A. Recorded speech or screen demo

Local original -> measured transcript -> repeated-take grouping -> reviewed cut
table -> `claw-edit check` -> short cut preview -> full clean cut + retimed SRT ->
optional title/caption/effects render -> probe/decode/frame/audio checks -> human
review. Change only the cut/style elements requested in later revisions.

Use the Rust helper for the explicit cut list, not to guess speech. An existing
Whisper backend can provide timestamps; neither this repository addition nor
the model can manufacture those timestamps. The helper does not burn captions.
HyperFrames or a verified existing graphics path can handle that next pass.

### B. Faceless explainer

One takeaway -> beat script -> narration timing or explicit silent choice ->
representative storyboard frames -> deterministic `renderFrame(t)` scene ->
short motion preview -> bounded contact-sheet/fix loop -> full encode -> actual
media checks -> review. Original local starter: `skills/canvas-explainer/assets/scene.html`.

The bundled Playwright capture script can render the starter directly. Use its
reference instructions for project-local dependencies and an existing Chrome.
No paid creator pack, special agent model or upstream animate plugin is needed.

### C. DJ tone play, on the other machine

Use the handoff in the **claw-dj** repository. Existing Python validation + Rust
scheduling + Mixxx DSP are the starting point. Reproduce synthetic checks before
choosing a real cue; authorization for playback/capture is separate. No need to
port this video-production tool into that live path.

## Do we need more code?

No new AI model/service is needed. The existing agent handles semantic decisions;
local ASR, when available, provides timing; established renderers handle media.
The new Rust helper fills a specific raw-recording cut-list gap. It complements
the inspected Monoclaw `Rust/shortform-video` pipeline, which handles script/TTS/
stock footage/PNG captions but lacks bring-your-own-audio Whisper timing.
No Monoclaw source or history was copied into this public project.

Rust is appropriate for validation, media orchestration and timing. Keep canvas
drawing in JavaScript and browser capture in the small Playwright adapter; a Rust
browser/DSP rewrite would add work before proving value. Add word-level ASR
adapters, richer effect schemas or a reusable brand format only after a real
pilot shows which repeated step is still expensive.

## What to do now

Start with **one public/synthetic 7–15-second faceless explainer**, review the
storyboard and preview, then reuse the style on a second topic. For a recorded
demo, provide one local clip plus real transcript timestamps and try the cut-list
workflow. Measure hands-on minutes, corrections at joins, readability and whether
the final result communicates the intended point. These are first-pilot measures,
not a new app project or a promise of social revenue. Publishing stays separate.

Verification and known limits: [VERIFICATION.md](VERIFICATION.md).

Current primary technical references:
[HyperFrames](https://github.com/heygen-com/hyperframes),
[Playwright screenshots](https://playwright.dev/docs/screenshots),
[FFmpeg filters](https://ffmpeg.org/ffmpeg-filters.html).
