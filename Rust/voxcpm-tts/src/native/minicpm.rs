//! Shared MiniCPM transformer, LongRoPE, GQA attention and per-layer KV cache.
use anyhow::{Result, ensure};
use candle_core::{DType, Device, Tensor};
use candle_nn::{Linear, Module, VarBuilder};

#[derive(Debug, Clone)]
pub struct StackConfig {
    pub num_layers: usize,
    pub hidden: usize,
    pub ffn: usize,
    pub num_heads: usize,
    pub num_kv_heads: usize,
    pub head_dim: usize,
    pub rms_norm_eps: f64,
    pub rope_factors: Option<Vec<f32>>,
    pub rope_theta: f32,
    pub causal: bool,
    pub dtype: DType,
}

#[derive(Debug, Default)]
pub struct KvCache {
    layers: Vec<Option<(Tensor, Tensor)>>,
}
impl KvCache {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn len(&self) -> usize {
        self.layers
            .first()
            .and_then(Option::as_ref)
            .map_or(0, |(k, _)| k.dims()[2])
    }
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

pub(super) fn linear(vb: VarBuilder, inp: usize, out: usize, bias: bool) -> Result<Linear> {
    let weight = vb.get((out, inp), "weight")?;
    let bias = if bias {
        Some(vb.get(out, "bias")?)
    } else {
        None
    };
    Ok(Linear::new(weight, bias))
}

struct Layer {
    q: Linear,
    k: Linear,
    v: Linear,
    o: Linear,
    gate: Linear,
    up: Linear,
    down: Linear,
    input_norm: Tensor,
    post_norm: Tensor,
}

pub struct MiniCpmStack {
    cfg: StackConfig,
    layers: Vec<Layer>,
    norm: Tensor,
}

/// Half-width LongRoPE tables for the non-interleaved candle rotary kernel.
pub fn rope_tables(
    cfg: &StackConfig,
    pos: usize,
    len: usize,
    device: &Device,
) -> Result<(Tensor, Tensor)> {
    let factors = cfg
        .rope_factors
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("RoPE disabled"))?;
    let half = cfg.head_dim / 2;
    ensure!(
        factors.len() == half && half > 0,
        "invalid LongRoPE factors"
    );
    let mut cos = Vec::with_capacity(len * half);
    let mut sin = Vec::with_capacity(len * half);
    for p in pos..pos + len {
        for (j, factor) in factors.iter().enumerate() {
            let inv = 1.0 / cfg.rope_theta.powf(j as f32 / half as f32);
            let angle = (p as f32 * (1.0 / factor)) * inv;
            cos.push(angle.cos());
            sin.push(angle.sin());
        }
    }
    Ok((
        Tensor::from_vec(cos, (len, half), device)?,
        Tensor::from_vec(sin, (len, half), device)?,
    ))
}

fn norm(x: &Tensor, w: &Tensor, eps: f64, dtype: DType) -> Result<Tensor> {
    Ok(
        candle_nn::ops::rms_norm(&x.to_dtype(DType::F32)?.contiguous()?, w, eps as f32)?
            .to_dtype(dtype)?,
    )
}

