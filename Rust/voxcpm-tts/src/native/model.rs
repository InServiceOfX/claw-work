//! The full VoxCPM2 model: prefill layout (PORTING_SPEC §5), the
//! autoregressive patch loop (§7) and VAE decoding.

use std::ops::ControlFlow;
use std::path::Path;

use anyhow::{Context, Result, bail, ensure};
use candle_core::{D, DType, Device, IndexOp, Tensor};
use candle_nn::{Embedding, Linear, Module, VarBuilder};

use super::cfm;
use super::config::ModelConfig;
use super::fsq::Fsq;
use super::locdit::LocDit;
use super::locenc::LocEnc;
use super::minicpm::{KvCache, MiniCpmStack, StackConfig};
use super::tokenizer::{AUDIO_START, REF_AUDIO_END, REF_AUDIO_START, TextTokenizer};
use super::vae::AudioVae;
use super::weights;

/// Samples of 16 kHz input per latent frame, and per LM patch.
pub const ENC_HOP: usize = 640;
/// 48 kHz output samples per LM patch (4 frames × 1920).
pub const SAMPLES_PER_PATCH_48K: usize = 7680;
/// Seconds of audio per LM patch.
pub const SECONDS_PER_PATCH: f64 = 0.16;
/// Prompt patches re-decoded as context in continuation mode.
const CONTINUATION_CONTEXT: usize = 3;

/// Source of the per-step diffusion noise `z_i` (`[1, 64, P]`).
pub trait NoiseSource {
    fn next(&mut self, feat_dim: usize, patch: usize, device: &Device) -> Result<Tensor>;
}

/// Deterministic standard-normal noise (SplitMix64 + Box–Muller).
pub struct SeededNoise {
    state: u64,
}

impl SeededNoise {
    pub fn new(seed: u64) -> Self {
        Self {
            state: seed ^ 0x9E37_79B9_7F4A_7C15,
        }
    }

    fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    fn uniform(&mut self) -> f64 {
        ((self.next_u64() >> 11) as f64 + 0.5) / (1u64 << 53) as f64
    }

    pub fn normals(&mut self, n: usize) -> Vec<f32> {
        let mut out = Vec::with_capacity(n + 1);
        while out.len() < n {
            let (u1, u2) = (self.uniform(), self.uniform());
            let r = (-2.0 * u1.ln()).sqrt();
            let th = 2.0 * std::f64::consts::PI * u2;
            out.push((r * th.cos()) as f32);
            out.push((r * th.sin()) as f32);
        }
        out.truncate(n);
        out
    }
}

impl NoiseSource for SeededNoise {
    fn next(&mut self, feat_dim: usize, patch: usize, device: &Device) -> Result<Tensor> {
        Ok(Tensor::from_vec(
            self.normals(feat_dim * patch),
            (1, feat_dim, patch),
            device,
        )?)
    }
}

/// Replays recorded noise rows (golden tests).
pub struct ReplayNoise {
    rows: Tensor,
    next: usize,
}

impl ReplayNoise {
    /// `rows`: `[n, 64, P]`.
    pub fn new(rows: Tensor) -> Self {
        Self { rows, next: 0 }
    }
}

