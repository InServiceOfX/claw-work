//! Defaults, well-known locations and the user's local settings.
//!
//! Unlike the macOS tool, this build never touches the Hugging Face hub cache.
//! The weights are a plain directory you `git clone`d; its absolute path comes
//! from (highest priority first):
//!
//! 1. `--model-dir /abs/path` on the command line,
//! 2. `VOXCPM_TTS_MODEL_DIR`,
//! 3. `model_dir` in the config file (`$VOXCPM_TTS_CONFIG`, otherwise
//!    `${XDG_CONFIG_HOME:-~/.config}/voxcpm-tts/config.toml`).
//!
//! `config.toml.example` in the source tree documents every key.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use serde::Deserialize;

/// The open-weight checkpoint this tool expects (clone it from Hugging Face).
pub const MODEL_ID: &str = "mlx-community/VoxCPM2-bf16";
pub const MODEL_GIT_URL: &str = "https://huggingface.co/mlx-community/VoxCPM2-bf16";
/// VoxCPM2's AudioVAE V2 decodes at 48 kHz.
pub const OUTPUT_SAMPLE_RATE: u32 = 48_000;
/// Default characters per synthesis chunk.
pub const DEFAULT_MAX_CHARS: usize = 300;
pub const DEFAULT_CFG_VALUE: f32 = 2.0;
pub const DEFAULT_INFERENCE_TIMESTEPS: usize = 10;

/// Reference voice clips outside this range are rejected / trimmed.
pub const MIN_REFERENCE_SECONDS: f64 = 2.0;
pub const MAX_REFERENCE_SECONDS: f64 = 30.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum DeviceKind {
    #[default]
    Cuda,
    Cpu,
}

/// Compute precision of the transformer stacks. The AudioVAE is always f32.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Precision {
    /// Verified path; ~10 GB of GPU memory for weights.
    #[default]
    F32,
    /// ~5 GB of GPU memory; fits comfortably on a 12 GB card.
    Bf16,
}

/// Raw contents of the config file; every key is optional.
#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct FileSettings {
    model_dir: Option<PathBuf>,
    device: Option<DeviceKind>,
    cuda_device: Option<usize>,
    dtype: Option<Precision>,
    data_dir: Option<PathBuf>,
}

/// Resolved settings (file < environment < command line).
#[derive(Debug, Clone)]
pub struct Settings {
    /// Absolute path of the cloned checkpoint directory, if configured.
    pub model_dir: Option<PathBuf>,
    pub device: DeviceKind,
    pub cuda_device: usize,
    pub dtype: Precision,
    /// Where calibration is stored.
    pub data_dir: PathBuf,
    /// The config file that was read, if any.
    pub config_file: Option<PathBuf>,
}

fn home() -> PathBuf {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."))
}

/// `${XDG_CONFIG_HOME:-~/.config}/voxcpm-tts/config.toml`, or `$VOXCPM_TTS_CONFIG`.
pub fn config_file_path() -> PathBuf {
    if let Some(p) = std::env::var_os("VOXCPM_TTS_CONFIG") {
        return PathBuf::from(p);
    }
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .filter(|p| p.is_absolute())
        .unwrap_or_else(|| home().join(".config"));
    base.join("voxcpm-tts").join("config.toml")
}

/// `${XDG_DATA_HOME:-~/.local/share}/voxcpm-tts` (override: `VOXCPM_TTS_HOME`).
pub fn default_data_dir() -> PathBuf {
    if let Some(p) = std::env::var_os("VOXCPM_TTS_HOME") {
        return PathBuf::from(p);
    }
    std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .filter(|p| p.is_absolute())
        .unwrap_or_else(|| home().join(".local").join("share"))
        .join("voxcpm-tts")
}

fn env_nonempty(name: &str) -> Option<String> {
    std::env::var(name).ok().filter(|v| !v.trim().is_empty())
}

