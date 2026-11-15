#!/usr/bin/env python3
"""Script-faithful caption track for a HyperFrames project.

Speech recognition gets the timing right and the spelling wrong ("Waterframes",
"78 Qs", "G sap"). This tool keeps the words of the approved script and takes
only the timing from the transcript: it aligns the two word sequences, gives
every matched script word its recognized time, and spreads unmatched script
words over the gap between their matched neighbours by character count.

Usage:
  python3 captions.py --script script.txt --transcript assets/transcript.json \
      --out compositions/captions.html [--highlight HyperFrames,GSAP] \
      [--max-words 4] [--offset 0.0] [--report captions.json]

The transcript is the `[{text, start, end}]` word list that
`npx hyperframes transcribe` writes. The output is a HyperFrames scene file
(`<template>`, `#root`, one paused timeline). Mount it from index.html on a host
with `data-composition-id="captions"` and `data-track-kind="captions"`.
Standard library only; no network.
"""
from __future__ import annotations

import argparse
import difflib
import html
import json
import re
import sys
from pathlib import Path

SENTENCE_END = re.compile(r"[.!?]$")
NUMBER_WORDS = {
    "zero": "0", "one": "1", "two": "2", "three": "3", "four": "4", "five": "5",
    "six": "6", "seven": "7", "eight": "8", "nine": "9", "ten": "10",
}


def norm(word: str) -> str:
    """Lowercase alphanumerics only; spelled-out digits become digits."""
    w = re.sub(r"[^0-9a-z]+", "", word.lower())
    return NUMBER_WORDS.get(w, w)


def script_words(text: str) -> list[str]:
    # Spoken-only hyphenations such as "Gee-sap" stay one caption word.
    return [w for w in text.split() if norm(w) or w.strip()]


def align(script: list[str], heard: list[dict]) -> list[tuple[float, float, bool]]:
    """Return (start, end, matched) for every script word."""
    a = [norm(w) for w in script]
    b = [norm(w["text"]) for w in heard]
    times: list[tuple[float, float, bool] | None] = [None] * len(script)
    matcher = difflib.SequenceMatcher(a=a, b=b, autojunk=False)
    for block in matcher.get_matching_blocks():
        for k in range(block.size):
            h = heard[block.b + k]
            times[block.a + k] = (float(h["start"]), float(h["end"]), True)
    # A replaced run of the same length (one misheard word for one script
    # word) keeps the heard word's timing even though the text differs.
    for tag, i1, i2, j1, j2 in matcher.get_opcodes():
        if tag == "replace" and i2 - i1 == j2 - j1:
            for k in range(i2 - i1):
                h = heard[j1 + k]
                times[i1 + k] = (float(h["start"]), float(h["end"]), False)
        elif tag == "replace":
            # Spread the heard span over the script span by character count.
            start, end = float(heard[j1]["start"]), float(heard[j2 - 1]["end"])
            _spread(script, times, i1, i2, start, end)
    # Script words with no heard counterpart: fill from neighbours.
    i = 0
    total_end = float(heard[-1]["end"]) if heard else 0.0
    while i < len(script):
        if times[i] is not None:
            i += 1
            continue
        j = i
        while j < len(script) and times[j] is None:
            j += 1
        start = times[i - 1][1] if i > 0 else 0.0
        end = times[j][0] if j < len(script) else total_end
        if end - start < 0.05 * (j - i) and i > 0:
            # No audible gap (the voice ran the words together): share the
            # previous word's span instead of overlapping the next word.
            i -= 1
            start = times[i][0]
        _spread(script, times, i, j, start, max(end, start + 0.05 * (j - i)))
        i = j
    return [t for t in times if t is not None]


def _spread(script, times, i1, i2, start, end):
    # Weight by spoken length: the written word, not its digit form ("three" not "3").
    weights = [max(1, len(re.sub(r"[^0-9A-Za-z]+", "", w))) for w in script[i1:i2]]
    total = float(sum(weights))
    t = start
    for k, wt in enumerate(weights):
        d = (end - start) * wt / total
        times[i1 + k] = (t, t + d, False)
        t += d


