//! `mlx-python` engine: supervises `reference/voxcpm_worker.py` (mlx-audio) as
//! a child process speaking JSON Lines over stdin/stdout.
//!
//! The weights live only in the child. Unload asks it to drop them; dropping
//! the engine kills the child, so the OS reclaims all of its memory even if the
//! worker misbehaves.

use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, RecvTimeoutError, channel};
use std::time::{Duration, Instant};

use anyhow::{Context, Result, anyhow, bail};
use serde_json::{Value, json};

use super::{BadGeneration, Cancelled, Engine, EngineEvent, GenerationParams, VoicePrompt};
use crate::audio::{Audio, read_wav};
use crate::config::app_support_dir;

/// Locate the worker's Python interpreter and script.
///
/// Order: `VOXCPM_TTS_PYTHON`/`VOXCPM_TTS_WORKER`, the installed copy under
/// the app-support directory, then the source checkout this binary was built from.
pub fn locate_worker() -> Result<(PathBuf, PathBuf)> {
    let candidates = [
        app_support_dir().join("reference"),
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("reference"),
    ];
    let python = std::env::var_os("VOXCPM_TTS_PYTHON")
        .map(PathBuf::from)
        .or_else(|| {
            candidates
                .iter()
                .map(|d| d.join(".venv/bin/python"))
                .find(|p| p.exists())
        });
    let script = std::env::var_os("VOXCPM_TTS_WORKER")
        .map(PathBuf::from)
        .or_else(|| {
            candidates
                .iter()
                .map(|d| d.join("voxcpm_worker.py"))
                .find(|p| p.exists())
        });
    match (python, script) {
        (Some(p), Some(s)) => Ok((p, s)),
        _ => bail!(
            "mlx-python engine is not installed (no reference/.venv). Run scripts/install.sh --with-python-engine, \
             or set VOXCPM_TTS_PYTHON and VOXCPM_TTS_WORKER"
        ),
    }
}

const STARTUP_TIMEOUT: Duration = Duration::from_secs(60);
const LOAD_TIMEOUT: Duration = Duration::from_secs(600);
const SYNTHESIS_TIMEOUT: Duration = Duration::from_secs(1800);

struct Worker {
    child: Child,
    stdin: ChildStdin,
    lines: Receiver<std::io::Result<String>>,
}

impl Worker {
    fn spawn(python: &Path, script: &Path, cancel: &AtomicBool) -> Result<Self> {
        Self::spawn_with_timeout(python, script, cancel, STARTUP_TIMEOUT)
    }

    fn spawn_with_timeout(
        python: &Path,
        script: &Path,
        cancel: &AtomicBool,
        timeout: Duration,
    ) -> Result<Self> {
        let mut child = Command::new(python)
            .arg(script)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .with_context(|| format!("starting worker {}", script.display()))?;
        let stdin = child.stdin.take().expect("piped stdin");
        let stdout = child.stdout.take().expect("piped stdout");
        let stderr = child.stderr.take().expect("piped stderr");
        let (tx, rx) = channel();
        std::thread::spawn(move || {
            for line in BufReader::new(stdout).lines() {
                if tx.send(line).is_err() {
                    break;
                }
            }
        });
        // Worker logs are only interesting when debugging.
        let verbose = std::env::var_os("VOXCPM_TTS_DEBUG").is_some();
        std::thread::spawn(move || {
            for line in BufReader::new(stderr).lines().map_while(Result::ok) {
                if verbose {
                    eprintln!("[worker] {line}");
                }
            }
        });
        let mut w = Worker {
            child,
            stdin,
            lines: rx,
        };
        let ready = w.next_message(cancel, &mut |_| {}, Instant::now() + timeout)?;
        if ready.get("event").and_then(Value::as_str) != Some("ready") {
            bail!("worker did not report ready: {ready}");
        }
        Ok(w)
    }

