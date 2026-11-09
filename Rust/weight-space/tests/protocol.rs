mod common;
use claw_weight_space::server::decode_json_response;
#[test]
fn local_http_error_keeps_backend_diagnostic() {
    let body = br#"{"error":{"message":"repeat_last_n must be nonnegative"}}"#;
    let error = decode_json_response(400, body).unwrap_err().to_string();
    assert!(error.contains("400") && error.contains("repeat_last_n must be nonnegative"));
    assert!(decode_json_response(200, b"not JSON").is_err());
    assert_eq!(
        decode_json_response(200, br#"{"status":"ok"}"#).unwrap()["status"],
        "ok"
    );
}

#[test]
fn generated_outputs_refuse_git_tree_and_symlink_alias() {
    let t = common::tempdir();
    let repo = t.path().join("repo");
    std::fs::create_dir(&repo).unwrap();
    std::fs::write(repo.join(".git"), "gitdir: synthetic").unwrap();
    assert!(claw_weight_space::write_json_new(&repo.join("result.json"), &42).is_err());
    assert!(!repo.join("result.json").exists());
    #[cfg(unix)]
    {
        let alias = t.path().join("alias");
        std::os::unix::fs::symlink(&repo, &alias).unwrap();
        assert!(claw_weight_space::ensure_external_output(&alias.join("new/output")).is_err());
    }
}
