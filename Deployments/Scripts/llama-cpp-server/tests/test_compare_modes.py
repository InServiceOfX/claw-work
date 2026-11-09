import importlib.util
import io
import json
import pathlib
import tempfile
import unittest
from unittest.mock import patch

spec = importlib.util.spec_from_file_location("compare_modes", pathlib.Path(__file__).parents[1] / "compare_modes.py")
compare = importlib.util.module_from_spec(spec)
spec.loader.exec_module(compare)


def event(value):
    return b"data: " + json.dumps(value).encode() + b"\n\n"


class StreamingMeasurementTests(unittest.TestCase):
    def measure(self, data):
        with tempfile.TemporaryDirectory() as directory:
            path = pathlib.Path(directory) / "response.sse"
            with patch.object(compare.urllib.request, "urlopen", return_value=io.BytesIO(data)):
                result = compare.stream_request("http://127.0.0.1:8080", {}, path)
            return result, path.with_suffix(".answer.txt").read_text()

    def test_reasoning_and_answer_are_separate_and_usage_is_authoritative(self):
        data = event({"choices": [{"delta": {"reasoning_content": "Check it."}}]})
        data += event({"choices": [{"delta": {"content": "42"}, "finish_reason": "stop"}]})
        data += event({"choices": [], "usage": {"prompt_tokens": 9, "completion_tokens": 7}})
        data += b"data: [DONE]\n\n"
        result, answer = self.measure(data)
        self.assertEqual(answer, "42")
        self.assertEqual(result["completion_tokens"], 7)
        self.assertEqual(result["thinking_characters"], len("Check it."))
        self.assertEqual(result["answer_characters"], 2)
        self.assertLessEqual(result["first_text_seconds"], result["first_answer_seconds"])
        self.assertAlmostEqual(result["tokens_per_second_end_to_end"] * result["seconds"], 7)

    def test_reasoning_only_length_limit_is_not_a_completed_answer(self):
        data = event({"choices": [{"delta": {"reasoning_content": "Still thinking"}, "finish_reason": "length"}],
                      "usage": {"prompt_tokens": 9, "completion_tokens": 3}}) + b"data: [DONE]\n\n"
        result, answer = self.measure(data)
        self.assertEqual(answer, "")
        self.assertIsNone(result["first_answer_seconds"])
        self.assertEqual(result["finish_reason"], "length")

    def test_missing_done_or_usage_is_rejected(self):
        for data in [event({"usage": {"prompt_tokens": 9, "completion_tokens": 3}}), b"data: [DONE]\n\n"]:
            with self.subTest(data=data), self.assertRaisesRegex(RuntimeError, "Incomplete SSE"):
                self.measure(data)

    def test_server_error_is_rejected(self):
        with self.assertRaisesRegex(RuntimeError, "synthetic failure"):
            self.measure(event({"error": {"message": "synthetic failure"}}))


if __name__ == "__main__":
    unittest.main()
