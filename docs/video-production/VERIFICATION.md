# Verification and limits

Local implementation/evaluation: 2026-11-05. No remote model, paid creator
resource, real media upload, social publication or global skill installation.

## Rust cut-list helper

Rust/Cargo 1.91.1 and FFmpeg 8.0.1. Commands from `Rust/claw-edit`:

```bash
cargo fmt --check
cargo test --offline --locked -- --include-ignored
cargo clippy --offline --locked --all-targets -- -D warnings
cargo build --offline --locked --release
```

Six unit tests and one synthetic FFmpeg integration test pass. The integration
test checks reordered red/blue video **and** 440/880 Hz audio, retimed SRT,
changed output aspect/dimensions, duration, full decode, preservation of original
bytes, existing-directory/file/symlink refusal, invalid source range and offset
audio/video refusal. This demonstrates cut-list mechanics, not good editorial
selection, speech boundaries, music quality or caption burn-in.

## Canvas example

Node 26.6.0, Playwright 1.64.1, existing Google Chrome selected with
`CANVAS_BROWSER_CHANNEL=chrome`, FFmpeg 8.0.1. Used a copied skill in a disposable
local directory and reused an existing Playwright dependency. No dependency or
browser installation, user browser profile or Nocode change was needed.

- Actual starter render: H.264, 360x640, 30 fps, 210 frames, 7.000 seconds, silent.
- ffprobe and complete FFmpeg decode pass.
- A sampled contact sheet was visually inspected; incoming-track bars were
  moved inward to preserve the right margin before the final render.
- Browser checks confirm identical canvas screenshots when returning to the
  same timestamp after drawing another, and different pictures at different
  timestamps. Re-running the renderer against an existing directory fails and
  leaves the prior MP4 byte-identical.
- The default Playwright browser revision was absent. Initial restricted Chrome
  launch also failed; the successful checks used an isolated headless Chrome
  launch with approved execution permissions. Neither failure was a successful
  browser/render test. Current script supports the explicit installed-channel
  choice and does not download a browser automatically.

This is a minimal silent illustration, not an audio-synchronized production
short or proof of polished motion. Screenshots establish frame/layout behavior;
real-time playback smoothness and an audio audition are not claimed. No ASR,
TTS, upstream animate plugin or HyperFrames render was exercised.

## Skills, docs and publication boundary

The skill-creator validator passes for `transcript-video-edit`,
`canvas-explainer` and the separate claw-dj `claw-dj-tone-play` skill. The Node
capture script passes syntax checking. New relative Markdown links and Git
whitespace checks are checked locally. No independent agent behavioral trial
was run; frontmatter validation is not proof of good editorial judgment.

The public repository gate passed against a disposable complete Git index
containing HEAD plus the explicitly selected new/changed files. This includes
the existing reviewed-binary manifest. The real staging index is left alone;
source, links and the proposed diff were also inspected. No generated frames,
videos, diagnostics or local dependency tree belong in the publication set.

The existing root public-boundary/Lean prompt graph and Monoclaw source remain
unchanged. The new helper is conventional code, not a claimed PDD-generated
dev unit. DJ work is documentation only in claw-dj: no fresh pitch-play test,
live audition, saved-plan mutation or musical approval is claimed here.

## Remaining production dependencies

- Recorded-speech editing needs actual timestamped transcription and an audition
  of joins. The tool executes supplied ranges; it cannot select takes itself.
- Titles, burned captions, zooms, grading and loudness need a verified subsequent
  effects pass; HyperFrames was researched, not installed/tested here.
- A canvas project with external/local media needs an explicit asset-loading
  scheme beyond the inline starter. The bundled page blocks remote resources;
  it is not an isolation boundary for hostile scene code. Long scenes need a
  deliberate compute/storage budget.
- The DJ implementation already exists in its own repository. Its cross-machine
  handoff makes source identity, current restrictions, device setup and separate
  live/capture authorization explicit; copying Markdown does not deploy it.
