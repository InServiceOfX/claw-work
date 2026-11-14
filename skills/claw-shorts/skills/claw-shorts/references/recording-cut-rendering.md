# Local rendering choices

## Reviewed cuts: claw-edit (Rust)

Locate `Rust/claw-edit` in a claw-work checkout; use its README for build and
verification. Skills copied away from the checkout require that tool separately.
The helper needs local FFmpeg/ffprobe, one source file with known-duration audio
and video streams, and a reviewed plan. It performs no ASR or editorial selection.

Example plan next to a local `raw.mp4` (the text/timing here is illustrative):

```json
{
  "version": 1,
  "source": "raw.mp4",
  "width": 1080,
  "height": 1920,
  "fps": 30,
  "clips": [
    {"start_ms": 2000, "end_ms": 4800, "text": "The complete final take."},
    {"start_ms": 7200, "end_ms": 10500, "text": "The next complete line."}
  ]
}
```

Source paths resolve relative to the plan. Times are integer milliseconds on the
source playback timeline; clips appear in array order. One plain caption line per
clip. For finer caption timing, split clips at actual speech boundaries or create
separate word-aligned cues from the accepted transcript, then retime them by the
same cut offsets. Do not split just to fabricate word timing.

```bash
claw-edit check --plan edit.json
claw-edit render --plan edit.json --output-dir first-cut
ffprobe -v error -show_streams -show_format first-cut/cut.mp4
ffmpeg -v error -i first-cut/cut.mp4 -f null -
```

Output: `cut.mp4`, `captions.srt`, `timeline.json`. Output directories must be new.
It pads to fit instead of cropping faces/screens; requested reframing, burn-in,
titles, loudness processing and motion graphics need a subsequent renderer.
The cut report records intended timing, not proof of visual or audio quality.
There is no model endpoint, stock media lookup, upload, or publish command.

## Branded effects and captions

Prefer an already-installed renderer. HyperFrames officially supports an
agent-driven HTML/seekable-animation/MP4 loop:
https://github.com/heygen-com/hyperframes

Check the installed CLI help and official release docs before using it. Do not
assume that an animated TikTok plugin panel is the actual UI. Do not run an
unreviewed installer or overwrite whole `.agents/skills` / `.claude/skills`
directories to fix one missing skill.

For an FFmpeg build with libass, captions can be burned into a subsequent copy
with its `subtitles` filter. Paths inside filter expressions need FFmpeg escaping;
using an argument array alone does not escape that grammar. If the filter is
missing, use HTML captions or a verified PNG-overlay path. Do not claim caption
burn-in merely because an SRT exists. Keep a clean master for later revisions.

Sarah's video is evidence of a storyboard/brand-library concept, not a verified
public CapCut API. Deliver a storyboard or use a separately verified adapter;
never claim a live CapCut edit without checking the resulting project.

FFmpeg filter reference: https://ffmpeg.org/ffmpeg-filters.html (trim, atrim,
setpts, asetpts, concat). Each kept audio/video segment resets timestamps before
concatenation; captions must use the resulting output offsets.
