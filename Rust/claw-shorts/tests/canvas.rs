//! The canvas path end to end with generated audio: `canvas` writes a
//! self-contained scene, `mux` joins a silent video with narration and rejects
//! a length mismatch. Skipped when ffmpeg is unavailable.

use std::fs;
use std::path::Path;
use std::process::Command;

use claw_shorts::media;

const BIN: &str = env!("CARGO_BIN_EXE_claw-shorts");

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
  "title": "This voice isn't real", "caption": "Runs on a Mac.",
  "hashtags": ["opensource"], "disclosure": "AI-generated voice",
  "scoreboard": ["8 s in", "$0"],
  "engagement": {"device": "judgment invite", "token": "comment",
                 "prompt": "Could you tell? Comment the second you knew.",
                 "pinned_comment": "Where did it sound fake?"},
  "scenes": [
    {"kind": "title", "narration": "This voice isn't real.", "audio": "a1.wav", "lines": ["THIS VOICE", "ISN'T REAL"]},
    {"kind": "clone", "narration": "It heard eight seconds.", "audio": "a2.wav", "lines": ["8 s in", "anything out"]},
    {"kind": "verdict", "narration": "Fact.", "audio": "a3.wav", "rows": [
      {"mark": "FACT", "tone": "pass", "text": "Runs locally for $0."}]},
    {"kind": "endcard", "narration": "On GitHub.", "audio": "a4.wav", "lines": ["demo"], "url": "github.com/Org/demo"}
  ]}"##;

const WORDS: &str = r#"[
  {"text":"This","start_ms":0,"end_ms":200},{"text":"voice","start_ms":200,"end_ms":500},
  {"text":"isn't","start_ms":500,"end_ms":800},{"text":"real.","start_ms":800,"end_ms":1100}]"#;

fn have_tools() -> bool {
    if media::have("ffmpeg") && media::have("ffprobe") {
        return true;
    }
    eprintln!("skipping: ffmpeg/ffprobe unavailable");
    false
}

#[test]
fn canvas_writes_a_self_contained_scene_and_matching_narration() {
    if !have_tools() {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    for (name, secs) in [("a1.wav", "1.2"), ("a2.wav", "2.0"), ("a3.wav", "1.0"), ("a4.wav", "1.0")] {
        sine(&dir.path().join(name), secs);
    }
    fs::write(dir.path().join("short.json"), SPEC).unwrap();
    fs::write(dir.path().join("words.json"), WORDS).unwrap();
    let out = Command::new(BIN)
        .current_dir(dir.path())
        .args(["canvas", "--spec", "short.json", "--work", "w", "--output", "scene", "--words", "words.json", "--strict"])
        .output()
        .unwrap();
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));

    let html = fs::read_to_string(dir.path().join("scene/scene.html")).unwrap();
    assert!(html.contains("window.renderFrame"), "the scene keeps its renderer");
    assert!(!html.contains("/*__SHORT_DATA__*/"), "the data marker must be replaced");
    assert!(!html.contains("http://") && !html.contains("https://"), "the scene must load nothing");
    let data: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(dir.path().join("scene/scene_data.json")).unwrap()).unwrap();
    let total = data["total_ms"].as_u64().unwrap();
    // 1.2 + 2.0 + 1.0 + 1.0 seconds of speech plus four 0.35 s pads.
    assert!(total.abs_diff(6_600) < 5, "{total}");
    assert_eq!(data["peaks"].as_array().unwrap().len(), (total as usize * 30).div_ceil(1000));
    assert!(data["peaks"].as_array().unwrap().iter().any(|p| p.as_f64().unwrap() > 0.5), "loudness is measured");
    assert_eq!(data["scenes"].as_array().unwrap().len(), 4);
    assert_eq!(data["scoreboard"][1], "$0");
    assert!(data["engagement_prompt"].as_str().unwrap().contains("Comment"));
    assert_eq!(data["words"].as_array().unwrap().len(), 11, "one entry per narration word, not per recognized word");

    let narration = media::probe_duration_ms(&dir.path().join("scene/narration.wav")).unwrap();
    assert!(narration.abs_diff(total) < 60, "narration {narration} vs timeline {total}");
}

#[test]
fn mux_verifies_and_rejects_a_length_mismatch() {
    if !have_tools() {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    sine(&dir.path().join("narration.wav"), "2.0");
    let video = |name: &str, secs: &str| {
        let status = Command::new("ffmpeg")
            .args(["-y", "-v", "error", "-f", "lavfi", "-i"])
            .arg(format!("color=c=black:s=360x640:r=30:d={secs}"))
            .args(["-c:v", "libx264", "-pix_fmt", "yuv420p"])
            .arg(dir.path().join(name))
            .status()
            .unwrap();
        assert!(status.success());
    };
    video("good.mp4", "2.0");
    video("short.mp4", "1.0");

    let ok = Command::new(BIN)
        .current_dir(dir.path())
        .args(["mux", "--video", "good.mp4", "--audio", "narration.wav", "--output", "final.mp4"])
        .output()
        .unwrap();
    assert!(ok.status.success(), "{}", String::from_utf8_lossy(&ok.stderr));
    let info = media::probe(&dir.path().join("final.mp4")).unwrap();
    assert!(info.has_audio && info.duration_ms.abs_diff(2000) < 150);

    let bad = Command::new(BIN)
        .current_dir(dir.path())
        .args(["mux", "--video", "short.mp4", "--audio", "narration.wav", "--output", "bad.mp4"])
        .output()
        .unwrap();
    assert!(!bad.status.success());
    assert!(String::from_utf8_lossy(&bad.stderr).contains("disagree"));

    let again = Command::new(BIN)
        .current_dir(dir.path())
        .args(["mux", "--video", "good.mp4", "--audio", "narration.wav", "--output", "final.mp4"])
        .output()
        .unwrap();
    assert!(!again.status.success(), "an existing output must be refused");
}
