use personal_files::{
    commands,
    operations::{self, Removal},
};
use std::{
    fs,
    os::unix::fs::symlink,
    path::Path,
    thread,
    time::{Duration, Instant},
};
use tempfile::TempDir;

fn result(task: &commands::Task) -> commands::Output {
    let deadline = Instant::now() + Duration::from_secs(8);
    loop {
        if let Some(result) = task.try_result() {
            return result.unwrap();
        }
        assert!(Instant::now() < deadline, "command did not finish");
        thread::sleep(Duration::from_millis(10));
    }
}

#[test]
fn permanent_delete_removes_nonempty_folders_and_links_without_following_them() {
    let temp = TempDir::new().unwrap();
    let target = temp.path().join("keep target");
    fs::create_dir(&target).unwrap();
    fs::write(target.join("keep"), "important").unwrap();
    let folder = temp.path().join("remove");
    fs::create_dir(&folder).unwrap();
    fs::write(folder.join("nested"), "fixture").unwrap();
    symlink(&target, folder.join("link")).unwrap();
    let link = temp.path().join("link");
    symlink(&target, &link).unwrap();
    let broken = temp.path().join("broken");
    symlink(temp.path().join("absent"), &broken).unwrap();
    let report = operations::remove(
        &[folder.clone(), link.clone(), broken.clone()],
        Removal::Permanent,
    );
    assert_eq!(report.completed, 3);
    assert!(report.errors.is_empty());
    assert!(!folder.exists());
    assert!(fs::symlink_metadata(link).is_err());
    assert!(fs::symlink_metadata(broken).is_err());
    assert_eq!(
        fs::read_to_string(target.join("keep")).unwrap(),
        "important"
    );
}

#[test]
fn removals_report_partial_failure_and_reject_root_and_relative_paths() {
    let temp = TempDir::new().unwrap();
    let good = temp.path().join("good");
    fs::write(&good, "fixture").unwrap();
    let report = operations::remove(
        &[
            temp.path().join("missing"),
            good.clone(),
            Path::new("/").into(),
            Path::new("relative").into(),
        ],
        Removal::Permanent,
    );
    assert_eq!(report.completed, 1);
    assert_eq!(report.errors.len(), 3);
    assert!(!good.exists());
    assert_eq!(operations::remove(&[], Removal::Permanent).completed, 0);
}

#[test]
fn failed_trash_never_falls_back_to_delete() {
    let temp = TempDir::new().unwrap();
    let source = temp.path().join("keep");
    fs::write(&source, "fixture").unwrap();
    let report = operations::remove_with(std::slice::from_ref(&source), Removal::Trash, |_| {
        Err("fixture trash unavailable".into())
    });
    assert_eq!(report.completed, 0);
    assert_eq!(report.errors.len(), 1);
    assert_eq!(fs::read_to_string(source).unwrap(), "fixture");
}

#[test]
fn commands_use_exact_cwd_shell_syntax_and_report_stdout_stderr_and_exit() {
    let temp = TempDir::new().unwrap();
    let dir = temp.path().join("a b's $folder Δ");
    fs::create_dir(&dir).unwrap();
    let task = commands::start(
        "pwd; printf hi | tr a-z A-Z; printf problem >&2; printf done > result; exit 7",
        &dir,
    )
    .unwrap();
    let output = result(&task);
    assert_eq!(output.code, Some(7));
    assert!(output.stdout.contains(dir.to_str().unwrap()));
    assert!(output.stdout.ends_with("HI"));
    assert_eq!(output.stderr, "problem");
    assert_eq!(fs::read_to_string(dir.join("result")).unwrap(), "done");
    assert!(!temp.path().join("result").exists());
}

#[test]
fn command_capture_is_bounded_and_drains_both_streams() {
    let temp = TempDir::new().unwrap();
    let task = commands::start(
        "head -c 131072 /dev/zero; head -c 131072 /dev/zero >&2",
        temp.path(),
    )
    .unwrap();
    let output = result(&task);
    assert_eq!(output.code, Some(0));
    assert!(output.truncated);
    assert!(output.stdout.len() <= commands::OUTPUT_LIMIT);
    assert!(output.stderr.len() <= commands::OUTPUT_LIMIT);
}

