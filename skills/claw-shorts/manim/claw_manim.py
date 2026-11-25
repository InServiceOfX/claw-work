"""Shared helpers for claw-shorts Math mode (Manim Community, vertical 1080x1920).

ShortScene adds two things to a Manim Scene:

1. A safe-area gate. After every `play` and `wait`, each visible text object
   (Text, MarkupText, Tex, MathTex) must sit inside the area that is clear on
   TikTok, Reels and Shorts: y 288-1248 px, x 120-888 px above y 840 and
   x 120-780 px below it. A violation stops the render with the offending text.
2. Timing from the narration. With CLAW_TIMELINE pointing at the
   timeline.json written by `claw-shorts canvas`, beat i ends where narration
   scene i ends. With CLAW_WORDS pointing at a words.json from
   Rust/claw-shorts/scripts/words.py, `word_time("taylor")` returns when that
   word is spoken. Without them, the scene's DEFAULT_BEATS (seconds) are used,
   so a scene renders the same way before narration exists.

Units: vertical.cfg makes the frame 9 x 16 units, 1 unit = 120 px.
"""

from __future__ import annotations

import json
import os
import re
import sys

import manimpango
from manim import MathTex, MarkupText, Scene, SingleStringMathTex, Tex, Text, config

PX = 120.0  # pixels per unit (1080 px / 9 units)


def px_to_x(px: float) -> float:
    return (px - config.pixel_width / 2) / PX


def px_to_y(px: float) -> float:
    return (config.pixel_height / 2 - px) / PX


# The safe area, in Manim units (y up, origin at the frame centre).
SAFE_TOP = px_to_y(288)  # 5.6
SAFE_BOTTOM = px_to_y(1248)  # -2.4
SAFE_LEFT = px_to_x(120)  # -3.5
SAFE_RIGHT_HIGH = px_to_x(888)  # 2.9, above BUTTONS_Y
SAFE_RIGHT_LOW = px_to_x(780)  # 2.0, at or below BUTTONS_Y
BUTTONS_Y = px_to_y(840)  # 1.0
# Where burned-in captions sit (y 1090-1248 px); keep main content above it.
CAPTION_TOP = px_to_y(1090)  # about -1.08
SAFE_CENTER_X = (SAFE_LEFT + SAFE_RIGHT_LOW) / 2  # centre of the narrowest band

TEXT_TYPES = (Text, MarkupText, Tex, MathTex, SingleStringMathTex)
TOLERANCE = 0.02  # units (2.4 px)

# claw-shorts default style (Rust/claw-shorts/src/style.rs).
BG = "#0b0f14"
FG = "#ffffff"
ACCENT = "#4ade80"
MUTED = "#9aa4b2"
PANEL = "#18202b"
WARN = "#fbbf24"
FAIL = "#f87171"
# Liberation Sans is metric-compatible with Arial, so layouts match on Linux.
_INSTALLED_FONTS = set(manimpango.list_fonts())
FONT = next((f for f in ("Arial", "Liberation Sans", "DejaVu Sans") if f in _INSTALLED_FONTS), "Arial")


def fit(mob, max_width: float):
    """Scale `mob` down (never up) so it is at most `max_width` units wide."""
    if mob.width > max_width:
        mob.scale_to_fit_width(max_width)
    return mob


def _visible(mob) -> bool:
    try:
        return mob.get_fill_opacity() > 0.01 or mob.get_stroke_opacity() > 0.01
    except Exception:
        return True


def _label(mob) -> str:
    text = getattr(mob, "text", None) or getattr(mob, "tex_string", None)
    return repr(text) if text else type(mob).__name__


def safe_area_problems(mobjects) -> list[str]:
    """Describe every visible text object outside the safe area."""
    problems = []
    seen = set()
    for top in mobjects:
        for mob in top.get_family():
            if not isinstance(mob, TEXT_TYPES) or id(mob) in seen or not _visible(mob):
                continue
            seen.add(id(mob))
            if len(mob.points) == 0 and not mob.submobjects:
                continue
            left, right = mob.get_left()[0], mob.get_right()[0]
            top_y, bottom = mob.get_top()[1], mob.get_bottom()[1]
            right_limit = SAFE_RIGHT_LOW if bottom < BUTTONS_Y else SAFE_RIGHT_HIGH
            faults = []
            if top_y > SAFE_TOP + TOLERANCE:
                faults.append(f"top {top_y:.2f} > {SAFE_TOP:.2f}")
            if bottom < SAFE_BOTTOM - TOLERANCE:
                faults.append(f"bottom {bottom:.2f} < {SAFE_BOTTOM:.2f}")
            if left < SAFE_LEFT - TOLERANCE:
                faults.append(f"left {left:.2f} < {SAFE_LEFT:.2f}")
            if right > right_limit + TOLERANCE:
                faults.append(f"right {right:.2f} > {right_limit:.2f}")
            if faults:
                problems.append(f"{_label(mob)}: " + ", ".join(faults))
    return problems


def _load_json(env: str):
    path = os.environ.get(env)
    if not path:
        return None
    with open(path, encoding="utf-8") as f:
        return json.load(f)


class ShortScene(Scene):
    """A vertical Short scene with a safe-area gate and narration-driven beats."""

    #: Beat lengths in seconds when no CLAW_TIMELINE is given.
    DEFAULT_BEATS: list[float] = []

    def setup(self):
        super().setup()
        timeline = _load_json("CLAW_TIMELINE")
        if timeline:
            self.beat_ends = [row["end_ms"] / 1000 for row in sorted(timeline, key=lambda r: r["index"])]
        else:
            total, self.beat_ends = 0.0, []
            for length in self.DEFAULT_BEATS:
                total += length
                self.beat_ends.append(total)
        self.words = _load_json("CLAW_WORDS") or []

    # -- timing ---------------------------------------------------------
    @property
    def now(self) -> float:
        return self.renderer.time

    def beat_end(self, index: int) -> float:
        return self.beat_ends[index]

    def word_time(self, word: str, after: float = 0.0, default: float | None = None) -> float:
        """Start time (s) of the first spoken `word` at or after `after`."""
        want = re.sub(r"[^a-z0-9]", "", word.lower())
        for w in self.words:
            start = w["start_ms"] / 1000
            if start >= after and re.sub(r"[^a-z0-9]", "", w["text"].lower()) == want:
                return start
        if default is None:
            raise KeyError(f"word {word!r} not found after {after:.2f}s in CLAW_WORDS")
        return default

    def hold_until(self, t: float) -> None:
        """Wait until scene time t (whole frames). No-op if already past it."""
        frames = round((t - self.now) * config.frame_rate)
        if frames > 0:
            self.wait(frames / config.frame_rate)

    def end_beat(self, index: int) -> None:
        """Hold until narration scene `index` ends; warn if the animations ran past it."""
        end = self.beat_end(index)
        if self.now > end + 1 / config.frame_rate:
            print(
                f"warning: beat {index} animations end at {self.now:.2f}s, after its narration "
                f"({end:.2f}s); shorten run_time or the video will outlast the voice",
                file=sys.stderr,
            )
        self.hold_until(end)

    # -- safe-area gate -------------------------------------------------
    def _check(self) -> None:
        problems = safe_area_problems(self.mobjects)
        if problems:
            raise RuntimeError(
                f"text outside the safe area at t={self.now:.2f}s:\n  " + "\n  ".join(problems)
            )

    def play(self, *args, **kwargs):
        result = super().play(*args, **kwargs)
        self._check()
        return result

    def wait(self, *args, **kwargs):
        result = super().wait(*args, **kwargs)
        self._check()
        return result
