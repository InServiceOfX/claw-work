/// Canonicalize only the OS temporary parent, not an untrusted review path.
/// macOS exposes /var and /tmp as aliases; keep strict symlink rejection intact.
pub fn tempdir() -> std::io::Result<tempfile::TempDir> {
    tempfile::tempdir_in(std::env::temp_dir().canonicalize()?)
}