    /// Next protocol message; progress events go to `on_progress`.
    fn next_message(
        &mut self,
        cancel: &AtomicBool,
        on_progress: &mut dyn FnMut(&Value),
        deadline: Instant,
    ) -> Result<Value> {
        loop {
            if cancel.load(Ordering::SeqCst) {
                let _ = self.child.kill();
                let _ = self.child.wait();
                return Err(Cancelled.into());
            }
            if Instant::now() >= deadline {
                let _ = self.child.kill();
                let _ = self.child.wait();
                bail!("worker timed out; its process was terminated");
            }
            match self.lines.recv_timeout(Duration::from_millis(100)) {
                Ok(line) => {
                    let line = line.context("reading worker output")?;
                    let Ok(v) = serde_json::from_str::<Value>(&line) else {
                        continue;
                    };
                    if v.get("event").and_then(Value::as_str) == Some("progress") {
                        on_progress(&v);
                        continue;
                    }
                    return Ok(v);
                }
                Err(RecvTimeoutError::Timeout) => {
                    if let Ok(Some(status)) = self.child.try_wait() {
                        bail!(
                            "worker exited unexpectedly ({status}); rerun with VOXCPM_TTS_DEBUG=1"
                        );
                    }
                }
                Err(RecvTimeoutError::Disconnected) => bail!("worker closed its output"),
            }
        }
    }

    fn request(
        &mut self,
        req: Value,
        cancel: &AtomicBool,
        on_progress: &mut dyn FnMut(&Value),
        timeout: Duration,
    ) -> Result<Value> {
        let deadline = Instant::now() + timeout;
        let id = req["id"].as_str().unwrap_or_default().to_string();
        writeln!(self.stdin, "{req}").context("sending request to worker")?;
        self.stdin.flush()?;
        loop {
            let reply = self.next_message(cancel, on_progress, deadline)?;
            if reply.get("id").and_then(Value::as_str) != Some(id.as_str()) {
                continue;
            }
            if reply.get("ok").and_then(Value::as_bool) == Some(true) {
                return Ok(reply);
            }
            if reply.get("code").and_then(Value::as_str) == Some("bad_generation") {
                return Err(BadGeneration(
                    reply
                        .get("error")
                        .and_then(Value::as_str)
                        .unwrap_or("generation did not stop")
                        .into(),
                )
                .into());
            }
            bail!(
                "worker error: {}",
                reply
                    .get("error")
                    .and_then(Value::as_str)
                    .unwrap_or("unknown")
            );
        }
    }
}