impl NoiseSource for ReplayNoise {
    fn next(&mut self, _feat_dim: usize, _patch: usize, device: &Device) -> Result<Tensor> {
        ensure!(self.next < self.rows.dim(0)?, "replay noise exhausted");
        let z = self.rows.i(self.next..self.next + 1)?.to_device(device)?;
        self.next += 1;
        Ok(z)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct GenOptions {
    pub inference_timesteps: usize,
    pub cfg_value: f32,
    /// Earliest stop is after `min_len + 2` patches.
    pub min_len: usize,
    /// Extra cap on patches (in addition to the upstream `6·tokens + 10` cap).
    pub max_patches: Option<usize>,
}

impl Default for GenOptions {
    fn default() -> Self {
        Self {
            inference_timesteps: 10,
            cfg_value: 2.0,
            min_len: 2,
            max_patches: None,
        }
    }
}

/// Voice conditioning: reference patches (timbre) and optionally the same or
/// another clip with its transcript as a continuation prompt.
#[derive(Default)]
pub struct Conditioning<'a> {
    /// `[R, P, D]` reference latents (right-padded encode).
    pub reference: Option<&'a Tensor>,
    /// `[Rp, P, D]` prompt latents (left-padded encode) and the prompt's transcript.
    pub prompt: Option<(&'a Tensor, &'a str)>,
}

pub struct Generation {
    /// `[n·P, D]` latents to decode (includes continuation context, if any).
    pub latents: Tensor,
    /// Generated patches (excluding continuation context).
    pub patches: usize,
    pub stopped: bool,
    /// Leading 48 kHz samples to drop after decoding (continuation context).
    pub trim_samples: usize,
}

pub struct VoxCpm2 {
    pub cfg: ModelConfig,
    device: Device,
    dtype: DType,
    tokenizer: TextTokenizer,
    embed_tokens: Embedding,
    base_lm: MiniCpmStack,
    residual_lm: MiniCpmStack,
    locenc: LocEnc,
    dit: LocDit,
    fsq: Fsq,
    vae: AudioVae,
    enc_to_lm_proj: Linear,
    lm_to_dit_proj: Linear,
    res_to_dit_proj: Linear,
    fusion_concat_proj: Linear,
    stop_proj: Linear,
    stop_head: Linear,
}

fn linear(vb: &VarBuilder, name: &str, inp: usize, out: usize, bias: bool) -> Result<Linear> {
    let vb = vb.pp(name);
    let w = vb.get((out, inp), "weight")?;
    let b = if bias {
        Some(vb.get(out, "bias")?)
    } else {
        None
    };
    Ok(Linear::new(w, b))
}

impl VoxCpm2 {
    /// Load from a checkpoint snapshot directory. `dtype` is the compute
    /// precision of the transformer stacks (F32 or BF16); the VAE is always f32.
    pub fn load(snapshot: &Path, dtype: DType, device: &Device) -> Result<Self> {
        let cfg = ModelConfig::load(&snapshot.join("config.json"))?;
        let tokenizer = TextTokenizer::load(&snapshot.join("tokenizer.json"))?;
        let vb = weights::var_builder(snapshot, dtype, device)?;
        let vb32 = weights::var_builder(snapshot, DType::F32, device)?;
        let lm = &cfg.lm_config;
        let h = lm.hidden_size;
        let stack = |layers: usize, rope: bool| StackConfig {
            num_layers: layers,
            hidden: h,
            ffn: lm.intermediate_size,
            num_heads: lm.num_attention_heads,
            num_kv_heads: lm.num_key_value_heads,
            head_dim: lm.kv_channels,
            rms_norm_eps: lm.rms_norm_eps,
            rope_factors: rope.then(|| lm.rope_scaling.short_factor.clone()),
            rope_theta: lm.rope_theta,
            causal: true,
            dtype,
        };
        let base_lm = MiniCpmStack::load(vb.pp("base_lm"), stack(lm.num_hidden_layers, true))
            .context("loading base_lm")?;
        let residual_lm = MiniCpmStack::load(
            vb.pp("residual_lm"),
            stack(cfg.residual_lm_num_layers, !cfg.residual_lm_no_rope),
        )
        .context("loading residual_lm")?;
        let embed_tokens = Embedding::new(
            vb.pp("base_lm.embed_tokens")
                .get((lm.vocab_size, h), "weight")?,
            h,
        );
        let locenc =
            LocEnc::load(vb.pp("feat_encoder"), &cfg, dtype).context("loading feat_encoder")?;
        let dit =
            LocDit::load(vb.pp("feat_decoder.estimator"), &cfg, dtype).context("loading LocDiT")?;
        let fsq = Fsq::load(
            vb.pp("fsq_layer"),
            cfg.scalar_quantization_latent_dim,
            h,
            cfg.scalar_quantization_scale,
            dtype,
        )?;
        let vae = AudioVae::load(vb32.pp("audio_vae"), &cfg.audio_vae_config)
            .context("loading audio_vae")?;
        let enc_h = cfg.encoder_config.hidden_dim;
        let dit_h = cfg.dit_config.hidden_dim;
        Ok(Self {
            enc_to_lm_proj: linear(&vb, "enc_to_lm_proj", enc_h, h, true)?,
            lm_to_dit_proj: linear(&vb, "lm_to_dit_proj", h, dit_h, true)?,
            res_to_dit_proj: linear(&vb, "res_to_dit_proj", h, dit_h, true)?,
            fusion_concat_proj: linear(&vb, "fusion_concat_proj", 2 * h, h, true)?,
            stop_proj: linear(&vb, "stop_proj", h, h, true)?,
            stop_head: linear(&vb, "stop_head", h, 2, false)?,
            cfg,
            device: device.clone(),
            dtype,
            tokenizer,
            embed_tokens,
            base_lm,
            residual_lm,
            locenc,
            dit,
            fsq,
            vae,
        })
    }

