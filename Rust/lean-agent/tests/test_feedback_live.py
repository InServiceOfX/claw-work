"""Opt-in acceptance of the installed feedback launcher, not a model provider."""
import json
import os
from pathlib import Path
import queue
import subprocess
import sys
import tempfile
import threading
import time
import unittest

ROOT = Path(__file__).resolve().parents[1]
TOOL = ROOT / "scripts/lean-container.py"


@unittest.skipUnless(os.environ.get("LEAN_AGENT_LIVE") == "1", "requires explicitly built local image")
class FeedbackLive(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.work = Path(self.temp.name).resolve()
        (self.work / "Good.lean").write_text("import Mathlib.Data.Nat.Basic\nexample (n : Nat) : n + 0 = n := by omega\n")
        (self.work / "Bad.lean").write_text("example : (1 : Nat) = 2 := by rfl\n")
        (self.work / "Program.lean").write_text('def main : IO Unit := IO.println "feedback-program-ok"\n')
        (self.work / "Wait.lean").write_text("#eval IO.sleep 30000\n")

    def tearDown(self):
        self.temp.cleanup()

    def command(self, *args):
        return [sys.executable, str(TOOL), "--input-dir", str(self.work), *args]

    def run_tool(self, *args):
        return subprocess.run(self.command(*args), capture_output=True, text=True, timeout=45)

    def test_version(self):
        result = self.run_tool("version")
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn("4.35.0", result.stdout)

    def test_good_mathlib_feedback(self):
        result = self.run_tool("check", "Good.lean")
        self.assertEqual(result.returncode, 0, result.stderr + result.stdout)

    def test_false_theorem_is_nonzero(self):
        result = self.run_tool("check", "Bad.lean")
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("error", result.stdout)

    def test_requested_program_execution(self):
        result = self.run_tool("run", "Program.lean")
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(result.stdout.strip(), "feedback-program-ok")

    def test_internal_deadline_never_passes(self):
        result = self.run_tool("--timeout-seconds", "1", "check", "Wait.lean")
        self.assertEqual(result.returncode, 124, result.stderr + result.stdout)

    def test_lsp_framing_is_not_contaminated(self):
        child = subprocess.Popen(self.command("--timeout-seconds", "30", "server"),
                                 stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
        received = queue.Queue()

        def read_one():
            try:
                headers = {}
                while True:
                    line = child.stdout.readline()
                    if line == b"\r\n":
                        break
                    if not line:
                        raise RuntimeError("LSP stream ended before headers")
                    name, value = line.decode().strip().split(":", 1)
                    headers[name.lower()] = value.strip()
                received.put(json.loads(child.stdout.read(int(headers["content-length"]))))
            except Exception as error:
                received.put(error)

        try:
            threading.Thread(target=read_one, daemon=True).start()
            body = json.dumps({"jsonrpc": "2.0", "id": 1, "method": "initialize",
                               "params": {"processId": None, "rootUri": "file:///input",
                                          "capabilities": {}}}).encode()
            child.stdin.write(f"Content-Length: {len(body)}\r\n\r\n".encode() + body)
            child.stdin.flush()
            response = received.get(timeout=25)
            self.assertIsInstance(response, dict, str(response))
            self.assertEqual(response.get("id"), 1)
            self.assertIn("capabilities", response["result"])
        finally:
            child.terminate()  # launcher handles TERM and removes its own container
            try:
                child.wait(timeout=15)
            except subprocess.TimeoutExpired:
                child.kill()
                child.wait()
            for stream in (child.stdin, child.stdout, child.stderr):
                stream.close()


if __name__ == "__main__":
    unittest.main()
