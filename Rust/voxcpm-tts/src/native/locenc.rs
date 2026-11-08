//! Patch-local transformer encoder.
use super::{
    config::ModelConfig,
    minicpm::{MiniCpmStack, StackConfig, linear},
};
use anyhow::Result;
use candle_core::{DType, Tensor};
use candle_nn::{Linear, Module, VarBuilder};

pub struct LocEnc {
    proj: Linear,
    cls: Tensor,
    stack: MiniCpmStack,
    dtype: DType,
    hidden: usize,
}
impl LocEnc {
    pub fn load(vb: VarBuilder, cfg: &ModelConfig, dtype: DType) -> Result<Self> {
        let c = &cfg.encoder_config;
        let lm = &cfg.lm_config;
        Ok(Self {
            proj: linear(vb.pp("in_proj"), cfg.feat_dim, c.hidden_dim, true)?,
            cls: vb
                .get((1, 1, 1, c.hidden_dim), "special_token")?
                .reshape((1, 1, c.hidden_dim))?,
            stack: MiniCpmStack::load(
                vb.pp("encoder"),
                StackConfig {
                    num_layers: c.num_layers,
                    hidden: c.hidden_dim,
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
            hidden: c.hidden_dim,
        })
    }
    pub fn forward(&self, patches: &Tensor) -> Result<Tensor> {
        let (n, _, _) = patches.dims3()?;
        let x = self.proj.forward(&patches.to_dtype(self.dtype)?)?;
        let cls = self.cls.broadcast_as((n, 1, self.hidden))?;
        Ok(self
            .stack
            .forward(&Tensor::cat(&[&cls, &x], 1)?, 0, None)?
            .narrow(1, 0, 1)?
            .squeeze(1)?
            .contiguous()?)
    }
}
