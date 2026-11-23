# claw-shorts Math mode (Manim)

Vertical 1080×1920 math animations rendered headless with
[Manim Community Edition](https://github.com/ManimCommunity/manim) (MIT), used
as a pinned dependency, not vendored or rewritten. Full guide:
[`../skills/claw-shorts/references/manim.md`](../skills/claw-shorts/references/manim.md).

```bash
# Silent video (5 s for the 18 s example on an Apple Silicon Mac)
scripts/render.sh scenes/small_angle.py SmallAngle /tmp/math-short

# Timed to narration from `claw-shorts canvas` (+ optional word cues), then mux
scripts/render.sh scenes/small_angle.py SmallAngle out \
  --timeline prep/timeline.json --words words.json
claw-shorts mux --video out/video.mp4 --audio prep/narration.wav --output short.mp4
```

| File | Purpose |
| --- | --- |
| `pyproject.toml`, `uv.lock` | `manim==0.22.1` and its locked dependencies, Python 3.13 via uv |
| `vertical.cfg` | 1080×1920, 30 fps, 9×16 units (1 unit = 120 px), claw-shorts background |
| `claw_manim.py` | `ShortScene`: safe-area gate on every text object, beats from `timeline.json`, word cues from `words.json`, `fit()` |
| `scenes/small_angle.py` | Example: why sin x ≈ x for small x (4 beats, 18 s) |
| `scripts/render.sh` | Render one scene to `OUT_DIR/video.mp4`, print size, length, sha256 |

Requirements: `uv`, FFmpeg, and LaTeX for `MathTex` (MacTeX or BasicTeX on
macOS, TeX Live on Linux). On macOS arm64 with Python 3.13, ManimPango comes
as a self-contained wheel, but pycairo 1.29 is compiled on first install and
links Homebrew `cairo` (needs `cairo` and `pkgconf`; `brew install cairo
pkgconf` if missing).
