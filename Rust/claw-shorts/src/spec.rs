//! The short's spec file and its validation.
//!
//! Validation is where the launch-kit lessons live:
//! * no placeholder text may reach a render (the old Python kit emitted
//!   "Tired of the old way?" and a literal `{name}`);
//! * superlatives ("killer", "best", "beats") need recorded evidence;
//! * on-screen text may only use glyphs the font can draw;
//! * "fact or hype" structure is checked as warnings: a visible source URL, a
//!   number, a verdict, and a disclosure when a voice is synthetic.

use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::style::Style;
use crate::text::{self, MONO_EM, PROP_EM};

pub const SPEC_VERSION: u32 = 1;

/// Words that make a promise the spec must back with evidence.
pub const CLAIM_TERMS: &[&str] = &[
    "killer",
    "best",
    "fastest",
    "#1",
    "number one",
    "beats",
    "better than",
    "faster than",
    "cheaper than",
    "replaces",
    "outperforms",
    "state of the art",
    "state-of-the-art",
];

/// Text that means a template was never filled in.
const PLACEHOLDERS: &[&str] = &[
    "todo",
    "tbd",
    "lorem",
    "show the pain point",
    "tired of the old way",
    "developers love",
];

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Spec {
    pub version: u32,
    pub project: String,
    /// Visible source URL, e.g. `github.com/Org/repo`. Optional but recommended.
    #[serde(default)]
    pub repo_url: Option<String>,
    pub title: String,
    pub caption: String,
    #[serde(default)]
    pub hashtags: Vec<String>,
    /// Synthetic-media disclosure, shown on screen and in the caption. Optional.
    #[serde(default)]
    pub disclosure: Option<String>,
    /// Credits for voices, models and sources. Optional.
    #[serde(default)]
    pub credits: Vec<String>,
    /// Evidence for every claim term used in the text.
    #[serde(default)]
    pub claims: Vec<Claim>,
    /// The close-reel "scoreboard": a few short real numbers shown from the
    /// first seconds and again on the end card, so a replay starts on the result.
    #[serde(default)]
    pub scoreboard: Vec<String>,
    /// The deliberate engagement device (a "viral token" design).
    #[serde(default)]
    pub engagement: Option<Engagement>,
    /// Voice used to narrate scenes that have no `audio` file.
    #[serde(default)]
    pub voice: Option<Voice>,
    #[serde(default)]
    pub style: Style,
    pub scenes: Vec<Scene>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Claim {
    pub term: String,
    pub evidence: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Engagement {
    /// What the device is, e.g. "judgment invite".
    pub device: String,
    /// Which token it earns: like, comment, save or share.
    pub token: String,
    /// Shown on the end card, so it must be short.
    pub prompt: String,
    /// Suggested pinned comment for the posting kit.
    #[serde(default)]
    pub pinned_comment: Option<String>,
}

pub const ENGAGEMENT_TOKENS: &[&str] = &["like", "comment", "save", "share"];

/// Openers that waste the first three seconds. Matched at the start of the
/// first scene's narration or on-screen text.
pub const GREETINGS: &[&str] = &[
    "hey guys",
    "hey you guys",
    "hi guys",
    "hello everyone",
    "welcome back",
    "welcome to my channel",
    "today we're going to",
    "today we are going to",
    "in this video",
];

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Voice {
    /// Reference recording of ONE speaker, relative to the spec file.
    pub reference: String,
    #[serde(default)]
    pub transcript: Option<String>,
    #[serde(default)]
    pub seed: Option<u64>,
    /// TTS executable; defaults to `voxcpm-tts`.
    #[serde(default)]
    pub binary: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SceneKind {
    Title,
    /// Reference voice in, generated voice out (two labels in `lines`).
    #[serde(rename = "clone")]
    Cloning,
    Terminal,
    Verdict,
    Endcard,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Tone {
    Pass,
    Warn,
    Fail,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Row {
    pub mark: String,
    pub tone: Tone,
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Scene {
    pub kind: SceneKind,
    /// What the voice says. Required; it sets the scene's length.
    pub narration: String,
    /// An existing WAV for this scene, relative to the spec; skips TTS.
    #[serde(default)]
    pub audio: Option<String>,
    /// `title` and `endcard`: centered lines, the last one in the accent color.
    #[serde(default)]
    pub lines: Vec<String>,
    /// `terminal`: the typed command and the output lines it prints.
    #[serde(default)]
    pub command: Option<String>,
    #[serde(default)]
    pub output: Vec<String>,
    /// `verdict`: marked rows.
    #[serde(default)]
    pub rows: Vec<Row>,
    /// `endcard`: the visible source URL.
    #[serde(default)]
    pub url: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Error,
    Warn,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Issue {
    pub severity: Severity,
    pub message: String,
}

impl Issue {
    fn error(message: impl Into<String>) -> Self {
        Self { severity: Severity::Error, message: message.into() }
    }
    fn warn(message: impl Into<String>) -> Self {
        Self { severity: Severity::Warn, message: message.into() }
    }
}

/// Maximum characters the terminal command may have (prompt included).
pub fn max_command_chars(style_min_size: u32) -> usize {
    (840.0 / (f64::from(style_min_size) * MONO_EM)).floor() as usize
}

/// Smallest monospace size the terminal scene will shrink to.
pub const TERMINAL_MIN_SIZE: u32 = 26;

pub fn parse(json: &str) -> Result<Spec, String> {
    serde_json::from_str(json).map_err(|e| format!("spec is not valid: {e}"))
}

impl Spec {
    /// Text that appears on screen (everything except narration, title, caption).
    pub fn on_screen_text(&self) -> Vec<&str> {
        let mut out: Vec<&str> = Vec::new();
        if let Some(d) = &self.disclosure {
            out.push(d);
        }
        out.extend(self.credits.iter().map(String::as_str));
        out.extend(self.scoreboard.iter().map(String::as_str));
        if let Some(e) = &self.engagement {
            out.push(&e.prompt);
        }
        for scene in &self.scenes {
            out.extend(scene.lines.iter().map(String::as_str));
            if let Some(c) = &scene.command {
                out.push(c);
            }
            out.extend(scene.output.iter().map(String::as_str));
            for row in &scene.rows {
                out.push(&row.mark);
                out.push(&row.text);
            }
            if let Some(u) = &scene.url {
                out.push(u);
            }
        }
        out
    }

    /// Every string a viewer can see or hear, plus the posting text.
    pub fn all_text(&self) -> Vec<&str> {
        let mut out = self.on_screen_text();
        out.push(&self.title);
        out.push(&self.caption);
        out.extend(self.hashtags.iter().map(String::as_str));
        out.extend(self.scenes.iter().map(|s| s.narration.as_str()));
        if let Some(u) = &self.repo_url {
            out.push(u);
        }
        if let Some(pinned) = self.engagement.as_ref().and_then(|e| e.pinned_comment.as_deref()) {
            out.push(pinned);
        }
        out
    }

    /// Validate the spec. Errors block a render; warnings are advice.
    pub fn validate(&self) -> Vec<Issue> {
        let mut issues = Vec::new();

        if self.version != SPEC_VERSION {
            issues.push(Issue::error(format!(
                "spec version {} is not supported (expected {SPEC_VERSION})",
                self.version
            )));
        }
        for (name, value) in [
            ("project", self.project.as_str()),
            ("title", self.title.as_str()),
            ("caption", self.caption.as_str()),
        ] {
            if value.trim().is_empty() {
                issues.push(Issue::error(format!("{name} must not be empty")));
            }
        }
        if self.scenes.is_empty() || self.scenes.len() > 12 {
            issues.push(Issue::error("a spec needs 1 to 12 scenes"));
        }
        for problem in self.style.problems() {
            issues.push(Issue::error(problem));
        }

        for text_item in self.all_text() {
            let lowered = text_item.to_lowercase();
            if text_item.chars().any(|c| c.is_control()) {
                issues.push(Issue::error(format!(
                    "text {text_item:?} contains a control character"
                )));
            }
            for placeholder in PLACEHOLDERS {
                if text::contains_term(&lowered, placeholder) {
                    issues.push(Issue::error(format!(
                        "placeholder text {placeholder:?} found in {text_item:?}; write the real copy"
                    )));
                }
            }
            if text_item.contains('{') || text_item.contains('}') {
                issues.push(Issue::error(format!(
                    "unfilled template braces in {text_item:?}"
                )));
            }
        }

        for text_item in self.on_screen_text() {
            if let Some(bad) = text::first_bad_glyph(text_item) {
                issues.push(Issue::error(format!(
                    "on-screen text {text_item:?} has {bad:?} (U+{:04X}), which the font cannot draw",
                    bad as u32
                )));
            }
        }

        for term in CLAIM_TERMS {
            let used = self.all_text().into_iter().any(|t| text::contains_term(t, term));
            if used {
                let backed = self.claims.iter().any(|c| {
                    c.term.eq_ignore_ascii_case(term) && !c.evidence.trim().is_empty()
                });
                if !backed {
                    issues.push(Issue::error(format!(
                        "the text uses the claim {term:?} without evidence; remove it or add a claims entry with evidence"
                    )));
                }
            }
        }
        for claim in &self.claims {
            if claim.evidence.trim().is_empty() {
                issues.push(Issue::error(format!(
                    "claim {:?} has empty evidence",
                    claim.term
                )));
            }
        }

        for (index, scene) in self.scenes.iter().enumerate() {
            self.validate_scene(index, scene, &mut issues);
        }
        self.hook_and_engagement(&mut issues);

        self.fact_or_hype_warnings(&mut issues);
        issues
    }

    fn validate_scene(&self, index: usize, scene: &Scene, issues: &mut Vec<Issue>) {
        let label = format!("scene {} ({:?})", index + 1, scene.kind);
        if scene.narration.trim().is_empty() {
            issues.push(Issue::error(format!("{label}: narration must not be empty")));
        }
        let mut forbid = |present: bool, field: &str| {
            if present {
                issues.push(Issue::error(format!(
                    "{label}: `{field}` does not apply to this scene kind"
                )));
            }
        };
        match scene.kind {
            SceneKind::Title | SceneKind::Cloning => {
                forbid(scene.command.is_some(), "command");
                forbid(!scene.output.is_empty(), "output");
                forbid(!scene.rows.is_empty(), "rows");
                forbid(scene.url.is_some(), "url");
            }
            SceneKind::Terminal => {
                forbid(!scene.lines.is_empty(), "lines");
                forbid(!scene.rows.is_empty(), "rows");
                forbid(scene.url.is_some(), "url");
            }
            SceneKind::Verdict => {
                forbid(!scene.lines.is_empty(), "lines");
                forbid(scene.command.is_some(), "command");
                forbid(!scene.output.is_empty(), "output");
                forbid(scene.url.is_some(), "url");
            }
            SceneKind::Endcard => {
                forbid(scene.command.is_some(), "command");
                forbid(!scene.output.is_empty(), "output");
                forbid(!scene.rows.is_empty(), "rows");
            }
        }
        match scene.kind {
            SceneKind::Title if scene.lines.is_empty() || scene.lines.len() > 6 => {
                issues.push(Issue::error(format!("{label}: needs 1 to 6 lines")));
            }
            SceneKind::Cloning if scene.lines.len() != 2 => {
                issues.push(Issue::error(format!(
                    "{label}: a clone scene needs exactly 2 lines (input label, output label)"
                )));
            }
            SceneKind::Terminal => match &scene.command {
                None => issues.push(Issue::error(format!("{label}: needs a command"))),
                Some(cmd) => {
                    let limit = max_command_chars(TERMINAL_MIN_SIZE).saturating_sub(2);
                    if cmd.chars().count() > limit {
                        issues.push(Issue::error(format!(
                            "{label}: command has {} characters; at most {limit} fit on one line",
                            cmd.chars().count()
                        )));
                    }
                    if scene.output.len() > 8 {
                        issues.push(Issue::error(format!("{label}: at most 8 output lines")));
                    }
                }
            },
            SceneKind::Verdict if scene.rows.is_empty() || scene.rows.len() > 5 => {
                issues.push(Issue::error(format!("{label}: needs 1 to 5 rows")));
            }
            SceneKind::Endcard if scene.url.as_deref().is_none_or(|u| u.trim().is_empty()) => {
                issues.push(Issue::error(format!("{label}: an endcard needs a url")));
            }
            _ => {}
        }
        for line in &scene.lines {
            if line.trim().is_empty() {
                issues.push(Issue::error(format!("{label}: lines must not be blank")));
            }
            let chars = line.chars().count();
            if 40.0 * PROP_EM * chars as f64 > 900.0 {
                issues.push(Issue::error(format!(
                    "{label}: line {line:?} is too long to fit even at the smallest size"
                )));
            }
        }
        for row in &scene.rows {
            if row.mark.trim().is_empty() || row.text.trim().is_empty() {
                issues.push(Issue::error(format!("{label}: rows need a mark and text")));
            }
            if row.mark.chars().count() > 14 {
                issues.push(Issue::error(format!(
                    "{label}: mark {:?} is too long for its badge (14 characters max)",
                    row.mark
                )));
            }
        }
    }

    /// Part 1 lessons: no greeting opens, a short on-screen hook, a scoreboard
    /// of real numbers, and a deliberately designed engagement device.
    fn hook_and_engagement(&self, issues: &mut Vec<Issue>) {
        if let Some(first) = self.scenes.first() {
            let mut openers: Vec<&str> = vec![first.narration.as_str()];
            openers.extend(first.lines.iter().map(String::as_str));
            for opener in openers {
                let lowered = opener.trim_start().to_lowercase();
                if let Some(greeting) = GREETINGS.iter().find(|g| lowered.starts_with(**g)) {
                    issues.push(Issue::error(format!(
                        "the opening starts with the greeting {greeting:?}; open mid-claim instead"
                    )));
                }
            }
            let hook_words: usize = first.lines.iter().map(|l| l.split_whitespace().count()).sum();
            if hook_words > 8 {
                issues.push(Issue::warn(format!(
                    "the on-screen hook is {hook_words} words; about 8 or fewer reads at a glance when muted"
                )));
            }
        }
        if self.scoreboard.len() > 4 {
            issues.push(Issue::error("the scoreboard holds at most 4 short numbers"));
        }
        for part in &self.scoreboard {
            if part.trim().is_empty() || part.chars().count() > 24 {
                issues.push(Issue::error(format!(
                    "scoreboard entry {part:?} must be 1 to 24 characters"
                )));
            }
        }
        match &self.engagement {
            None => issues.push(Issue::warn(
                "no engagement device; design at least one reason to comment, save or share",
            )),
            Some(e) => {
                if !ENGAGEMENT_TOKENS.contains(&e.token.as_str()) {
                    issues.push(Issue::error(format!(
                        "engagement token {:?} must be one of {ENGAGEMENT_TOKENS:?}",
                        e.token
                    )));
                }
                if e.device.trim().is_empty() || e.prompt.trim().is_empty() {
                    issues.push(Issue::error("engagement needs a device and a prompt"));
                }
                if e.prompt.chars().count() > 64 {
                    issues.push(Issue::error("the engagement prompt must be 64 characters or fewer"));
                }
            }
        }
    }

    fn fact_or_hype_warnings(&self, issues: &mut Vec<Issue>) {
        let has_url = self.scenes.iter().any(|s| s.url.is_some())
            || self
                .on_screen_text()
                .iter()
                .any(|t| t.contains("github.com") || t.contains("http"));
        if !has_url {
            issues.push(Issue::warn(
                "no visible source URL; show where viewers can verify the claim (an endcard with a url)",
            ));
        }
        if !self
            .on_screen_text()
            .iter()
            .any(|t| t.chars().any(|c| c.is_ascii_digit()))
        {
            issues.push(Issue::warn(
                "no number on screen; add a measured result such as time, cost or size",
            ));
        }
        if !self.scenes.iter().any(|s| s.kind == SceneKind::Verdict) {
            issues.push(Issue::warn("no verdict scene; end on a clear takeaway"));
        }
        let narrated = self.voice.is_some() || self.scenes.iter().any(|s| s.audio.is_some());
        if narrated && self.disclosure.as_deref().is_none_or(|d| d.trim().is_empty()) {
            issues.push(Issue::warn(
                "narrated with a synthetic voice but no disclosure; platforms may require one",
            ));
        }
        for scene in &self.scenes {
            if scene.audio.is_none() && self.voice.is_none() {
                issues.push(Issue::warn(
                    "a scene has no audio file and the spec has no voice; render will fail until one is set",
                ));
                break;
            }
        }
        let words: usize = self.scenes.iter().map(|s| s.narration.split_whitespace().count()).sum();
        if words > 150 {
            issues.push(Issue::warn(format!(
                "narration is {words} words (about {} seconds); shorts hold attention best under 45 seconds",
                words * 60 / 150
            )));
        }
    }

    /// Footer text shown on every scene: disclosure then credits.
    pub fn footer_text(&self) -> Option<String> {
        let mut parts: Vec<&str> = Vec::new();
        if let Some(d) = self.disclosure.as_deref().filter(|d| !d.trim().is_empty()) {
            parts.push(d);
        }
        parts.extend(self.credits.iter().map(String::as_str).filter(|c| !c.trim().is_empty()));
        if parts.is_empty() { None } else { Some(parts.join("  |  ")) }
    }
}

/// Resolve a path from the spec against the spec file's directory.
pub fn resolve(spec_dir: &Path, relative: &str) -> std::path::PathBuf {
    let p = Path::new(relative);
    if p.is_absolute() { p.to_path_buf() } else { spec_dir.join(p) }
}

#[cfg(test)]
mod tests {
    use super::*;

    pub(crate) fn good() -> Spec {
        parse(
            r##"{
              "version": 1, "project": "demo", "repo_url": "github.com/Org/demo",
              "title": "Local voice cloning", "caption": "Runs on a Mac.",
              "hashtags": ["#opensource"], "disclosure": "AI-generated voice",
              "credits": ["Voice: A Reader, LibriVox (CC0)"],
              "scoreboard": ["8 s in", "$0"],
              "engagement": {"device": "judgment invite", "token": "comment",
                             "prompt": "Could you tell? Comment the second you knew."},
              "voice": {"reference": "ref.flac", "seed": 42},
              "scenes": [
                {"kind": "title", "narration": "No cloud.", "lines": ["No cloud.", "Fact or hype?"]},
                {"kind": "terminal", "narration": "Here is the run.", "command": "demo say a.txt",
                 "output": ["21s of audio in 32s"]},
                {"kind": "verdict", "narration": "Fact.", "rows": [
                  {"mark": "FACT", "tone": "pass", "text": "Runs locally."}]},
                {"kind": "endcard", "narration": "On GitHub.", "lines": ["demo"], "url": "github.com/Org/demo"}
              ]}"##,
        )
        .unwrap()
    }

    fn errors(spec: &Spec) -> Vec<String> {
        spec.validate()
            .into_iter()
            .filter(|i| i.severity == Severity::Error)
            .map(|i| i.message)
            .collect()
    }

    #[test]
    fn a_complete_spec_has_no_errors_or_warnings() {
        let spec = good();
        assert_eq!(spec.validate(), vec![]);
    }

    #[test]
    fn placeholders_are_rejected() {
        let mut spec = good();
        spec.scenes[0].lines[0] = "TODO hook".into();
        spec.caption = "Tired of the old way? Try it.".into();
        let errs = errors(&spec).join("\n");
        assert!(errs.contains("todo"), "{errs}");
        assert!(errs.contains("tired of the old way"), "{errs}");
    }

    #[test]
    fn template_braces_are_rejected() {
        let mut spec = good();
        spec.scenes[2].rows[0].text = "After: the {name} way".into();
        assert!(errors(&spec).iter().any(|e| e.contains("braces")));
    }

    #[test]
    fn claims_need_evidence() {
        let mut spec = good();
        spec.title = "The voice cloning killer".into();
        assert!(errors(&spec).iter().any(|e| e.contains("killer")));
        spec.claims.push(Claim { term: "killer".into(), evidence: "".into() });
        assert!(errors(&spec).iter().any(|e| e.contains("empty evidence")));
        spec.claims[0].evidence = "blind A/B on 20 listeners, see docs/AB.md".into();
        assert_eq!(errors(&spec), Vec::<String>::new());
    }

    #[test]
    fn narration_claims_count_too() {
        let mut spec = good();
        spec.scenes[0].narration = "It is better than ElevenLabs.".into();
        assert!(errors(&spec).iter().any(|e| e.contains("better than")));
    }

    #[test]
    fn undrawable_glyphs_are_rejected_on_screen_only() {
        let mut spec = good();
        spec.scenes[1].output = vec!["\u{2713} Wrote file".into()];
        assert!(errors(&spec).iter().any(|e| e.contains("U+2713")));
        let mut spec = good();
        spec.scenes[0].narration = "A check \u{2713} is fine to say.".into();
        assert!(errors(&spec).is_empty());
    }

    #[test]
    fn fields_must_match_the_scene_kind() {
        let mut spec = good();
        spec.scenes[0].command = Some("ls".into());
        assert!(errors(&spec).iter().any(|e| e.contains("`command`")));
    }

    #[test]
    fn long_commands_and_marks_are_errors() {
        let mut spec = good();
        spec.scenes[1].command = Some("x".repeat(80));
        assert!(errors(&spec).iter().any(|e| e.contains("command has")));
        let mut spec = good();
        spec.scenes[2].rows[0].mark = "A VERY LONG BADGE TEXT".into();
        assert!(errors(&spec).iter().any(|e| e.contains("badge")));
    }

    #[test]
    fn unknown_fields_fail_to_parse() {
        let err = parse(r#"{"version":1,"project":"p","title":"t","caption":"c","scenes":[],"sparkle":1}"#);
        assert!(err.is_err());
    }

    #[test]
    fn optional_pieces_warn_but_do_not_error() {
        let mut spec = good();
        spec.disclosure = None;
        spec.credits.clear();
        spec.repo_url = None;
        spec.scenes.pop();
        let issues = spec.validate();
        assert!(issues.iter().all(|i| i.severity == Severity::Warn), "{issues:?}");
        let text = format!("{issues:?}");
        assert!(text.contains("no disclosure") && text.contains("no visible source URL"));
        assert!(spec.footer_text().is_none());
    }

    #[test]
    fn greeting_opens_are_errors() {
        let mut spec = good();
        spec.scenes[0].narration = "Hey guys, welcome back to my channel.".into();
        assert!(errors(&spec).iter().any(|e| e.contains("hey guys")));
        let mut spec = good();
        spec.scenes[0].lines = vec!["Welcome back".into()];
        assert!(errors(&spec).iter().any(|e| e.contains("welcome back")));
    }

    #[test]
    fn long_hooks_and_missing_engagement_warn() {
        let mut spec = good();
        spec.scenes[0].lines = vec!["one two three four five".into(), "six seven eight nine".into()];
        spec.engagement = None;
        let warnings: Vec<String> = spec.validate().into_iter()
            .filter(|i| i.severity == Severity::Warn).map(|i| i.message).collect();
        assert!(warnings.iter().any(|w| w.contains("9 words")), "{warnings:?}");
        assert!(warnings.iter().any(|w| w.contains("no engagement device")), "{warnings:?}");
    }

    #[test]
    fn engagement_and_scoreboard_are_validated() {
        let mut spec = good();
        spec.engagement = Some(Engagement {
            device: "judgment invite".into(), token: "comment".into(),
            prompt: "Could you tell? Comment the second you knew.".into(), pinned_comment: None,
        });
        spec.scoreboard = vec!["8 s in".into(), "$0".into()];
        assert_eq!(errors(&spec), Vec::<String>::new());
        spec.engagement.as_mut().unwrap().token = "scream".into();
        spec.scoreboard.push("x".repeat(30));
        let errs = errors(&spec).join("\n");
        assert!(errs.contains("token") && errs.contains("scoreboard entry"), "{errs}");
    }

    #[test]
    fn a_clone_scene_needs_two_labels() {
        let mut spec = good();
        spec.scenes[0].kind = SceneKind::Cloning;
        spec.scenes[0].lines = vec!["only one".into()];
        assert!(errors(&spec).iter().any(|e| e.contains("exactly 2 lines")));
        spec.scenes[0].lines = vec!["8 s clip".into(), "anything".into()];
        assert_eq!(errors(&spec), Vec::<String>::new());
    }

    #[test]
    fn footer_joins_disclosure_and_credits() {
        assert_eq!(
            good().footer_text().unwrap(),
            "AI-generated voice  |  Voice: A Reader, LibriVox (CC0)"
        );
    }
}
