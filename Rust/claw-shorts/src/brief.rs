//! `brief`: read a project and write a spec *template* plus evidence candidates.
//!
//! The template deliberately cannot be rendered: its copy is `TODO` text that
//! `check` rejects. The brief never invents a hook, a pain point or a testimonial;
//! it only collects facts and measured results the project already documents, so
//! a human or agent writes copy from real evidence.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::json;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Facts {
    pub name: String,
    pub description: Option<String>,
    pub license: Option<String>,
    pub license_file: bool,
    pub repo_url: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Candidate {
    pub file: String,
    pub line: usize,
    pub text: String,
}

fn toml_value(toml: &str, key: &str) -> Option<String> {
    toml.lines().find_map(|line| {
        let rest = line.trim().strip_prefix(key)?.trim_start().strip_prefix('=')?;
        Some(rest.trim().trim_matches('"').to_string())
    })
}

/// `git@github.com:Org/repo.git` or `https://github.com/Org/repo.git` to `github.com/Org/repo`.
pub fn normalize_remote(remote: &str) -> Option<String> {
    let trimmed = remote.trim().trim_end_matches(".git");
    let stripped = trimmed
        .strip_prefix("git@")
        .map(|s| s.replacen(':', "/", 1))
        .or_else(|| trimmed.strip_prefix("https://").map(str::to_string))
        .or_else(|| trimmed.strip_prefix("ssh://git@").map(str::to_string))?;
    if stripped.contains('/') { Some(stripped) } else { None }
}

fn git(dir: &Path, args: &[&str]) -> Option<String> {
    let out = Command::new("git").arg("-C").arg(dir).args(args).output().ok()?;
    if out.status.success() {
        Some(String::from_utf8_lossy(&out.stdout).trim().to_string()).filter(|s| !s.is_empty())
    } else {
        None
    }
}

fn repo_url(dir: &Path) -> Option<String> {
    let base = normalize_remote(&git(dir, &["remote", "get-url", "origin"])?)?;
    let branch = git(dir, &["symbolic-ref", "--short", "refs/remotes/origin/HEAD"])
        .and_then(|b| b.strip_prefix("origin/").map(str::to_string))
        .unwrap_or_else(|| "main".to_string());
    match git(dir, &["rev-parse", "--show-prefix"]) {
        Some(prefix) => Some(format!("{base}/tree/{branch}/{}", prefix.trim_end_matches('/'))),
        None => Some(base),
    }
}

fn license_file_near(dir: &Path) -> bool {
    let mut current = Some(dir);
    while let Some(d) = current {
        if let Ok(entries) = fs::read_dir(d) {
            if entries
                .flatten()
                .any(|e| e.file_name().to_string_lossy().to_uppercase().starts_with("LICENSE"))
            {
                return true;
            }
        }
        if d.join(".git").exists() {
            break;
        }
        current = d.parent();
    }
    false
}

pub fn read_facts(dir: &Path) -> Facts {
    let cargo = fs::read_to_string(dir.join("Cargo.toml")).unwrap_or_default();
    let name = toml_value(&cargo, "name").unwrap_or_else(|| {
        dir.canonicalize()
            .ok()
            .and_then(|p| p.file_name().map(|n| n.to_string_lossy().into_owned()))
            .unwrap_or_else(|| "project".into())
    });
    Facts {
        name,
        description: toml_value(&cargo, "description"),
        license: toml_value(&cargo, "license"),
        license_file: license_file_near(dir),
        repo_url: repo_url(dir),
    }
}

const KEYWORDS: &[&str] = &[
    "real time", "real-time", "tests", "passed", " gb", " mb", " ms", "seconds", "audio", "hz", "faster", "latency",
];

fn markdown_files(dir: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    for root in [dir.to_path_buf(), dir.join("docs")] {
        if let Ok(entries) = fs::read_dir(&root) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().is_some_and(|e| e == "md") && path.is_file() {
                    files.push(path);
                }
            }
        }
    }
    files.sort();
    files
}

/// Lines in the README and docs that state a measured number. These are
/// *candidates*: a human confirms each one before it goes on screen.
pub fn proof_candidates(dir: &Path) -> Vec<Candidate> {
    let mut out = Vec::new();
    for path in markdown_files(dir) {
        let Ok(content) = fs::read_to_string(&path) else { continue };
        let rel = path.strip_prefix(dir).unwrap_or(&path).to_string_lossy().into_owned();
        for (i, line) in content.lines().enumerate() {
            let lowered = line.to_lowercase();
            if line.chars().any(|c| c.is_ascii_digit())
                && KEYWORDS.iter().any(|k| lowered.contains(k))
                && !line.trim_start().starts_with('#')
                && !line.trim_start().starts_with('|')
            {
                let text: String = line
                    .trim()
                    .trim_start_matches(['-', '*', ' '])
                    .trim_start_matches("[x]")
                    .trim_start_matches("[ ]")
                    .trim()
                    .chars()
                    .take(220)
                    .collect();
                out.push(Candidate { file: rel.clone(), line: i + 1, text });
            }
        }
    }
    out.truncate(25);
    out
}

