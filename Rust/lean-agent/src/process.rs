use crate::Result;
use serde::Serialize;
use std::{
    io::Read,
    process::{Child, Command, Stdio},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    thread,
    time::{Duration, Instant},
};

#[derive(Debug, Serialize)]
pub struct ProcessResult {
    pub exit_code: Option<i32>,
    pub timed_out: bool,
    pub output_limited: bool,
    pub cancelled: bool,
    pub wall_ms: u128,
    #[serde(skip)]
    pub stdout: Vec<u8>,
    #[serde(skip)]
    pub stderr: Vec<u8>,
}
fn reader(
    mut stream: impl Read + Send + 'static,
    max: usize,
    full: Arc<AtomicBool>,
) -> thread::JoinHandle<std::io::Result<Vec<u8>>> {
    thread::spawn(move || {
        let mut out = Vec::new();
        let mut chunk = [0; 8192];
        loop {
            match stream.read(&mut chunk) {
                Ok(0) => break,
                Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
                Err(e) => return Err(e),
                Ok(n) => {
                    let keep = n.min(max.saturating_sub(out.len()));
                    out.extend_from_slice(&chunk[..keep]);
                    if keep < n {
                        full.store(true, Ordering::SeqCst);
                        break;
                    }
                }
            }
        }
        Ok(out)
    })
}
pub fn stop_child(child: &mut Child) {
    let _ = child.kill();
    let _ = child.wait();
}
fn cancellation() -> Arc<AtomicBool> {
    static FLAG: std::sync::OnceLock<Arc<AtomicBool>> = std::sync::OnceLock::new();
    FLAG.get_or_init(|| Arc::new(AtomicBool::new(false)))
        .clone()
}
/// Opt-in CLI signal handling; embedding the library does not replace host handlers.
pub fn install_cli_signals() -> Result<()> {
    static INSTALLED: std::sync::OnceLock<std::result::Result<(), String>> =
        std::sync::OnceLock::new();
    INSTALLED
        .get_or_init(|| {
            for sig in [signal_hook::consts::SIGINT, signal_hook::consts::SIGTERM] {
                signal_hook::flag::register(sig, cancellation()).map_err(|e| e.to_string())?;
            }
            Ok(())
        })
        .clone()
        .map_err(Into::into)
}
pub fn run_bounded(
    command: &mut Command,
    timeout: Duration,
    max_output: usize,
) -> Result<ProcessResult> {
    run_inner(command, timeout, max_output, true)
}
pub fn run_cleanup(command: &mut Command) -> Result<ProcessResult> {
    run_inner(command, Duration::from_secs(10), 4096, false)
}
fn run_inner(
    command: &mut Command,
    timeout: Duration,
    max_output: usize,
    cancellable: bool,
) -> Result<ProcessResult> {
    let flag = cancellation();
    let start = Instant::now();
    let mut child = command
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .stdin(Stdio::null())
        .spawn()?;
    let full = Arc::new(AtomicBool::new(false));
    let out = reader(
        child.stdout.take().ok_or("No stdout")?,
        max_output,
        full.clone(),
    );
    let err = reader(
        child.stderr.take().ok_or("No stderr")?,
        max_output,
        full.clone(),
    );
    let mut timed_out = false;
    let exit_code = loop {
        if full.load(Ordering::SeqCst)
            || start.elapsed() >= timeout
            || (cancellable && flag.load(Ordering::SeqCst))
        {
            timed_out = start.elapsed() >= timeout;
            stop_child(&mut child);
            break None;
        }
        if let Some(status) = child.try_wait()? {
            break status.code();
        }
        thread::sleep(Duration::from_millis(10));
    };
    let stdout = out.join().map_err(|_| "stdout reader failed")??;
    let stderr = err.join().map_err(|_| "stderr reader failed")??;
    Ok(ProcessResult {
        exit_code,
        timed_out,
        output_limited: full.load(Ordering::SeqCst),
        cancelled: cancellable && flag.load(Ordering::SeqCst),
        wall_ms: start.elapsed().as_millis(),
        stdout,
        stderr,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn output_and_deadline_never_succeed() {
        let mut c = Command::new("sh");
        c.args(["-c", "while :; do printf xxxxxxxxxxxxxxxxx; done"]);
        let r = run_bounded(&mut c, Duration::from_secs(3), 1024).unwrap();
        assert!(r.output_limited);
        let mut c = Command::new("sleep");
        c.arg("5");
        let r = run_bounded(&mut c, Duration::from_millis(20), 1024).unwrap();
        assert!(r.timed_out);
        assert_ne!(r.exit_code, Some(0));
    }
}
