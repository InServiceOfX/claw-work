//! Euler flow-matching with sway schedule and CFG-Zero*.
use super::locdit::LocDit;
use anyhow::{Result, ensure};
use candle_core::{DType, Tensor};

pub fn sample(
    dit: &LocDit,
    mu: &Tensor,
    cond: &Tensor,
    noise: &Tensor,
    inference_timesteps: usize,
    cfg_value: f32,
) -> Result<Tensor> {
    ensure!(
        inference_timesteps > 0,
        "inference timesteps must be positive"
    );
    ensure!(cfg_value.is_finite(), "CFG must be finite");
    let n = noise.dim(0)?;
    let grid = time_grid(inference_timesteps);
    let mut x = noise.to_dtype(DType::F32)?;
    let mu = Tensor::cat(&[mu, &mu.zeros_like()?], 0)?;
    let cond = Tensor::cat(&[cond, cond], 0)?;
    let zero_steps = ((inference_timesteps + 1) as f32 * 0.04) as usize;
    let mut t = grid[0];
    let mut dt = t - grid[1];
    for step in 1..=inference_timesteps {
        if step > zero_steps.max(1) {
            let times = Tensor::full(t, (2 * n,), x.device())?;
            let out = dit.forward(&Tensor::cat(&[&x, &x], 0)?, &mu, &times, &cond)?;
            let vp = out.narrow(0, 0, n)?;
            let vn = out.narrow(0, n, n)?;
            let dot = (&vp * &vn)?.flatten_from(1)?.sum_keepdim(1)?;
            let den = (vn.sqr()?.flatten_from(1)?.sum_keepdim(1)? + 1e-8)?;
            let st = (dot / den)?.unsqueeze(2)?;
            let neg = vn.broadcast_mul(&st)?;
            let velocity = (&neg + ((&vp - &neg)? * cfg_value as f64)?)?;
            x = (&x - (velocity * dt as f64)?)?;
        }
        t -= dt;
        if step < inference_timesteps {
            dt = t - grid[step + 1];
        }
    }
    Ok(x.transpose(1, 2)?.contiguous()?)
}

pub fn time_grid(n: usize) -> Vec<f32> {
    if n == 0 {
        return vec![1.0];
    }
    (0..=n)
        .map(|k| {
            let s = 1.0 - k as f32 / n as f32;
            s + ((std::f32::consts::FRAC_PI_2 * s).cos() - 1.0 + s)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    #[allow(clippy::approx_constant)] // Independently rounded upstream schedule.
    fn sway_schedule_matches_reference() {
        let g = time_grid(10);
        let expected = [
            1.0, 0.956434, 0.909017, 0.853990, 0.787785, 0.707107, 0.609017, 0.491007, 0.351057,
            0.187688, 0.0,
        ];
        for (a, b) in g.iter().zip(expected) {
            assert!((a - b).abs() < 1e-6, "{a} vs {b}");
        }
        assert!(g.windows(2).all(|w| w[0] > w[1]));
    }
}
