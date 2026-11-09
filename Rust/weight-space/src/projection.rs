use anyhow::{Result, ensure};
use std::collections::HashSet;

/// Output-major W[out,in]; only selected input columns are projected.
pub fn project(
    w: &[f32],
    rows: usize,
    cols: usize,
    direction: &[f32],
    selected: &[usize],
    alpha: f32,
) -> Result<Vec<f32>> {
    ensure!(
        rows > 0 && cols > 0 && rows.checked_mul(cols) == Some(w.len()),
        "invalid matrix shape"
    );
    ensure!(
        direction.len() == rows && !selected.is_empty(),
        "invalid direction/selection"
    );
    ensure!(
        alpha.is_finite() && w.iter().chain(direction).all(|v| v.is_finite()),
        "nonfinite projection input"
    );
    let norm = direction
        .iter()
        .map(|x| f64::from(*x).powi(2))
        .sum::<f64>()
        .sqrt();
    ensure!(norm.is_finite() && norm > 0., "zero or invalid direction");
    let mut seen = HashSet::new();
    ensure!(
        selected.iter().all(|c| *c < cols && seen.insert(*c)),
        "duplicate or invalid column"
    );
    let u: Vec<f64> = direction.iter().map(|x| f64::from(*x) / norm).collect();
    let mut out = w.to_vec();
    for &c in selected {
        let dot: f64 = (0..rows).map(|r| u[r] * f64::from(w[r * cols + c])).sum();
        for r in 0..rows {
            out[r * cols + c] = (f64::from(w[r * cols + c]) - f64::from(alpha) * u[r] * dot) as f32;
        }
    }
    ensure!(out.iter().all(|x| x.is_finite()), "projection overflow");
    Ok(out)
}
