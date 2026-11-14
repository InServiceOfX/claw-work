//! Weight loading from the checkpoint's sharded safetensors (memory-mapped).

use std::path::{Path, PathBuf};

use anyhow::{Context, Result, ensure};
use candle_core::{DType, Device};
use candle_nn::VarBuilder;

use crate::config::{DeviceKind, Settings};

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

/// The device selected by the user's settings (CUDA ordinal, or CPU).
pub fn device_for(settings: &Settings) -> Result<Device> {
    match settings.device {
        DeviceKind::Cpu => Ok(Device::Cpu),
        DeviceKind::Cuda => {
            ensure!(
                cfg!(feature = "cuda"),
                "this build has no CUDA support; rebuild with `--features cuda` or set device = \"cpu\""
            );
            Device::new_cuda(settings.cuda_device).with_context(|| {
                format!(
                    "opening CUDA device {} (check `nvidia-smi`, or set device = \"cpu\")",
                    settings.cuda_device
                )
            })
        }
    }
}

/// Device from the default settings; for tests and tools.
pub fn device() -> Result<Device> {
    device_for(&Settings::load(None)?)
}
