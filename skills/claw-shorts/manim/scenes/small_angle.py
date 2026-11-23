"""Example Math-mode Short: why sin x is about x for small x (18 s).

Beats (DEFAULT_BEATS, replaced by CLAW_TIMELINE when narration exists):
  0 hook       "sin x ≈ x. Why is that allowed?"
  1 graph      the two curves lie on top of each other near zero
  2 series     Taylor series; the first thing dropped is x^3/6
  3 numbers    at x = 0.1 the error is 0.00017; halve x and it drops 8 times

Suggested narration, one spec scene per beat:
  0 "Physicists write sine of x equals x all the time. Why is that allowed?"
  1 "Plot both. Near zero, the curves sit on top of each other."
  2 "The Taylor series says why. Keep only x, and the first term you drop is x cubed over six."
  3 "At x equals zero point one, that is under two ten-thousandths. Halve x, and the error drops eight times."
"""

import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

import numpy as np  # noqa: E402
from manim import (  # noqa: E402
    DOWN,
    LEFT,
    UP,
    Axes,
    Create,
    FadeIn,
    FadeOut,
    Indicate,
    MathTex,
    Rectangle,
    SurroundingRectangle,
    Text,
    VGroup,
    Write,
)

from claw_manim import (  # noqa: E402
    ACCENT,
    FAIL,
    FG,
    FONT,
    MUTED,
    PANEL,
    SAFE_CENTER_X,
    SAFE_LEFT,
    SAFE_RIGHT_HIGH,
    SAFE_RIGHT_LOW,
    WARN,
    ShortScene,
    fit,
)

HIGH_CENTER_X = (SAFE_LEFT + SAFE_RIGHT_HIGH) / 2  # -0.3: centre above the buttons
HIGH_WIDTH = SAFE_RIGHT_HIGH - SAFE_LEFT - 0.2  # 6.2 units usable above y = 1
LOW_WIDTH = SAFE_RIGHT_LOW - SAFE_LEFT - 0.2  # 5.3 units usable below y = 1


class SmallAngle(ShortScene):
    DEFAULT_BEATS = [3.5, 4.5, 5.5, 4.5]

    def construct(self):
        # Beat 0: hook. The claim stays at the top for the whole video.
        claim = MathTex(r"\sin x", r"\approx", r"x", font_size=150, color=FG)
        fit(claim, HIGH_WIDTH).move_to([HIGH_CENTER_X, 4.75, 0])
        ask = Text("Why is that allowed?", font=FONT, weight="BOLD", font_size=40, color=MUTED)
        fit(ask, HIGH_WIDTH).next_to(claim, DOWN, buff=0.4).set_x(HIGH_CENTER_X)
        self.play(Write(claim), run_time=1.2)
        self.play(FadeIn(ask, shift=UP * 0.2), run_time=0.6)
        self.end_beat(0)

        # Beat 1: both curves; they coincide near zero.
        axes = Axes(
            x_range=[-3.2, 3.2, 1],
            y_range=[-1.4, 1.4, 1],
            x_length=6.2,
            y_length=2.6,
            tips=False,
            axis_config={"color": MUTED, "stroke_width": 3},
        ).move_to([HIGH_CENTER_X, 2.55, 0])
        sine = axes.plot(np.sin, x_range=[-3.2, 3.2], color=ACCENT, stroke_width=7)
        line = axes.plot(lambda x: x, x_range=[-1.4, 1.4], color=WARN, stroke_width=7)
        sine_label = MathTex(r"\sin x", color=ACCENT, font_size=72)
        sine_label.move_to(axes.c2p(2.3, -0.75))
        line_label = MathTex(r"y = x", color=WARN, font_size=72)
        line_label.move_to(axes.c2p(-2.1, 0.75))
        near = Rectangle(
            width=axes.x_axis.unit_size * 1.2,
            height=axes.y_axis.unit_size * 1.6,
            stroke_width=0,
            fill_color=PANEL,
            fill_opacity=1,
        ).move_to(axes.c2p(0, 0))
        self.play(FadeOut(ask), Create(axes), run_time=0.8)
        self.play(Create(sine), Create(line), FadeIn(sine_label), FadeIn(line_label), run_time=1.6)
        self.bring_to_back(near)
        self.play(FadeIn(near), Indicate(line, color=FG, scale_factor=1.0), run_time=1.0)
        self.end_beat(1)

        # Beat 2: the Taylor series, and the first term we drop.
        series = MathTex(r"\sin x", r"=", r"x", r"-\frac{x^3}{6}", r"+\cdots", font_size=80, color=FG)
        series[4].set_color(MUTED)
        fit(series, LOW_WIDTH).set_x(SAFE_CENTER_X)
        series.shift(UP * (1.0 - series.get_top()[1]))  # just under the graph
        self.play(Write(series), run_time=1.6)
        # With CLAW_WORDS, box the term as the voice says "cubed"; otherwise now.
        self.hold_until(self.word_time("cubed", after=self.now, default=self.now))
        error = series[3]
        box = SurroundingRectangle(error, color=FAIL, buff=0.12, stroke_width=6)
        dropped = Text("first term dropped", font=FONT, weight="BOLD", font_size=36, color=FAIL)
        fit(dropped, LOW_WIDTH).next_to(series, DOWN, buff=0.35).set_x(SAFE_CENTER_X)
        self.play(Create(box), FadeIn(dropped), error.animate.set_color(FAIL), run_time=1.0)
        self.end_beat(2)

        # Beat 3: put numbers on it.
        graph = VGroup(axes, sine, line, sine_label, line_label, near)
        self.play(FadeOut(graph), FadeOut(box), FadeOut(dropped), run_time=0.6)
        grow = min(HIGH_WIDTH / series.width, 1.4)
        self.play(series.animate.scale(grow).move_to([HIGH_CENTER_X, 3.05, 0]), run_time=0.7)
        at = MathTex(r"x = 0.1", font_size=96, color=MUTED)
        diff = MathTex(r"x - \sin x", r"\approx 0.00017", font_size=96, color=FG)
        diff[1].set_color(ACCENT)
        rule = MathTex(r"x \to x/2", r"\ \Rightarrow\ ", r"\text{error} \div 8", font_size=88, color=FG)
        rule[2].set_color(ACCENT)
        fit(at, LOW_WIDTH).move_to([SAFE_CENTER_X, 1.45, 0])
        fit(diff, LOW_WIDTH).move_to([SAFE_CENTER_X, 0.45, 0])
        fit(rule, LOW_WIDTH).move_to([SAFE_CENTER_X, -0.65, 0])
        self.play(FadeIn(at), Write(diff), run_time=1.2)
        self.play(FadeIn(rule, shift=UP * 0.2), run_time=0.8)
        self.end_beat(3)
