"""Tests for hyperframes/scripts/captions.py (script-faithful caption timing)."""
import importlib.util
import json
import tempfile
import unittest
from pathlib import Path

HERE = Path(__file__).resolve().parent
SPEC = importlib.util.spec_from_file_location("captions", HERE.parent / "hyperframes" / "scripts" / "captions.py")
captions = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(captions)


def heard(*items):
    return [{"text": t, "start": s, "end": e} for t, s, e in items]


class AlignTests(unittest.TestCase):
    def test_exact_match_takes_heard_times(self):
        words = ["Here", "are", "three."]
        t = captions.align(words, heard(("Here", 0.0, 0.3), ("are", 0.3, 0.5), ("three.", 0.5, 0.9)))
        self.assertEqual([(s, e) for s, e, _ in t], [(0.0, 0.3), (0.3, 0.5), (0.5, 0.9)])
        self.assertTrue(all(m for _, _, m in t))

    def test_misheard_word_keeps_script_text_and_heard_time(self):
        # ASR wrote "Waterframes" for "HyperFrames": one-for-one replacement.
        words = ["One:", "HyperFrames.", "You"]
        t = captions.align(words, heard(("One", 0.0, 0.2), ("Waterframes,", 0.3, 1.1), ("you", 1.1, 1.3)))
        self.assertEqual(t[1][:2], (0.3, 1.1))
        self.assertFalse(t[1][2])

    def test_spelled_number_matches_digits(self):
        t = captions.align(["three", "tools"], heard(("3", 1.0, 1.4), ("tools", 1.4, 1.9)))
        self.assertTrue(t[0][2])

    def test_dropped_word_shares_previous_span_without_overlap(self):
        # The voice ran "Three free" together; ASR heard only "Three".
        words = ["Three:", "free", "sound"]
        t = captions.align(words, heard(("Three", 15.36, 15.84), ("sound", 15.84, 16.24)))
        starts = [s for s, _, _ in t]
        self.assertEqual(starts, sorted(starts))
        for (s1, e1, _), (s2, _, _) in zip(t, t[1:]):
            self.assertLessEqual(e1, s2 + 1e-9)
        self.assertAlmostEqual(t[0][0], 15.36)
        self.assertAlmostEqual(t[1][1], 15.84)


class GroupTests(unittest.TestCase):
    def test_sentence_end_closes_group_and_last_word_is_never_alone(self):
        words = "Links are in the caption. Save this".split()
        times = [(i * 0.3, i * 0.3 + 0.25, True) for i in range(len(words))]
        groups = captions.group(words, times, max_words=4)
        texts = [" ".join(words[i] for i in g) for g in groups]
        self.assertEqual(texts[0], "Links are in the caption.")
        self.assertEqual(texts[1], "Save this")

    def test_tts_size_gaps_do_not_split_a_group(self):
        words = ["and", "it", "renders", "video"]
        times = [(0.0, 0.2, True), (0.36, 0.5, True), (0.66, 1.0, True), (1.16, 1.4, True)]
        self.assertEqual(len(captions.group(words, times, max_words=4)), 1)


class CliTests(unittest.TestCase):
    def test_writes_a_valid_scene_file(self):
        with tempfile.TemporaryDirectory() as d:
            d = Path(d)
            (d / "script.txt").write_text("This short used zero paid tools.\n")
            (d / "t.json").write_text(json.dumps(heard(
                ("This", 0.0, 0.2), ("short", 0.2, 0.5), ("used", 0.5, 0.7),
                ("zero", 0.7, 1.0), ("paid", 1.0, 1.3), ("tools.", 1.3, 1.8))))
            out = d / "captions.html"
            rc = captions.main(["--script", str(d / "script.txt"), "--transcript", str(d / "t.json"),
                                "--out", str(out), "--highlight", "zero"])
            self.assertEqual(rc, 0)
            html = out.read_text()
            self.assertTrue(html.startswith("<template>"))
            self.assertIn('data-composition-id="captions"', html)
            self.assertIn('window.__timelines["captions"] = tl;', html)
            self.assertIn('<span class="cap-word hot">zero</span>', html)
            # Every group is hard-killed after its exit.
            self.assertEqual(html.count('visibility: "hidden"'), html.count('class="cap-group"'))
            # No caption starts at exactly 0 s (it would look pre-existing).
            self.assertNotIn("}, 0.000);", html)


if __name__ == "__main__":
    unittest.main()