    pub fn device(&self) -> &Device {
        &self.device
    }

    pub fn tokenizer(&self) -> &TextTokenizer {
        &self.tokenizer
    }

    /// Apply a projection in the compute dtype; returns f32.
    fn proj(&self, layer: &Linear, x: &Tensor) -> Result<Tensor> {
        Ok(layer
            .forward(&x.to_dtype(self.dtype)?.contiguous()?)?
            .to_dtype(DType::F32)?)
    }

    fn patch_len(&self) -> usize {
        self.cfg.patch_size * ENC_HOP
    }

    /// Encode 16 kHz mono audio into `[R, P, D]` patches. `left_pad` selects
    /// prompt-style padding (the last patch ends on real audio).
    pub fn encode_audio(&self, wav16k: &[f32], left_pad: bool) -> Result<Tensor> {
        ensure!(!wav16k.is_empty(), "empty reference audio");
        let pl = self.patch_len();
        let pad = (pl - wav16k.len() % pl) % pl;
        let mut padded = Vec::with_capacity(wav16k.len() + pad);
        if left_pad {
            padded.extend(std::iter::repeat_n(0.0, pad));
            padded.extend_from_slice(wav16k);
        } else {
            padded.extend_from_slice(wav16k);
            padded.extend(std::iter::repeat_n(0.0, pad));
        }
        let n = padded.len();
        let wav = Tensor::from_vec(padded, n, &self.device)?;
        let latents = self.vae.encode(&wav)?; // [n/640, D]
        let (p, d) = (self.cfg.patch_size, self.cfg.feat_dim);
        Ok(latents.reshape((n / pl, p, d))?)
    }

    /// LocEnc + `enc_to_lm_proj` for `[N, P, D]` patches → `[N, H]`.
    fn embed_patches(&self, patches: &Tensor) -> Result<Tensor> {
        let enc = self.locenc.forward(patches)?;
        self.proj(&self.enc_to_lm_proj, &enc)
    }

    fn stop_logits(&self, lm_hidden: &Tensor) -> Result<Tensor> {
        let x = candle_nn::ops::silu(&self.proj(&self.stop_proj, lm_hidden)?)?;
        self.proj(&self.stop_head, &x)
    }

    /// Upstream cap on generated patches for `target_tokens` text tokens.
    pub fn max_len_cap(target_tokens: usize) -> usize {
        (target_tokens as f64 * 6.0 + 10.0) as usize
    }

