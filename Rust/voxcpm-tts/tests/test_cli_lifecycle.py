"""Offline process acceptance tests. Run after cargo build --bin voxcpm-tts.

No model, Metal device, provider, network, or private voice fixtures are used.
"""
import json
import os
from pathlib import Path
import signal
import subprocess
import sys
import tempfile
import time
import unittest
import wave

ROOT = Path(__file__).resolve().parents[1]
BINARY = Path(os.environ.get("VOXCPM_TTS_TEST_BINARY", ROOT / "target/debug/voxcpm-tts"))
MOCK = r'''
import json, math, os, pathlib, struct, sys, time, wave
log = pathlib.Path(os.environ["MOCK_LOG"])
mode = os.environ.get("MOCK_MODE", "success")
previous = None
count = 0
def record(event, **fields):
    with log.open("a") as out:
        out.write(json.dumps(dict(event=event, pid=os.getpid(), **fields))+"\n")
def emit(value):
    print(json.dumps(value),flush=True)
record("started")
if mode=="startup_cancel": time.sleep(60)
emit(dict(event="ready"))
for line in sys.stdin:
    req=json.loads(line); op=req["op"]
    record(op, previous_exists=bool(previous and previous.exists()), out_wav=req.get("out_wav"))
    if op=="load" and mode=="load_cancel": time.sleep(60)
    if op=="shutdown":
        if mode=="hang_unload": time.sleep(60)
        break
    if op=="unload" and mode=="hang_unload": time.sleep(60)
    if op=="synthesize":
        count+=1
        previous=pathlib.Path(req["out_wav"])
        previous.write_bytes(b"partial output")
        if mode=="crash": os._exit(9)
        if mode=="cancel":
            emit(dict(event="progress",audio_seconds=0.1))
            time.sleep(60)
        if mode=="error" or (mode=="error_once" and count==1):
            emit(dict(id=req["id"],ok=False,error="synthetic inference failure"))
            continue
        seconds=max(0.6,sum(c.isalnum() or c.isspace() for c in req["text"])/15)
        with wave.open(str(previous),"wb") as out:
            out.setnchannels(1);out.setsampwidth(2);out.setframerate(48000)
            out.writeframes(b"".join(struct.pack("<h",int(6000*math.sin(2*math.pi*220*i/48000))) for i in range(int(48000*seconds))))
    emit(dict(id=req["id"],ok=True))
record("exited")
'''

