#[test]
#[cfg(target_os = "macos")]
fn packaging_preserves_working_apps_on_failure() {
    let output = std::process::Command::new("python3")
        .arg(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/packaging.py"))
        .arg(env!("CARGO_BIN_EXE_personal-files-macos"))
        .output()
        .expect("run synthetic packaging checks with Python 3");
    assert!(
        output.status.success(),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}