    /// Generate latents for `text`. `on_step(patches_so_far)` may break to cancel.
    pub fn generate(
        &self,
        text: &str,
        cond: &Conditioning,
        opts: &GenOptions,
        noise: &mut dyn NoiseSource,
        on_step: &mut dyn FnMut(usize) -> ControlFlow<()>,
    ) -> Result<Generation> {
        let (p, d) = (self.cfg.patch_size, self.cfg.feat_dim);
        let target = TextTokenizer::prenormalize(text);
        if target.trim().is_empty() {
            bail!("empty text");
        }
        let target_ids = self.tokenizer.encode(&target)?;
        let full_text = match cond.prompt {
            Some((_, prompt_text)) => {
                let sep = if prompt_text
                    .chars()
                    .last()
                    .is_some_and(|c| c.is_ascii() && !c.is_whitespace())
                    && text.chars().next().is_some_and(|c| !c.is_whitespace())
                {
                    " "
                } else {
                    ""
                };
                TextTokenizer::prenormalize(&format!("{prompt_text}{sep}{text}"))
            }
            None => target,
        };
        let ids = self.tokenizer.encode(&full_text)?;

        // Sequence layout (§5).
        let mut tokens: Vec<u32> = Vec::new();
        let mut feats: Vec<Tensor> = Vec::new(); // one [P, D] per position
        let mut audio_mask: Vec<f32> = Vec::new();
        let zero_patch = Tensor::zeros((p, d), DType::F32, &self.device)?;
        if let Some(reference) = cond.reference {
            let r = reference.dim(0)?;
            tokens.push(REF_AUDIO_START);
            feats.push(zero_patch.clone());
            audio_mask.push(0.0);
            for i in 0..r {
                tokens.push(0);
                feats.push(reference.i(i)?.to_dtype(DType::F32)?);
                audio_mask.push(1.0);
            }
            tokens.push(REF_AUDIO_END);
            feats.push(zero_patch.clone());
            audio_mask.push(0.0);
        }
        for &id in ids.iter().chain(std::iter::once(&AUDIO_START)) {
            tokens.push(id);
            feats.push(zero_patch.clone());
            audio_mask.push(0.0);
        }
        let mut prompt_tail: Vec<Tensor> = Vec::new();
        if let Some((prompt, _)) = cond.prompt {
            let rp = prompt.dim(0)?;
            for i in 0..rp {
                tokens.push(0);
                feats.push(prompt.i(i)?.to_dtype(DType::F32)?);
                audio_mask.push(1.0);
            }
            for i in rp.saturating_sub(CONTINUATION_CONTEXT)..rp {
                prompt_tail.push(prompt.i(i)?.to_dtype(DType::F32)?);
            }
        }
        let s = tokens.len();
        let text_mask: Vec<f32> = audio_mask.iter().map(|m| 1.0 - m).collect();
        let tok = Tensor::from_vec(tokens, s, &self.device)?;
        let feat = Tensor::stack(&feats, 0)?; // [S, P, D]
        let am = Tensor::from_vec(audio_mask.clone(), (s, 1), &self.device)?;
        let tm = Tensor::from_vec(text_mask, (s, 1), &self.device)?;

        // Prefill (§6.2, §6.4).
        let text_embed = self.embed_tokens.forward(&tok)?.to_dtype(DType::F32)?; // [S, H]
        let feat_embed = self.embed_patches(&feat)?; // [S, H]
        let combined = (text_embed.broadcast_mul(&tm)? + feat_embed.broadcast_mul(&am)?)?;
        let mut lm_cache = KvCache::new();
        let mut res_cache = KvCache::new();
        let h = self
            .base_lm
            .forward(&combined.unsqueeze(0)?, 0, Some(&mut lm_cache))?
            .squeeze(0)?; // [S, H]
        let enc_out = (self.fsq.forward(&h)?.broadcast_mul(&am)? + h.broadcast_mul(&tm)?)?;
        let mut lm_hidden = enc_out.i(s - 1..s)?; // [1, H]
        let res_in = self.proj(
            &self.fusion_concat_proj,
            &Tensor::cat(&[&enc_out, &feat_embed.broadcast_mul(&am)?], 1)?,
        )?;
        let res = self
            .residual_lm
            .forward(&res_in.unsqueeze(0)?, 0, Some(&mut res_cache))?
            .squeeze(0)?;
        let mut residual_hidden = res.i(s - 1..s)?; // [1, H]

        // Generation loop (§7).
        let mut prefix_cond = feat.i(s - 1)?.t()?.unsqueeze(0)?.contiguous()?; // [1, D, P]
        let mut pred_seq: Vec<Tensor> = prompt_tail.clone();
        let mut cap = Self::max_len_cap(target_ids.len());
        if let Some(m) = opts.max_patches {
            cap = cap.min(m.max(opts.min_len + 2));
        }
        let mut stopped = false;
        let mut generated = 0;
        for i in 0..cap {
            let mu = Tensor::cat(
                &[
                    self.proj(&self.lm_to_dit_proj, &lm_hidden)?,
                    self.proj(&self.res_to_dit_proj, &residual_hidden)?,
                ],
                1,
            )?; // [1, 2·Hd]
            let z = noise.next(d, p, &self.device)?;
            let pred = cfm::sample(
                &self.dit,
                &mu,
                &prefix_cond,
                &z,
                opts.inference_timesteps,
                opts.cfg_value,
            )?; // [1, P, D]
            let curr = self.embed_patches(&pred)?; // [1, H]
            pred_seq.push(pred.squeeze(0)?);
            generated += 1;
            let stop = self
                .stop_logits(&lm_hidden)?
                .argmax(D::Minus1)?
                .to_vec1::<u32>()?[0];
            if on_step(generated).is_break() {
                return Err(crate::engine::Cancelled.into());
            }
            if i > opts.min_len && stop == 1 {
                stopped = true;
                break;
            }
            let hstep = self
                .base_lm
                .forward(&curr.unsqueeze(0)?, s + i, Some(&mut lm_cache))?
                .squeeze(0)?;
            lm_hidden = self.fsq.forward(&hstep)?;
            let rin = self.proj(
                &self.fusion_concat_proj,
                &Tensor::cat(&[&lm_hidden, &curr], 1)?,
            )?;
            residual_hidden = self
                .residual_lm
                .forward(&rin.unsqueeze(0)?, s + i, Some(&mut res_cache))?
                .squeeze(0)?;
            prefix_cond = pred.transpose(1, 2)?.contiguous()?;
        }
        let latents = Tensor::cat(&pred_seq, 0)?; // [n·P, D]
        Ok(Generation {
            latents,
            patches: generated,
            stopped,
            trim_samples: prompt_tail.len() * SAMPLES_PER_PATCH_48K,
        })
    }

