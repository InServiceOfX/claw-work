//! Time estimates: a persisted per-engine calibration (seconds of compute per
//! character, model load time) refined live as chunks complete.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// Conservative first-run guesses (bf16 on an M-series Mac).
const DEFAULT_SECS_PER_CHAR: f64 = 0.15;
const DEFAULT_LOAD_SECS: f64 = 20.0;
/// Weight of the newest observation in the persisted moving average.
const EMA_ALPHA: f64 = 0.3;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct EngineCalibration {
    pub secs_per_char: f64,
    pub load_secs: f64,
    /// Jobs that contributed to `secs_per_char`.
    pub samples: u32,
    /// Loads that contributed to `load_secs`.
    #[serde(default)]
    pub load_samples: u32,
}

impl Default for EngineCalibration {
    fn default() -> Self {
        Self {
            secs_per_char: DEFAULT_SECS_PER_CHAR,
            load_secs: DEFAULT_LOAD_SECS,
            samples: 0,
            load_samples: 0,
        }
    }
}

/// Calibration store, keyed by engine + generation settings.
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Calibration {
    entries: BTreeMap<String, EngineCalibration>,
    #[serde(skip)]
    path: Option<PathBuf>,
}

impl Calibration {
    pub fn key(engine: &str, inference_timesteps: usize) -> String {
        format!("{engine}/t{inference_timesteps}")
    }

    /// Load from `path`; a missing or unreadable file yields defaults.
    pub fn load(path: &Path) -> Self {
        let mut c: Calibration = std::fs::read_to_string(path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default();
        c.path = Some(path.to_path_buf());
        c
    }

    pub fn get(&self, key: &str) -> EngineCalibration {
        self.entries.get(key).copied().unwrap_or_default()
    }

    fn blend(old: f64, new: f64, samples: u32) -> f64 {
        if samples == 0 {
            new
        } else {
            (1.0 - EMA_ALPHA) * old + EMA_ALPHA * new
        }
    }

    pub fn record_rate(&mut self, key: &str, secs_per_char: f64) {
        if !secs_per_char.is_finite() || secs_per_char <= 0.0 {
            return;
        }
        let e = self.entries.entry(key.to_string()).or_default();
        e.secs_per_char = Self::blend(e.secs_per_char, secs_per_char, e.samples);
        e.samples = e.samples.saturating_add(1);
    }

    pub fn record_load(&mut self, key: &str, load_secs: f64) {
        if !load_secs.is_finite() || load_secs <= 0.0 {
            return;
        }
        let e = self.entries.entry(key.to_string()).or_default();
        e.load_secs = Self::blend(e.load_secs, load_secs, e.load_samples);
        e.load_samples = e.load_samples.saturating_add(1);
    }

    /// Best-effort save; estimates are a convenience, never a failure.
    pub fn save(&self) {
        let Some(path) = &self.path else { return };
        if let Some(dir) = path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        if let Ok(s) = serde_json::to_string_pretty(self) {
            let tmp = path.with_extension("json.partial");
            if std::fs::write(&tmp, s).is_ok() {
                let _ = std::fs::rename(tmp, path);
            }
        }
    }
}

/// Live estimator for one job.
#[derive(Debug, Clone)]
pub struct Estimator {
    prior_secs_per_char: f64,
    done_chars: usize,
    done_secs: f64,
    total_chars: usize,
}

impl Estimator {
    pub fn new(prior: EngineCalibration, total_chars: usize) -> Self {
        Self {
            prior_secs_per_char: prior.secs_per_char,
            done_chars: 0,
            done_secs: 0.0,
            total_chars,
        }
    }

    /// Current rate: the prior until real data exists, then the observed rate
    /// (blended with the prior while little text has been processed).
    pub fn secs_per_char(&self) -> f64 {
        if self.done_chars == 0 {
            return self.prior_secs_per_char;
        }
        let observed = self.done_secs / self.done_chars as f64;
        let confidence = (self.done_chars as f64 / 300.0).min(1.0);
        confidence * observed + (1.0 - confidence) * self.prior_secs_per_char
    }

    pub fn record_chunk(&mut self, chars: usize, secs: f64) {
        self.done_chars += chars;
        self.done_secs += secs;
    }

    /// Seconds remaining, given the current chunk's size and time spent on it.
    pub fn remaining_secs(&self, current_chunk_chars: usize, current_elapsed: f64) -> f64 {
        let rate = self.secs_per_char();
        let remaining_chars = self
            .total_chars
            .saturating_sub(self.done_chars + current_chunk_chars);
        let current_left = (current_chunk_chars as f64 * rate - current_elapsed).max(0.0);
        remaining_chars as f64 * rate + current_left
    }

    pub fn observed_secs_per_char(&self) -> Option<f64> {
        (self.done_chars > 0).then(|| self.done_secs / self.done_chars as f64)
    }
}

/// Human-friendly duration, e.g. `42s`, `3m 05s`, `1h 02m`.
pub fn format_duration(secs: f64) -> String {
    let s = secs.max(0.0).round() as u64;
    if s < 60 {
        format!("{s}s")
    } else if s < 3600 {
        format!("{}m {:02}s", s / 60, s % 60)
    } else {
        format!("{}h {:02}m", s / 3600, (s % 3600) / 60)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn estimator_uses_prior_then_converges_to_observed() {
        let prior = EngineCalibration {
            secs_per_char: 0.1,
            load_secs: 5.0,
            samples: 3,
            load_samples: 3,
        };
        let mut e = Estimator::new(prior, 1000);
        assert!((e.remaining_secs(0, 0.0) - 100.0).abs() < 1e-9);
        e.record_chunk(300, 60.0); // observed 0.2 s/char, full confidence at 300 chars
        assert!((e.secs_per_char() - 0.2).abs() < 1e-9);
        assert!((e.remaining_secs(0, 0.0) - 140.0).abs() < 1e-9);
        // Mid-chunk: 200 chars in flight, 10 s spent.
        assert!((e.remaining_secs(200, 10.0) - (500.0 * 0.2 + 30.0)).abs() < 1e-9);
        // Overrun of the current chunk never makes the estimate negative.
        assert!(e.remaining_secs(200, 1000.0) >= 0.0);
    }

    #[test]
    fn calibration_round_trips_and_ignores_garbage() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("sub").join("calibration.json");
        let mut c = Calibration::load(&path);
        let key = Calibration::key("native", 10);
        assert_eq!(c.get(&key), EngineCalibration::default());
        c.record_rate(&key, 0.08);
        c.record_rate(&key, f64::NAN);
        c.record_load(&key, 7.0);
        c.save();
        let c2 = Calibration::load(&path);
        let e = c2.get(&key);
        assert!((e.secs_per_char - 0.08).abs() < 1e-12);
        assert!((e.load_secs - 7.0).abs() < 1e-12);
        assert_eq!(e.samples, 1);
        std::fs::write(&path, "{not json").unwrap();
        assert_eq!(
            Calibration::load(&path).get(&key),
            EngineCalibration::default()
        );
    }

    #[test]
    fn durations_format() {
        assert_eq!(format_duration(4.4), "4s");
        assert_eq!(format_duration(185.0), "3m 05s");
        assert_eq!(format_duration(3720.0), "1h 02m");
        assert_eq!(format_duration(-3.0), "0s");
    }
}
