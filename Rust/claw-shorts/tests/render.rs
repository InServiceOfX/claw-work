//! End-to-end: real FFmpeg, synthetic sine-wave "narration", no TTS model.
//! Skipped (with a message) when ffmpeg or the default fonts are missing.

use std::fs;
use std::path::Path;
use std::process::Command;

use claw_shorts::media;
use claw_shorts::render::{self, RenderOptions};
use claw_shorts::spec;
use claw_shorts::style::Style;

fn can_run() -> bool {
    let fonts = Style::default().missing_fonts();
    if !media::have("ffmpeg") || !media::have("ffprobe") || !fonts.is_empty() {
        eprintln!("skipping: ffmpeg/ffprobe/fonts unavailable ({fonts:?})");
        return false;
    }
    true
}

fn sine(path: &Path, seconds: &str) {
    let status = Command::new("ffmpeg")
        .args(["-y", "-v", "error", "-f", "lavfi", "-i"])
        .arg(format!("sine=frequency=330:duration={seconds}:sample_rate=48000"))
        .arg(path)
        .status()
        .unwrap();
    assert!(status.success());
}

const SPEC: &str = r##"{
  "version": 1, "project": "demo", "repo_url": "github.com/Org/demo",
  "title": "Local voice cloning", "caption": "Runs on a Mac.",
  "hashtags": ["opensource"], "disclosure": "AI-generated voice",
  "credits": ["Voice: A Reader, LibriVox (CC0)"],
  "scenes": [
    {"kind": "title", "narration": "No cloud. Fact or hype?", "audio": "a1.wav",
     "lines": ["Local voice cloning.", "Fact or hype?"]},
    {"kind": "terminal", "narration": "The real run.", "audio": "a2.wav",
     "command": "demo say script.txt --voice ref.flac",
     "output": ["Wrote voice.wav (21s of audio)", "generated in 32s, 1.49x real time"]},
    {"kind": "verdict", "narration": "Verdict.", "audio": "a3.wav", "rows": [
      {"mark": "FACT", "tone": "pass", "text": "Runs locally on a Mac for zero dollars."},
      {"mark": "UNTESTED", "tone": "warn", "text": "Quality versus a paid service."}]},
    {"kind": "endcard", "narration": "Find it on GitHub.", "audio": "a4.wav",
     "lines": ["demo"], "url": "github.com/Org/demo/tree/master/Rust/demo"}
  ]}"##;

#[test]
fn renders_verifies_and_writes_companions() {
    if !can_run() {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    for (name, secs) in [("a1.wav", "1.5"), ("a2.wav", "3.0"), ("a3.wav", "2.5"), ("a4.wav", "1.5")] {
        sine(&dir.path().join(name), secs);
    }
    let parsed = spec::parse(SPEC).unwrap();
    let opts = RenderOptions {
        spec_dir: dir.path().into(),
        work_dir: dir.path().join("work"),
        output_dir: dir.path().join("out"),
        force_tts: false,
    };
    let report = render::render(&parsed, &opts).expect("render");

    let info = media::probe(&report.video).unwrap();
    assert_eq!((info.width, info.height), (1080, 1920));
    assert!(info.has_audio);
    // 1.5 + 3.0 + 2.5 + 1.5 seconds of speech plus four 0.35 s pads.
    assert!(info.duration_ms.abs_diff(9_900) < 150, "{}", info.duration_ms);
    media::decode_check(&report.video).unwrap();

    for file in [&report.contact_sheet, &report.captions_srt, &report.timeline_json, &report.posting_kit_md] {
        assert!(file.is_file(), "{} missing", file.display());
    }
    let width = Command::new("ffprobe")
        .args(["-v", "error", "-show_entries", "stream=width", "-of", "csv=p=0"])
        .arg(&report.contact_sheet)
        .output()
        .unwrap();
    let width: u32 = String::from_utf8_lossy(&width.stdout).trim().parse().unwrap();
    assert!(width >= 4 * 360, "contact sheet is {width} wide");
    let verification = fs::read_to_string(dir.path().join("out/verification.json")).unwrap();
    assert!(verification.contains("\"full_decode_ok\": true"), "{verification}");
}

#[test]
fn refuses_an_existing_output_directory_and_bad_specs() {
    if !can_run() {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let parsed = spec::parse(SPEC).unwrap();
    let existing = dir.path().join("out");
    fs::create_dir(&existing).unwrap();
    let opts = RenderOptions {
        spec_dir: dir.path().into(),
        work_dir: dir.path().join("work"),
        output_dir: existing,
        force_tts: false,
    };
    assert!(render::render(&parsed, &opts).unwrap_err().contains("already exists"));

    let mut bad = parsed.clone();
    bad.title = "TODO title".into();
    let opts = RenderOptions { output_dir: dir.path().join("fresh"), ..opts };
    let err = render::render(&bad, &opts).unwrap_err();
    assert!(err.contains("todo"), "{err}");
    assert!(!opts.output_dir.exists(), "nothing may be written when validation fails");
}

/// Regression: FFmpeg runs inside the work directory, so relative `--output`
/// and `--work` paths used to resolve against the wrong folder.
#[test]
fn cli_accepts_relative_work_and_output_paths() {
    if !can_run() {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    for (name, secs) in [("a1.wav", "1.0"), ("a2.wav", "1.5"), ("a3.wav", "1.0"), ("a4.wav", "1.0")] {
        sine(&dir.path().join(name), secs);
    }
    fs::write(dir.path().join("short.json"), SPEC).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_claw-shorts"))
        .current_dir(dir.path())
        .args(["render", "--spec", "short.json", "--work", "w", "--output", "o"])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(dir.path().join("o/short.mp4").is_file());
    assert!(dir.path().join("o/contact.png").is_file());
    assert!(!dir.path().join("w/o").exists(), "output must not land inside the work dir");
}

#[test]
fn cli_check_rejects_the_brief_template() {
    let dir = tempfile::tempdir().unwrap();
    let project = dir.path().join("proj");
    fs::create_dir(&project).unwrap();
    fs::write(project.join("Cargo.toml"), "[package]\nname = \"demo\"\n").unwrap();
    let bin = env!("CARGO_BIN_EXE_claw-shorts");
    let brief = Command::new(bin)
        .args(["brief", "--repo"]).arg(&project).arg("--output").arg(dir.path().join("b"))
        .output().unwrap();
    assert!(brief.status.success(), "{}", String::from_utf8_lossy(&brief.stderr));
    let check = Command::new(bin)
        .args(["check", "--spec"]).arg(dir.path().join("b/short.template.json"))
        .output().unwrap();
    assert!(!check.status.success(), "the template must not pass check");
    assert!(String::from_utf8_lossy(&check.stderr).contains("placeholder"));
}
