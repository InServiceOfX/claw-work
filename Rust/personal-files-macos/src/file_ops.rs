//! Explicit clipboard operations. No shell, no replacing existing items.
//! Copies use Darwin fcopyfile (data, ACLs, resource forks, xattrs and stat).
//! Directory traversal is descriptor-relative and never follows source links.
use std::{
    fs, io,
    path::{Path, PathBuf},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    Copy,
    Move,
}

pub fn destination(source: &Path, directory: &Path) -> io::Result<PathBuf> {
    let metadata = fs::symlink_metadata(source)?;
    let name = source
        .file_name()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "Source has no name"))?;
    let directory = fs::canonicalize(directory)?;
    if !directory.is_dir() {
        return Err(io::Error::new(
            io::ErrorKind::NotADirectory,
            "Destination is not a folder",
        ));
    }
    if metadata.is_dir() && directory.starts_with(fs::canonicalize(source)?) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Cannot paste a folder into itself or its descendant",
        ));
    }
    let target = directory.join(name);
    match fs::symlink_metadata(&target) {
        Ok(_) => {
            return Err(io::Error::new(
                io::ErrorKind::AlreadyExists,
                "An item with that name already exists; nothing was replaced",
            ))
        }
        Err(e) if e.kind() == io::ErrorKind::NotFound => {}
        Err(e) => return Err(e),
    }
    Ok(target)
}

pub fn perform(action: Action, source: &Path, directory: &Path) -> io::Result<PathBuf> {
    #[cfg(target_os = "macos")]
    {
        darwin::perform(action, source, directory, false, || {})
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = (action, source, directory);
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "macOS is required",
        ))
    }
}

