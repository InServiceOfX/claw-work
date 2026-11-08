//! Weight loading from the checkpoint's sharded safetensors (memory-mapped).

use std::path::{Path, PathBuf};

use anyhow::{Context, Result, ensure};
use candle_core::{DType, Device};
use candle_nn::VarBuilder;

/// Shard files listed in `model.safetensors.index.json`.
pub fn shard_paths(snapshot: &Path) -> Result<Vec<PathBuf>> {
    let index_path = snapshot.join("model.safetensors.index.json");
    let index: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(&index_path)
            .with_context(|| format!("reading {}", index_path.display()))?,
    )?;
    let mut files: Vec<String> = index["weight_map"]
        .as_object()
        .context("weight_map missing")?
        .values()
        .filter_map(|v| v.as_str().map(str::to_string))
        .collect();
    files.sort();
    files.dedup();
    Ok(files.into_iter().map(|f| snapshot.join(f)).collect())
}

/// A VarBuilder over all shards that converts every tensor to `dtype` on `device`.
pub fn var_builder(snapshot: &Path, dtype: DType, device: &Device) -> Result<VarBuilder<'static>> {
    let shards = shard_paths(snapshot)?;
    // SAFETY: the files are memory-mapped read-only; the HF cache does not
    // modify snapshot files in place.
    let vb = unsafe { VarBuilder::from_mmaped_safetensors(&shards, dtype, device)? };
    Ok(vb)
}

/// Metal GPU 0, or the CPU when `VOXCPM_TTS_CPU` is set (debugging).
pub fn device() -> Result<Device> {
    if std::env::var_os("VOXCPM_TTS_CPU").is_some() {
        return Ok(Device::Cpu);
    }
    // candle 0.11 indexes an empty Metal list and panics in restricted sessions.
    ensure!(
        !candle_metal_kernels::metal::Device::all().is_empty(),
        "no Metal device available; run outside a GPU-restricted sandbox"
    );
    Device::new_metal(0).context("no Metal device available")
}