impl Drop for Worker {
    fn drop(&mut self) {
        let _ = writeln!(self.stdin, "{}", json!({"id": "bye", "op": "shutdown"}));
        let _ = self.stdin.flush();
        let deadline = Instant::now() + Duration::from_secs(3);
        while Instant::now() < deadline {
            if let Ok(Some(_)) = self.child.try_wait() {
                return;
            }
            std::thread::sleep(Duration::from_millis(50));
        }
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

pub struct WorkerEngine {
    model_id: String,
    worker: Option<Worker>,
    loaded: bool,
    scratch: tempfile::TempDir,
    counter: u64,
}

impl WorkerEngine {
    pub fn new(model_id: &str) -> Result<Self> {
        locate_worker()?;
        Ok(Self {
            model_id: model_id.to_string(),
            worker: None,
            loaded: false,
            scratch: tempfile::Builder::new()
                .prefix("voxcpm-tts-worker-")
                .tempdir()?,
            counter: 0,
        })
    }

    fn next_id(&mut self) -> String {
        self.counter += 1;
        format!("r{}", self.counter)
    }

    fn worker(&mut self, cancel: &AtomicBool) -> Result<&mut Worker> {
        if self.worker.is_none() {
            let (python, script) = locate_worker()?;
            self.worker = Some(Worker::spawn(&python, &script, cancel)?);
        }
        Ok(self.worker.as_mut().expect("just spawned"))
    }
}

impl Engine for WorkerEngine {
    fn name(&self) -> &'static str {
        "mlx-python"
    }

    fn load(&mut self) -> Result<f64> {
        self.load_cancellable(&AtomicBool::new(false))
    }

    fn load_cancellable(&mut self, cancel: &AtomicBool) -> Result<f64> {
        if cancel.load(Ordering::SeqCst) {
            self.unload()?;
            return Err(Cancelled.into());
        }
        if self.loaded {
            return Ok(0.0);
        }
        let start = Instant::now();
        let id = self.next_id();
        let model = self.model_id.clone();
        let result = self.worker(cancel)?.request(
            json!({"id": id, "op": "load", "model": model}),
            cancel,
            &mut |_| {},
            LOAD_TIMEOUT,
        );
        if result.is_err() {
            self.worker = None;
        }
        result?;
        self.loaded = true;
        Ok(start.elapsed().as_secs_f64())
    }

    fn is_loaded(&self) -> bool {
        self.loaded
    }

    fn synthesize(
        &mut self,
        text: &str,
        voice: Option<&VoicePrompt>,
        params: &GenerationParams,
        on_event: &mut dyn FnMut(EngineEvent),
        cancel: &AtomicBool,
    ) -> Result<Audio> {
        if !self.loaded {
            self.load_cancellable(cancel)?;
        }
        let id = self.next_id();
        // The directory guard removes partial outputs on every return path,
        // including protocol errors while the session keeps the model loaded.
        let request_dir = tempfile::Builder::new()
            .prefix("request-")
            .tempdir_in(self.scratch.path())?;
        let out = request_dir.path().join("output.wav");
        let req = json!({
            "id": id,
            "op": "synthesize",
            "text": text,
            "ref_audio": voice.map(|v| v.audio_path.to_string_lossy().to_string()),
            "ref_text": voice.and_then(|v| v.transcript.clone()),
            "out_wav": out.to_string_lossy(),
            "cfg_value": params.cfg_value,
            "inference_timesteps": params.inference_timesteps,
            "seed": params.seed,
            "max_audio_seconds": params.max_audio_seconds,
            "retry_badcase": false,
        });
        let result = self.worker(cancel)?.request(
            req,
            cancel,
            &mut |v| {
                if let Some(s) = v.get("audio_seconds").and_then(Value::as_f64) {
                    on_event(EngineEvent::Generated { audio_seconds: s });
                }
            },
            SYNTHESIS_TIMEOUT,
        );
        let exited = self
            .worker
            .as_mut()
            .is_some_and(|w| w.child.try_wait().ok().flatten().is_some());
        if result.is_err() && (cancel.load(Ordering::SeqCst) || exited) {
            self.worker = None;
            self.loaded = false;
        }
        result?;
        read_wav(&out).map_err(|e| anyhow!("worker output unreadable: {e}"))
    }

    fn unload(&mut self) -> Result<()> {
        // Process exit is the reliable memory boundary. Do not wait for an
        // unbounded unload reply before reaching Worker's bounded shutdown.
        self.worker = None;
        self.loaded = false;
        Ok(())
    }
}

impl Drop for WorkerEngine {
    fn drop(&mut self) {
        let _ = self.unload();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn python() -> PathBuf {
        std::env::var_os("VOXCPM_TTS_TEST_PYTHON")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("python3"))
    }

    #[test]
    fn startup_deadline_terminates_unresponsive_child() {
        let work = tempfile::tempdir().unwrap();
        let script = work.path().join("startup.py");
        std::fs::write(&script, "import time\ntime.sleep(60)\n").unwrap();
        let start = Instant::now();
        let result = Worker::spawn_with_timeout(
            &python(),
            &script,
            &AtomicBool::new(false),
            Duration::from_millis(150),
        );
        assert!(result.is_err());
        assert!(result.err().unwrap().to_string().contains("timed out"));
        assert!(start.elapsed() < Duration::from_secs(3));
    }

    #[test]
    fn request_deadline_terminates_and_reaps_unresponsive_child() {
        let work = tempfile::tempdir().unwrap();
        let script = work.path().join("request.py");
        std::fs::write(&script, "import json,sys,time\nprint(json.dumps({'event':'ready'}),flush=True)\nfor line in sys.stdin:\n time.sleep(60)\n").unwrap();
        let mut worker = Worker::spawn_with_timeout(
            &python(),
            &script,
            &AtomicBool::new(false),
            Duration::from_secs(2),
        )
        .unwrap();
        let start = Instant::now();
        let result = worker.request(
            json!({"id":"test", "op":"load"}),
            &AtomicBool::new(false),
            &mut |_| {},
            Duration::from_millis(150),
        );
        assert!(result.is_err());
        assert!(result.err().unwrap().to_string().contains("timed out"));
        assert!(
            worker.child.try_wait().unwrap().is_some(),
            "deadline must reap the child"
        );
        assert!(start.elapsed() < Duration::from_secs(3));
    }
}