impl MiniCpmStack {
    pub fn load(vb: VarBuilder, cfg: StackConfig) -> Result<Self> {
        ensure!(
            cfg.num_heads > 0
                && cfg.num_kv_heads > 0
                && cfg.num_heads.is_multiple_of(cfg.num_kv_heads),
            "invalid GQA head counts"
        );
        ensure!(
            cfg.head_dim.is_multiple_of(2),
            "RoPE requires even head dimension"
        );
        let mut layers = Vec::with_capacity(cfg.num_layers);
        for i in 0..cfg.num_layers {
            let lv = vb.pp(format!("layers.{i}"));
            let attn = lv.pp("self_attn");
            let mlp = lv.pp("mlp");
            layers.push(Layer {
                q: linear(
                    attn.pp("q_proj"),
                    cfg.hidden,
                    cfg.num_heads * cfg.head_dim,
                    false,
                )?,
                k: linear(
                    attn.pp("k_proj"),
                    cfg.hidden,
                    cfg.num_kv_heads * cfg.head_dim,
                    false,
                )?,
                v: linear(
                    attn.pp("v_proj"),
                    cfg.hidden,
                    cfg.num_kv_heads * cfg.head_dim,
                    false,
                )?,
                o: linear(
                    attn.pp("o_proj"),
                    cfg.num_heads * cfg.head_dim,
                    cfg.hidden,
                    false,
                )?,
                gate: linear(mlp.pp("gate_proj"), cfg.hidden, cfg.ffn, false)?,
                up: linear(mlp.pp("up_proj"), cfg.hidden, cfg.ffn, false)?,
                down: linear(mlp.pp("down_proj"), cfg.ffn, cfg.hidden, false)?,
                input_norm: lv
                    .pp("input_layernorm")
                    .get(cfg.hidden, "weight")?
                    .to_dtype(DType::F32)?,
                post_norm: lv
                    .pp("post_attention_layernorm")
                    .get(cfg.hidden, "weight")?
                    .to_dtype(DType::F32)?,
            });
        }
        let norm = vb
            .pp("norm")
            .get(cfg.hidden, "weight")?
            .to_dtype(DType::F32)?;
        Ok(Self { cfg, layers, norm })
    }

