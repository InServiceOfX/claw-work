use claw_lean_agent::proof::{Limits, Manifest, check_proofs, validate_manifest};
use std::{fs, path::Path};
mod common;
use common::tempdir;

fn live() -> bool {
    std::env::var("LEAN_AGENT_LIVE").as_deref() == Ok("1")
}
fn case(
    solution: &str,
    theorem: &str,
    limits: Limits,
) -> Option<claw_lean_agent::proof::ProofReport> {
    assert!(
        live(),
        "Set LEAN_AGENT_LIVE=1 for explicitly requested live tests"
    );
    let t = tempdir().unwrap();
    fs::write(
        t.path().join("Challenge.lean"),
        "import Mathlib.Data.Nat.Basic\ntheorem accepted (n : Nat) : n + 0 = n := by sorry\n",
    )
    .unwrap();
    fs::write(t.path().join("Solution.lean"), solution).unwrap();
    let mut manifest = Manifest::new("Challenge.lean", "Solution.lean", vec![theorem.into()]);
    manifest
        .intended_properties
        .push("Natural addition identity".into());
    Some(check_proofs(&manifest, t.path(), &t.path().join("out"), &limits).unwrap())
}
const GOOD: &str =
    "import Mathlib.Data.Nat.Basic\ntheorem accepted (n : Nat) : n + 0 = n := by omega\n";

#[test]
#[ignore = "requires the locally built Docker image and LEAN_AGENT_LIVE=1"]
fn valid_model_does_not_become_source_verification() {
    if let Some(r) = case(GOOD, "accepted", Limits::default()) {
        assert_eq!(r.model_proof_status, "checked", "{r:#?}");
        assert_eq!(r.source_correspondence_status, "unproved");
        assert_eq!(r.specification_review_status, "not_attested");
        assert!(r.image_id.starts_with("sha256:"));
    }
}
#[test]
#[ignore = "requires the locally built Docker image and LEAN_AGENT_LIVE=1"]
fn rejects_sorry_and_transitive_admission() {
    for text in [
        "import Mathlib.Data.Nat.Basic\ntheorem accepted (n : Nat) : n + 0 = n := by sorry\n",
        "import Mathlib.Data.Nat.Basic\ntheorem hidden (n : Nat) : n + 0 = n := by sorry\ntheorem accepted (n : Nat) : n + 0 = n := hidden n\n",
    ] {
        if let Some(r) = case(text, "accepted", Limits::default()) {
            assert_eq!(r.model_proof_status, "rejected", "{r:#?}");
        }
    }
}
#[test]
#[ignore = "requires the locally built Docker image and LEAN_AGENT_LIVE=1"]
fn rejects_axiom_missing_or_changed_statement() {
    for (text, name) in [
        (
            "axiom magic (n : Nat) : n + 0 = n\ntheorem accepted (n : Nat) : n + 0 = n := magic n\n",
            "accepted",
        ),
        (GOOD, "missing"),
        ("theorem accepted : True := by trivial\n", "accepted"),
        (
            "#eval IO.println \"CLAW_FAKE_ACCEPTED\"\ntheorem accepted : True := by trivial\n",
            "accepted",
        ),
    ] {
        if let Some(r) = case(text, name, Limits::default()) {
            assert_eq!(r.model_proof_status, "rejected", "{r:#?}");
        }
    }
}
#[test]
#[ignore = "requires the locally built Docker image and LEAN_AGENT_LIVE=1"]
fn timeout_is_unknown_never_pass() {
    let limits = Limits {
        timeout_ms: 20,
        ..Default::default()
    };
    if let Some(r) = case(GOOD, "accepted", limits) {
        assert_ne!(r.model_proof_status, "checked");
    }
}
#[test]
fn manifest_requires_nonempty_distinct_names_and_properties() {
    let mut m = Manifest::new("a.lean", "b.lean", vec![]);
    assert!(validate_manifest(&m).is_err());
    m.theorems = vec!["same".into(), "same".into()];
    assert!(validate_manifest(&m).is_err());
    m.theorems = vec!["a;bad".into()];
    assert!(validate_manifest(&m).is_err());
    m.theorems = vec!["Namespace.valid_name".into()];
    assert!(validate_manifest(&m).is_err());
    m.intended_properties = vec!["Independent intended statement".into()];
    assert!(validate_manifest(&m).is_ok());
    let _: &Path = Path::new(".");
}
