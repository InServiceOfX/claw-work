//! Defense-in-depth checks of indexed blobs, never a guarantee of absence of all private facts.
use crate::{Result, input::excluded, process::run_bounded};
use regex::Regex;
use serde::Serialize;
use std::{path::Path, process::Command, sync::OnceLock, time::Duration};

#[derive(Debug, Serialize)]
pub struct Violation {
    pub path: String,
    pub rule: String,
}
fn patterns() -> &'static Vec<(&'static str, Regex)> {
    static RULES: OnceLock<Vec<(&str, Regex)>> = OnceLock::new();
    RULES.get_or_init(|| [
        ("aws_key", r"(?:AKIA|ASIA)[A-Z0-9]{16}"),
        ("provider_token", r"(?:sk-[A-Za-z0-9_-]{20,}|gh[pousr]_[A-Za-z0-9]{20,}|xox[baprs]-[A-Za-z0-9-]{15,})"),
        ("private_key", r"-----BEGIN (?:RSA |EC |OPENSSH |DSA )?PRIVATE KEY-----"),
        ("personal_absolute_path", r"(?:/home|/Users|/media)/[A-Za-z0-9_.-]+/"),
        ("credential_url", r"https?://[^/\s:@]+:[^/\s@]+@"),
        ("credential_assignment", r"(?m)^\s*(?:export\s+)?[A-Z0-9_]*(?:TOKEN|SECRET|PASSWORD|API_KEY)\s*=\s*[^\s$<]{8,}"),
    ].into_iter().map(|(name, re)| (name, Regex::new(re).expect("constant regex"))).collect())
}
pub fn scan_blob(path: &str, bytes: &[u8]) -> Vec<Violation> {
    let mut rules = vec![];
    if path.split('/').any(excluded)
        || path.ends_with(".jsonl")
        || path.ends_with(".sqlite")
        || path.ends_with(".db")
        || path.ends_with(".log")
        || path.ends_with(".csv")
    {
        rules.push("forbidden_artifact_path");
    }
    match std::str::from_utf8(bytes) {
        Ok(text) if !bytes.contains(&0) => {
            for (rule, re) in patterns() {
                if re.is_match(text) {
                    rules.push(rule);
                }
            }
        }
        _ => rules.push("binary_requires_explicit_publication_review"),
    }
    rules
        .into_iter()
        .map(|rule| Violation {
            path: path.into(),
            rule: rule.into(),
        })
        .collect()
}
fn git(repo: &Path, args: &[&str], max: usize) -> Result<Vec<u8>> {
    let r = run_bounded(
        Command::new("git")
            .args(args)
            .current_dir(repo)
            .env("GIT_OPTIONAL_LOCKS", "0")
            .env("GIT_NO_REPLACE_OBJECTS", "1"),
        Duration::from_secs(15),
        max,
    )?;
    if r.exit_code != Some(0) || r.output_limited || r.timed_out {
        return Err("Git privacy scan failed or exceeded its limits; no content printed".into());
    }
    Ok(r.stdout)
}
pub fn scan_index(repo: &Path) -> Result<Vec<Violation>> {
    let index = git(repo, &["ls-files", "--stage", "-z"], 16 * 1_048_576)?;
    let mut violations = vec![];
    for record in index.split(|b| *b == 0).filter(|r| !r.is_empty()) {
        let record =
            std::str::from_utf8(record).map_err(|_| "Non-UTF8 indexed path requires review")?;
        let (meta, path) = record.split_once('\t').ok_or("Malformed index record")?;
        let parts: Vec<_> = meta.split_whitespace().collect();
        if parts.len() != 3 || parts[2] != "0" {
            return Err("Unmerged/malformed index".into());
        }
        if parts[0] != "100644" && parts[0] != "100755" {
            violations.push(Violation {
                path: path.into(),
                rule: "symlink_or_submodule_requires_review".into(),
            });
            continue;
        }
        if !parts[1].bytes().all(|b| b.is_ascii_hexdigit()) {
            return Err("Malformed object ID".into());
        }
        let bytes = git(repo, &["cat-file", "blob", parts[1]], 8 * 1_048_576)?;
        violations.extend(scan_blob(path, &bytes));
    }
    Ok(violations)
}
pub const CONTEXT_FILES: &[&str] = &[
    ".dockerignore",
    "Dockerfile",
    "Dockerfile.source",
    "docker/entrypoint.sh",
    "docker/pins.json",
    "docker/pins-arm64.json",
    "docker/primitives.txt",
];
pub fn scan_context(root: &Path) -> Result<Vec<Violation>> {
    let ignore = std::fs::read_to_string(root.join(".dockerignore"))?;
    let expected = [
        "**",
        "!Dockerfile",
        "!Dockerfile.source",
        "!.dockerignore",
        "!docker/pins.json",
        "!docker/pins-arm64.json",
        "!docker/entrypoint.sh",
        "!docker/primitives.txt",
    ];
    let actual: Vec<_> = ignore
        .lines()
        .filter(|s| !s.trim().is_empty() && !s.starts_with('#'))
        .collect();
    if actual != expected {
        return Err("Docker context must use the reviewed exact file allowlist".into());
    }
    let mut violations = vec![];
    for path in CONTEXT_FILES {
        let full = root.join(path);
        crate::input::no_symlink_components(&full)?;
        violations.extend(scan_blob(path, &std::fs::read(full)?));
    }
    Ok(violations)
}
