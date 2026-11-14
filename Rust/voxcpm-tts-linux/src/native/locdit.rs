//! VoxCPM2 local diffusion transformer with two conditioning tokens.
use super::{
    config::ModelConfig,
    minicpm::{MiniCpmStack, StackConfig, linear},
};
use anyhow::{Result, ensure};
use candle_core::{DType, Tensor};
use candle_nn::{Linear, Module, VarBuilder};

struct TimeMlp {
    first: Linear,
    second: Linear,
}
impl TimeMlp {
    fn load(vb: VarBuilder, hidden: usize) -> Result<Self> {
        Ok(Self {
            first: linear(vb.pp("linear_1"), hidden, hidden, true)?,
            second: linear(vb.pp("linear_2"), hidden, hidden, true)?,
        })
    }
    fn forward(&self, x: &Tensor) -> Result<Tensor> {
        Ok(self
            .second
            .forward(&candle_nn::ops::silu(&self.first.forward(x)?)?)?)
    }
}

pub struct LocDit {
    input: Linear,
    cond: Linear,
    output: Linear,
    time: TimeMlp,
    dt: Tensor,
    frequencies: Tensor,
    stack: MiniCpmStack,
    dtype: DType,
    hidden: usize,
}
impl LocDit {
    pub fn load(vb: VarBuilder, cfg: &ModelConfig, dtype: DType) -> Result<Self> {
        let c = &cfg.dit_config;
        let lm = &cfg.lm_config;
        ensure!(!c.mean_mode, "VoxCPM2 port requires mean_mode=false");
        let h = c.hidden_dim;
        let half = h / 2;
        let freq: Vec<f32> = (0..half)
            .map(|i| (-(i as f32) * (10000f32.ln() / (half - 1) as f32)).exp())
            .collect();
        let frequencies = Tensor::from_vec(freq, (1, half), vb.device())?;
        let zeros = Tensor::zeros((1, half), DType::F32, vb.device())?;
        let ones = Tensor::ones((1, half), DType::F32, vb.device())?;
        let dt = TimeMlp::load(vb.pp("delta_time_mlp"), h)?
            .forward(&Tensor::cat(&[zeros, ones], 1)?.to_dtype(dtype)?)?;
        Ok(Self {
            input: linear(vb.pp("in_proj"), cfg.feat_dim, h, true)?,
            cond: linear(vb.pp("cond_proj"), cfg.feat_dim, h, true)?,
            output: linear(vb.pp("out_proj"), h, cfg.feat_dim, true)?,
            time: TimeMlp::load(vb.pp("time_mlp"), h)?,
            dt,
            frequencies,
            stack: MiniCpmStack::load(
                vb.pp("decoder"),
                StackConfig {
                    num_layers: c.num_layers,
                    hidden: h,
                    ffn: c.ffn_dim,
                    num_heads: c.num_heads,
                    num_kv_heads: lm.num_key_value_heads,
                    head_dim: c.kv_channels,
                    rms_norm_eps: lm.rms_norm_eps,
                    rope_factors: Some(lm.rope_scaling.short_factor.clone()),
                    rope_theta: lm.rope_theta,
                    causal: false,
                    dtype,
                },
            )?,
            dtype,
            hidden: h,
        })
    }
    pub fn forward(&self, x: &Tensor, mu: &Tensor, t: &Tensor, cond: &Tensor) -> Result<Tensor> {
        let (n, _, patch) = x.dims3()?;
        let xp = self
            .input
            .forward(&x.transpose(1, 2)?.to_dtype(self.dtype)?)?;
        let cp = self
            .cond
            .forward(&cond.transpose(1, 2)?.to_dtype(self.dtype)?)?;
        let angle =
            (t.to_dtype(DType::F32)?.unsqueeze(1)? * 1000.0)?.broadcast_mul(&self.frequencies)?;
        let sinus = Tensor::cat(&[angle.sin()?, angle.cos()?], 1)?;
        let time = self
            .time
            .forward(&sinus.to_dtype(self.dtype)?)?
            .broadcast_add(&self.dt)?
            .unsqueeze(1)?;
        let mu = mu.reshape((n, 2, self.hidden))?.to_dtype(self.dtype)?;
        let prefix = cp.dim(1)? + 3;
        let seq = Tensor::cat(&[mu, time, cp, xp], 1)?;
        let hidden = self
            .stack
            .forward(&seq, 0, None)?
            .narrow(1, prefix, patch)?
            .to_dtype(self.dtype)?;
        Ok(self
            .output
            .forward(&hidden)?
            .transpose(1, 2)?
            .contiguous()?
            .to_dtype(DType::F32)?)
    }
}
