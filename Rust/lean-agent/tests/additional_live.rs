mod common;
use claw_lean_agent::proof::{Limits, Manifest, check_proofs};
use std::{
    fs,
    process::{Child, Command, Stdio},
    thread,
    time::{Duration, Instant},
};
fn check(challenge: &str, solution: &str, limits: Limits) -> claw_lean_agent::proof::ProofReport {
    assert_eq!(std::env::var("LEAN_AGENT_LIVE").as_deref(), Ok("1"));
    let t = common::tempdir().unwrap();
    fs::write(t.path().join("Challenge.lean"), challenge).unwrap();
    fs::write(t.path().join("Solution.lean"), solution).unwrap();
    let mut m = Manifest::new("Challenge.lean", "Solution.lean", vec!["accepted".into()]);
    m.intended_properties
        .push("Independent test property".into());
    check_proofs(&m, t.path(), &t.path().join("report"), &limits).unwrap()
}
#[test]
#[ignore = "requires built image and LEAN_AGENT_LIVE=1"]
fn changed_definition_is_rejected_even_with_reflexive_theorem() {
    let r = check(
        "def value : Nat := 7\ntheorem accepted : value = value := by sorry\n",
        "def value : Nat := 8\ntheorem accepted : value = value := by rfl\n",
        Limits::default(),
    );
    assert_eq!(r.model_proof_status, "rejected", "{r:#?}");
    assert_eq!(r.phases.last().unwrap().name, "judge");
}
#[test]
#[ignore = "requires built image and LEAN_AGENT_LIVE=1"]
fn native_oracle_is_not_a_permitted_proof() {
    let r = check(
        "import Mathlib.Tactic\ntheorem accepted : (List.range 10).sum = 45 := by sorry\n",
        "import Mathlib.Tactic\ntheorem accepted : (List.range 10).sum = 45 := by native_decide\n",
        Limits::default(),
    );
    assert_eq!(r.model_proof_status, "rejected", "{r:#?}");
    assert_eq!(r.phases.last().unwrap().name, "judge", "{r:#?}");
}
#[test]
#[ignore = "requires built image and LEAN_AGENT_LIVE=1"]
fn export_output_limit_is_unknown_not_checked() {
    let r = check(
        "theorem accepted : True := by sorry\n",
        "theorem accepted : True := by trivial\n",
        Limits {
            max_output_bytes: 1024,
            ..Default::default()
        },
    );
    assert_eq!(r.model_proof_status, "unknown");
    assert_eq!(r.phases[0].status, "output_limit");
}
struct Cleanup(Child);
impl Drop for Cleanup {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}
#[test]
#[ignore = "requires built image and LEAN_AGENT_LIVE=1"]
fn sigterm_cancels_and_removes_own_container() {
    assert_eq!(std::env::var("LEAN_AGENT_LIVE").as_deref(), Ok("1"));
    let t = common::tempdir().unwrap();
    fs::write(
        t.path().join("Challenge.lean"),
        "theorem accepted : True := by sorry\n",
    )
    .unwrap();
    fs::write(
        t.path().join("Solution.lean"),
        "#eval IO.sleep 30000\ntheorem accepted : True := by trivial\n",
    )
    .unwrap();
    fs::write(
        t.path().join("manifest.json"),
        serde_json::to_vec(&serde_json::json!({
            "schema_version":1,"challenge":"Challenge.lean","solution":"Solution.lean",
            "theorems":["accepted"],"intended_properties":["True test"]
        }))
        .unwrap(),
    )
    .unwrap();
    let out = t.path().join("report");
    let mut child = Cleanup(
        Command::new(env!("CARGO_BIN_EXE_claw-lean"))
            .args(["check", "--manifest"])
            .arg(t.path().join("manifest.json"))
            .arg("--output")
            .arg(&out)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap(),
    );
    let deadline = Instant::now() + Duration::from_secs(20);
    while !out.join("solution-source").exists() {
        assert!(
            Instant::now() < deadline,
            "Checker did not reach solution phase"
        );
        thread::sleep(Duration::from_millis(20));
    }
    thread::sleep(Duration::from_millis(150));
    assert_eq!(unsafe { libc::kill(child.0.id() as i32, libc::SIGTERM) }, 0);
    let deadline = Instant::now() + Duration::from_secs(10);
    let status = loop {
        if let Some(s) = child.0.try_wait().unwrap() {
            break s;
        }
        assert!(
            Instant::now() < deadline,
            "Cancellation did not terminate promptly"
        );
        thread::sleep(Duration::from_millis(20));
    };
    assert_eq!(status.code(), Some(2));
    let report: serde_json::Value =
        serde_json::from_slice(&fs::read(out.join("report.json")).unwrap()).unwrap();
    assert_eq!(report["model_proof_status"], "unknown");
    assert_eq!(
        report["phases"].as_array().unwrap().last().unwrap()["status"],
        "cancelled"
    );
    let names = Command::new("docker")
        .args([
            "ps",
            "-a",
            "--filter",
            &format!("name=claw-lean-{}-", child.0.id()),
            "--format",
            "{{.Names}}",
        ])
        .output()
        .unwrap();
    assert!(
        names.status.success() && names.stdout.is_empty(),
        "Own container leaked after cancellation"
    );
}
