//! Publish a staged application atomically without replacing a concurrent install.
use std::{io, path::Path};

#[cfg(target_os = "macos")]
pub fn publish(staged: &Path, destination: &Path) -> io::Result<()> {
    use std::{ffi::CString, os::unix::ffi::OsStrExt};
    unsafe extern "C" {
        fn renamex_np(
            from: *const std::ffi::c_char,
            to: *const std::ffi::c_char,
            flags: u32,
        ) -> i32;
    }
    let from = CString::new(staged.as_os_str().as_bytes()).map_err(io::Error::other)?;
    let to = CString::new(destination.as_os_str().as_bytes()).map_err(io::Error::other)?;
    // SAFETY: valid NUL-terminated paths, correct Darwin signature. RENAME_EXCL
    // (sys/stdio.h) prevents replacement atomically, unlike a preflight exists().
    if unsafe { renamex_np(from.as_ptr(), to.as_ptr(), 0x4) } == 0 {
        Ok(())
    } else {
        Err(io::Error::last_os_error())
    }
}
#[cfg(not(target_os = "macos"))]
pub fn publish(_: &Path, _: &Path) -> io::Result<()> {
    Err(io::Error::new(
        io::ErrorKind::Unsupported,
        "macOS is required",
    ))
}

#[cfg(all(test, target_os = "macos"))]
mod tests {
    use super::*;
    #[test]
    fn publish_is_atomic_and_never_merges_or_replaces_existing_app() {
        let temp = tempfile::tempdir().unwrap();
        let staged = temp.path().join("staged.app");
        let destination = temp.path().join("installed.app");
        std::fs::create_dir(&staged).unwrap();
        std::fs::write(staged.join("binary"), "complete fixture").unwrap();
        publish(&staged, &destination).unwrap();
        assert!(!staged.exists());
        assert_eq!(
            std::fs::read_to_string(destination.join("binary")).unwrap(),
            "complete fixture"
        );
        std::fs::create_dir(&staged).unwrap();
        std::fs::write(staged.join("binary"), "replacement").unwrap();
        assert_eq!(
            publish(&staged, &destination).unwrap_err().kind(),
            io::ErrorKind::AlreadyExists
        );
        assert_eq!(
            std::fs::read_to_string(destination.join("binary")).unwrap(),
            "complete fixture"
        );
        assert_eq!(
            std::fs::read_to_string(staged.join("binary")).unwrap(),
            "replacement"
        );
    }
}
