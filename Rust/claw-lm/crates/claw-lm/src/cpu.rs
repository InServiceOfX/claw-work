//! Scalar FP32 correctness reference. GPU deployment uses CUDA C++.
use crate::{
    assets::{Assets, Config},
    generation::{Attention, Decoder},
};
use anyhow::{Result, ensure};

pub fn yarn_frequencies(c: &Config) -> Vec<f32> {
    let r = &c.rope_parameters;
    let d = c.head_dim as f64;
    let correction = |rot: f64| {
        d * (r.original_max_position_embeddings as f64 / (rot * 2. * std::f64::consts::PI)).ln()
            / (2. * r.rope_theta.ln())
    };
    let low = correction(r.beta_fast).floor().max(0.);
    let high = correction(r.beta_slow).ceil().min(d - 1.);
    let width = if low == high { 0.001 } else { high - low };
    (0..c.head_dim / 2)
        .map(|j| {
            let ramp = ((j as f64 - low) / width).clamp(0., 1.);
            (r.rope_theta.powf(-2. * j as f64 / d) * (1. - ramp + ramp / r.factor)) as f32
        })
        .collect()
}
fn round_bf16(value: f32) -> f32 {
    let bits = value.to_bits();
    f32::from_bits(bits.wrapping_add(0x7fff + ((bits >> 16) & 1)) & 0xffff0000)
}
fn rms(x: &[f32], w: &[f32], eps: f32) -> Vec<f32> {
    let scale = (x.iter().map(|v| v * v).sum::<f32>() / x.len() as f32 + eps)
        .sqrt()
        .recip();
    x.iter().zip(w).map(|(a, b)| a * scale * b).collect()
}
pub struct CpuDecoder {
    assets: Assets,
    limit: usize,
    cache: usize,
    keys: Vec<Vec<Vec<f32>>>,
    values: Vec<Vec<Vec<f32>>>,
    freq: Vec<f32>,
}
impl CpuDecoder {
    pub fn new(assets: Assets, limit: usize) -> Result<Self> {
        ensure!(
            limit > 0 && limit <= assets.config.max_position_embeddings,
            "Invalid context limit"
        );
        let n = assets.config.num_hidden_layers;
        let freq = yarn_frequencies(&assets.config);
        Ok(Self {
            assets,
            limit,
            cache: 0,
            keys: vec![Vec::new(); n],
            values: vec![Vec::new(); n],
            freq,
        })
    }
    fn linear(&self, name: &str, x: &[f32]) -> Result<Vec<f32>> {
        let t = self.assets.weights.tensor(name)?;
        Ok((0..t.shape[0])
            .map(|r| {
                x.iter()
                    .enumerate()
                    .map(|(i, v)| t.value(r * x.len() + i) * round_bf16(*v))
                    .sum()
            })
            .collect())
    }
    fn norm(&self, name: &str, x: &[f32]) -> Result<Vec<f32>> {
        Ok(rms(
            x,
            &self.assets.weights.tensor(name)?.to_f32(),
            self.assets.config.rms_norm_eps,
        ))
    }
    fn rotate(&self, x: &mut [f32], position: usize) {
        let c = &self.assets.config;
        let r = &c.rope_parameters;
        let mscale = |s: f64| {
            if r.factor <= 1. {
                1.
            } else {
                1. + 0.1 * s * r.factor.ln()
            }
        };
        let magnitude = (mscale(r.mscale) / mscale(r.mscale_all_dim)) as f32;
        for head in x.chunks_exact_mut(c.head_dim) {
            for (j, &freq) in self.freq.iter().enumerate() {
                let angle = position as f32 * freq;
                let (sin, cos) = angle.sin_cos();
                let a = head[j];
                let b = head[j + c.head_dim / 2];
                head[j] = (a * cos - b * sin) * magnitude;
                head[j + c.head_dim / 2] = (b * cos + a * sin) * magnitude;
            }
        }
    }
}
impl Decoder for CpuDecoder {
    fn vocab_size(&self) -> usize {
        self.assets.config.vocab_size
    }
    fn context_limit(&self) -> usize {
        self.limit
    }
    fn cache_len(&self) -> usize {
        self.cache
    }
    fn reset(&mut self) -> Result<()> {
        self.crop(0)
    }
    fn crop(&mut self, n: usize) -> Result<()> {
        ensure!(n <= self.cache, "Invalid cache crop");
        for k in &mut self.keys {
            k.truncate(n);
        }
        for v in &mut self.values {
            v.truncate(n);
        }
        self.cache = n;
        Ok(())
    }
    fn forward(
        &mut self,
        tokens: &[u32],
        attention: Attention,
        commit: bool,
        last: bool,
    ) -> Result<Vec<Vec<f32>>> {
        let c = self.assets.config.clone();
        ensure!(
            !tokens.is_empty() && self.cache + tokens.len() <= self.limit,
            "Invalid forward size"
        );
        let embed = self.assets.weights.tensor("encoder.embed_tokens.weight")?;
        let mut x = tokens
            .iter()
            .map(|&id| {
                ensure!((id as usize) < c.vocab_size, "Token outside vocabulary");
                Ok((0..c.hidden_size)
                    .map(|j| embed.value(id as usize * c.hidden_size + j))
                    .collect::<Vec<_>>())
            })
            .collect::<Result<Vec<_>>>()?;
        for layer in 0..c.num_hidden_layers {
            let prefix = format!("encoder.layers.{layer}.");
            let mut queries = Vec::new();
            let mut keys = Vec::new();
            let mut values = Vec::new();
            for (i, row) in x.iter().enumerate() {
                let n = self.norm(&(prefix.clone() + "input_layernorm.weight"), row)?;
                let mut q = self.linear(&(prefix.clone() + "self_attn.q_proj.weight"), &n)?;
                let mut k = self.linear(&(prefix.clone() + "self_attn.k_proj.weight"), &n)?;
                self.rotate(&mut q, self.cache + i);
                self.rotate(&mut k, self.cache + i);
                let scale = 1.
                    + c.rope_parameters.llama_4_scaling_beta as f32
                        * (1.
                            + ((self.cache + i)
                                / c.rope_parameters.original_max_position_embeddings)
                                as f32)
                            .ln();
                q.iter_mut().for_each(|v| *v *= scale);
                queries.push(q);
                keys.push(k);
                values.push(self.linear(&(prefix.clone() + "self_attn.v_proj.weight"), &n)?);
            }
            for i in 0..tokens.len() {
                let end = if attention == Attention::Causal {
                    self.cache + i + 1
                } else {
                    self.cache + tokens.len()
                };
                let mut joined = vec![0.; c.num_attention_heads * c.head_dim];
                for head in 0..c.num_attention_heads {
                    let kv = head / (c.num_attention_heads / c.num_key_value_heads);
                    let q = &queries[i][head * c.head_dim..(head + 1) * c.head_dim];
                    let mut scores = Vec::new();
                    for j in 0..end {
                        let k = if j < self.cache {
                            &self.keys[layer][j]
                        } else {
                            &keys[j - self.cache]
                        };
                        scores.push(
                            q.iter()
                                .zip(&k[kv * c.head_dim..(kv + 1) * c.head_dim])
                                .map(|(a, b)| a * b)
                                .sum::<f32>()
                                / (c.head_dim as f32).sqrt(),
                        );
                    }
                    let m = scores.iter().copied().fold(f32::NEG_INFINITY, f32::max);
                    scores.iter_mut().for_each(|s| *s = (*s - m).exp());
                    let total = scores.iter().sum::<f32>();
                    for (j, s) in scores.iter().enumerate() {
                        let v = if j < self.cache {
                            &self.values[layer][j]
                        } else {
                            &values[j - self.cache]
                        };
                        for d in 0..c.head_dim {
                            joined[head * c.head_dim + d] += s / total * v[kv * c.head_dim + d];
                        }
                    }
                }
                let p = self.linear(&(prefix.clone() + "self_attn.o_proj.weight"), &joined)?;
                x[i].iter_mut().zip(p).for_each(|(a, b)| *a += b);
                let norm =
                    self.norm(&(prefix.clone() + "post_attention_layernorm.weight"), &x[i])?;
                let gate = self.linear(&(prefix.clone() + "mlp.gate_proj.weight"), &norm)?;
                let up = self.linear(&(prefix.clone() + "mlp.up_proj.weight"), &norm)?;
                let fused = gate
                    .iter()
                    .zip(up)
                    .map(|(a, b)| a / (1. + (-a).exp()) * b)
                    .collect::<Vec<_>>();
                let p = self.linear(&(prefix.clone() + "mlp.down_proj.weight"), &fused)?;
                x[i].iter_mut().zip(p).for_each(|(a, b)| *a += b);
            }
            if commit {
                self.keys[layer].extend(keys);
                self.values[layer].extend(values);
            }
        }
        if commit {
            self.cache += tokens.len();
        }
        let rows = if last { &x[x.len() - 1..] } else { &x };
        rows.iter()
            .map(|r| {
                self.linear(
                    "diffusion_head.weight",
                    &self.norm("encoder.norm.weight", r)?,
                )
            })
            .collect()
    }
}