fn parse_device(value: &str) -> Result<DeviceKind> {
    match value.trim().to_ascii_lowercase().as_str() {
        "cuda" | "gpu" => Ok(DeviceKind::Cuda),
        "cpu" => Ok(DeviceKind::Cpu),
        other => bail!("device must be `cuda` or `cpu`, not `{other}`"),
    }
}

fn parse_dtype(value: &str) -> Result<Precision> {
    match value.trim().to_ascii_lowercase().as_str() {
        "f32" | "float32" => Ok(Precision::F32),
        "bf16" | "bfloat16" => Ok(Precision::Bf16),
        other => bail!("dtype must be `f32` or `bf16`, not `{other}`"),
    }
}

impl Settings {
    /// Load the config file (if present), then apply environment and CLI overrides.
    pub fn load(cli_model_dir: Option<&Path>) -> Result<Self> {
        let path = config_file_path();
        let explicit = std::env::var_os("VOXCPM_TTS_CONFIG").is_some();
        let (file, config_file) = match std::fs::read_to_string(&path) {
            Ok(text) => (
                toml::from_str::<FileSettings>(&text)
                    .with_context(|| format!("parsing {}", path.display()))?,
                Some(path),
            ),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound && !explicit => {
                (FileSettings::default(), None)
            }
            Err(e) => return Err(e).with_context(|| format!("reading {}", path.display())),
        };

        let mut device = file.device.unwrap_or_default();
        if let Some(v) = env_nonempty("VOXCPM_TTS_DEVICE") {
            device = parse_device(&v)?;
        }
        // Legacy debugging switch kept from the macOS tool.
        if std::env::var_os("VOXCPM_TTS_CPU").is_some() {
            device = DeviceKind::Cpu;
        }
        let mut cuda_device = file.cuda_device.unwrap_or(0);
        if let Some(v) = env_nonempty("VOXCPM_TTS_CUDA_DEVICE") {
            cuda_device = v
                .trim()
                .parse()
                .with_context(|| format!("VOXCPM_TTS_CUDA_DEVICE `{v}` is not a number"))?;
        }
        let mut dtype = file.dtype.unwrap_or_default();
        if let Some(v) = env_nonempty("VOXCPM_TTS_DTYPE") {
            dtype = parse_dtype(&v)?;
        }
        let model_dir = cli_model_dir
            .map(Path::to_path_buf)
            .or_else(|| env_nonempty("VOXCPM_TTS_MODEL_DIR").map(PathBuf::from))
            .or(file.model_dir)
            .map(|p| expand_home(&p));
        let data_dir = if std::env::var_os("VOXCPM_TTS_HOME").is_some() {
            default_data_dir()
        } else {
            file.data_dir
                .map(|p| expand_home(&p))
                .unwrap_or_else(default_data_dir)
        };
        Ok(Self {
            model_dir,
            device,
            cuda_device,
            dtype,
            data_dir,
            config_file,
        })
    }

    /// The configured weights directory, checked for completeness.
    pub fn require_model_dir(&self) -> Result<PathBuf> {
        let Some(dir) = &self.model_dir else {
            bail!(
                "no model directory configured. Clone the weights:\n  git lfs install\n  git clone {MODEL_GIT_URL} /absolute/path/VoxCPM2-bf16\n\
                 then set `model_dir` in {} (see config.toml.example), export VOXCPM_TTS_MODEL_DIR, or pass --model-dir.",
                config_file_path().display()
            );
        };
        if !dir.is_absolute() {
            bail!("model_dir must be an absolute path, got {}", dir.display());
        }
        match missing_files(dir) {
            Ok(()) => Ok(dir.clone()),
            Err(problem) => bail!(
                "{} is not a complete VoxCPM2 checkpoint: {problem}\n\
                 If it was cloned without git-lfs the .safetensors files are tiny text pointers; run `git lfs pull` inside it.",
                dir.display()
            ),
        }
    }

    pub fn calibration_path(&self) -> PathBuf {
        self.data_dir.join("calibration.json")
    }
}