#[test]
fn command_cancellation_stops_process_group_and_returns_promptly() {
    let temp = TempDir::new().unwrap();
    let task = commands::start("sleep 20; printf wrong > late", temp.path()).unwrap();
    task.cancel();
    let output = result(&task);
    assert!(output.cancelled);
    assert!(!temp.path().join("late").exists());
}

#[test]
fn command_stdin_is_closed_and_missing_cwd_fails_without_execution() {
    let temp = TempDir::new().unwrap();
    let task = commands::start("read answer; printf eof", temp.path()).unwrap();
    assert_eq!(result(&task).stdout, "eof");
    assert!(commands::start("printf bad > marker", &temp.path().join("missing")).is_err());
    assert!(!temp.path().join("marker").exists());
}

#[test]
fn native_trash_moves_files_folders_and_links_with_metadata() {
    // GIO caches user directories; isolate XDG state in a child process instead
    // of changing global environment while other acceptance tests run.
    if let Some(root) = std::env::var_os("PERSONAL_FILES_TRASH_FIXTURE") {
        let root = std::path::PathBuf::from(root);
        let target = root.join("keep");
        fs::create_dir(&target).unwrap();
        fs::write(target.join("important"), "keep").unwrap();
        let file = root.join("trash file");
        fs::write(&file, "fixture").unwrap();
        let folder = root.join("trash folder");
        fs::create_dir(&folder).unwrap();
        fs::write(folder.join("nested"), "fixture").unwrap();
        let link = root.join("trash link");
        symlink(&target, &link).unwrap();
        let report = operations::remove(
            &[file.clone(), folder.clone(), link.clone()],
            Removal::Trash,
        );
        assert!(report.errors.is_empty(), "{:?}", report.errors);
        assert_eq!(report.completed, 3);
        assert!(!file.exists());
        assert!(!folder.exists());
        assert!(fs::symlink_metadata(link).is_err());
        assert!(target.join("important").exists());
        let trash = root.join("data/Trash");
        assert_eq!(fs::read_dir(trash.join("files")).unwrap().count(), 3);
        let info = fs::read_dir(trash.join("info"))
            .unwrap()
            .map(|e| fs::read_to_string(e.unwrap().path()).unwrap())
            .collect::<Vec<_>>();
        assert_eq!(info.len(), 3);
        assert!(info
            .iter()
            .all(|text| text.contains("[Trash Info]") && text.contains("DeletionDate=")));
    } else {
        let temp = TempDir::new().unwrap();
        let data = temp.path().join("data");
        fs::create_dir(&data).unwrap();
        let status = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "native_trash_moves_files_folders_and_links_with_metadata",
                "--nocapture",
            ])
            .env("PERSONAL_FILES_TRASH_FIXTURE", temp.path())
            .env("XDG_DATA_HOME", data)
            .env("XDG_CONFIG_HOME", temp.path().join("config"))
            .status()
            .unwrap();
        assert!(status.success());
    }
}

#[test]
fn dropping_task_cancels_even_when_the_host_process_exits_immediately() {
    if let Some(root) = std::env::var_os("PERSONAL_FILES_EXIT_FIXTURE") {
        let root = std::path::PathBuf::from(root);
        let task = commands::start("sleep 1; printf orphan > after-exit", &root).unwrap();
        drop(task);
        std::process::exit(0);
    }
    let temp = TempDir::new().unwrap();
    let status = std::process::Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "dropping_task_cancels_even_when_the_host_process_exits_immediately",
        ])
        .env("PERSONAL_FILES_EXIT_FIXTURE", temp.path())
        .status()
        .unwrap();
    assert!(status.success());
    thread::sleep(Duration::from_millis(1200));
    assert!(
        !temp.path().join("after-exit").exists(),
        "closing the last window must not leave a command running"
    );
}