def group(words, times, max_words: int, pause: float = 0.3):
    """Break at a sentence end, at the word limit, at a comma once a group has
    three words, or at a pause once it has two. TTS leaves ~0.15 s gaps between
    most words, so a lone word only stands alone when it ends a sentence."""
    groups, cur = [], []
    for idx, (w, (s, e, _)) in enumerate(zip(words, times)):
        if cur:
            prev_end = times[idx - 1][1]
            prev = words[idx - 1]
            # Never strand a sentence's last word on its own.
            if SENTENCE_END.search(w) and len(cur) <= max_words:
                pass
            elif (len(cur) >= max_words
                    or (len(cur) >= 2 and s - prev_end >= pause)
                    or (len(cur) >= 3 and prev.endswith(","))):
                groups.append(cur)
                cur = []
        cur.append(idx)
        if SENTENCE_END.search(w):
            groups.append(cur)
            cur = []
    if cur:
        groups.append(cur)
    return groups


def render(words, times, groups, highlight: set[str], offset: float, end_of_audio: float):
    out_groups, js = [], []
    starts = [max(0.12, times[g[0]][0] + offset) for g in groups]
    for n, g in enumerate(groups):
        start = starts[n]
        nxt = starts[n + 1] if n + 1 < len(groups) else end_of_audio + offset + 0.6
        # Hold through short gaps, never past the next group's start.
        end = min(nxt, max(times[g[-1]][1] + offset + 0.25, start + 0.6))
        spans = []
        for i in g:
            word = words[i]
            cls = "cap-word hot" if norm(word) in highlight else "cap-word"
            spans.append(f'<span class="{cls}">{html.escape(word)}</span>')
        gid = f"cap-{n:02d}"
        out_groups.append(f'      <div id="{gid}" class="cap-group">{" ".join(spans)}</div>')
        js.append(
            f'    tl.fromTo("#{gid}", {{ opacity: 0, y: 18 }}, '
            f'{{ opacity: 1, y: 0, duration: 0.14, ease: "power3.out" }}, {start:.3f});\n'
            f'    tl.to("#{gid}", {{ opacity: 0, duration: 0.1, ease: "power2.in" }}, {end - 0.1:.3f});\n'
            f'    tl.set("#{gid}", {{ visibility: "hidden" }}, {end:.3f});'
        )
    return out_groups, js


TEMPLATE = """<template>
  <style>
    #root {{ position: absolute; inset: 0; pointer-events: none; }}
    .cap-group {{
      position: absolute; left: 120px; right: 300px; top: 1090px;
      text-align: center; overflow: visible;
      font-family: var(--font-display, "Archivo Black"), sans-serif;
      font-size: 58px; line-height: 1.12; color: var(--color-ink, #ffffff);
      text-shadow: 0 4px 0 rgba(0, 0, 0, 0.55);
      opacity: 0;
    }}
    .cap-word.hot {{ color: var(--color-accent, #4ade80); }}
  </style>
  <div id="root" data-composition-id="captions" data-width="1080" data-height="1920">
{groups}
  </div>
  <script>
    const tl = gsap.timeline({{ paused: true }});
{js}
    window.__timelines["captions"] = tl;
  </script>
</template>
"""


def main(argv=None) -> int:
    p = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    p.add_argument("--script", required=True, type=Path)
    p.add_argument("--transcript", required=True, type=Path)
    p.add_argument("--out", required=True, type=Path)
    p.add_argument("--highlight", default="", help="comma-separated words in the accent colour")
    p.add_argument("--max-words", type=int, default=4)
    p.add_argument("--offset", type=float, default=0.0, help="narration start in the film, seconds")
    p.add_argument("--report", type=Path, help="write the aligned words as JSON")
    a = p.parse_args(argv)

    words = script_words(a.script.read_text(encoding="utf-8"))
    heard = json.loads(a.transcript.read_text(encoding="utf-8"))
    if not words or not heard:
        print("empty script or transcript", file=sys.stderr)
        return 2
    times = align(words, heard)
    matched = sum(1 for t in times if t[2])
    groups = group(words, times, a.max_words)
    highlight = {norm(h) for h in a.highlight.split(",") if h.strip()}
    g_html, js = render(words, times, groups, highlight, a.offset, float(heard[-1]["end"]))
    a.out.parent.mkdir(parents=True, exist_ok=True)
    a.out.write_text(TEMPLATE.format(groups="\n".join(g_html), js="\n".join(js)), encoding="utf-8")
    if a.report:
        a.report.write_text(json.dumps([
            {"text": w, "start": round(s + a.offset, 3), "end": round(e + a.offset, 3), "matched": m}
            for w, (s, e, m) in zip(words, times)], indent=1), encoding="utf-8")
    print(f"{len(words)} script words, {matched} matched to the transcript, "
          f"{len(groups)} caption groups -> {a.out}")
    if matched < 0.7 * len(words):
        print("warning: under 70% of the script matched; check the narration against the script",
              file=sys.stderr)
    return 0


if __name__ == "__main__":
    sys.exit(main())
