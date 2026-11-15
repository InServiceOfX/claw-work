static DESKTOP_FIXTURES: std::sync::Mutex<()> = std::sync::Mutex::new(());

#[test]
#[ignore = "requires a logged-in macOS graphical session; creates only synthetic fixtures"]
fn native_desktop_story() {
    let _serial = DESKTOP_FIXTURES.lock().unwrap();
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_personal-files-macos"))
        .arg("--self-test")
        .output()
        .expect("launch native fixture");
    assert!(
        output.status.success(),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("PASS native desktop fixture"));
    println!("{stdout}");
    let tiling = std::process::Command::new(env!("CARGO_BIN_EXE_personal-files-macos"))
        .arg("--self-test")
        .env("PERSONAL_FILES_ASYNC_TILING_TEST", "1")
        .output()
        .expect("launch asynchronous native tiling fixture");
    assert!(
        tiling.status.success(),
        "async tiling stdout: {}\nasync tiling stderr: {}",
        String::from_utf8_lossy(&tiling.stdout),
        String::from_utf8_lossy(&tiling.stderr)
    );
    let stdout = String::from_utf8_lossy(&tiling.stdout);
    assert!(stdout.contains("PASS asynchronous native tiling"));
    println!("{stdout}");
}

#[test]
#[ignore = "requires a logged-in macOS graphical session; synthetic saved-tab fixtures"]
fn native_saved_session_story() {
    let _serial = DESKTOP_FIXTURES.lock().unwrap();
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_personal-files-macos"))
        .arg("--self-test")
        .env("PERSONAL_FILES_SAVED_TABS_TEST", "1")
        .output()
        .expect("launch native saved-session fixture");
    assert!(
        output.status.success(),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(String::from_utf8_lossy(&output.stdout).contains("PASS native saved tabs"));
    println!("{}", String::from_utf8_lossy(&output.stdout));
}

#[test]
#[ignore = "requires a logged-in macOS graphical session; synthetic multi-window fixtures"]
fn native_new_windows_story() {
    let _serial = DESKTOP_FIXTURES.lock().unwrap();
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_personal-files-macos"))
        .arg("--self-test")
        .env("PERSONAL_FILES_NEW_WINDOWS_TEST", "1")
        .output()
        .expect("launch native new-window fixture");
    assert!(
        output.status.success(),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("PASS native new windows"));
    println!("{stdout}");
}
