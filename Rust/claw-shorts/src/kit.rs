//! The posting kit: ready-to-paste text plus a pre-post checklist. Posting is
//! always manual; nothing here talks to a platform.

use std::fs;
use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::spec::Spec;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct Kit {
    pub project: String,
    pub title: String,
    /// Caption with source, credits and disclosure appended.
    pub caption: String,
    pub hashtags: Vec<String>,
    pub duration_ms: Option<u64>,
    pub checklist: Vec<String>,
}

pub fn build(spec: &Spec, duration_ms: Option<u64>) -> Kit {
    let mut caption = spec.caption.trim().to_string();
    if let Some(url) = spec.repo_url.as_deref().filter(|u| !u.trim().is_empty()) {
        caption.push_str(&format!("\n\nSource: {url}"));
    }
    for credit in spec.credits.iter().filter(|c| !c.trim().is_empty()) {
        caption.push_str(&format!("\n{credit}"));
    }
    if let Some(d) = spec.disclosure.as_deref().filter(|d| !d.trim().is_empty()) {
        caption.push_str(&format!("\n{d}"));
    }
    let hashtags: Vec<String> = spec
        .hashtags
        .iter()
        .map(|h| if h.starts_with('#') { h.clone() } else { format!("#{h}") })
        .collect();

    let mut checklist = vec![
        "Listen to the whole video with sound; the render check cannot judge speech quality.".to_string(),
        "Open contact.png and look for clipped text, low contrast and text covered by platform buttons.".to_string(),
    ];
    if spec.repo_url.is_some() {
        checklist.push("Confirm the linked repository is public, pushed, and has its LICENSE.".into());
    }
    if spec.disclosure.as_deref().is_none_or(|d| d.trim().is_empty()) {
        checklist.push("The voice is synthetic and no disclosure is set; add one if the platform asks.".into());
    }
    if spec.credits.is_empty() {
        checklist.push("No credits are set; credit any voice, model or source whose license asks for it.".into());
    }
    checklist.push("Post by hand. This tool never uploads.".into());

    Kit {
        project: spec.project.clone(),
        title: spec.title.clone(),
        caption,
        hashtags,
        duration_ms,
        checklist,
    }
}

pub fn markdown(kit: &Kit) -> String {
    let mut out = format!("# Posting kit: {}\n\n", kit.project);
    if let Some(ms) = kit.duration_ms {
        out.push_str(&format!("Length: {:.1} s\n\n", ms as f64 / 1000.0));
    }
    out.push_str(&format!("## Title\n\n{}\n\n## Caption\n\n{}\n\n", kit.title, kit.caption));
    out.push_str(&format!("## Hashtags\n\n{}\n\n## Before you post\n\n", kit.hashtags.join(" ")));
    for item in &kit.checklist {
        out.push_str(&format!("- [ ] {item}\n"));
    }
    out
}

/// Write `posting_kit.json` and `posting_kit.md`.
pub fn write(spec: &Spec, duration_ms: u64, dir: &Path) -> Result<(PathBuf, PathBuf), String> {
    write_kit(&build(spec, Some(duration_ms)), dir)
}

pub fn write_kit(kit: &Kit, dir: &Path) -> Result<(PathBuf, PathBuf), String> {
    fs::create_dir_all(dir).map_err(|e| format!("cannot create {}: {e}", dir.display()))?;
    let json = dir.join("posting_kit.json");
    let md = dir.join("posting_kit.md");
    fs::write(&json, serde_json::to_string_pretty(kit).map_err(|e| e.to_string())?)
        .map_err(|e| format!("cannot write {}: {e}", json.display()))?;
    fs::write(&md, markdown(kit)).map_err(|e| format!("cannot write {}: {e}", md.display()))?;
    Ok((json, md))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spec::parse;

    fn spec(extra: &str) -> Spec {
        parse(&format!(
            r##"{{"version":1,"project":"demo","title":"Local voice","caption":"Runs on a Mac.",
              "hashtags":["opensource","#rust"],"scenes":[{{"kind":"title","narration":"n","lines":["a"]}}]{extra}}}"##
        ))
        .unwrap()
    }

    #[test]
    fn caption_appends_source_credits_and_disclosure() {
        let kit = build(
            &spec(r#","repo_url":"github.com/Org/demo","credits":["Voice: A Reader"],"disclosure":"AI-generated voice""#),
            Some(22_000),
        );
        assert_eq!(
            kit.caption,
            "Runs on a Mac.\n\nSource: github.com/Org/demo\nVoice: A Reader\nAI-generated voice"
        );
        assert_eq!(kit.hashtags, vec!["#opensource", "#rust"]);
        assert!(markdown(&kit).contains("Length: 22.0 s"));
    }

    #[test]
    fn optional_pieces_become_checklist_items_not_text() {
        let kit = build(&spec(""), None);
        assert_eq!(kit.caption, "Runs on a Mac.");
        let list = kit.checklist.join("\n");
        assert!(list.contains("no disclosure") && list.contains("No credits"), "{list}");
        assert!(!list.contains("LICENSE"));
    }

    #[test]
    fn writes_both_files() {
        let dir = tempfile::tempdir().unwrap();
        let (json, md) = write(&spec(""), 5000, &dir.path().join("kit")).unwrap();
        assert!(fs::read_to_string(json).unwrap().contains("\"duration_ms\": 5000"));
        assert!(fs::read_to_string(md).unwrap().starts_with("# Posting kit: demo"));
    }
}