    /// Cached keys have layout `[B, kv_heads, positions, head_dim]` after RoPE.
    pub fn forward(
        &self,
        x: &Tensor,
        pos: usize,
        mut cache: Option<&mut KvCache>,
    ) -> Result<Tensor> {
        let (b, len, h) = x.dims3()?;
        ensure!(h == self.cfg.hidden && len > 0, "invalid transformer input");
        if let Some(c) = cache.as_deref_mut() {
            ensure!(
                c.len() == pos,
                "cache length {} differs from position {pos}",
                c.len()
            );
            if c.layers.is_empty() {
                c.layers.resize_with(self.layers.len(), || None);
            }
            ensure!(c.layers.len() == self.layers.len(), "cache stack mismatch");
        }
        let tables = self
            .cfg
            .rope_factors
            .as_ref()
            .map(|_| rope_tables(&self.cfg, pos, len, x.device()))
            .transpose()?;
        let mut x = x.to_dtype(self.cfg.dtype)?;
        for (index, layer) in self.layers.iter().enumerate() {
            let a = norm(&x, &layer.input_norm, self.cfg.rms_norm_eps, self.cfg.dtype)?;
            let q = layer
                .q
                .forward(&a)?
                .reshape((b, len, self.cfg.num_heads, self.cfg.head_dim))?
                .transpose(1, 2)?
                .contiguous()?;
            let k = layer
                .k
                .forward(&a)?
                .reshape((b, len, self.cfg.num_kv_heads, self.cfg.head_dim))?
                .transpose(1, 2)?
                .contiguous()?;
            let v = layer
                .v
                .forward(&a)?
                .reshape((b, len, self.cfg.num_kv_heads, self.cfg.head_dim))?
                .transpose(1, 2)?
                .contiguous()?;
            let (q, k) = if let Some((cos, sin)) = &tables {
                (
                    candle_nn::rotary_emb::rope(&q.to_dtype(DType::F32)?, cos, sin)?
                        .to_dtype(self.cfg.dtype)?,
                    candle_nn::rotary_emb::rope(&k.to_dtype(DType::F32)?, cos, sin)?
                        .to_dtype(self.cfg.dtype)?,
                )
            } else {
                (q, k)
            };
            let (k, v) = if let Some(c) = cache.as_deref_mut() {
                let (k, v) = match &c.layers[index] {
                    Some((old_k, old_v)) => {
                        (Tensor::cat(&[old_k, &k], 2)?, Tensor::cat(&[old_v, &v], 2)?)
                    }
                    None => (k, v),
                };
                c.layers[index] = Some((k.clone(), v.clone()));
                (k, v)
            } else {
                (k, v)
            };
            let total = k.dim(2)?;
            let offset = total - len;
            let mask = if self.cfg.causal && len > 1 {
                let vals: Vec<f32> = (0..len)
                    .flat_map(|i| {
                        (0..total).map(move |j| {
                            if j > offset + i {
                                f32::NEG_INFINITY
                            } else {
                                0.0
                            }
                        })
                    })
                    .collect();
                Some(
                    Tensor::from_vec(vals, (1, 1, len, total), x.device())?
                        .to_dtype(self.cfg.dtype)?,
                )
            } else {
                None
            };
            let scale = 1.0 / (self.cfg.head_dim as f32).sqrt();
            let out = if x.device().is_metal() {
                // Candle's Metal SDPA validates the full mask shape; a
                // broadcasted layout is supported but implicit broadcast is not.
                let metal_mask = mask
                    .as_ref()
                    .map(|m| m.broadcast_as((b, self.cfg.num_heads, len, total)))
                    .transpose()?;
                candle_nn::ops::sdpa(
                    &q,
                    &k.contiguous()?,
                    &v.contiguous()?,
                    metal_mask.as_ref(),
                    false,
                    scale,
                    1.0,
                )?
            } else {
                let repeat = self.cfg.num_heads / self.cfg.num_kv_heads;
                let expand = |t: &Tensor| -> candle_core::Result<Tensor> {
                    t.unsqueeze(2)?
                        .broadcast_as((b, self.cfg.num_kv_heads, repeat, total, self.cfg.head_dim))?
                        .reshape((b, self.cfg.num_heads, total, self.cfg.head_dim))
                };
                let logits = (q
                    .to_dtype(DType::F32)?
                    .matmul(&expand(&k)?.to_dtype(DType::F32)?.transpose(2, 3)?)?
                    * scale as f64)?;
                let logits = match &mask {
                    Some(m) => logits.broadcast_add(&m.to_dtype(DType::F32)?)?,
                    None => logits,
                };
                candle_nn::ops::softmax_last_dim(&logits)?
                    .matmul(&expand(&v)?.to_dtype(DType::F32)?)?
                    .to_dtype(self.cfg.dtype)?
            };
            let out =
                out.transpose(1, 2)?
                    .reshape((b, len, self.cfg.num_heads * self.cfg.head_dim))?;
            x = (&x + layer.o.forward(&out)?)?;
            let a = norm(&x, &layer.post_norm, self.cfg.rms_norm_eps, self.cfg.dtype)?;
            let ff = (candle_nn::ops::silu(&layer.gate.forward(&a)?)? * layer.up.forward(&a)?)?;
            x = (&x + layer.down.forward(&ff)?)?;
        }
        norm(&x, &self.norm, self.cfg.rms_norm_eps, DType::F32)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::native::{
        cfm, config::ModelConfig, fsq::Fsq, golden, locdit::LocDit, locenc::LocEnc, weights,
    };
    use candle_core::IndexOp;

    fn check(label: &str, a: &Tensor, b: &Tensor, tol: f32) {
        let e = golden::rel_err(a, b).unwrap();
        eprintln!("{label}: rel error {e:.3e}");
        assert!(e < tol, "{label}: {e:.3e} >= {tol:.3e}");
    }
    fn config(cfg: &ModelConfig, residual: bool) -> StackConfig {
        let lm = &cfg.lm_config;
        StackConfig {
            num_layers: if residual {
                cfg.residual_lm_num_layers
            } else {
                lm.num_hidden_layers
            },
            hidden: lm.hidden_size,
            ffn: lm.intermediate_size,
            num_heads: lm.num_attention_heads,
            num_kv_heads: lm.num_key_value_heads,
            head_dim: lm.kv_channels,
            rms_norm_eps: lm.rms_norm_eps,
            rope_factors: (!residual).then(|| lm.rope_scaling.short_factor.clone()),
            rope_theta: lm.rope_theta,
            causal: true,
            dtype: DType::F32,
        }
    }

    #[test]
    fn invalid_config_and_cache_are_rejected() {
        let mut c = StackConfig {
            num_layers: 0,
            hidden: 4,
            ffn: 8,
            num_heads: 0,
            num_kv_heads: 0,
            head_dim: 2,
            rms_norm_eps: 1e-5,
            rope_factors: None,
            rope_theta: 10000.0,
            causal: true,
            dtype: DType::F32,
        };
        let vb = VarBuilder::zeros(DType::F32, &Device::Cpu);
        assert!(MiniCpmStack::load(vb.clone(), c.clone()).is_err());
        c.num_heads = 2;
        c.num_kv_heads = 1;
        let stack = MiniCpmStack::load(vb, c).unwrap();
        let x = Tensor::zeros((1, 1, 4), DType::F32, &Device::Cpu).unwrap();
        assert!(stack.forward(&x, 1, Some(&mut KvCache::new())).is_err());
    }

    fn small_stack(causal: bool) -> MiniCpmStack {
        let mut tensors = std::collections::HashMap::new();
        for (i, (name, rows, cols)) in [
            ("self_attn.q_proj", 4, 4),
            ("self_attn.k_proj", 2, 4),
            ("self_attn.v_proj", 2, 4),
            ("self_attn.o_proj", 4, 4),
            ("mlp.gate_proj", 8, 4),
            ("mlp.up_proj", 8, 4),
            ("mlp.down_proj", 4, 8),
        ]
        .iter()
        .enumerate()
        {
            let values: Vec<f32> = (0..rows * cols)
                .map(|j| ((j + i * 3) as f32 * 0.7).sin() * 0.2)
                .collect();
            tensors.insert(
                format!("layers.0.{name}.weight"),
                Tensor::from_vec(values, (*rows, *cols), &Device::Cpu).unwrap(),
            );
        }
        for name in [
            "norm.weight",
            "layers.0.input_layernorm.weight",
            "layers.0.post_attention_layernorm.weight",
        ] {
            tensors.insert(
                name.to_string(),
                Tensor::ones(4, DType::F32, &Device::Cpu).unwrap(),
            );
        }
        let vb = VarBuilder::from_tensors(tensors, DType::F32, &Device::Cpu);
        MiniCpmStack::load(
            vb,
            StackConfig {
                num_layers: 1,
                hidden: 4,
                ffn: 8,
                num_heads: 2,
                num_kv_heads: 1,
                head_dim: 2,
                rms_norm_eps: 1e-5,
                rope_factors: Some(vec![1.0]),
                rope_theta: 10000.0,
                causal,
                dtype: DType::F32,
            },
        )
        .unwrap()
    }

    #[test]
    fn causal_cache_matches_full_sequence_and_cannot_see_future() {
        let x = Tensor::from_vec(
            vec![
                0.2f32, 0.4, 0.1, -0.3, 0.5, 0.1, -0.7, 0.2, 0.9, -0.8, 0.5, 0.3,
            ],
            (1, 3, 4),
            &Device::Cpu,
        )
        .unwrap();
        let changed = Tensor::cat(
            &[
                x.narrow(1, 0, 2).unwrap(),
                Tensor::from_vec(vec![-0.1f32, 1.5, 2.0, -2.5], (1, 1, 4), &Device::Cpu).unwrap(),
            ],
            1,
        )
        .unwrap();
        let stack = small_stack(true);
        let full = stack.forward(&x, 0, None).unwrap();
        let future = stack.forward(&changed, 0, None).unwrap();
        check(
            "causal future isolation",
            &full.narrow(1, 0, 2).unwrap(),
            &future.narrow(1, 0, 2).unwrap(),
            1e-7,
        );
        let mut cache = KvCache::new();
        let prefix = stack
            .forward(&x.narrow(1, 0, 1).unwrap(), 0, Some(&mut cache))
            .unwrap();
        let suffix = stack
            .forward(&x.narrow(1, 1, 2).unwrap(), 1, Some(&mut cache))
            .unwrap();
        assert_eq!(cache.len(), 3);
        check(
            "cached multi-token suffix",
            &Tensor::cat(&[prefix, suffix], 1).unwrap(),
            &full,
            1e-6,
        );
        let bidir = small_stack(false);
        let a = bidir.forward(&x, 0, None).unwrap().narrow(1, 0, 1).unwrap();
        let b = bidir
            .forward(&changed, 0, None)
            .unwrap()
            .narrow(1, 0, 1)
            .unwrap();
        assert!(
            golden::max_abs_err(&a, &b).unwrap() > 1e-4,
            "bidirectional attention must see later tokens"
        );
    }

    /// T2/T6–T12/T14. Run serially: this test holds one stack at a time.
    #[test]
    fn t2_t14_transformer_golden() {
        let Some((_, snapshot)) = golden::require() else {
            return;
        };
        let dev = weights::device().unwrap();
        let cfg = ModelConfig::load(&snapshot.join("config.json")).unwrap();
        let vb = weights::var_builder(&snapshot, DType::F32, &dev).unwrap();
        let base = golden::load("04_base_lm_prefill.safetensors", &dev).unwrap();
        let residual = golden::load("05_residual_lm_prefill.safetensors", &dev).unwrap();
        let steps = golden::load("06_steps.safetensors", &dev).unwrap();
        let c = config(&cfg, false);
        let (cos, sin) = rope_tables(&c, 0, 57, &dev).unwrap();
        // MLX Metal's float pow approximation differs from scalar powf by a
        // few ulps; angles magnify that to ~4e-6 at position 56.
        for (name, table) in [("rope_cos", cos), ("rope_sin", sin)] {
            let error =
                golden::max_abs_err(&Tensor::cat(&[&table, &table], 1).unwrap(), &base[name])
                    .unwrap();
            eprintln!("T2 {name}: max absolute error {error:.3e}");
            assert!(error < 5e-6, "{name}: {error}");
        }
        {
            let loc = LocEnc::load(vb.pp("feat_encoder"), &cfg, DType::F32).unwrap();
            let g = golden::load("03_locenc.safetensors", &dev).unwrap();
            let out = loc.forward(&g["input"]).unwrap();
            check("T6 LocEnc", &out, &g["locenc_out"], 1e-5);
            let proj = linear(
                vb.pp("enc_to_lm_proj"),
                cfg.encoder_config.hidden_dim,
                cfg.lm_config.hidden_size,
                true,
            )
            .unwrap();
            assert!(
                out.is_contiguous(),
                "LocEnc CLS output must be dense for Metal projections"
            );
            check(
                "T6 projected prefill LocEnc",
                &proj.forward(&out).unwrap(),
                &g["enc_to_lm_out"],
                1e-5,
            );
            let generated = loc.forward(&steps["pred_feat"].i(0..1).unwrap()).unwrap();
            check(
                "T12 generated LocEnc",
                &proj.forward(&generated).unwrap(),
                &steps["curr_embed"].i(0..1).unwrap(),
                1e-5,
            );
        }
        {
            let fsq = Fsq::load(
                vb.pp("fsq_layer"),
                cfg.scalar_quantization_latent_dim,
                cfg.lm_config.hidden_size,
                cfg.scalar_quantization_scale,
                DType::F32,
            )
            .unwrap();
            check(
                "T8 FSQ",
                &fsq.forward(&base["final_norm_out"]).unwrap(),
                &base["fsq_out_all_positions"],
                1e-4,
            );
        }
        {
            let lm = MiniCpmStack::load(vb.pp("base_lm"), c).unwrap();
            let mut cache = KvCache::new();
            let out = lm
                .forward(
                    &base["combined_embed"].unsqueeze(0).unwrap(),
                    0,
                    Some(&mut cache),
                )
                .unwrap()
                .squeeze(0)
                .unwrap();
            check("T7 TSLM prefill", &out, &base["final_norm_out"], 1e-4);
            assert_eq!(cache.len(), 57);
            let (k, v) = cache.layers[0].as_ref().unwrap();
            check(
                "T7 K cache",
                &k.squeeze(0).unwrap().transpose(0, 1).unwrap(),
                &base["k_cache_layer0"],
                1e-4,
            );
            check(
                "T7 V cache",
                &v.squeeze(0).unwrap().transpose(0, 1).unwrap(),
                &base["v_cache_layer0"],
                1e-4,
            );
            for i in 0..29 {
                let curr = steps["curr_embed"]
                    .i(i..i + 1)
                    .unwrap()
                    .unsqueeze(0)
                    .unwrap();
                let out = lm
                    .forward(&curr, 57 + i, Some(&mut cache))
                    .unwrap()
                    .squeeze(0)
                    .unwrap();
                check(
                    &format!("T14 TSLM step {i}"),
                    &out,
                    &steps["lm_step_out_pre_fsq"].i(i..i + 1).unwrap(),
                    1e-4,
                );
            }
        }
        {
            let lm = MiniCpmStack::load(vb.pp("residual_lm"), config(&cfg, true)).unwrap();
            let mut cache = KvCache::new();
            let out = lm
                .forward(
                    &residual["residual_input"].unsqueeze(0).unwrap(),
                    0,
                    Some(&mut cache),
                )
                .unwrap()
                .squeeze(0)
                .unwrap();
            check("T9 RALM prefill", &out, &residual["final_norm_out"], 1e-4);
            let proj = linear(
                vb.pp("fusion_concat_proj"),
                cfg.lm_config.hidden_size * 2,
                cfg.lm_config.hidden_size,
                true,
            )
            .unwrap();
            for i in 0..29 {
                let hidden = steps["lm_hidden"].i(i + 1..i + 2).unwrap();
                let curr = steps["curr_embed"].i(i..i + 1).unwrap();
                let inp = proj
                    .forward(&Tensor::cat(&[hidden, curr], 1).unwrap())
                    .unwrap()
                    .unsqueeze(0)
                    .unwrap();
                let out = lm
                    .forward(&inp, 57 + i, Some(&mut cache))
                    .unwrap()
                    .squeeze(0)
                    .unwrap();
                check(
                    &format!("T14 RALM step {i}"),
                    &out,
                    &steps["residual_hidden"].i(i + 1..i + 2).unwrap(),
                    1e-4,
                );
            }
        }
        {
            let dit = LocDit::load(vb.pp("feat_decoder.estimator"), &cfg, DType::F32).unwrap();
            let g = golden::load("07_cfm_trace.safetensors", &dev).unwrap();
            let mu = steps["mu"].i(0..1).unwrap();
            let cond = steps["cond"].i(0..1).unwrap();
            let x = g["step0_x_per_step"].i(1..2).unwrap();
            let tval = g["step0_t_used"].to_vec1::<f32>().unwrap()[1];
            let t = Tensor::full(tval, (2,), &dev).unwrap();
            let out = dit
                .forward(
                    &Tensor::cat(&[&x, &x], 0).unwrap(),
                    &Tensor::cat(&[&mu, &mu.zeros_like().unwrap()], 0).unwrap(),
                    &t,
                    &Tensor::cat(&[&cond, &cond], 0).unwrap(),
                )
                .unwrap();
            check("T10 DiT", &out, &g["step0_first_eval_velocity"], 1e-5);
            for i in 0..2 {
                let out = cfm::sample(
                    &dit,
                    &steps["mu"].i(i..i + 1).unwrap(),
                    &steps["cond"].i(i..i + 1).unwrap(),
                    &steps["noise"].i(i..i + 1).unwrap(),
                    10,
                    2.0,
                )
                .unwrap();
                check(
                    &format!("T11 CFM patch {i}"),
                    &out,
                    &steps["pred_feat"].i(i..i + 1).unwrap(),
                    1e-4,
                );
            }
        }
    }
}
