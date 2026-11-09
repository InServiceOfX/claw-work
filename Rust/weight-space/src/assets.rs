use crate::hash_file;
use anyhow::{Context, Result, ensure};
use std::{io::Write, path::Path, time::Duration};

pub const MODEL_REVISION: &str = "807dfd9a6fadbf703207aac3bfb0b2b3040d077c";
pub const PATCH_REVISION: &str = "4d09ba6d6a7e891dd8063974dfe81b3d78e3494f";
pub const MODEL_SHA256: &str = "9a7bed4041b7975e0f71fa34670d1e9025213bc92905ac0db75d36c4fa3fa623";

/// Explicit pinned public downloads only. No ambient tokens or proxy configuration.
pub fn fetch(asset: &str, output: &Path) -> Result<()> {
    crate::ensure_external_output(output)?;
    let (url, expected, limit) = match asset {
        "model" => (
            format!(
                "https://huggingface.co/ggml-org/Qwen3.5-0.8B-GGUF/resolve/{MODEL_REVISION}/Qwen3.5-0.8B-BF16.gguf"
            ),
            MODEL_SHA256,
            2_000_000_000u64,
        ),
        "patch" => (
            format!(
                "https://raw.githubusercontent.com/RikiyaT/lm_understand-logs/{PATCH_REVISION}/exp8_math/runs/p50c/edit/edit.npz"
            ),
            "e92a92aafd1f1068b4fd94aa0fbe081ba3ddea2047ee7bcac7161d861be299e7",
            1_000_000,
        ),
        "manifest" => (
            format!(
                "https://raw.githubusercontent.com/RikiyaT/lm_understand-logs/{PATCH_REVISION}/exp8_math/runs/p50c/edit/manifest.json"
            ),
            "83185c1db46e3dafb34af645f8c5ce033c09cc99d4512b9e1b8e3aba9d269225",
            1_000_000,
        ),
        _ => anyhow::bail!("asset must be model, patch or manifest"),
    };
    ensure!(
        std::fs::symlink_metadata(output).is_err(),
        "download output exists"
    );
    let client = reqwest::blocking::Client::builder()
        .no_proxy()
        .connect_timeout(Duration::from_secs(20))
        .timeout(Duration::from_secs(600))
        .redirect(reqwest::redirect::Policy::custom(|attempt| {
            if attempt.previous().len() >= 5 || attempt.url().scheme() != "https" {
                attempt.error("HTTPS redirect limit")
            } else {
                attempt.follow()
            }
        }))
        .build()?;
    let mut response = client.get(url).send()?.error_for_status()?;
    if let Some(n) = response.content_length() {
        ensure!(n <= limit, "download too large");
    }
    let mut temp =
        tempfile::NamedTempFile::new_in(output.parent().context("download needs parent")?)?;
    let copied = std::io::copy(
        &mut std::io::Read::take(&mut response, limit + 1),
        &mut temp,
    )?;
    ensure!(copied <= limit, "download exceeds limit");
    temp.flush()?;
    temp.as_file().sync_all()?;
    ensure!(
        hash_file(temp.path())? == expected,
        "download hash mismatch"
    );
    temp.persist_noclobber(output)
        .context("download publish failed")?;
    Ok(())
}
