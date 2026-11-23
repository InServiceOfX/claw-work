# claw-motion

Headless, deterministic motion graphics for vertical Shorts, in Rust:

- **`physics`** simulates 2D circles falling through walls and pegs (a Galton
  board, Plinko, a funnel or a pile-up), with an impact sound synthesized on the
  exact simulation step of every collision.
- **`morph`** turns one shape smoothly into another: circle, polygon, star,
  heart, explicit points, or an SVG path.

Each command renders a 1080×1920 MP4 straight from a TOML scene file. There is
no browser and no GUI. The same scene gives the same bytes on every run.

The ideas come from two JavaScript libraries, and both were rewritten here from
scratch rather than wrapped:

| Idea | Source (both MIT) | What we kept | What differs |
| --- | --- | --- | --- |
| Rigid-body physics | [Matter.js](https://github.com/liabru/matter-js) (liabru) | Fixed timestep, iterative impulse solver, positional correction pass | Circles against static capsules only, with no rotation, sleeping or joints. Collisions are recorded with time and speed so sound can follow them |
| Shape morphing | [Flubber](https://github.com/veltman/flubber) (veltman) | Same point count on both rings, matching winding, rotation to the best alignment, point-wise interpolation | Even arc-length resampling instead of edge bisection, an exhaustive rotation search, and a small SVG path flattener (M L H V C Q Z) |

The research and verdicts behind this crate are in
[`research/tiktok-animation-tools-2026-11-22/`](../../research/tiktok-animation-tools-2026-11-22/README.md).

## Use

```bash
cargo build --release --manifest-path Rust/claw-motion/Cargo.toml
M=Rust/claw-motion/target/release/claw-motion

$M example plinko > board.toml                 # a starting scene (Galton board)
$M physics board.toml --still 8 -o check.png   # one frame, in well under a second: look before rendering
$M physics board.toml -o board.mp4             # video + impact sounds at -14 LUFS
voxcpm-tts say narration.txt -o voice.wav --voice ref.wav  # local TTS (Rust/voxcpm-tts)
$M physics board.toml --narration voice.wav -o board.mp4   # narration on top, SFX 14 dB under it
$M morph Rust/claw-motion/examples/morph.toml -o morph.mp4
```

The output must be a new file; `--force` replaces it. A physics render also writes:

| File | What |
| --- | --- |
| `<name>.impacts.json` | Every collision: time, approach speed, position, radius, static or not |
| `<name>.sfx.wav` | The impact-sound stem on its own (48 kHz mono), for remixing elsewhere |
| `<name>.report.json` | Frames, render time, bodies, impacts heard, loudness before and after |

Audio is mixed in its own FFmpeg pass, normalized with `claw-shorts`' shared
loudness module (-14 LUFS integrated, -1 dBTP, limiter step when needed), then
muxed. A result off target by more than 1 LU fails the render. A morph without
`--narration` is silent and has no audio track.

## Scene files

See [`examples/plinko.toml`](examples/plinko.toml) and
[`examples/morph.toml`](examples/morph.toml); every field is shown there.
Unknown fields are errors, so typos fail loudly.

- **Units** are pixels and seconds, with y pointing down. Gravity 1960 px/s²
  is "1 m = 200 px".
- **`[[wall]]`** is a polyline of capsules (`points`, `radius`, `closed`).
  **`[[pegs]]`** is a grid, staggered by default. **`[[emitter]]`** drops
  `count` balls every `every` seconds with seeded jitter, radius range and
  colors.
- **`air`** is drag per second. It caps speed at about `gravity / air`, and it
  is what makes a Galton board give a bell curve. Without it a glancing hit
  flings a ball across the board and the bins fill flat. It stands in for a
  board that is tilted, as real ones are.
- **`[sfx]`**: `base_hz` (pitch of a 14 px ball; smaller balls are higher),
  `full_speed` (approach speed at full level), `per_window` (hits kept per
  12 ms, which stops a pile-up from turning into noise).
- **Morph keyframes** have `hold` and `morph` seconds. `loop = true` morphs the
  last shape back to the first. Shapes live in a unit box (-1..1) scaled by
  `scale` around `center`.

Keep the subject inside the 1080×1920 safe area (y 288-1248, x 120-960); both
examples do, leaving room for a title or captions.

## Verified (2026-11-22, Apple M5, FFmpeg 8)

- 26 unit tests: free-fall time and speed of the first impact, bounce height
  from restitution, elastic head-on swap, no sinking in stacks, deterministic
  worlds, knocks on their impact sample, thinning, SVG parsing, morph alignment
  and timeline, scene validation.
- The Galton example (18 s, 300 balls, about 3,000 collisions) renders in about
  5 s. Two renders gave byte-identical MP4 and SFX files. The first knock starts
  on the sample of the first impact (0.9750 s).
- Loudness: SFX only reached -14.04 LUFS, peak -4.0 dBTP. With a macOS `say`
  test narration -14.51 LUFS (real Shorts use `voxcpm-tts`). Morph plus narration -14.03 LUFS.
- Watched and listened to on 2026-11-22; the Galton board and morph
  examples were approved as they are (1,976 knocks kept over 18 s, about 110
  per second on average). Not yet tried under a real `voxcpm-tts` narration.

## Limits

- Circles only, with no rotation. Rolling looks like sliding, which does not
  show on plain disks.
- Static geometry only: no moving paddles or joints yet.
- About 5,000 bodies at most; the solver is meant for hundreds.
- Morph shapes are single filled rings: no holes, and only the first subpath
  of an SVG path. Arcs (`A`) are rejected.
