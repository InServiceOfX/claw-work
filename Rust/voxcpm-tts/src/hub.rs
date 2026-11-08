//! Model download into the standard Hugging Face hub cache (shared with
//! mlx-audio and the `hf` CLI, so weights are stored once).

use std::path::PathBuf;

use anyhow::{Context, Result};

/// Files a VoxCPM2 MLX checkpoint consists of (weights listed in the index).
const FILES: &[&str] = &[
    "config.json",
    "model.safetensors.index.json",
    "tokenizer.json",
    "tokenizer_config.json",
    "special_tokens_map.json",
];

/// Download `model_id` and return its snapshot directory.
pub fn fetch_model(model_id: &str) -> Result<PathBuf> {
    if let Some(path) = crate::config::model_snapshot_dir(model_id) {
        return Ok(path);
    }
    if std::env::var("HF_HUB_OFFLINE")
        .is_ok_and(|s| matches!(s.to_ascii_lowercase().as_str(), "1" | "true" | "yes" | "on"))
    {
        anyhow::bail!(
            "model is not completely cached and HF_HUB_OFFLINE is enabled; run `voxcpm-tts fetch` online first"
        );
    }
    let api = hf_hub::api::sync::ApiBuilder::new()
        .with_cache_dir(crate::config::hf_hub_cache())
        .with_progress(true)
        .build()
        .context("initializing Hugging Face client")?;
    let repo = api.model(model_id.to_string());
    let mut snapshot = None;
    for f in FILES {
        let p = repo.get(f).with_context(|| format!("downloading {f}"))?;
        snapshot = p.parent().map(PathBuf::from);
    }
    let index_path = snapshot
        .as_ref()
        .expect("files downloaded")
        .join("model.safetensors.index.json");
    let index: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(&index_path)?)?;
    let mut shards: Vec<String> = index["weight_map"]
        .as_object()
        .context("weight_map missing from index")?
        .values()
        .filter_map(|v| v.as_str().map(str::to_string))
        .collect();
    shards.sort();
    shards.dedup();
    for shard in shards {
        repo.get(&shard)
            .with_context(|| format!("downloading {shard}"))?;
    }
    snapshot.context("no files downloaded")
}
