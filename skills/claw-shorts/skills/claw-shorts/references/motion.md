# Physics and morph mode (`Rust/claw-motion`)

Use this mode when the picture is the point. Two kinds of scene:

- Something physical: balls through a Galton board ("every peg is a coin flip,
  and the bins become a bell curve"), Plinko, a funnel, a pile-up, a bounce
  that shows restitution.
- A shape turning into another: an icon becoming an icon, a logo reveal, a
  "this becomes that" beat.

Both are drawn in Rust and piped to FFmpeg. Collisions come with sounds placed
on their exact frame. Nothing runs in a browser.

## Steps

1. **Start from an example.**
   `claw-motion example plinko > scene.toml` or `claw-motion example morph`.
   Change geometry, colors, counts and timing. Unknown fields are errors.
2. **Look before rendering.** `claw-motion physics scene.toml --still 8 -o f8.png`
   draws one frame in under a second. Check two or three times: the start,
   the busiest moment, the end. Keep the subject inside y 288-1248 and
   x 120-960.
3. **Narrate (optional).** Write the narration with `short-form-strategy`
   rules and voice it with our local TTS, `Rust/voxcpm-tts` (the same engine
   the voiced-explainer mode uses):
   `voxcpm-tts say narration.txt -o voice.wav --voice <authorized reference.wav>`.
   Disclose the synthetic voice. Set the scene's `duration` to the narration
   length plus about 1 s.
4. **Render.** `claw-motion physics scene.toml --narration voice.wav -o short.mp4`.
   It normalizes to -14 LUFS / -1 dBTP and fails if the result misses.
   Without `--narration`, the impact sounds alone are normalized.
5. **Check.** Read `short.report.json`: frames, impacts heard, loudness. Look
   at frames from the MP4 and listen. If the knocks are too dense, raise
   `physics.min_impact_speed` or lower `sfx.per_window`.
6. **Add captions or a title if needed.** This mode draws no text. Put the
   rendered MP4 under word-timed captions with the canvas or HyperFrames modes,
   or use it as B-roll in `Rust/shortform-video`.

## Physics tuning that matters

- **A bell curve needs drag.** Set `air` so that `gravity / air` is a few
  hundred px/s. Make peg spacing about 2.5-3 ball diameters. Without that,
  balls skip sideways several columns and the bins fill flat. The example uses
  gravity 1960, air 4.5, spacing 36 px and balls of 6-7 px radius.
- **Feeding too fast backs up the funnel.** `every` × `count` sets how long
  the drop takes. Leave a couple of seconds at the end for the last balls to
  land.
- **The seed fixes the run.** Change `seed` for a different but equally
  repeatable outcome.

## Not for

- Text-heavy explainers: use the voiced explainer or HyperFrames.
- Equations and graphs: use [manim.md](manim.md).
- 3D, characters or rigs: out of scope. Blender was judged not needed yet.
