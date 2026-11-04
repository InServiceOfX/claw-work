use claw_lean_agent::{
    process::{run_cleanup, stop_child},
    proof::{Limits, docker_args, resolve_image},
};
use serde_json::{Value, json};
use std::{
    fs,
    io::{BufRead, BufReader, Read, Write},
    process::{Child, Command, Stdio},
    sync::mpsc,
    thread,
    time::{Duration, Instant},
};
struct Session {
    child: Child,
    name: String,
}
impl Drop for Session {
    fn drop(&mut self) {
        stop_child(&mut self.child);
        let _ = run_cleanup(Command::new("docker").args(["rm", "-f", &self.name]));
    }
}
fn send(s: &mut Session, msg: Value) {
    let data = serde_json::to_vec(&msg).unwrap();
    let stdin = s.child.stdin.as_mut().unwrap();
    write!(stdin, "Content-Length: {}\r\n\r\n", data.len()).unwrap();
    stdin.write_all(&data).unwrap();
    stdin.flush().unwrap();
}
fn wait(rx: &mpsc::Receiver<Value>, predicate: impl Fn(&Value) -> bool) -> Value {
    let deadline = Instant::now() + Duration::from_secs(45);
    loop {
        let left = deadline.saturating_duration_since(Instant::now());
        let msg = rx
            .recv_timeout(left)
            .expect("LSP response/diagnostic deadline");
        if predicate(&msg) {
            return msg;
        }
    }
}
#[test]
#[ignore = "requires built Docker image and LEAN_AGENT_LIVE=1"]
fn interactive_diagnostics_repair_and_goal_feedback() {
    assert_eq!(std::env::var("LEAN_AGENT_LIVE").as_deref(), Ok("1"));
    let tmp = tempfile::tempdir().unwrap();
    let bad = "import Mathlib.Data.Nat.Basic\nexample : (1 : Nat) = 2 := by rfl\n";
    fs::write(tmp.path().join("Interactive.lean"), bad).unwrap();
    let limits = Limits::default();
    let id = resolve_image(&limits.image).unwrap();
    let name = format!("claw-lsp-test-{}", std::process::id());
    let mut args = docker_args(&id, &name, Some(tmp.path()), &limits).unwrap();
    args.insert(1, "-i".into());
    args.push("server".into());
    let child = Command::new("docker")
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()
        .unwrap();
    let mut session = Session { child, name };
    let stdout = session.child.stdout.take().unwrap();
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        let mut reader = BufReader::new(stdout);
        loop {
            let mut size = None;
            loop {
                let mut line = String::new();
                if reader.read_line(&mut line).unwrap_or(0) == 0 {
                    return;
                }
                if line == "\r\n" {
                    break;
                }
                if let Some(n) = line.strip_prefix("Content-Length:") {
                    size = n.trim().parse::<usize>().ok();
                }
            }
            let Some(size) = size.filter(|n| *n <= 4 * 1_048_576) else {
                return;
            };
            let mut data = vec![0; size];
            if reader.read_exact(&mut data).is_err() {
                return;
            }
            if let Ok(msg) = serde_json::from_slice(&data) {
                if tx.send(msg).is_err() {
                    return;
                }
            }
        }
    });
    send(
        &mut session,
        json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{"processId":null,"rootUri":"file:///input","capabilities":{}}}),
    );
    let initialized = wait(&rx, |m| m["id"] == 1);
    assert!(initialized.get("result").is_some(), "{initialized}");
    send(
        &mut session,
        json!({"jsonrpc":"2.0","method":"initialized","params":{}}),
    );
    let uri = "file:///input/Interactive.lean";
    send(
        &mut session,
        json!({"jsonrpc":"2.0","method":"textDocument/didOpen","params":{"textDocument":{"uri":uri,"languageId":"lean4","version":1,"text":bad}}}),
    );
    wait(&rx, |m| {
        m["method"] == "textDocument/publishDiagnostics"
            && m["params"]["diagnostics"]
                .as_array()
                .is_some_and(|a| a.iter().any(|d| d["severity"] == 1))
    });
    let good = "import Mathlib.Data.Nat.Basic\nexample (n : Nat) : n + 0 = n := by\n  omega\n";
    send(
        &mut session,
        json!({"jsonrpc":"2.0","method":"textDocument/didChange","params":{"textDocument":{"uri":uri,"version":2},"contentChanges":[{"text":good}]}}),
    );
    wait(&rx, |m| {
        m["method"] == "textDocument/publishDiagnostics"
            && m["params"]["version"] == 2
            && m["params"]["diagnostics"]
                .as_array()
                .is_some_and(|a| a.is_empty())
    });
    send(
        &mut session,
        json!({"jsonrpc":"2.0","id":2,"method":"$/lean/plainGoal","params":{"textDocument":{"uri":uri},"position":{"line":2,"character":2}}}),
    );
    let goal = wait(&rx, |m| m["id"] == 2);
    assert!(goal.get("error").is_none(), "{goal}");
    assert!(goal["result"].to_string().contains("goals"), "{goal}");
}
