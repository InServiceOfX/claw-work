//! The shipped examples must stay valid, and the style recipe must match the
//! built-in defaults so documentation cannot drift from behavior.

use claw_shorts::spec::{self, Severity};
use claw_shorts::style::Style;

#[test]
fn the_example_spec_is_complete() {
    let json = std::fs::read_to_string("examples/voxcpm-tts.short.json").unwrap();
    let spec = spec::parse(&json).unwrap();
    assert_eq!(spec.validate(), vec![], "the example must be clean even under --strict");
    assert!(spec.scenes.iter().all(|s| s.audio.is_none()), "examples narrate with TTS");
}

#[test]
fn the_example_contains_no_personal_paths() {
    let json = std::fs::read_to_string("examples/voxcpm-tts.short.json").unwrap();
    assert!(!json.contains("/Users/") && !json.contains("/home/"));
}

#[test]
fn the_style_example_matches_the_defaults() {
    let json = std::fs::read_to_string("examples/style.json").unwrap();
    let style: Style = serde_json::from_str(&json).unwrap();
    assert_eq!(style, Style::default());
    assert!(style.problems().is_empty());
}

#[test]
fn errors_are_distinguished_from_warnings() {
    let json = std::fs::read_to_string("examples/voxcpm-tts.short.json").unwrap();
    let mut spec = spec::parse(&json).unwrap();
    spec.title = "The voice cloning killer".into();
    let issues = spec.validate();
    assert!(issues.iter().any(|i| i.severity == Severity::Error && i.message.contains("killer")));
}

#[test]
fn the_canvas_example_is_complete_and_uses_the_research_levers() {
    let json = std::fs::read_to_string("examples/voxcpm-tts.canvas.short.json").unwrap();
    assert!(!json.contains("/Users/") && !json.contains("/home/"));
    let spec = spec::parse(&json).unwrap();
    assert_eq!(spec.validate(), vec![], "the example must be clean even under --strict");
    assert!(spec.scenes.iter().any(|s| s.kind == spec::SceneKind::Cloning));
    assert!(spec.scenes.iter().any(|s| s.kind == spec::SceneKind::Endcard));
    assert!(!spec.scoreboard.is_empty() && spec.engagement.is_some());
    let first_words: usize = spec.scenes[0].lines.iter().map(|l| l.split_whitespace().count()).sum();
    assert!(first_words <= 8, "the on-screen hook must stay short");
}