/// A spec skeleton whose copy is all `TODO`, so it fails `check` until written.
pub fn template(facts: &Facts) -> String {
    let todo = |what: &str| format!("TODO {what}");
    let mut spec = json!({
        "version": 1,
        "project": facts.name,
        "title": todo("title"),
        "caption": todo("caption"),
        "hashtags": [],
        "claims": [],
        "voice": { "reference": "reference.flac", "seed": 42 },
        "scenes": [
            { "kind": "title", "narration": todo("hook: a testable claim, spoken"),
              "lines": [todo("claim line 1"), todo("claim line 2")] },
            { "kind": "terminal", "narration": todo("describe the real run and its measured result"),
              "command": todo("the real command"), "output": [todo("real output line")] },
            { "kind": "verdict", "narration": todo("the verdict, including what was not tested"),
              "rows": [ { "mark": "FACT", "tone": "pass", "text": todo("a claim you can show") },
                        { "mark": "UNTESTED", "tone": "warn", "text": todo("a claim you did not test") } ] },
            { "kind": "endcard", "narration": todo("where to find the code"),
              "lines": [facts.name], "url": facts.repo_url.clone().unwrap_or_else(|| todo("source url")) }
        ]
    });
    if let Some(license) = &facts.license {
        spec["credits"] = json!([todo(&format!("credits (project license: {license})"))]);
    }
    serde_json::to_string_pretty(&spec).expect("static json")
}

pub fn candidates_markdown(facts: &Facts, candidates: &[Candidate]) -> String {
    let mut out = format!("# Evidence for {}\n\n", facts.name);
    out.push_str("Facts read from the project (confirm before use):\n\n");
    out.push_str(&format!("- description: {}\n", facts.description.as_deref().unwrap_or("none found")));
    out.push_str(&format!("- license field: {}\n", facts.license.as_deref().unwrap_or("none found")));
    out.push_str(&format!(
        "- LICENSE file: {}\n",
        if facts.license_file { "found" } else { "NOT FOUND; add one before linking the repo" }
    ));
    out.push_str(&format!("- source url: {}\n\n", facts.repo_url.as_deref().unwrap_or("none found")));
    out.push_str("## Measured-result candidates\n\n");
    out.push_str("Each line comes from the project's own docs. Re-run or re-read the source before putting a number on screen.\n\n");
    if candidates.is_empty() {
        out.push_str("None found. Run the tool and record a real measurement.\n");
    }
    for c in candidates {
        out.push_str(&format!("- `{}:{}` {}\n", c.file, c.line, c.text));
    }
    out
}

/// Write `short.template.json` and `evidence.md` into a new directory.
pub fn write_brief(project_dir: &Path, out_dir: &Path) -> Result<(PathBuf, PathBuf, Facts), String> {
    if out_dir.exists() {
        return Err(format!("{} already exists; choose a new output directory", out_dir.display()));
    }
    let facts = read_facts(project_dir);
    let candidates = proof_candidates(project_dir);
    fs::create_dir_all(out_dir).map_err(|e| format!("cannot create {}: {e}", out_dir.display()))?;
    let spec_path = out_dir.join("short.template.json");
    let evidence_path = out_dir.join("evidence.md");
    fs::write(&spec_path, template(&facts)).map_err(|e| e.to_string())?;
    fs::write(&evidence_path, candidates_markdown(&facts, &candidates)).map_err(|e| e.to_string())?;
    Ok((spec_path, evidence_path, facts))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spec::{parse, Severity};

    fn project() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        fs::write(
            dir.path().join("Cargo.toml"),
            "[package]\nname = \"demo-tts\"\nlicense = \"MIT\"\ndescription = \"Local TTS\"\n",
        )
        .unwrap();
        fs::write(dir.path().join("README.md"), "# demo\n\nFast. Runs in 32 seconds of audio per run.\n").unwrap();
        fs::create_dir(dir.path().join("docs")).unwrap();
        fs::write(
            dir.path().join("docs/HANDOFF.md"),
            "# Status\n\n- [x] Native say: 1.49\u{d7} real time on the dev Mac.\n- [x] Unrelated line without numbers\n| table | 5 tests |\n",
        )
        .unwrap();
        dir
    }

    #[test]
    fn reads_cargo_facts() {
        let dir = project();
        let facts = read_facts(dir.path());
        assert_eq!(facts.name, "demo-tts");
        assert_eq!(facts.license.as_deref(), Some("MIT"));
        assert_eq!(facts.description.as_deref(), Some("Local TTS"));
    }

    #[test]
    fn collects_measured_lines_but_not_headings_or_tables() {
        let dir = project();
        let found = proof_candidates(dir.path());
        let texts: Vec<&str> = found.iter().map(|c| c.text.as_str()).collect();
        assert!(texts.iter().any(|t| t.contains("1.49\u{d7} real time")), "{texts:?}");
        assert!(texts.iter().any(|t| t.contains("32 seconds")), "{texts:?}");
        assert!(!texts.iter().any(|t| t.contains("5 tests") || t.contains("Unrelated")), "{texts:?}");
        assert!(found.iter().any(|c| c.file == "docs/HANDOFF.md" && c.line == 3));
    }

    #[test]
    fn the_template_parses_but_never_validates() {
        let dir = project();
        let template = template(&read_facts(dir.path()));
        let spec = parse(&template).expect("template must parse");
        let errors: Vec<_> = spec.validate().into_iter().filter(|i| i.severity == Severity::Error).collect();
        assert!(errors.len() >= 5, "template must fail loudly: {errors:?}");
    }

    #[test]
    fn writes_into_a_new_directory_only() {
        let dir = project();
        let out = dir.path().join("brief-out");
        let (spec, evidence, _) = write_brief(dir.path(), &out).unwrap();
        assert!(spec.is_file() && evidence.is_file());
        assert!(fs::read_to_string(evidence).unwrap().contains("1.49"));
        assert!(write_brief(dir.path(), &out).unwrap_err().contains("already exists"));
    }

    #[test]
    fn normalizes_remotes() {
        assert_eq!(normalize_remote("git@github.com:Org/repo.git").as_deref(), Some("github.com/Org/repo"));
        assert_eq!(normalize_remote("https://github.com/Org/repo.git").as_deref(), Some("github.com/Org/repo"));
        assert_eq!(normalize_remote("nonsense"), None);
    }
}
