// The harness may set TMPDIR inside a Git tree. Generated artifacts must remain
// outside source checkouts, so the Linux experiment tests use explicit scratch.
pub fn tempdir() -> tempfile::TempDir {
    tempfile::Builder::new()
        .prefix("claw-wsp-test-")
        .tempdir_in("/tmp")
        .unwrap()
}
