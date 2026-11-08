//! Inference scalar quantization bottleneck.
use super::minicpm::linear;
use anyhow::{Result, ensure};
use candle_core::{DType, Tensor};
use candle_nn::{Linear, Module, VarBuilder};

pub struct Fsq {
    input: Linear,
    output: Linear,
    scale: f64,
    dtype: DType,
}
impl Fsq {
    pub fn load(
        vb: VarBuilder,
        latent_dim: usize,
        hidden: usize,
        scale: f32,
        dtype: DType,
    ) -> Result<Self> {
        ensure!(
            scale.is_finite() && scale > 0.0,
            "FSQ scale must be positive"
        );
        Ok(Self {
            input: linear(vb.pp("in_proj"), hidden, latent_dim, true)?,
            output: linear(vb.pp("out_proj"), latent_dim, hidden, true)?,
            scale: scale as f64,
            dtype,
        })
    }
    pub fn forward(&self, x: &Tensor) -> Result<Tensor> {
        let y = self.input.forward(&x.to_dtype(self.dtype)?)?.tanh()?;
        let quant = (round_ties_even(&(y * self.scale)?)? / self.scale)?;
        Ok(self.output.forward(&quant)?.to_dtype(DType::F32)?)
    }
}

/// PyTorch/MLX use ties-to-even; Metal's native `round` uses ties-away.
fn round_ties_even(x: &Tensor) -> Result<Tensor> {
    let floor = x.floor()?;
    let half = (x - &floor)?.eq(0.5)?;
    let parity = (&floor - ((&floor * 0.5)?.floor()? * 2.0)?)?;
    let even = (&floor + parity)?;
    Ok(half.where_cond(&even, &x.round()?)?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use candle_core::Device;
    #[test]
    fn quantization_ties_use_even_levels() {
        let x = Tensor::new(
            &[-2.5f32, -1.5, -0.5, 0.5, 1.5, 2.5, 1.49, 1.51],
            &Device::Cpu,
        )
        .unwrap();
        assert_eq!(
            round_ties_even(&x).unwrap().to_vec1::<f32>().unwrap(),
            vec![-2.0, -2.0, 0.0, 0.0, 2.0, 2.0, 1.0, 2.0]
        );
    }
}