/// A leading `~/` is expanded; anything else is returned as given.
fn expand_home(p: &Path) -> PathBuf {
    match p.strip_prefix("~") {
        Ok(rest) => home().join(rest),
        Err(_) => p.to_path_buf(),
    }
}

/// `Ok` when `dir` holds every inference file; otherwise a human-readable problem.
pub fn missing_files(dir: &Path) -> std::result::Result<(), String> {
    if !dir.is_dir() {
        return Err("directory does not exist".into());
    }
    for file in [
        "config.json",
        "tokenizer.json",
        "tokenizer_config.json",
        "special_tokens_map.json",
        "model.safetensors.index.json",
    ] {
        if !dir.join(file).is_file() {
            return Err(format!("missing {file}"));
        }
    }
    let index = std::fs::read(dir.join("model.safetensors.index.json"))
        .ok()
        .and_then(|b| serde_json::from_slice::<serde_json::Value>(&b).ok())
        .ok_or("model.safetensors.index.json is not valid JSON")?;
    let map = index["weight_map"]
        .as_object()
        .filter(|m| !m.is_empty())
        .ok_or("weight_map missing from the index")?;
    let mut shards: Vec<&str> = map.values().filter_map(|v| v.as_str()).collect();
    shards.sort_unstable();
    shards.dedup();
    for shard in shards {
        let path = dir.join(shard);
        match std::fs::metadata(&path) {
            // A git-lfs pointer file is ~130 bytes; real shards are hundreds of MB.
            Ok(m) if m.is_file() && m.len() > 1_000_000 => {}
            Ok(m) if m.is_file() => {
                return Err(format!(
                    "{shard} is only {} bytes (an un-pulled git-lfs pointer?)",
                    m.len()
                ));
            }
            _ => return Err(format!("missing {shard}")),
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture(shard_len: usize) -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        for file in [
            "config.json",
            "tokenizer.json",
            "tokenizer_config.json",
            "special_tokens_map.json",
        ] {
            std::fs::write(dir.path().join(file), "{}").unwrap();
        }
        std::fs::write(
            dir.path().join("model.safetensors.index.json"),
            r#"{"weight_map":{"a":"part.safetensors"}}"#,
        )
        .unwrap();
        std::fs::write(dir.path().join("part.safetensors"), vec![0u8; shard_len]).unwrap();
        dir
    }

    #[test]
    fn complete_checkpoint_is_accepted() {
        assert!(missing_files(fixture(2_000_000).path()).is_ok());
    }

    #[test]
    fn lfs_pointer_shard_is_rejected() {
        let err = missing_files(fixture(130).path()).unwrap_err();
        assert!(err.contains("git-lfs pointer"), "{err}");
    }

    #[test]
    fn missing_shard_and_missing_dir_are_rejected() {
        let dir = fixture(2_000_000);
        std::fs::remove_file(dir.path().join("part.safetensors")).unwrap();
        assert!(
            missing_files(dir.path())
                .unwrap_err()
                .contains("missing part")
        );
        assert!(missing_files(&dir.path().join("nope")).is_err());
    }

    #[test]
    fn config_file_keys_parse_and_unknown_keys_fail() {
        let ok: FileSettings = toml::from_str(
            "model_dir = \"/m/VoxCPM2-bf16\"\ndevice = \"cpu\"\ncuda_device = 1\ndtype = \"bf16\"\n",
        )
        .unwrap();
        assert_eq!(ok.device, Some(DeviceKind::Cpu));
        assert_eq!(ok.dtype, Some(Precision::Bf16));
        assert_eq!(ok.cuda_device, Some(1));
        assert!(toml::from_str::<FileSettings>("modle_dir = \"x\"").is_err());
    }

    #[test]
    fn tilde_expands_but_relative_paths_are_untouched() {
        assert_eq!(expand_home(Path::new("rel/x")), PathBuf::from("rel/x"));
        assert!(expand_home(Path::new("~/x")).ends_with("x"));
        assert!(expand_home(Path::new("~/x")).is_absolute() || home() == Path::new("."));
    }
}
