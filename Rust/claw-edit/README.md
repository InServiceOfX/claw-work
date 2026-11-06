# claw-edit

A small Rust execution tool for reviewed, transcript-derived cut lists. It
trims/joins the corresponding audio and video, fits the requested dimensions
without cropping, and retimes one caption per clip. An AI agent or human chooses
the cuts; this tool validates and executes them locally through FFmpeg.

It has no model calls, transcription, media downloads or upload/publish command.
It does not infer the best/final take, burn captions, add titles, grade footage
or implement motion graphics. Those are separate production stages.

## Build and use

Requirements: Rust/Cargo, FFmpeg with libx264/AAC and ffprobe on PATH. Source
must be a local regular file with first audio/video streams of known finite
duration and aligned starts (within 1 ms); normalize offset/missing-duration
media separately. Local-file protocols only are enabled for probe/render.

```bash
cargo build --offline --locked --release
./target/release/claw-edit check --plan /tmp/my-edit/edit.json
./target/release/claw-edit render --plan /tmp/my-edit/edit.json --output-dir /tmp/my-edit/first-cut
```

Example plan, beside `raw.mp4`:

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

Relative sources resolve beside the plan. Clips are emitted in array order;
reordering/repeating is allowed. All times are integer milliseconds, with the
end exclusive. One nonempty plain caption line per clip; no control characters
or SRT timing arrows. Unknown fields are errors so unsupported effects are not
silently ignored. Up to 128 clips, ten minutes of output, even dimensions 2..3840,
fps 1..60, source up to 24 hours, plan at most 1 MB.

`check` probes and prints the intended output timeline without writing files.
`render` atomically reserves a **new directory** and writes `cut.mp4`,
`captions.srt` and finally `timeline.json`. Existing directories/files/symlinks
are refused. Original media is never opened for writing. A failed FFmpeg job can
leave a partial directory; retry with a new directory. There is no automatic
overwrite/cleanup, sandbox for hostile media, or process deadline manager.

Captions cover each entire kept clip and use edited offsets. They are a sidecar,
not word-aligned karaoke or burn-in. Frames/audio packets quantize timing to
their media clocks; the JSON timeline is intended timing, not measurement.
Inspect actual outputs before delivery, including audio around the joins.

## Verification

```bash
cargo fmt --check
cargo test --offline --locked
cargo test --offline --locked --test media -- --include-ignored
cargo clippy --offline --locked --all-targets -- -D warnings
```

The ignored media test requires FFmpeg. It generates red/440 Hz and blue/880 Hz
footage, reverses two source spans, checks output pixels and audio frequencies,
dimensions/duration and edited caption offsets, fully decodes the output,
compares original bytes and verifies existing-output/invalid-range/offset-media
refusal. No private media or live audio playback is used.

This complements an existing `shortform-video` faceless pipeline rather than
replacing script/TTS/stock-media stages. Rendering follows the official FFmpeg
[trim/atrim/setpts/concat](https://ffmpeg.org/ffmpeg-filters.html) semantics.
