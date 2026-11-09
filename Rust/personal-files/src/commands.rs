//! A bounded-output, noninteractive shell runner. No GTK work occurs here.
use std::{
    io::{self, Read},
    os::unix::process::CommandExt,
    path::Path,
    process::{Command, Stdio},
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc, Arc,
    },
    thread,
    time::Duration,
};

pub const OUTPUT_LIMIT: usize = 16 * 1024;

#[derive(Debug)]
pub struct Output {
    pub stdout: String,
    pub stderr: String,
    pub code: Option<i32>,
    pub truncated: bool,
    pub cancelled: bool,
}

pub struct Task {
    cancel: Arc<AtomicBool>,
    finished: Arc<AtomicBool>,
    process_group: u32,
    results: mpsc::Receiver<io::Result<Output>>,
}
impl Task {
    pub fn cancel(&self) {
        if !self.finished.load(Ordering::Acquire) && !self.cancel.swap(true, Ordering::AcqRel) {
            // Deliver the signal before returning: closing the final GTK window
            // can exit the application before a worker gets another timeslice.
            terminate_group(self.process_group);
        }
    }
    pub fn try_result(&self) -> Option<io::Result<Output>> {
        match self.results.try_recv() {
            Ok(result) => Some(result),
            Err(mpsc::TryRecvError::Empty) => None,
            Err(mpsc::TryRecvError::Disconnected) => {
                Some(Err(io::Error::other("Command worker disconnected")))
            }
        }
    }
}
impl Drop for Task {
    fn drop(&mut self) {
        self.cancel();
    }
}

fn terminate_group(process_group: u32) {
    let _ = Command::new("/bin/kill")
        .args(["-KILL", "--", &format!("-{process_group}")])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
}

fn capture(mut reader: impl Read) -> io::Result<(String, bool)> {
    let mut bytes = Vec::with_capacity(OUTPUT_LIMIT);
    let mut buffer = [0u8; 4096];
    let mut truncated = false;
    loop {
        let count = match reader.read(&mut buffer) {
            Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
            result => result?,
        };
        if count == 0 {
            break;
        }
        let keep = count.min(OUTPUT_LIMIT - bytes.len());
        bytes.extend_from_slice(&buffer[..keep]);
        truncated |= keep < count;
    }
    Ok((String::from_utf8_lossy(&bytes).into_owned(), truncated))
}

pub fn start(command: &str, cwd: &Path) -> io::Result<Task> {
    let mut child = Command::new("/bin/sh")
        .args(["-c", command])
        .current_dir(cwd)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .process_group(0)
        .spawn()?;
    let process_group = child.id();
    let stdout = child.stdout.take().expect("piped stdout");
    let stderr = child.stderr.take().expect("piped stderr");
    let cancel = Arc::new(AtomicBool::new(false));
    let stop = cancel.clone();
    let finished = Arc::new(AtomicBool::new(false));
    let complete = finished.clone();
    let (sender, results) = mpsc::channel();
    thread::spawn(move || {
        let output = thread::spawn(move || capture(stdout));
        let errors = thread::spawn(move || capture(stderr));
        let result = (|| {
            let mut status = None;
            let mut cancelled = false;
            loop {
                if stop.load(Ordering::Relaxed) && !cancelled {
                    cancelled = true;
                    // The controller already signalled the owned process group.
                    let _ = child.kill();
                }
                if status.is_none() {
                    status = child.try_wait()?;
                }
                if status.is_some() && output.is_finished() && errors.is_finished() {
                    break;
                }
                thread::sleep(Duration::from_millis(20));
            }
            let (stdout, out_truncated) = output
                .join()
                .map_err(|_| io::Error::other("Output reader failed"))??;
            let (stderr, err_truncated) = errors
                .join()
                .map_err(|_| io::Error::other("Error reader failed"))??;
            Ok(Output {
                stdout,
                stderr,
                code: status.and_then(|s| s.code()),
                truncated: out_truncated || err_truncated,
                cancelled,
            })
        })();
        if result.is_err() {
            terminate_group(process_group);
            let _ = child.kill();
            let _ = child.wait();
        }
        complete.store(true, Ordering::Release);
        let _ = sender.send(result);
    });
    Ok(Task {
        cancel,
        finished,
        process_group,
        results,
    })
}
