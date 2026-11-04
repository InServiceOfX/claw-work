use claw_lean_agent::{
    input::{InventoryLimits, create_private_dir, inventory_target, prepare_review},
    privacy::{scan_blob, scan_index},
};
use std::{fs, process::Command};
use tempfile::tempdir;

#[test]
fn inventory_is_bounded_private_and_does_not_follow_links() {
    let t = tempdir().unwrap();
    let src = t.path().join("src");
    fs::create_dir(&src).unwrap();
    fs::write(src.join("good.rs"), "pub fn f() {}").unwrap();
    fs::write(src.join(".env"), "DO_NOT_READ=synthetic").unwrap();
    fs::create_dir(src.join("target")).unwrap();
    fs::write(src.join("target/generated.rs"), "generated").unwrap();
    fs::write(src.join("binary"), [0, 1, 2]).unwrap();
    #[cfg(unix)]
    std::os::unix::fs::symlink(t.path().join("outside"), src.join("escape")).unwrap();
    let inv = inventory_target(&src, &InventoryLimits::default()).unwrap();
    assert_eq!(inv.files.len(), 1);
    assert_eq!(inv.files[0].path, "good.rs");
    assert_eq!(inv.files[0].sha256.len(), 64);
    assert!(inv.skipped.len() >= 3);
    assert_eq!(
        fs::read_to_string(src.join(".env")).unwrap(),
        "DO_NOT_READ=synthetic"
    );
    let small = InventoryLimits {
        max_file_bytes: 2,
        ..Default::default()
    };
    assert!(inventory_target(&src, &small).unwrap().files.is_empty());
}

#[test]
fn private_output_rejects_git_tree_existing_dir_and_links() {
    let t = tempdir().unwrap();
    let git = t.path().join("repo");
    fs::create_dir(&git).unwrap();
    fs::create_dir(git.join(".git")).unwrap();
    assert!(create_private_dir(&git.join("report")).is_err());
    assert!(create_private_dir(t.path()).is_err());
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(t.path(), t.path().join("link")).unwrap();
        assert!(create_private_dir(&t.path().join("link/out")).is_err());
    }
    let out = t.path().join("private");
    create_private_dir(&out).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(
            fs::metadata(out).unwrap().permissions().mode() & 0o777,
            0o700
        );
    }
}

#[test]
fn prepare_is_a_snapshot_not_a_source_rewrite_or_a_proof() {
    let t = tempdir().unwrap();
    let source = t.path().join("one.rs");
    fs::write(&source, "fn sample() {}").unwrap();
    let out = t.path().join("review");
    let bundle = prepare_review(&source, &out, &Default::default()).unwrap();
    assert_eq!(bundle.files.len(), 1);
    assert!(out.join("inventory.json").is_file());
    assert!(out.join("source/one.rs").is_file());
    assert_eq!(fs::read_to_string(source).unwrap(), "fn sample() {}");
}

#[test]
fn gate_rejects_patterns_and_artifacts_without_echoing_secrets() {
    assert!(scan_blob("src/good.rs", b"fn main() {}").is_empty());
    assert!(!scan_blob(".env", b"harmless").is_empty());
    assert!(!scan_blob(".pdd/log.json", b"{}").is_empty());
    let token = ["AKIA", "ABCDEFGHIJKLMNOP"].concat();
    let violations = scan_blob("config.txt", token.as_bytes());
    assert!(!violations.is_empty());
    assert!(!serde_json::to_string(&violations).unwrap().contains(&token));
    let path = ["/home/", "synthetic-person", "/private.txt"].concat();
    assert!(!scan_blob("doc.md", path.as_bytes()).is_empty());
    let key = ["-----BEGIN ", "PRIVATE KEY-----"].concat();
    assert!(!scan_blob("config.txt", key.as_bytes()).is_empty());
}

#[test]
fn gate_reads_index_not_repaired_working_copy() {
    let t = tempdir().unwrap();
    let run = |args: &[&str]| {
        assert!(
            Command::new("git")
                .args(args)
                .current_dir(t.path())
                .env("GIT_CONFIG_GLOBAL", "/dev/null")
                .env("GIT_CONFIG_NOSYSTEM", "1")
                .output()
                .unwrap()
                .status
                .success()
        );
    };
    run(&["init", "--template="]);
    let token = ["AKIA", "ABCDEFGHIJKLMNOP"].concat();
    fs::write(t.path().join("config.txt"), &token).unwrap();
    run(&["add", "config.txt"]);
    fs::write(t.path().join("config.txt"), "safe working copy").unwrap();
    assert!(!scan_index(t.path()).unwrap().is_empty());
    run(&["add", "config.txt"]);
    assert!(scan_index(t.path()).unwrap().is_empty());
}

#[test]
fn inventory_entry_limit_is_reported() {
    let t = tempdir().unwrap();
    for n in 0..8 {
        fs::write(t.path().join(format!("{n}.rs")), "fn f(){}").unwrap();
    }
    let limits = InventoryLimits {
        max_entries: 3,
        ..Default::default()
    };
    let inv = inventory_target(t.path(), &limits).unwrap();
    assert!(inv.truncated);
    assert!(inv.files.len() <= 3);
}
