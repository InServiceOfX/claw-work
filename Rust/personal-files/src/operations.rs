//! Explicit user-requested removal; browsing never calls these operations.
use gtk::{gio, prelude::*};
use std::{
    fs,
    path::{Component, Path, PathBuf},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Removal {
    Trash,
    Permanent,
}

#[derive(Debug, Default)]
pub struct Report {
    pub completed: usize,
    pub errors: Vec<String>,
}

pub fn remove(paths: &[PathBuf], mode: Removal) -> Report {
    remove_with(paths, mode, |path| {
        gio::File::for_path(path)
            .trash(None::<&gio::Cancellable>)
            .map_err(|error| error.to_string())
    })
}

/// A trash service failure is always reported; it never authorizes deletion.
pub fn remove_with(
    paths: &[PathBuf],
    mode: Removal,
    trash: impl Fn(&Path) -> Result<(), String>,
) -> Report {
    let mut report = Report::default();
    for path in paths {
        let valid = path.is_absolute()
            && path.components().any(|p| matches!(p, Component::Normal(_)))
            && !path.components().any(|p| matches!(p, Component::ParentDir));
        let result = if !valid {
            Err("Refusing to remove a root or ambiguous relative path".into())
        } else if mode == Removal::Trash {
            trash(path)
        } else {
            permanent(path).map_err(|error| error.to_string())
        };
        match result {
            Ok(()) => report.completed += 1,
            Err(error) => report.errors.push(format!("{}: {error}", path.display())),
        }
    }
    report
}

fn permanent(path: &Path) -> std::io::Result<()> {
    // symlink_metadata and remove_file ensure links (including broken links)
    // are unlinked, rather than recursing into their targets.
    let metadata = fs::symlink_metadata(path)?;
    if metadata.is_dir() && !metadata.file_type().is_symlink() {
        fs::remove_dir_all(path)
    } else {
        fs::remove_file(path)
    }
}
