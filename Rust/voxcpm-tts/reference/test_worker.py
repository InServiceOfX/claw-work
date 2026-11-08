"""Offline protocol and generation-boundary regressions; no model/GPU required.

Run: reference/.venv/bin/python -m unittest discover -s reference -p test_worker.py -v
"""
from __future__ import annotations

import importlib.util
import json
import io
from pathlib import Path
import subprocess
import sys
import tempfile
import types
import unittest
from unittest.mock import patch

import numpy as np

HERE = Path(__file__).resolve().parent
spec = importlib.util.spec_from_file_location("voxcpm_worker", HERE / "voxcpm_worker.py")
worker = importlib.util.module_from_spec(spec)
spec.loader.exec_module(worker)


class FakeModel:
    sample_rate = 48000

    def __init__(self, patches=5):
        self.patches = patches
        self.feat_decoder = types.SimpleNamespace(sample=lambda: None)
        self.calls = []

    def _tokenize(self, text):
        return list(range(len(text.split())))

    def generate(self, **kwargs):
        self.calls.append(kwargs)
        count = min(self.patches, kwargs["max_tokens"])
        for _ in range(count):
            self.feat_decoder.sample()
        yield types.SimpleNamespace(audio=np.zeros(count * 7680 + 8, dtype=np.float32))


class GenerationTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.mx = types.ModuleType("mlx.core")
        self.mx.random = types.SimpleNamespace(seed=lambda seed: None)
        self.mx.reset_peak_memory = lambda: None
        self.mx.get_peak_memory = lambda: 0
        self.mx.get_cache_memory = lambda: 0
        self.mx.clear_cache = lambda: None
        self.modules = patch.dict(sys.modules, {"mlx": types.ModuleType("mlx"), "mlx.core": self.mx})
        self.modules.start()
        self.addCleanup(self.modules.stop)
        self.engine = worker.Engine()
        self.engine.model = FakeModel(patches=4)
        self.request = {"text": "A short synthetic test.", "out_wav": str(Path(self.temp.name) / "out.wav"),
                        "retry_badcase": False, "progress": False}

    def test_seconds_cap_reaches_model(self):
        self.engine.synthesize({**self.request, "max_audio_seconds": 0.8}, "test")
        self.assertEqual(self.engine.model.calls[0]["max_tokens"], 5)

    def test_seconds_cap_never_overruns_wav(self):
        result = self.engine.synthesize({**self.request, "max_audio_seconds": 0.8}, "test")
        self.assertLessEqual(result["audio_seconds"], 0.8)
        self.assertEqual(result["audio_seconds"], 0.64)

    def test_invalid_caps_are_rejected(self):
        for cap in (0, -1, float("nan"), float("inf")):
            with self.subTest(cap=cap), self.assertRaises(ValueError):
                self.engine.synthesize({**self.request, "max_audio_seconds": cap}, "test")

    def test_exhausted_runaway_is_not_success(self):
        self.engine.model = FakeModel(patches=1000)
        with self.assertRaisesRegex(RuntimeError, "cap|runaway|badcase"):
            self.engine.synthesize({**self.request, "max_tokens": 5, "retry_badcase": True,
                                    "retry_badcase_max_times": 0}, "test")
        self.assertFalse(Path(self.request["out_wav"]).exists())

    def test_cap_is_error_even_without_internal_retry(self):
        self.engine.model = FakeModel(patches=1000)
        with self.assertRaisesRegex(RuntimeError, "cap|runaway|badcase"):
            self.engine.synthesize({**self.request, "max_tokens": 5}, "test")

    def test_progress_counts_completed_patches(self):
        completed = []
        self.engine.model = FakeModel(patches=26)
        self.engine.model.feat_decoder.sample = lambda: completed.append(True)
        observations = []
        with patch.object(worker, "emit", side_effect=lambda event: observations.append((event, len(completed)))):
            self.engine.synthesize({**self.request, "text": "word " * 30, "progress": True}, "test")
        self.assertEqual(len(observations), 1)
        self.assertEqual(observations[0][0]["patches"], 25)
        self.assertEqual(observations[0][1], 25)

    def test_failure_restores_sampler(self):
        orig = self.engine.model.feat_decoder.sample
        with patch.object(self.engine.model, "generate", side_effect=RuntimeError("mock failed")):
            with self.assertRaisesRegex(RuntimeError, "mock failed"):
                self.engine.synthesize(self.request, "test")
        self.assertIs(self.engine.model.feat_decoder.sample, orig)

    def test_bad_generation_protocol_is_typed(self):
        request = json.dumps({"id": "bad", "op": "synthesize"}) + "\n"
        events = []
        with patch.object(worker.sys, "stdin", io.StringIO(request)), \
             patch.object(worker, "emit", side_effect=events.append), \
             patch.object(worker.Engine, "synthesize", side_effect=worker.BadGenerationError("cap")), \
             patch.object(worker, "log"):
            self.assertEqual(worker.serve(), 0)
        self.assertEqual(events[-1]["code"], "bad_generation")
        self.assertFalse(events[-1]["ok"])

    def test_non_finite_audio_is_not_written(self):
        def generate(**kwargs):
            self.engine.model.feat_decoder.sample()
            yield types.SimpleNamespace(audio=np.full(7680, np.nan))
        with patch.object(self.engine.model, "generate", side_effect=generate):
            with self.assertRaisesRegex(worker.BadGenerationError, "non-finite"):
                self.engine.synthesize(self.request, "test")
        self.assertFalse(Path(self.request["out_wav"]).exists())

    def test_float_wav_and_scratch_cleanup(self):
        result = self.engine.synthesize(self.request, "test")
        out = Path(self.request["out_wav"])
        self.assertTrue(out.read_bytes().startswith(b"RIFF"))
        self.assertEqual(result["sample_rate"], 48000)
        self.assertEqual(result["audio_seconds"], 0.64)
        self.assertFalse(Path(str(out) + ".part").exists())


class ProtocolTests(unittest.TestCase):
    def test_invalid_requests_recover_then_shutdown(self):
        requests = ["not json", "[]", json.dumps({"id": "bad", "op": "unknown"}),
                    json.dumps({"id": "ping", "op": "ping"}),
                    json.dumps({"id": "bye", "op": "shutdown"})]
        result = subprocess.run([sys.executable, str(HERE / "voxcpm_worker.py")],
                                input="\n".join(requests) + "\n", text=True,
                                capture_output=True, timeout=15)
        self.assertEqual(result.returncode, 0, result.stderr)
        lines = [json.loads(line) for line in result.stdout.splitlines()]
        self.assertEqual(lines[0]["event"], "ready")
        self.assertEqual([line.get("ok") for line in lines[1:]], [False, False, False, True, True])
        self.assertFalse(lines[-2]["loaded"])
        self.assertEqual(lines[-1]["id"], "bye")

    def test_eof_exits_cleanly(self):
        result = subprocess.run([sys.executable, str(HERE / "voxcpm_worker.py")],
                                input="", text=True, capture_output=True, timeout=15)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(len(result.stdout.splitlines()), 1)


if __name__ == "__main__":
    unittest.main()