#[cfg(target_os = "macos")]
mod darwin {
    use super::*;
    use std::{
        ffi::{CStr, CString, OsStr, OsString},
        fs::File,
        os::{
            fd::{AsRawFd, FromRawFd},
            unix::{
                ffi::{OsStrExt, OsStringExt},
                fs::MetadataExt,
            },
        },
    };
    unsafe extern "C" {
        fn fcopyfile(from: i32, to: i32, state: *mut std::ffi::c_void, flags: u32) -> i32;
        fn renameatx_np(
            fromfd: i32,
            from: *const libc::c_char,
            tofd: i32,
            to: *const libc::c_char,
            flags: u32,
        ) -> i32;
    }
    fn c(s: &OsStr) -> io::Result<CString> {
        CString::new(s.as_bytes()).map_err(io::Error::other)
    }
    fn check(result: i32) -> io::Result<()> {
        if result == 0 {
            Ok(())
        } else {
            Err(io::Error::last_os_error())
        }
    }
    fn open_at(parent: &File, name: &OsStr, flags: i32) -> io::Result<File> {
        let name = c(name)?;
        // SAFETY: live descriptors, NUL-terminated name, Darwin openat ABI.
        let fd = unsafe {
            libc::openat(
                parent.as_raw_fd(),
                name.as_ptr(),
                flags
                    | libc::O_CLOEXEC
                    | if flags & libc::O_SYMLINK == 0 {
                        libc::O_NOFOLLOW
                    } else {
                        0
                    },
                0o600 as libc::c_uint,
            )
        };
        if fd < 0 {
            Err(io::Error::last_os_error())
        } else {
            Ok(unsafe { File::from_raw_fd(fd) })
        }
    }
    fn open_dir(path: &Path) -> io::Result<File> {
        use std::os::unix::fs::OpenOptionsExt;
        fs::OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC)
            .open(path)
    }
    fn rename(from: &File, a: &OsStr, to: &File, b: &OsStr) -> io::Result<()> {
        let (a, b) = (c(a)?, c(b)?);
        // RENAME_EXCL atomically refuses all collisions, including symlinks.
        check(unsafe { renameatx_np(from.as_raw_fd(), a.as_ptr(), to.as_raw_fd(), b.as_ptr(), 4) })
    }
    fn remove_empty_claim(parent: &File, recovery_path: &Path) {
        if let Some(name) = recovery_path.file_name().and_then(|name| c(name).ok()) {
            // Only remove an empty directory, anchored to the original parent.
            // Never recursively delete a claimed source on the error path.
            let _ =
                unsafe { libc::unlinkat(parent.as_raw_fd(), name.as_ptr(), libc::AT_REMOVEDIR) };
        }
    }
    fn same(a: &fs::Metadata, b: &fs::Metadata) -> bool {
        a.dev() == b.dev() && a.ino() == b.ino() && a.mode() == b.mode()
    }
    fn metadata_copy(input: &File, output: &File, data: bool) -> io::Result<()> {
        // COPYFILE_ACL | STAT | XATTR | optional DATA from SDK copyfile.h.
        check(unsafe {
            fcopyfile(
                input.as_raw_fd(),
                output.as_raw_fd(),
                std::ptr::null_mut(),
                if data { 15 } else { 7 },
            )
        })
    }
    fn names(dir: &File) -> io::Result<Vec<OsString>> {
        // fdopendir owns its argument. openat(".") obtains an independent offset.
        let stream_fd = open_at(dir, OsStr::new("."), libc::O_RDONLY | libc::O_DIRECTORY)?;
        use std::os::fd::IntoRawFd;
        let fd = stream_fd.into_raw_fd();
        let stream = unsafe { libc::fdopendir(fd) };
        if stream.is_null() {
            unsafe {
                libc::close(fd);
            }
            return Err(io::Error::last_os_error());
        }
        let mut result = Vec::new();
        let error = loop {
            unsafe {
                *libc::__error() = 0;
            }
            let entry = unsafe { libc::readdir(stream) };
            if entry.is_null() {
                let errno = unsafe { *libc::__error() };
                break if errno == 0 {
                    None
                } else {
                    Some(io::Error::from_raw_os_error(errno))
                };
            }
            let name = unsafe { CStr::from_ptr((*entry).d_name.as_ptr()) }.to_bytes();
            if name != b"." && name != b".." {
                result.push(OsString::from_vec(name.to_vec()));
            }
        };
        unsafe {
            libc::closedir(stream);
        }
        if let Some(e) = error {
            Err(e)
        } else {
            Ok(result)
        }
    }
    /// Pin the final entry without following even a replaced symbolic link.
    fn source_at(parent: &File, name: &OsStr) -> io::Result<File> {
        // O_SYMLINK opens the link itself; O_NONBLOCK avoids hanging on FIFOs.
        open_at(
            parent,
            name,
            libc::O_RDONLY | libc::O_SYMLINK | libc::O_NONBLOCK,
        )
    }
    #[derive(PartialEq, Eq)]
    struct Identity {
        dev: u64,
        ino: u64,
        mode: u32,
    }
    fn identity(meta: &fs::Metadata) -> Identity {
        Identity {
            dev: meta.dev(),
            ino: meta.ino(),
            mode: meta.mode(),
        }
    }
    fn identity_at(parent: &File, name: &OsStr) -> io::Result<Identity> {
        let name = c(name)?;
        let mut stat = std::mem::MaybeUninit::<libc::stat>::uninit();
        check(unsafe {
            libc::fstatat(
                parent.as_raw_fd(),
                name.as_ptr(),
                stat.as_mut_ptr(),
                libc::AT_SYMLINK_NOFOLLOW,
            )
        })?;
        let stat = unsafe { stat.assume_init() };
        Ok(Identity {
            dev: stat.st_dev as u64,
            ino: stat.st_ino,
            mode: stat.st_mode as u32,
        })
    }
    fn ensure_outside(source: &File, destination: &File) -> io::Result<()> {
        ensure_outside_identity(&identity(&source.metadata()?), destination)
    }
    fn ensure_outside_identity(source: &Identity, destination: &File) -> io::Result<()> {
        if source.mode & libc::S_IFMT as u32 != libc::S_IFDIR as u32 {
            return Ok(());
        }
        let mut ancestor = destination.try_clone()?;
        for _ in 0..1024 {
            let here = ancestor.metadata()?;
            if source.dev == here.dev() && source.ino == here.ino() {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "Destination is inside the source folder",
                ));
            }
            let parent = open_at(
                &ancestor,
                OsStr::new(".."),
                libc::O_RDONLY | libc::O_DIRECTORY,
            )?;
            if same(&here, &parent.metadata()?) {
                return Ok(());
            }
            ancestor = parent;
        }
        Err(io::Error::other("Destination ancestry is too deep"))
    }
    fn copy_entry(
        parent: &File,
        name: &OsStr,
        output_parent: &File,
        output_name: &OsStr,
        depth: usize,
    ) -> io::Result<()> {
        if depth > 256 {
            return Err(io::Error::other("Folder nesting exceeds 256 levels"));
        }
        let input = source_at(parent, name)?;
        let before = input.metadata()?;
        if before.file_type().is_symlink() {
            let name_c = c(name)?;
            let mut bytes = vec![0u8; 32768];
            let len = unsafe {
                libc::readlinkat(
                    parent.as_raw_fd(),
                    name_c.as_ptr(),
                    bytes.as_mut_ptr().cast(),
                    bytes.len(),
                )
            };
            if len < 0 {
                return Err(io::Error::last_os_error());
            }
            if len as usize == bytes.len() {
                return Err(io::Error::other("Symbolic link is too long"));
            }
            if !same(&before, &source_at(parent, name)?.metadata()?) {
                return Err(io::Error::other("Source link changed during copy"));
            }
            let value = CString::new(&bytes[..len as usize]).map_err(io::Error::other)?;
            let out = c(output_name)?;
            check(unsafe {
                libc::symlinkat(value.as_ptr(), output_parent.as_raw_fd(), out.as_ptr())
            })?;
            let output = source_at(output_parent, output_name)?;
            // Darwin supports metadata-only fcopyfile on O_SYMLINK descriptors.
            metadata_copy(&input, &output, false)?;
        } else if before.is_dir() {
            let out = c(output_name)?;
            check(unsafe { libc::mkdirat(output_parent.as_raw_fd(), out.as_ptr(), 0o700) })?;
            let output = open_at(
                output_parent,
                output_name,
                libc::O_RDONLY | libc::O_DIRECTORY,
            )?;
            for child in names(&input)? {
                copy_entry(&input, &child, &output, &child, depth + 1)?;
            }
            metadata_copy(&input, &output, false)?;
        } else if before.is_file() {
            let output = open_at(
                output_parent,
                output_name,
                libc::O_RDWR | libc::O_CREAT | libc::O_EXCL,
            )?;
            metadata_copy(&input, &output, true)?;
            output.sync_all()?;
        } else {
            return Err(io::Error::new(
                io::ErrorKind::Unsupported,
                "Cannot copy sockets, devices or other special items",
            ));
        }
        let after = input.metadata()?;
        if before.len() != after.len()
            || before.mtime() != after.mtime()
            || before.mtime_nsec() != after.mtime_nsec()
        {
            return Err(io::Error::other(
                "Source changed while being copied; retry when it is no longer in use",
            ));
        }
        Ok(())
    }
    fn staged_copy(
        source_parent: &File,
        source_name: &OsStr,
        directory: &Path,
        dest: &File,
        name: &OsStr,
        before_publish: impl FnOnce(),
    ) -> io::Result<()> {
        let stage = tempfile::Builder::new()
            .prefix(".personal-files-copy-")
            .tempdir_in(directory)?;
        let stage_fd = open_dir(stage.path())?;
        let input = source_at(source_parent, source_name)?;
        ensure_outside(&input, dest)?;
        copy_entry(source_parent, source_name, &stage_fd, OsStr::new("item"), 0)?;
        before_publish();
        if !same(
            &input.metadata()?,
            &source_at(source_parent, source_name)?.metadata()?,
        ) {
            return Err(io::Error::other(
                "Source was replaced during copy; nothing was published",
            ));
        }
        ensure_outside(&input, dest)?;
        rename(&stage_fd, OsStr::new("item"), dest, name)
    }
    pub(super) fn perform(
        action: Action,
        source: &Path,
        directory: &Path,
        force_copy_move: bool,
        before_publish: impl FnOnce(),
    ) -> io::Result<PathBuf> {
        let target = destination(source, directory)?;
        let directory = target.parent().unwrap();
        let parent_path = fs::canonicalize(
            source
                .parent()
                .ok_or_else(|| io::Error::other("Source has no parent"))?,
        )?;
        let parent = open_dir(&parent_path)?;
        let dest = open_dir(directory)?;
        let name = source.file_name().unwrap();
        // fstatat does not require file-content read access for renames.
        let original = identity_at(&parent, name)?;
        ensure_outside_identity(&original, &dest)?;
        if action == Action::Copy {
            staged_copy(&parent, name, directory, &dest, name, before_publish)?;
        } else {
            // Claim a private name on the source volume, verify its identity,
            // then atomically rename it. A TempDir destructor must never delete
            // a claimed source on failure. Cross-volume moves are refused.
            let recovery = tempfile::Builder::new()
                .prefix(".personal-files-move-")
                .tempdir_in(&parent_path)?;
            let claim = open_dir(recovery.path())?;
            let recovery_path = recovery.keep();
            if let Err(e) = rename(&parent, name, &claim, OsStr::new("item")) {
                remove_empty_claim(&parent, &recovery_path);
                return Err(e);
            }
            let result = (|| {
                let claimed = identity_at(&claim, OsStr::new("item"))?;
                before_publish();
                if original != claimed {
                    return Err(io::Error::other("Source changed before move; retry"));
                }
                ensure_outside_identity(&claimed, &dest)?;
                let renamed = if force_copy_move {
                    Err(io::Error::from_raw_os_error(libc::EXDEV))
                } else {
                    rename(&claim, OsStr::new("item"), &dest, name)
                };
                match renamed {
                    Ok(()) => Ok(()),
                    Err(e) if e.raw_os_error() == Some(libc::EXDEV) => Err(io::Error::new(
                        io::ErrorKind::Unsupported,
                        "Move between volumes is not supported; source retained. Use Copy/Paste instead.",
                    )),
                    Err(e) => Err(e),
                }
            })();
            if let Err(e) = result {
                // Exclusive restoration never replaces a concurrently created
                // original name. Keep the private recovery path if blocked.
                if let Err(restore) = rename(&claim, OsStr::new("item"), &parent, name) {
                    let mut bytes = vec![0u8; libc::PATH_MAX as usize];
                    let path = if unsafe {
                        libc::fcntl(claim.as_raw_fd(), libc::F_GETPATH, bytes.as_mut_ptr())
                    } == 0
                    {
                        let length = bytes.iter().position(|b| *b == 0).unwrap_or(bytes.len());
                        PathBuf::from(OsString::from_vec(bytes[..length].to_vec()))
                    } else {
                        recovery_path.clone()
                    };
                    return Err(io::Error::other(format!(
                        "{e}; could not restore source ({restore}). Recover it at {}",
                        path.join("item").display()
                    )));
                }
                remove_empty_claim(&parent, &recovery_path);
                return Err(e);
            }
            remove_empty_claim(&parent, &recovery_path);
        }
        Ok(target)
    }
}

