mod common;
use claw_weight_space::{
    evaluation::{Outcome, Settings, compare, synthetic_cases},
    hash_bytes,
    server::ServerConfig,
    write_json_new,
};
use std::{fs, io::Write, path::Path};
fn fixture(root: &Path, arm: &str, correct: &[bool], errors: &[bool]) {
    fs::create_dir(root).unwrap();
    let cases = synthetic_cases(75, 4).unwrap();
    let settings = Settings {
        arm: arm.into(),
        max_tokens: 8192,
        thinking: true,
        case_sha256: hash_bytes(&serde_json::to_vec(&cases).unwrap()),
        model_sha256: if arm == "edit" { "b" } else { "a" }.repeat(64),
        tool_sha256: "c".repeat(64),
        input_case_file_sha256: "d".repeat(64),
        seed: 20261109,
        repeat_penalty: if arm == "decoding" { 1.1 } else { 1. },
        server: ServerConfig {
            image: "pinned".into(),
            gpu: "test".into(),
            port: 8097,
            context: 40960,
        },
        protocol: "test".into(),
    };
    write_json_new(&root.join("settings.json"), &settings).unwrap();
    write_json_new(&root.join("cases.json"), &cases).unwrap();
    let mut f = fs::File::create(root.join("results.jsonl")).unwrap();
    for (i, c) in cases.iter().enumerate() {
        let o = Outcome {
            id: c.id.clone(),
            seed: settings.seed + i as u32,
            correct: correct[i],
            thinking_closed: true,
            cap_hit: false,
            repetition: false,
            tokens: 100,
            seconds: 1.,
            error: errors[i].then(|| "synthetic failure".into()),
            response: None,
        };
        serde_json::to_writer(&mut f, &o).unwrap();
        f.write_all(b"\n").unwrap();
    }
}
#[test]
fn paired_comparison_retains_errors_and_denominator() {
    let t = common::tempdir();
    let a = t.path().join("a");
    let b = t.path().join("b");
    fixture(&a, "base", &[true, false, false, true], &[false; 4]);
    fixture(
        &b,
        "edit",
        &[true, true, false, true],
        &[false, false, false, true],
    );
    let r = compare(&a, &b).unwrap();
    assert_eq!(r["n"], 4);
    assert_eq!(r["rescued"], 1);
    assert_eq!(r["regressed"], 1);
    assert_eq!(r["paired_accuracy_difference"], 0.);
    assert_eq!(r["right_summary"]["errors"], 1);
}
#[test]
fn mismatched_budget_cases_and_missing_results_rejected() {
    let t = common::tempdir();
    let a = t.path().join("a");
    let b = t.path().join("b");
    fixture(&a, "base", &[false; 4], &[false; 4]);
    fixture(&b, "edit", &[true; 4], &[false; 4]);
    let p = b.join("settings.json");
    let mut s: serde_json::Value = serde_json::from_slice(&fs::read(&p).unwrap()).unwrap();
    s["max_tokens"] = 4096.into();
    fs::write(&p, serde_json::to_vec(&s).unwrap()).unwrap();
    assert!(compare(&a, &b).is_err());
    s["max_tokens"] = 8192.into();
    fs::write(&p, serde_json::to_vec(&s).unwrap()).unwrap();
    fs::write(b.join("results.jsonl"), "").unwrap();
    assert!(compare(&a, &b).is_err());
}
