//! Test helpers for comparing against `reference/golden/` (see PORTING_SPEC §11–12).
//! Tests that need golden files or model weights skip (pass with a note) when absent.

use std::collections::HashMap;
use std::path::PathBuf;

use anyhow::Result;
use candle_core::{DType, Device, Tensor};

pub fn golden_dir() -> Option<PathBuf> {
    let d = std::env::var_os("VOXCPM_TTS_GOLDEN")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("reference")
                .join("golden")
        });
    d.join("meta.json").exists().then_some(d)
}

pub fn snapshot_dir() -> Option<PathBuf> {
    crate::config::model_snapshot_dir(crate::config::MODEL_ID)
}

/// Golden dir + model snapshot, or `None` (with a note) so the test can return early.
pub fn require() -> Option<(PathBuf, PathBuf)> {
    match (golden_dir(), snapshot_dir()) {
        (Some(g), Some(s)) => {
            let meta: serde_json::Value = serde_json::from_slice(
                &std::fs::read(g.join("meta.json")).expect("read golden metadata"),
            )
            .expect("parse golden metadata");
            assert_eq!(
                meta["mlx_enable_tf32"].as_str(),
                Some("0"),
                "regenerate golden with MLX_ENABLE_TF32=0 for full-f32 numerical checks"
            );
            Some((g, s))
        }
        _ => {
            eprintln!("skipping: golden tensors or model weights not available");
            None
        }
    }
}

pub fn load(file: &str, device: &Device) -> Result<HashMap<String, Tensor>> {
    let dir = golden_dir().ok_or_else(|| anyhow::anyhow!("no golden dir"))?;
    Ok(candle_core::safetensors::load(dir.join(file), device)?)
}

pub fn meta() -> Result<serde_json::Value> {
    let dir = golden_dir().ok_or_else(|| anyhow::anyhow!("no golden dir"))?;
    Ok(serde_json::from_str(&std::fs::read_to_string(
        dir.join("meta.json"),
    )?)?)
}

/// ‖a−b‖₂ / ‖b‖₂ computed in f32 on the CPU.
pub fn rel_err(a: &Tensor, b: &Tensor) -> Result<f32> {
    let a = a
        .to_dtype(DType::F32)?
        .to_device(&Device::Cpu)?
        .flatten_all()?;
    let b = b
        .to_dtype(DType::F32)?
        .to_device(&Device::Cpu)?
        .flatten_all()?;
    anyhow::ensure!(
        a.dims() == b.dims(),
        "shape mismatch {:?} vs {:?}",
        a.dims(),
        b.dims()
    );
    let num = (&a - &b)?.sqr()?.sum_all()?.to_scalar::<f32>()?.sqrt();
    let den = b.sqr()?.sum_all()?.to_scalar::<f32>()?.sqrt().max(1e-12);
    Ok(num / den)
}

pub fn max_abs_err(a: &Tensor, b: &Tensor) -> Result<f32> {
    let a = a
        .to_dtype(DType::F32)?
        .to_device(&Device::Cpu)?
        .flatten_all()?;
    let b = b
        .to_dtype(DType::F32)?
        .to_device(&Device::Cpu)?
        .flatten_all()?;
    Ok((&a - &b)?.abs()?.max(0)?.to_scalar::<f32>()?)
}