#[cfg(all(test, target_os = "macos"))]
mod tests {
    use super::*;
    use std::os::unix::fs::symlink;
    #[test]
    fn copies_and_moves_files_directories_and_symlinks() {
        let t = tempfile::tempdir().unwrap();
        let src = t.path().join("src");
        let dst = t.path().join("dst");
        fs::create_dir_all(src.join("folder")).unwrap();
        fs::create_dir(&dst).unwrap();
        fs::write(src.join("folder/file"), "content").unwrap();
        symlink("missing", src.join("folder/link")).unwrap();
        perform(Action::Copy, &src.join("folder"), &dst).unwrap();
        assert_eq!(
            fs::read_to_string(dst.join("folder/file")).unwrap(),
            "content"
        );
        assert_eq!(
            fs::read_link(dst.join("folder/link")).unwrap(),
            Path::new("missing")
        );
        assert!(src.join("folder/file").exists());
        symlink("missing", src.join("link")).unwrap();
        perform(Action::Move, &src.join("link"), &dst).unwrap();
        assert_eq!(
            fs::read_link(dst.join("link")).unwrap(),
            Path::new("missing")
        );
        assert!(fs::symlink_metadata(src.join("link")).is_err());
        perform(Action::Move, &src.join("folder/file"), &dst).unwrap();
        assert!(!src.join("folder/file").exists());
        assert_eq!(fs::read_to_string(dst.join("file")).unwrap(), "content");
    }
    #[test]
    fn rejects_collisions_and_descendants_through_aliases() {
        let t = tempfile::tempdir().unwrap();
        let src = t.path().join("src");
        let dst = t.path().join("dst");
        fs::create_dir_all(src.join("child")).unwrap();
        fs::create_dir(&dst).unwrap();
        fs::write(src.join("item"), "original").unwrap();
        symlink("missing", dst.join("item")).unwrap();
        for action in [Action::Copy, Action::Move] {
            assert_eq!(
                perform(action, &src.join("item"), &dst).unwrap_err().kind(),
                io::ErrorKind::AlreadyExists
            );
            assert!(perform(action, &src, &src.join("child")).is_err());
            symlink(src.join("child"), t.path().join("alias")).unwrap();
            assert!(perform(action, &src, &t.path().join("alias")).is_err());
            fs::remove_file(t.path().join("alias")).unwrap();
        }
        assert_eq!(fs::read_to_string(src.join("item")).unwrap(), "original");
        assert_eq!(
            fs::read_link(dst.join("item")).unwrap(),
            Path::new("missing")
        );
    }
    #[test]
    fn cross_volume_move_is_refused_and_source_restored() {
        let t = tempfile::tempdir().unwrap();
        let src = t.path().join("source");
        let dst = t.path().join("dst");
        fs::create_dir_all(src.join("nested")).unwrap();
        fs::create_dir(&dst).unwrap();
        fs::write(src.join("nested/data"), "value").unwrap();
        let error = darwin::perform(Action::Move, &src, &dst, true, || {}).unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::Unsupported);
        assert_eq!(
            fs::read_to_string(src.join("nested/data")).unwrap(),
            "value"
        );
        assert!(!dst.join("source").exists());
    }
    #[test]
    fn special_items_fail_copy_without_publishing_and_remain_movable() {
        use std::os::unix::net::UnixListener;
        let t = tempfile::tempdir().unwrap();
        let src = t.path().join("source");
        let dst = t.path().join("dst");
        fs::create_dir(&src).unwrap();
        fs::create_dir(&dst).unwrap();
        let _socket = UnixListener::bind(src.join("socket")).unwrap();
        assert!(perform(Action::Copy, &src, &dst).is_err());
        assert!(src.join("socket").exists());
        assert!(!dst.join("source").exists());
        perform(Action::Move, &src, &dst).unwrap();
        assert!(dst.join("source/socket").exists());
    }
    #[test]
    fn publication_race_does_not_overwrite_or_delete_source() {
        let t = tempfile::tempdir().unwrap();
        let src = t.path().join("item");
        let dst = t.path().join("dst");
        fs::create_dir(&dst).unwrap();
        fs::write(&src, "source").unwrap();
        for action in [Action::Copy, Action::Move] {
            let error = darwin::perform(action, &src, &dst, false, || {
                fs::write(dst.join("item"), "racer").unwrap();
            })
            .unwrap_err();
            assert_eq!(error.kind(), io::ErrorKind::AlreadyExists);
            assert_eq!(fs::read_to_string(&src).unwrap(), "source");
            assert_eq!(fs::read_to_string(dst.join("item")).unwrap(), "racer");
            fs::remove_file(dst.join("item")).unwrap();
        }
    }
    #[test]
    fn blocked_restore_retains_original_in_reported_recovery_directory() {
        let t = tempfile::tempdir().unwrap();
        let src = t.path().join("item");
        let dst = t.path().join("dst");
        fs::create_dir(&dst).unwrap();
        fs::write(&src, "source").unwrap();
        let error = darwin::perform(Action::Move, &src, &dst, true, || {
            fs::write(&src, "new occupant").unwrap();
        })
        .unwrap_err();
        assert_eq!(fs::read_to_string(&src).unwrap(), "new occupant");
        let recovery = fs::read_dir(t.path())
            .unwrap()
            .map(|e| e.unwrap().path())
            .find(|p| {
                p.file_name()
                    .unwrap()
                    .to_string_lossy()
                    .starts_with(".personal-files-move-")
            })
            .unwrap();
        assert_eq!(fs::read_to_string(recovery.join("item")).unwrap(), "source");
        assert!(error.to_string().contains(recovery.to_str().unwrap()));
    }
    #[test]
    fn substituted_source_link_is_not_followed_or_published() {
        let t = tempfile::tempdir().unwrap();
        let src = t.path().join("item");
        let dst = t.path().join("dst");
        fs::create_dir(&dst).unwrap();
        fs::write(&src, "source").unwrap();
        let secret = t.path().join("other");
        fs::write(&secret, "must not copy").unwrap();
        assert!(darwin::perform(Action::Copy, &src, &dst, false, || {
            fs::rename(&src, t.path().join("parked")).unwrap();
            symlink(&secret, &src).unwrap();
        })
        .is_err());
        assert!(!dst.join("item").exists());
        assert_eq!(fs::read_to_string(&secret).unwrap(), "must not copy");
        assert_eq!(
            fs::read_to_string(t.path().join("parked")).unwrap(),
            "source"
        );
    }
    #[test]
    fn failed_move_restores_through_descriptors_after_parent_rename() {
        let t = tempfile::tempdir().unwrap();
        let parent = t.path().join("parent");
        let renamed = t.path().join("renamed");
        let dst = t.path().join("dst");
        fs::create_dir(&parent).unwrap();
        fs::create_dir(&dst).unwrap();
        let src = parent.join("item");
        fs::write(&src, "source").unwrap();
        let error = darwin::perform(Action::Move, &src, &dst, true, || {
            fs::rename(&parent, &renamed).unwrap();
        })
        .unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::Unsupported);
        assert_eq!(fs::read_to_string(renamed.join("item")).unwrap(), "source");
        assert!(!dst.join("item").exists());
        assert_eq!(
            fs::read_dir(&renamed).unwrap().count(),
            1,
            "empty recovery directory must be removed too"
        );
    }
    #[test]
    fn same_volume_move_does_not_require_read_permission() {
        use std::os::unix::fs::PermissionsExt;
        let t = tempfile::tempdir().unwrap();
        let src = t.path().join("item");
        let dst = t.path().join("dst");
        fs::create_dir(&dst).unwrap();
        fs::write(&src, "source").unwrap();
        fs::set_permissions(&src, fs::Permissions::from_mode(0o000)).unwrap();
        perform(Action::Move, &src, &dst).unwrap();
        assert!(!src.exists());
        fs::set_permissions(dst.join("item"), fs::Permissions::from_mode(0o600)).unwrap();
        assert_eq!(fs::read_to_string(dst.join("item")).unwrap(), "source");
    }
    #[test]
    fn copy_preserves_extended_attributes_resource_fork_permissions_and_times() {
        use std::os::unix::fs::{MetadataExt, PermissionsExt};
        let t = tempfile::tempdir().unwrap();
        let src = t.path().join("file");
        let dst = t.path().join("dst");
        fs::write(&src, "data").unwrap();
        fs::create_dir(&dst).unwrap();
        for (name, value) in [
            ("com.example.personal-files", "attribute"),
            ("com.apple.ResourceFork", "fork bytes"),
        ] {
            let status = std::process::Command::new("/usr/bin/xattr")
                .args(["-w", name, value])
                .arg(&src)
                .status()
                .unwrap();
            assert!(status.success());
        }
        fs::set_permissions(&src, fs::Permissions::from_mode(0o640)).unwrap();
        assert!(std::process::Command::new("/bin/chmod")
            .args([
                "+a",
                "everyone allow read,readattr,readextattr,readsecurity"
            ])
            .arg(&src)
            .status()
            .unwrap()
            .success());
        perform(Action::Copy, &src, &dst).unwrap();
        let acls = |p: &Path| {
            let output = std::process::Command::new("/bin/ls")
                .arg("-le")
                .arg(p)
                .output()
                .unwrap();
            assert!(output.status.success());
            String::from_utf8(output.stdout)
                .unwrap()
                .lines()
                .skip(1)
                .map(str::to_owned)
                .collect::<Vec<_>>()
        };
        assert!(!acls(&src).is_empty());
        assert_eq!(acls(&src), acls(&dst.join("file")));
        for name in ["com.example.personal-files", "com.apple.ResourceFork"] {
            let read = |p: &Path| {
                std::process::Command::new("/usr/bin/xattr")
                    .args(["-p", name])
                    .arg(p)
                    .output()
                    .unwrap()
            };
            let a = read(&src);
            let b = read(&dst.join("file"));
            assert!(a.status.success() && b.status.success());
            assert_eq!(a.stdout, b.stdout);
        }
        let (a, b) = (
            fs::metadata(&src).unwrap(),
            fs::metadata(dst.join("file")).unwrap(),
        );
        assert_eq!(a.mode() & 0o777, b.mode() & 0o777);
        assert_eq!(a.mtime(), b.mtime());
        assert_eq!(a.mtime_nsec(), b.mtime_nsec());
    }
}