class CliLifecycle(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory(prefix="voxcpm acceptance ")
        self.root = Path(self.tmp.name)
        self.scratch = self.root / "temporary files"
        self.scratch.mkdir()
        self.log = self.root / "worker events.jsonl"
        self.worker = self.root / "mock worker.py"
        self.worker.write_text(MOCK)
        self.script = self.root / "script with spaces.txt"
        self.script.write_text("A clear first sentence. A second small sentence. Here is the final sentence.")
        self.output = self.root / "spoken script.wav"
        self.env = dict(os.environ, VOXCPM_TTS_PYTHON=sys.executable,
                        VOXCPM_TTS_WORKER=str(self.worker), VOXCPM_TTS_HOME=str(self.root / "app support"),
                        TMPDIR=str(self.scratch), MOCK_LOG=str(self.log))
        self.processes = []

    def events(self):
        return [json.loads(line) for line in self.log.read_text().splitlines()] if self.log.exists() else []

    def tearDown(self):
        for proc in self.processes:
            if proc.poll() is None:
                os.killpg(proc.pid, signal.SIGKILL)
                proc.wait()
        for event in self.events():
            try:
                os.kill(event["pid"], signal.SIGKILL)
            except ProcessLookupError:
                pass
        self.tmp.cleanup()

    def launch(self, mode="success", session=False):
        cmd = [str(BINARY), "session" if session else "say"]
        if not session:
            cmd += [str(self.script), "--output", str(self.output)]
        cmd += ["--engine", "mlx-python", "--max-chars", "26", "--retries", "0"]
        proc = subprocess.Popen(cmd, env=dict(self.env, MOCK_MODE=mode), stdin=subprocess.PIPE,
                                stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True, start_new_session=True)
        self.processes.append(proc)
        return proc

    def finish(self, proc, text=None, timeout=8):
        stdout, stderr = proc.communicate(text, timeout=timeout)
        return proc.returncode, stdout, stderr

    def assert_worker_gone_and_parts_removed(self):
        started = [event for event in self.events() if event["event"] == "started"]
        self.assertTrue(started, "mock worker must actually run")
        for event in started:
            with self.assertRaises(ProcessLookupError):
                os.kill(event["pid"], 0)
        self.assertEqual(list(self.scratch.iterdir()), [], "all intermediate directories must be removed")
        for event in self.events():
            if event.get("out_wav"):
                self.assertFalse(Path(event["out_wav"]).exists(), "worker partial WAV remains")

    def test_file_path_with_spaces_chunks_stitches_and_releases_child(self):
        code, _, stderr = self.finish(self.launch())
        self.assertEqual(code, 0, stderr)
        chunks = [e for e in self.events() if e["event"] == "synthesize"]
        self.assertGreaterEqual(len(chunks), 3)
        with wave.open(str(self.output)) as out:
            self.assertEqual((out.getframerate(), out.getnchannels(), out.getsampwidth()), (48000,1,2))
            self.assertGreater(out.getnframes(), 48000)
        self.assert_worker_gone_and_parts_removed()

    def test_existing_output_is_preserved_without_loading_worker(self):
        self.output.write_bytes(b"preserve these bytes")
        code, _, stderr = self.finish(self.launch())
        self.assertNotEqual(code,0)
        self.assertIn("already exists",stderr)
        self.assertEqual(self.output.read_bytes(),b"preserve these bytes")
        self.assertEqual(self.events(),[])

    def test_inference_error_reaps_worker_and_removes_partial_outputs(self):
        code, _, stderr = self.finish(self.launch("error"))
        self.assertNotEqual(code,0)
        self.assertIn("synthetic inference failure",stderr)
        self.assertFalse(self.output.exists())
        self.assert_worker_gone_and_parts_removed()

    def test_worker_crash_reaps_worker_and_removes_partial_outputs(self):
        code, _, _ = self.finish(self.launch("crash"))
        self.assertNotEqual(code,0)
        self.assertFalse(self.output.exists())
        self.assert_worker_gone_and_parts_removed()

    def cancel_during(self, mode, event):
        proc = self.launch(mode)
        deadline = time.monotonic()+5
        while not any(e["event"]==event for e in self.events()):
            self.assertLess(time.monotonic(),deadline,"worker did not receive synthesis request")
            time.sleep(0.02)
        proc.send_signal(signal.SIGINT)
        code, _, stderr = self.finish(proc)
        self.assertEqual(code,130,stderr)
        self.assertFalse(self.output.exists())
        self.assert_worker_gone_and_parts_removed()

    def test_ctrl_c_cancels_inference_and_reaps_worker(self):
        self.cancel_during("cancel", "synthesize")

    def test_ctrl_c_cancels_worker_startup_and_reaps_worker(self):
        self.cancel_during("startup_cancel", "started")

    def test_ctrl_c_cancels_model_loading_and_reaps_worker(self):
        self.cancel_during("load_cancel", "load")

    def test_unresponsive_shutdown_has_bounded_cleanup(self):
        started = time.monotonic()
        code, _, stderr = self.finish(self.launch("hang_unload"),timeout=7)
        self.assertEqual(code,0,stderr)
        self.assertLess(time.monotonic()-started,6)
        self.assert_worker_gone_and_parts_removed()

    def test_failed_request_cleanup_precedes_next_session_job(self):
        code, _, stderr = self.finish(self.launch("error_once",session=True),f"{self.script}\n{self.script}\n:quit\n")
        self.assertEqual(code,0,stderr)
        requests = [e for e in self.events() if e["event"]=="synthesize"]
        self.assertGreaterEqual(len(requests),2)
        self.assertFalse(any(r["previous_exists"] for r in requests[1:]),"failed output survives into the next request")
        self.assert_worker_gone_and_parts_removed()

if __name__ == "__main__":
    if not BINARY.is_file():
        raise SystemExit("Build first: cargo build --bin voxcpm-tts")
    unittest.main()
