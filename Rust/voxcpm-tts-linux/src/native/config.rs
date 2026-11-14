//! `config.json` of the VoxCPM2 checkpoint (see docs/PORTING_SPEC.md §1).

use std::path::Path;

use anyhow::{Context, Result};
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct RopeScaling {
    #[serde(rename = "type")]
    pub kind: String,
    pub long_factor: Vec<f32>,
    pub short_factor: Vec<f32>,
    pub original_max_position_embeddings: usize,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LmConfig {
    pub hidden_size: usize,
    pub intermediate_size: usize,
    pub max_position_embeddings: usize,
    pub num_attention_heads: usize,
    pub num_hidden_layers: usize,
    pub num_key_value_heads: usize,
    pub rms_norm_eps: f64,
    pub rope_theta: f32,
    pub kv_channels: usize,
    pub rope_scaling: RopeScaling,
    pub vocab_size: usize,
    pub use_mup: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SmallStackConfig {
    pub hidden_dim: usize,
    pub ffn_dim: usize,
    pub num_heads: usize,
    pub num_layers: usize,
    pub kv_channels: usize,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DitConfig {
    pub hidden_dim: usize,
    pub ffn_dim: usize,
    pub num_heads: usize,
    pub num_layers: usize,
    pub kv_channels: usize,
    #[serde(default)]
    pub mean_mode: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct VaeConfig {
    pub encoder_dim: usize,
    pub encoder_rates: Vec<usize>,
    pub latent_dim: usize,
    pub decoder_dim: usize,
    pub decoder_rates: Vec<usize>,
    pub sr_bin_boundaries: Vec<u32>,
    pub sample_rate: u32,
    pub out_sample_rate: u32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ModelConfig {
    pub lm_config: LmConfig,
    pub patch_size: usize,
    pub feat_dim: usize,
    pub scalar_quantization_latent_dim: usize,
    pub scalar_quantization_scale: f32,
    pub residual_lm_num_layers: usize,
    pub residual_lm_no_rope: bool,
    pub encoder_config: SmallStackConfig,
    pub dit_config: DitConfig,
    pub audio_vae_config: VaeConfig,
    pub max_length: usize,
}

impl ModelConfig {
    pub fn load(path: &Path) -> Result<Self> {
        let s =
            std::fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
        serde_json::from_str(&s).with_context(|| format!("parsing {}", path.display()))
    }
}
