# Math mode (Manim)

Equations that transform, graphs that draw themselves, a term boxed as the
voice names it: the visual language of math explainers. Built on
[Manim Community Edition](https://github.com/ManimCommunity/manim) (MIT),
pinned and used as is. Files: `skills/claw-shorts/manim/`.

## When to use it

| Use Math mode | Use another mode |
| --- | --- |
| A STEM explainer whose point is an equation, a derivation, a plot or a geometric picture | A claim about a tool or project (voiced explainer, canvas) |
| You need real typeset math (LaTeX) that moves | A listicle with cards, charts and kinetic captions (HyperFrames) |
| Short beats: one idea per narration scene | An equation-heavy talk you will record yourself (`docs/shortform/obs-deck-pipeline/`) |

## Install (once per machine)

- `uv` and FFmpeg on PATH.
- LaTeX for `MathTex`/`Tex`: MacTeX or BasicTeX on macOS (`/Library/TeX/texbin`,
  which `render.sh` adds to PATH if needed), TeX Live on Linux. `Text` works
  without it.
- macOS: Homebrew `cairo` and `pkgconf`. pycairo 1.29 has no macOS wheel for
  this setup and is compiled on first install against Homebrew cairo.
  ManimPango ships a self-contained wheel.
- The first `scripts/render.sh` creates `skills/claw-shorts/manim/.venv` from
  `uv.lock` (about 280 MB, a few seconds when cairo is present). Nothing is
  installed globally.

## Render

```bash
cd skills/claw-shorts/manim
scripts/render.sh scenes/small_angle.py SmallAngle OUT_DIR
```

Writes `OUT_DIR/video.mp4` (silent, 1080×1920, 30 fps, H.264) and prints its
size, length and sha256; Manim's working files stay in `OUT_DIR/manim-media`. It
refuses to overwrite an existing `video.mp4`. The 18 s example renders in
5-6 s on an Apple Silicon Mac.

## Writing a scene

Subclass `claw_manim.ShortScene`, not `Scene`:

- **Units.** The frame is 9 × 16 units, 1 unit = 120 px, origin at the centre.
  Constants: `SAFE_TOP` 5.6, `SAFE_BOTTOM` -2.4, `SAFE_LEFT` -3.5,
  `SAFE_RIGHT_HIGH` 2.9 (above `BUTTONS_Y` 1.0), `SAFE_RIGHT_LOW` 2.0 (below
  it), `CAPTION_TOP` about -1.08.
- **Safe-area gate.** After every `play` and `wait`, each visible `Text`,
  `MarkupText`, `Tex` or `MathTex` must be inside the safe area (y 288-1248 px;
  x 120-888 px above y 840, x 120-780 px below). A violation stops the render
  and names the text, its edge and the time. Graph lines and shapes are not
  checked; keep them out from under the right-hand buttons anyway.
- **Size.** `font_size` is large in this frame: `Text` at 40 is about 60 px
  tall, `MathTex` at 80 with a fraction is about 1.5 units. Use `fit(mob,
  width)` to scale down only when needed; usable widths are about 6.2 units
  above y = 1 and 5.3 below.
- **One line per equation.** A two-line `VGroup` of fraction-heavy `MathTex`
  is about 4 units tall and will not fit under a graph. Put `+\cdots` on the
  same line, or fade the graph first.
- **Colors.** `claw_manim` exports the claw-shorts default style (`BG`, `FG`,
  `ACCENT`, `MUTED`, `PANEL`, `WARN`, `FAIL`); `vertical.cfg` sets the
  background.
- **Beats.** Set `DEFAULT_BEATS` (seconds per beat) and end each beat with
  `self.end_beat(i)`. That holds to the end of beat i, so the scene renders at
  its intended length before narration exists.

## Narration

Reuse the claw-shorts tools; do not mix audio here.

1. Write a spec with one scene per beat and its narration (the example's
   docstring has a 4-beat script), then
   `claw-shorts canvas --spec short.json --work work --output prep --strict`.
   That writes `prep/narration.wav`, normalized to -14 LUFS / -1 dBTP, and
   `prep/timeline.json`, where each narration scene has a `start_ms` and `end_ms`.
2. Optional word cues: `.venv/bin/python Rust/claw-shorts/scripts/words.py prep/narration.wav words.json`
   (its venv and local Whisper model: see the Setup note in `Rust/claw-shorts/README.md`).
3. Render with both:
   `scripts/render.sh scenes/x.py X out --timeline prep/timeline.json --words words.json`.
   Each `end_beat(i)` now ends where narration scene i ends, and
   `self.hold_until(self.word_time("cubed", after=self.now, default=self.now))`
   starts the next animation on that spoken word (with no words file it falls
   back to `default`).
4. `claw-shorts mux --video out/video.mp4 --audio prep/narration.wav --output short.mp4`.
   It refuses if video and narration differ by more than 150 ms and reports the
   final loudness.

If a beat's animations run longer than its narration, `end_beat` prints a
warning and the video drifts late; shorten `run_time`s. Tested on 2026-11-22
with a 4-beat stand-in timeline (16.4 s): the render matched to the frame and
`mux` accepted it.

## Determinism

Two renders of the same scene with the same lock file on the same machine
were byte-identical (same sha256), with or without a timeline.
`disable_caching = True` in `vertical.cfg` keeps stale partial movies from
leaking in after an edit. Across machines, expect different bytes: LaTeX,
fonts, cairo and FFmpeg versions all change the pixels.

## Gotchas

- `-ql`/`-qh` quality flags override `vertical.cfg`'s resolution; do not pass them.
- `MathTex` substrings are split by the strings you pass
  (`MathTex(r"\sin x", r"=", r"x", r"-\frac{x^3}{6}")`); index them to color
  or box one term.
- `Text` fonts come from the system through Pango (`FONT = "Arial"`). A missing
  font silently falls back, so check frames on a new machine.
- LaTeX output is cached under `OUT_DIR/manim-media/Tex`; delete it to force re-typesetting.
- Manim logs before it reads the config; `render.sh` passes `-v WARNING`.

## License

Manim Community Edition is MIT: two notices, 3Blue1Brown LLC (2018) and the Manim
Community Developers (2024) (`LICENSE`, `LICENSE.community` in the package). [3b1b/manim](https://github.com/3b1b/manim) (ManimGL) is a separate
project with a different API and an OpenGL renderer; this mode uses the
Community Edition only.
