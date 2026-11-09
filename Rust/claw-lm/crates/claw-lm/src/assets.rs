use anyhow::{Context, Result, ensure};
use memmap2::Mmap;
use serde::Deserialize;
use std::{collections::BTreeMap, fs::File, path::Path};

#[derive(Clone, Debug, Deserialize)]
pub struct Rope {
    pub rope_type: String,
    pub rope_theta: f64,
    pub factor: f64,
    pub original_max_position_embeddings: usize,
    pub beta_fast: f64,
    pub beta_slow: f64,
    pub mscale: f64,
    pub mscale_all_dim: f64,
    pub llama_4_scaling_beta: f64,
}
#[derive(Clone, Debug, Deserialize)]
pub struct Config {
    pub model_type: String,
    pub hidden_size: usize,
    pub intermediate_size: usize,
    pub num_hidden_layers: usize,
    pub num_attention_heads: usize,
    pub num_key_value_heads: usize,
    pub head_dim: usize,
    pub vocab_size: usize,
    pub max_position_embeddings: usize,
    pub rms_norm_eps: f32,
    pub mask_token_id: u32,
    pub eos_token_id: u32,
    pub rope_parameters: Rope,
    pub hidden_act: String,
    pub dlm_paradigm: String,
    #[serde(default)]
    pub attention_bias: bool,
    #[serde(default)]
    pub mlp_bias: bool,
    #[serde(default)]
    pub tie_word_embeddings: bool,
    pub sliding_window: Option<usize>,
}
impl Config {
    pub fn load(path: &Path) -> Result<Self> {
        let metadata = path.metadata().context("Cannot inspect config.json")?;
        ensure!(metadata.len() <= 1_048_576, "Configuration exceeds 1 MiB");
        let config: Self =
            serde_json::from_reader(File::open(path)?).context("Invalid config.json")?;
        config.validate()?;
        Ok(config)
    }
    pub fn validate(&self) -> Result<()> {
        ensure!(
            self.model_type == "nemotron_labs_diffusion",
            "Unsupported model_type"
        );
        ensure!(
            self.hidden_act == "silu" && self.dlm_paradigm == "bidirectional",
            "Unsupported activation or diffusion paradigm"
        );
        ensure!(
            !self.attention_bias
                && !self.mlp_bias
                && !self.tie_word_embeddings
                && self.sliding_window.is_none(),
            "Unsupported bias, tied embedding or sliding window"
        );
        for d in [
            self.hidden_size,
            self.intermediate_size,
            self.num_hidden_layers,
            self.num_attention_heads,
            self.num_key_value_heads,
            self.head_dim,
            self.vocab_size,
            self.max_position_embeddings,
        ] {
            ensure!(d > 0 && d <= i32::MAX as usize, "Invalid model dimension");
        }
        ensure!(
            self.head_dim.is_multiple_of(2)
                && self
                    .num_attention_heads
                    .is_multiple_of(self.num_key_value_heads),
            "Invalid grouped attention dimensions"
        );
        ensure!(
            (self.mask_token_id as usize) < self.vocab_size
                && (self.eos_token_id as usize) < self.vocab_size
                && self.mask_token_id != self.eos_token_id,
            "Invalid special token IDs"
        );
        ensure!(
            self.rms_norm_eps.is_finite() && self.rms_norm_eps > 0.,
            "Invalid RMS epsilon"
        );
        ensure!(
            self.hidden_size <= 16384
                && self.intermediate_size <= 65536
                && self.num_hidden_layers <= 64
                && self.num_attention_heads <= 128
                && self.num_key_value_heads <= 128
                && self.head_dim <= 256
                && self.vocab_size <= 1048576
                && self.max_position_embeddings <= 1048576,
            "Configuration exceeds native runtime dimension bounds"
        );
        let r = &self.rope_parameters;
        ensure!(
            r.rope_type == "yarn"
                && r.original_max_position_embeddings > 0
                && r.original_max_position_embeddings <= i32::MAX as usize,
            "Only configured YaRN is supported"
        );
        for v in [
            r.rope_theta,
            r.factor,
            r.beta_fast,
            r.beta_slow,
            r.mscale,
            r.mscale_all_dim,
        ] {
            ensure!(v.is_finite() && v > 0., "Invalid YaRN parameters");
        }
        ensure!(
            r.rope_theta > 1.
                && r.factor >= 1.
                && r.beta_fast >= r.beta_slow
                && r.llama_4_scaling_beta.is_finite()
                && r.llama_4_scaling_beta >= 0.,
            "Unsupported YaRN scaling"
        );
        Ok(())
    }
    pub fn tensor_shapes(&self) -> Result<BTreeMap<String, Vec<usize>>> {
        let mut result = BTreeMap::new();
        let h = self.hidden_size;
        let q = self
            .num_attention_heads
            .checked_mul(self.head_dim)
            .context("Q width overflow")?;
        let kv = self
            .num_key_value_heads
            .checked_mul(self.head_dim)
            .context("KV width overflow")?;
        result.insert("diffusion_head.weight".into(), vec![self.vocab_size, h]);
        result.insert(
            "encoder.embed_tokens.weight".into(),
            vec![self.vocab_size, h],
        );
        result.insert("encoder.norm.weight".into(), vec![h]);
        for i in 0..self.num_hidden_layers {
            for (suffix, shape) in [
                ("input_layernorm.weight", vec![h]),
                ("post_attention_layernorm.weight", vec![h]),
                ("self_attn.q_proj.weight", vec![q, h]),
                ("self_attn.k_proj.weight", vec![kv, h]),
                ("self_attn.v_proj.weight", vec![kv, h]),
                ("self_attn.o_proj.weight", vec![h, q]),
                ("mlp.gate_proj.weight", vec![self.intermediate_size, h]),
                ("mlp.up_proj.weight", vec![self.intermediate_size, h]),
                ("mlp.down_proj.weight", vec![h, self.intermediate_size]),
            ] {
                result.insert(format!("encoder.layers.{i}.{suffix}"), shape);
            }
        }
        Ok(result)
    }
}
#[derive(Debug, Deserialize)]
struct TensorInfo {
    dtype: String,
    shape: Vec<usize>,
    data_offsets: [usize; 2],
}
/// The caller must keep model assets immutable while this read-only mapping lives.
pub struct Weights {
    map: Mmap,
    data_start: usize,
    tensors: BTreeMap<String, TensorInfo>,
}
impl Weights {
    pub fn open(path: &Path, config: &Config) -> Result<Self> {
        let file = File::open(path).context("Cannot open model.safetensors")?;
        // SAFETY: read-only mapping; the API documents that model assets stay immutable.
        let map = unsafe { Mmap::map(&file) }.context("Cannot map model weights")?;
        ensure!(map.len() >= 8, "Truncated safetensors header");
        let length = u64::from_le_bytes(map[..8].try_into()?) as usize;
        ensure!(
            length <= 64 * 1024 * 1024,
            "Safetensors header exceeds 64 MiB"
        );
        let data_start = length.checked_add(8).context("Header length overflow")?;
        ensure!(data_start <= map.len(), "Truncated safetensors metadata");
        let mut raw: BTreeMap<String, serde_json::Value> =
            serde_json::from_slice(&map[8..data_start]).context("Invalid safetensors metadata")?;
        raw.remove("__metadata__");
        let tensors: BTreeMap<String, TensorInfo> = raw
            .into_iter()
            .map(|(k, v)| Ok((k, serde_json::from_value(v)?)))
            .collect::<Result<_>>()?;
        let expected = config.tensor_shapes()?;
        ensure!(
            expected.len() == tensors.len() && expected.keys().eq(tensors.keys()),
            "Unexpected or missing tensor names"
        );
        let mut ranges = Vec::new();
        for (name, info) in &tensors {
            ensure!(
                info.dtype == "BF16" && info.shape == expected[name],
                "Wrong dtype or shape for {name}"
            );
            let bytes = info
                .shape
                .iter()
                .try_fold(2usize, |a, b| a.checked_mul(*b))
                .context("Tensor shape overflow")?;
            let [start, end] = info.data_offsets;
            ensure!(
                end >= start && end - start == bytes,
                "Invalid byte range for {name}"
            );
            ranges.push((start, end));
        }
        ranges.sort_unstable();
        let mut cursor = 0;
        for (start, end) in ranges {
            ensure!(
                start == cursor,
                "Overlapping or noncontiguous tensor ranges"
            );
            cursor = end;
        }
        ensure!(
            cursor.checked_add(data_start) == Some(map.len()),
            "Truncated weights or unexpected trailing bytes"
        );
        Ok(Self {
            map,
            data_start,
            tensors,
        })
    }
    pub fn tensor(&self, name: &str) -> Result<Tensor<'_>> {
        let info = self
            .tensors
            .get(name)
            .with_context(|| format!("Missing tensor {name}"))?;
        let [a, b] = info.data_offsets;
        Ok(Tensor {
            bytes: &self.map[self.data_start + a..self.data_start + b],
            shape: &info.shape,
        })
    }
    pub fn byte_len(&self) -> usize {
        self.map.len() - self.data_start
    }
    pub fn names(&self) -> impl Iterator<Item = &str> {
        self.tensors.keys().map(String::as_str)
    }
}
#[derive(Clone, Copy)]
pub struct Tensor<'a> {
    pub bytes: &'a [u8],
    pub shape: &'a [usize],
}
impl Tensor<'_> {
    pub fn value(&self, i: usize) -> f32 {
        f32::from_bits(
            (u16::from_le_bytes([self.bytes[2 * i], self.bytes[2 * i + 1]]) as u32) << 16,
        )
    }
    pub fn to_f32(&self) -> Vec<f32> {
        self.bytes
            .chunks_exact(2)
            .map(|v| f32::from_bits((u16::from_le_bytes([v[0], v[1]]) as u32) << 16))
            .collect()
    }
}
pub struct Assets {
    pub config: Config,
    pub weights: Weights,
}
impl Assets {
    pub fn open(directory: impl AsRef<Path>) -> Result<Self> {
        let directory = directory.as_ref();
        ensure!(directory.is_dir(), "Model directory does not exist");
        let config = Config::load(&directory.join("config.json"))?;
        let weights = Weights::open(&directory.join("model.safetensors"), &config)?;
        Ok(Self { config, weights })
    }
}
/// Public device policy is deliberately an exact allowlist, never >= sm_86.
pub fn supported_gpu(major: i32, minor: i32, name: &str) -> bool {
    let family = if (major, minor) == (8, 6) {
        "GeForce RTX 30"
    } else if (major, minor) == (12, 0) {
        "GeForce RTX 50"
    } else {
        return false;
    };
    name.contains(family)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn device_policy_is_exact() {
        assert!(supported_gpu(8, 6, "NVIDIA GeForce RTX 3060"));
        assert!(supported_gpu(12, 0, "NVIDIA GeForce RTX 5090"));
        for (a, b, n) in [
            (8, 9, "NVIDIA GeForce RTX 4090"),
            (5, 2, "GeForce GTX 980 Ti"),
            (8, 6, "NVIDIA A10"),
            (12, 1, "NVIDIA GB10"),
            (13, 0, "GeForce RTX 6090"),
        ] {
            assert!(!supported_gpu(a, b, n));
        }
    }
}
