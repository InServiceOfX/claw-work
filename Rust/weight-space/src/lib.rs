pub mod assets;
pub mod edit;
pub mod evaluation;
pub mod gguf;
pub mod projection;
pub mod server;

use anyhow::Result;
use sha2::{Digest, Sha256};
use std::{fs::File, io::Read, path::Path};

pub fn hash_bytes(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

pub fn hash_file(path: &Path) -> Result<String> {
    let mut f = File::open(path)?;
    let mut hash = Sha256::new();
    let mut buf = vec![0; 1024 * 1024];
    loop {
        let n = f.read(&mut buf)?;
        if n == 0 {
            break;
        }
        hash.update(&buf[..n]);
    }
    Ok(format!("{:x}", hash.finalize()))
}

pub fn write_json_new(path: &Path, value: &impl serde::Serialize) -> Result<()> {
    use std::io::Write;
    ensure_external_output(path)?;
    let mut f = File::options().write(true).create_new(true).open(path)?;
    serde_json::to_writer_pretty(&mut f, value)?;
    f.write_all(b"\n")?;
    f.sync_all()?;
    Ok(())
}

/// Resolve existing ancestors (including symlinks) and refuse generated assets
/// anywhere under a Git checkout. No model or raw inference evidence in source.
pub fn ensure_external_output(path: &Path) -> Result<()> {
    use anyhow::{Context, ensure};
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()?.join(path)
    };
    let mut existing = absolute.as_path();
    while !existing.exists() {
        existing = existing
            .parent()
            .context("output has no existing ancestor")?;
    }
    let resolved = std::fs::canonicalize(existing)?;
    for ancestor in resolved.ancestors() {
        let marker = ancestor.join(".git");
        // Sandboxed writable roots can contain an empty, read-only .git guard.
        // A real checkout has a gitdir file or a directory containing HEAD.
        ensure!(
            !(marker.is_file() || marker.join("HEAD").is_file()),
            "generated model/evidence output must be outside Git checkouts"
        );
    }
    Ok(())
}
