//! Defaults and well-known locations.

use std::path::PathBuf;

/// The open-weight checkpoint this tool deploys.
pub const MODEL_ID: &str = "mlx-community/VoxCPM2-bf16";
/// VoxCPM2's AudioVAE V2 decodes at 48 kHz.
pub const OUTPUT_SAMPLE_RATE: u32 = 48_000;
/// Default characters per synthesis chunk (see docs/REFERENCE_ENGINE.md).
pub const DEFAULT_MAX_CHARS: usize = 300;
pub const DEFAULT_CFG_VALUE: f32 = 2.0;
pub const DEFAULT_INFERENCE_TIMESTEPS: usize = 10;

/// Reference voice clips outside this range are rejected / trimmed.
pub const MIN_REFERENCE_SECONDS: f64 = 2.0;
pub const MAX_REFERENCE_SECONDS: f64 = 30.0;

fn home() -> PathBuf {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."))
}

/// `~/Library/Application Support/voxcpm-tts` (override: `VOXCPM_TTS_HOME`).
pub fn app_support_dir() -> PathBuf {
    if let Some(p) = std::env::var_os("VOXCPM_TTS_HOME") {
        return PathBuf::from(p);
    }
    home()
        .join("Library")
        .join("Application Support")
        .join("voxcpm-tts")
}

/// The Hugging Face hub cache (`$HF_HUB_CACHE`, `$HF_HOME/hub`, or `~/.cache/huggingface/hub`).
pub fn hf_hub_cache() -> PathBuf {
    if let Some(p) = std::env::var_os("HF_HUB_CACHE") {
        return PathBuf::from(p);
    }
    if let Some(p) = std::env::var_os("HF_HOME") {
        return PathBuf::from(p).join("hub");
    }
    home().join(".cache").join("huggingface").join("hub")
}

/// Local snapshot directory of `model_id` in the hub cache, if downloaded.
pub fn model_snapshot_dir(model_id: &str) -> Option<PathBuf> {
    let repo = hf_hub_cache().join(format!("models--{}", model_id.replace('/', "--")));
    let rev = std::fs::read_to_string(repo.join("refs").join("main")).ok()?;
    let dir = repo.join("snapshots").join(rev.trim());
    complete_snapshot(&dir).then_some(dir)
}

/// A config alone is not a downloaded checkpoint. Validate all inference files.
pub fn complete_snapshot(dir: &std::path::Path) -> bool {
    for file in [
        "config.json",
        "tokenizer.json",
        "tokenizer_config.json",
        "special_tokens_map.json",
    ] {
        if !dir.join(file).is_file() {
            return false;
        }
    }
    let index = std::fs::read(dir.join("model.safetensors.index.json"))
        .ok()
        .and_then(|b| serde_json::from_slice::<serde_json::Value>(&b).ok());
    index
        .and_then(|i| i["weight_map"].as_object().cloned())
        .is_some_and(|m| {
            !m.is_empty()
                && m.values()
                    .all(|v| v.as_str().is_some_and(|s| dir.join(s).is_file()))
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn partial_download_is_not_a_complete_snapshot() {
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
        assert!(!complete_snapshot(dir.path()));
        std::fs::write(dir.path().join("part.safetensors"), "fixture").unwrap();
        assert!(complete_snapshot(dir.path()));
    }
}