    /// Decode a generation to 48 kHz samples.
    pub fn decode(&self, generation: &Generation) -> Result<Vec<f32>> {
        let wav = self
            .vae
            .decode(&generation.latents)?
            .to_dtype(DType::F32)?
            .to_vec1::<f32>()?;
        Ok(wav
            .get(generation.trim_samples..)
            .unwrap_or_default()
            .to_vec())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::native::golden;

    #[test]
    fn seeded_noise_is_deterministic_and_standard_normal() {
        let a = SeededNoise::new(7).normals(20_000);
        let b = SeededNoise::new(7).normals(20_000);
        assert_eq!(a, b);
        assert_ne!(a, SeededNoise::new(8).normals(20_000));
        let mean = a.iter().map(|&x| x as f64).sum::<f64>() / a.len() as f64;
        let var = a.iter().map(|&x| (x as f64 - mean).powi(2)).sum::<f64>() / a.len() as f64;
        assert!(
            mean.abs() < 0.03 && (var - 1.0).abs() < 0.05,
            "mean {mean} var {var}"
        );
    }

    #[test]
    fn length_cap_matches_upstream() {
        assert_eq!(VoxCpm2::max_len_cap(20), 130);
    }

    /// T18: end to end with injected golden noise, reference mode.
    #[test]
    fn t18_end_to_end_with_golden_noise() {
        let Some((_, snapshot)) = golden::require() else {
            return;
        };
        let device = weights::device().unwrap();
        let model = VoxCpm2::load(&snapshot, DType::F32, &device).unwrap();
        let meta = golden::meta().unwrap();
        let inputs = golden::load("00_inputs.safetensors", &device).unwrap();
        let ref_latents = golden::load("02_ref_latents.safetensors", &device).unwrap();
        let steps = golden::load("06_steps.safetensors", &device).unwrap();

        // Reference encode from the padded 16k audio must match the golden latents.
        let wav: Vec<f32> = inputs["ref_audio_16k"].to_vec1().unwrap();
        let reference = model.encode_audio(&wav, false).unwrap();
        let e = golden::rel_err(&reference, &ref_latents["ref_latents"]).unwrap();
        assert!(e < 1e-4, "reference latents rel err {e}");

        let mut noise = ReplayNoise::new(steps["noise"].clone());
        let cond = Conditioning {
            reference: Some(&ref_latents["ref_latents"]),
            prompt: None,
        };
        let gen_ = model
            .generate(
                meta["text"].as_str().unwrap(),
                &cond,
                &GenOptions::default(),
                &mut noise,
                &mut |_| ControlFlow::Continue(()),
            )
            .unwrap();
        let n_ref = meta["n_ar_steps"].as_u64().unwrap() as usize;
        eprintln!(
            "T18: generated {} patches (golden {n_ref}), stopped={}",
            gen_.patches, gen_.stopped
        );
        let golden_pred = steps["pred_feat"].clone(); // [n, P, D]
        for i in 0..5.min(gen_.patches) {
            let ours = gen_.latents.i(i * 4..(i + 1) * 4).unwrap();
            let e = golden::rel_err(&ours, &golden_pred.i(i).unwrap()).unwrap();
            eprintln!("  step {i}: pred_feat rel err {e:.2e}");
            assert!(e < 1e-3, "step {i} rel err {e}");
        }
        assert!(gen_.stopped, "stop head must end the golden utterance");
        assert!(
            (gen_.patches as i64 - n_ref as i64).abs() <= 3,
            "patch count {} vs {n_ref}",
            gen_.patches
        );
        let wav = model.decode(&gen_).unwrap();
        assert_eq!(wav.len(), gen_.patches * SAMPLES_PER_PATCH_48K);
    }
}
