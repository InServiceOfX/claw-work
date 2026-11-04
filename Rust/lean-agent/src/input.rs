use crate::Result;
use cap_std::{ambient_authority, fs::Dir};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::{Read, Write},
    path::{Path, PathBuf},
};

#[derive(Debug, Clone)]
pub struct InventoryLimits {
    pub max_entries: usize,
    pub max_files: usize,
    pub max_file_bytes: u64,
    pub max_total_bytes: u64,
}
impl Default for InventoryLimits {
    fn default() -> Self {
        Self {
            max_entries: 20_000,
            max_files: 2_000,
            max_file_bytes: 1_048_576,
            max_total_bytes: 64 * 1_048_576,
        }
    }
}
#[derive(Debug, Serialize, Deserialize)]
pub struct SourceFile {
    pub path: String,
    pub bytes: u64,
    pub sha256: String,
}
#[derive(Debug, Serialize, Deserialize)]
pub struct Skipped {
    pub path: String,
    pub reason: String,
}
#[derive(Debug, Serialize, Deserialize)]
pub struct Inventory {
    pub schema_version: u32,
    pub files: Vec<SourceFile>,
    pub skipped: Vec<Skipped>,
    pub truncated: bool,
    pub total_bytes: u64,
}
pub fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

pub fn excluded(name: &str) -> bool {
    let n = name.to_ascii_lowercase();
    matches!(
        n.as_str(),
        ".git"
            | ".pdd"
            | ".lake"
            | ".local"
            | ".ssh"
            | ".aws"
            | ".gnupg"
            | ".kube"
            | ".config"
            | ".agents"
            | ".claude"
            | ".codex"
            | ".pi"
            | ".hermes"
            | "target"
            | "build"
            | "dist"
            | "node_modules"
            | "__pycache__"
            | ".venv"
            | "litellm_cache.sqlite"
    ) || matches!(
        n.as_str(),
        ".envrc" | ".npmrc" | ".netrc" | ".pypirc" | ".docker" | ".gitconfig"
    ) || n.ends_with(".csv")
        || n.ends_with(".jsonl")
        || n.ends_with(".log")
        || n.ends_with(".sqlite")
        || n.ends_with(".db")
        || n == ".env"
        || n.starts_with(".env.")
        || n.starts_with(".agentic_prompt_")
        || n.ends_with(".pem")
        || n.ends_with(".key")
        || n.ends_with(".p12")
        || n.ends_with(".olean")
        || n.ends_with(".ilean")
        || n.ends_with(".ir")
        || n == "credentials"
        || n == "credentials.json"
        || n == "id_rsa"
        || n == "id_ed25519"
}
pub fn no_symlink_components(path: &Path) -> Result<PathBuf> {
    let absolute = if path.is_absolute() {
        path.to_owned()
    } else {
        std::env::current_dir()?.join(path)
    };
    let mut p = PathBuf::new();
    for part in absolute.components() {
        p.push(part);
        match fs::symlink_metadata(&p) {
            Ok(m) if m.file_type().is_symlink() => {
                return Err("Symlinked input/output paths are not accepted".into());
            }
            Ok(_) => (),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => (),
            Err(e) => return Err(e.into()),
        }
    }
    Ok(absolute)
}
pub fn create_private_dir(path: &Path) -> Result<PathBuf> {
    let p = no_symlink_components(path)?;
    let parent = p
        .parent()
        .ok_or("Output needs a parent directory")?
        .canonicalize()?;
    if parent.ancestors().any(|a| a.join(".git").exists()) {
        return Err("Private outputs must be outside every Git checkout".into());
    }
    let p = parent.join(p.file_name().ok_or("Output needs a fresh directory name")?);
    #[cfg(unix)]
    {
        use std::os::unix::fs::DirBuilderExt;
        fs::DirBuilder::new().mode(0o700).create(&p)?;
    }
    #[cfg(not(unix))]
    return Err("Private workspace permissions currently require a Unix host".into());
    Ok(p)
}
pub fn write_private(path: &Path, bytes: &[u8]) -> Result<()> {
    let mut opts = fs::OpenOptions::new();
    opts.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        opts.mode(0o600);
    }
    opts.open(path)?.write_all(bytes)?;
    Ok(())
}
pub fn read_source(path: &Path, max: u64) -> Result<Vec<u8>> {
    let path = no_symlink_components(path)?;
    if excluded(
        path.file_name()
            .and_then(|x| x.to_str())
            .ok_or("Non-UTF8 filename")?,
    ) {
        return Err("Input is a credential/generated artifact path".into());
    }
    let dir = Dir::open_ambient_dir(path.parent().ok_or("No input parent")?, ambient_authority())?;
    let name = path.file_name().ok_or("No input name")?;
    let meta = dir.symlink_metadata(name)?;
    if !meta.is_file() || meta.len() > max {
        return Err("Input must be a bounded regular file".into());
    }
    let mut bytes = Vec::new();
    dir.open(name)?.take(max + 1).read_to_end(&mut bytes)?;
    if bytes.len() as u64 > max || bytes.contains(&0) || std::str::from_utf8(&bytes).is_err() {
        return Err("Input is oversized, binary or not UTF-8".into());
    }
    Ok(bytes)
}

