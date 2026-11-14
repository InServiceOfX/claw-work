# shortform-ingest

Turn TikTok / YouTube Shorts (and other yt-dlp-supported short-form) videos into
claw-work research notes under `docs/shortform/<platform>/<creator>-<videoid>/`
(`README.md`, `transcript.md`, `screenshots/`). Used for the first batch in
[`docs/shortform/`](../../docs/shortform/README.md) (7 TikToks, 1 YouTube Short).

The script automates the mechanical steps. **Choosing key frames and writing the
notes is a human/LLM review step**: frames are picked by content, not at a fixed
frequency.

## Requirements

- `yt-dlp` (worked on TikTok and YouTube Shorts with no extra extractor args)
- `ffmpeg` (with `drawtext`, for labelled review sheets)
- Python 3.10+ venv with a whisper engine:

```bash
python3 -m venv .venv && . .venv/bin/activate
pip install -r requirements.txt          # faster-whisper (default engine, CPU int8)
pip install openai-whisper               # optional: --engine openai
```

OCR is optional and not wired in. On the first run `tesseract` could not be
installed (the package server returned 500), so on-screen text was read by eye
from the frames. That worked, but means every list item must be typed in by the
reviewer.

## Steps

All commands take `--work DIR` (default `./shortform-work`). Raw video, audio,
and candidate frames live there; **never commit them**.

```bash
S=Scripts/shortform-ingest/shortform_ingest.py
# 1. Download video + metadata. Slug = <creator>-<videoid> (creator lower-cased,
#    video id keeps its case, e.g. wanderlustalley-cY6GojPFOYc).
python3 $S download https://www.tiktok.com/@user/video/123 https://youtube.com/shorts/abc

# 2. Timestamped transcript (whisper "small"). Add --vad over loud music; if a
#    VAD pass and a plain pass disagree, compare both and note it.
python3 $S transcribe [--engine faster|openai] [--model small] [--vad]

# 3. Candidate frames + contact sheets in WORK/cand/<slug>/:
#    ffmpeg scene changes (+0.3s), a dense 0.2s grid over the first 3s, a 2s grid,
#    and the last frames. sheet_NN.jpg are 5x4 grids labelled with the time.
python3 $S candidates [--early-seconds 3 --early-step 0.2 --scene-threshold 0.3]

# 4. Review the sheets (and transcript cue points). Where text flashes, step
#    frame by frame around it. Then grab the chosen times at 720px:
python3 $S grab <slug> 0.6 12 41.5

# 5. Write transcript.md, a README.md skeleton (metadata, caption, TODO sections),
#    and copy the chosen screenshots:
python3 $S scaffold <slug> --platform tiktok --out docs/shortform \
    --shot "0.6:Full 12-item income list" --shot "12:Whiteboard rule"
```

Then fill in the README (summary, takeaways, verbatim phrases, every on-screen
list item, how to apply), correct obvious transcript mishearings in the notes,
and update `docs/shortform/README.md` (and `tiktok/README.md` / `PLAYBOOK.md`)
plus `docs/shortform/tiktok/PROGRESS.md`.

## Success criteria (review checklist)

- **Every on-screen list or key data frame is captured legibly**: numbered
  lists, analytics cards, payouts, income lists, prices. If a list builds up
  or scrolls, capture each distinct state so every item is readable, and
  transcribe every item into the notes.
- Lists often appear for only ~1s in the first seconds (the YouTube Short's
  12-item list was on screen from 0:00.4 to 0:01.4). The dense early grid
  exists for this; neither scene detection nor a 2s grid reliably catches it.
  Also check the final frames, which often hold the complete overlay.
- Screenshots are **unedited** frames (crops of an overlay are fine as an
  extra shot, but never blur or redact). Creator payout figures are public
  data from the video and are kept, not redacted.
- **Never commit raw video** (`*.mp4`) or extracted audio. Commit only the
  notes and the chosen JPG screenshots.
- No secrets, cookies, or account data in commands or notes.
