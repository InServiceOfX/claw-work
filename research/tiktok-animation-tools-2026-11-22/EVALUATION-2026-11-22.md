# Hands-on evaluation: what we built from the 2026-11-22 research

The [README](README.md) ranked tools from one TikTok (@nicksaraevdaily, "20 free
tools") and one Notion link page by reading their sources. Three tools cleared
the bar of being headless, frame-exact and new to our stack. This file records
what we did with each, run on 2026-11-22 (Apple M5, macOS, FFmpeg 8, Rust 1.91,
uv with Python 3.13).

## Decisions

| Tool | Decision | Why not the other option |
| --- | --- | --- |
| **Manim CE** | **Use as is**, pinned `manim==0.22.1` from PyPI via `uv` (no git clone) | Mature (MIT, years of releases), deterministic, and the only tool in the list that typesets and animates math. Rewriting it in Rust would cost months for no gain. Clone only if we need to patch it |
| **Matter.js** | **Wrote our own in Rust** (`Rust/claw-motion physics`) | We need one narrow part of it: circles, static walls, offline stepping, and the time of every collision. A JS library would also need a browser and a pre-simulation to work under HyperFrames' per-frame seeking |
| **Flubber** | **Wrote our own in Rust** (`Rust/claw-motion morph`) | About 300 lines including an SVG path flattener. Upstream has been dormant since 2022 |
| Blender, Lottie | **Closed**; see the README table for what would reopen them | — |
| Inochi2D | **Skip, final**: no avatar personas | — |
| Everything else | Unchanged from the README verdicts | — |

## Math mode (Manim): `skills/claw-shorts/manim/`

- **Frame:** 1080×1920, 30 fps, 9×16 scene units (1 unit = 120 px), on the
  claw-shorts background color.
- **Safe-area gate:** a `ShortScene` base class checks every visible
  Text/Tex/MathTex after each `play` and `wait`. A word outside y 288-1248 /
  x 120-960 stops the render, naming the text, the edge and the time. It
  caught three real layout faults while the example was being built.
- **Narration timing:** beats can end where narration scenes end
  (`--timeline` with the `timeline.json` from `claw-shorts canvas`), or at word
  cues (`--words`). Audio and -14 LUFS come from the existing `claw-shorts mux`;
  no new mixing code.
- **Example:** `scenes/small_angle.py` (why sin x ≈ x for small x: plots, the
  Taylor series, the error term boxed on its cue), 18 s. It renders in 5-6 s
  warm. Two renders were byte-identical on this machine; another machine's
  LaTeX, fonts, cairo or FFmpeg will give different bytes.
- **Install hurdle:** pycairo compiles against Homebrew `cairo` and `pkgconf`
  (already present here; another Mac needs `brew install cairo pkgconf`). The
  `.venv` is 277 MB and ignored by git.
- **Not done:** no real TTS narration was put under it (the `mux` path was
  tested with a timeline and a tone), and nobody has listened to it.

## Physics and morph: `Rust/claw-motion`

- **Physics:** fixed 240 Hz step, iterative impulse solver with restitution and
  Coulomb friction, positional correction, a grid broadphase, seeded spawns.
  Every new contact faster than a threshold is logged as an impact.
- **Sound:** each kept impact becomes a synthesized knock. Pitch follows size
  and level follows speed, and the knock starts on the step of the hit.
  Thinning keeps at most N hits per 12 ms. Audio is normalized with the shared
  `claw_shorts::loudness` code.
- **Morph:** arc-length resampling, winding match, best rotation, eased
  interpolation, color blend, and an SVG path flattener (M L H V C Q Z).
- **Checks:** 26 unit tests and a clean Clippy.

| Check | Result |
| --- | --- |
| Galton example render (18 s, 300 balls, 3,033 impacts, 1,976 heard) | about 5 s |
| Same scene rendered twice | MP4 and SFX stem byte-identical |
| First knock vs first impact | both at 0.9750 s |
| Loudness, SFX only / with a macOS `say` test narration / morph with the same narration | -14.04 / -14.51 / -14.03 LUFS, all under -1 dBTP |

### The lesson from tuning the Galton board

The first two boards filled their bins **flat**, not in a bell curve. The
solver was fine; the board was not. Balls were small relative to the peg
spacing and kept their sideways speed after a glancing hit, so they skipped
several columns per row. Real Galton boards are tilted, which keeps the balls
slow, and their pegs are about two to three ball widths apart. Adding drag
(`air`, so speed tops out near `gravity / air`) and tightening the pegs gave a
clean bell. Three variants were compared side by side before choosing gravity
1960, air 4.5. The tuning is written into
[`motion.md`](../../skills/claw-shorts/skills/claw-shorts/references/motion.md)
so the next scene starts from it.

## How this changes what we can make

- **STEM Shorts with real demos:** "why the bell curve shows up everywhere"
  (Galton board) and "why sin x ≈ x" (Manim) are now each one command plus a
  narration. Video 01's best idea was that each item should get a tiny
  animated demo of itself, not a logo card, and these two modes provide that.
- **Satisfying physics loops** as a faceless format, with sound on every hit.
- **Shape morphs** as a transition beat inside any mode (render the morph, then
  use it as a clip).

## Not done

- Nothing was posted.
- The test narration was macOS `say`. Real Shorts use our local TTS, `Rust/voxcpm-tts`
  (`voxcpm-tts say script.txt -o voice.wav --voice <authorized reference>`), whose
  WAV goes to `claw-motion --narration` or `claw-shorts mux` unchanged.
- Audio from these modes was approved by eye and ear on 2026-11-22 for the
  Galton board, morph and Manim example, but the knock timbre and density
  have not been tuned against a real voxcpm narration. The knock density and
  timbre, and the Manim narration path with a real voice, need a human ear.
- The five hidden tools on the video's grid are still unidentified.