type Snapshot = (Inventory, Vec<(String, Vec<u8>)>);
fn collect(path: &Path, limits: &InventoryLimits) -> Result<Snapshot> {
    let absolute = no_symlink_components(path)?;
    let meta = fs::symlink_metadata(&absolute)?;
    let mut inv = Inventory {
        schema_version: 1,
        files: vec![],
        skipped: vec![],
        truncated: false,
        total_bytes: 0,
    };
    let mut contents = vec![];
    if meta.is_file() {
        let data = read_source(&absolute, limits.max_file_bytes)?;
        if limits.max_files == 0 || data.len() as u64 > limits.max_total_bytes {
            inv.truncated = true;
            return Ok((inv, contents));
        }
        let name = absolute
            .file_name()
            .and_then(|n| n.to_str())
            .ok_or("Non-UTF8 filename")?
            .to_owned();
        inv.total_bytes = data.len() as u64;
        inv.files.push(SourceFile {
            path: name.clone(),
            bytes: data.len() as u64,
            sha256: hash(&data),
        });
        contents.push((name, data));
        return Ok((inv, contents));
    }
    if !meta.is_dir() {
        return Err("Input must be a regular file or directory".into());
    }
    if absolute
        .file_name()
        .and_then(|n| n.to_str())
        .is_some_and(excluded)
    {
        return Err("Input directory is excluded by privacy policy".into());
    }
    // Capabilities anchor opens to the selected directory; path resolution cannot
    // read outside it even if an input tree is changed while being inventoried.
    let root = Dir::open_ambient_dir(&absolute, ambient_authority())?;
    let mut stack = vec![(String::new(), root)];
    let mut entries_seen = 0;
    'walk: while let Some((prefix, dir)) = stack.pop() {
        let mut names = vec![];
        for entry in dir.entries()? {
            entries_seen += 1;
            if entries_seen > limits.max_entries {
                inv.truncated = true;
                break 'walk;
            }
            let entry = entry?;
            let Ok(name) = entry.file_name().into_string() else {
                inv.skipped.push(Skipped {
                    path: "<non-utf8-name>".into(),
                    reason: "non_utf8_name".into(),
                });
                continue;
            };
            names.push(name);
        }
        names.sort();
        for name in names {
            let rel = if prefix.is_empty() {
                name.clone()
            } else {
                format!("{prefix}/{name}")
            };
            let m = dir.symlink_metadata(&name)?;
            let reason = if m.file_type().is_symlink() {
                Some("symlink")
            } else if excluded(&name) {
                Some("excluded_path")
            } else if !m.is_file() && !m.is_dir() {
                Some("not_regular")
            } else if m.is_file() && m.len() > limits.max_file_bytes {
                Some("oversized")
            } else {
                None
            };
            if let Some(reason) = reason {
                inv.skipped.push(Skipped {
                    path: rel,
                    reason: reason.into(),
                });
                continue;
            }
            if m.is_dir() {
                stack.push((rel, dir.open_dir(&name)?));
                continue;
            }
            if inv.files.len() >= limits.max_files {
                inv.truncated = true;
                break 'walk;
            }
            let mut data = vec![];
            dir.open(&name)?
                .take(limits.max_file_bytes + 1)
                .read_to_end(&mut data)?;
            let reason = if data.len() as u64 > limits.max_file_bytes {
                Some("oversized")
            } else if data.contains(&0) || std::str::from_utf8(&data).is_err() {
                Some("binary_or_non_utf8")
            } else {
                None
            };
            if let Some(reason) = reason {
                inv.skipped.push(Skipped {
                    path: rel,
                    reason: reason.into(),
                });
                continue;
            }
            if inv.total_bytes + data.len() as u64 > limits.max_total_bytes {
                inv.truncated = true;
                break 'walk;
            }
            inv.total_bytes += data.len() as u64;
            inv.files.push(SourceFile {
                path: rel.clone(),
                bytes: data.len() as u64,
                sha256: hash(&data),
            });
            contents.push((rel, data));
        }
    }
    inv.files.sort_by(|a, b| a.path.cmp(&b.path));
    inv.skipped.sort_by(|a, b| a.path.cmp(&b.path));
    Ok((inv, contents))
}
pub fn inventory_target(path: &Path, limits: &InventoryLimits) -> Result<Inventory> {
    Ok(collect(path, limits)?.0)
}
pub fn prepare_review(input: &Path, output: &Path, limits: &InventoryLimits) -> Result<Inventory> {
    let source = no_symlink_components(input)?.canonicalize()?;
    let out_abs = no_symlink_components(output)?;
    if out_abs.starts_with(&source) {
        return Err("Output must not be inside the input tree".into());
    }
    let (inv, contents) = collect(&source, limits)?;
    let out = create_private_dir(output)?;
    fs::create_dir(out.join("source"))?;
    for (rel, bytes) in contents {
        let file = out.join("source").join(rel);
        fs::create_dir_all(file.parent().ok_or("No snapshot parent")?)?;
        write_private(&file, &bytes)?;
    }
    write_private(
        &out.join("inventory.json"),
        &serde_json::to_vec_pretty(&inv)?,
    )?;
    write_private(&out.join("NEXT.md"), b"Private review snapshot. No proof has been attempted. Have the authorized agent select independent intended properties, document semantics and source mappings, and write Challenge.lean, Solution.lean and manifest.json. Obtain approval before remote model processing. A model proof does not establish source correspondence.\n")?;
    Ok(inv)
}
