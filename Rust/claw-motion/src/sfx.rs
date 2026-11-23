//! Impact sounds synthesized from the simulation's collision list.
//!
//! Every [`Impact`] becomes a short knock: a few inharmonic partials (the
//! 1 : 2.76 : 5.40 ratios of a struck free bar, which reads as wood or a hard
//! marble) with an exponential decay, plus a 1.5 ms noise transient for the
//! attack. Pitch follows size (small balls tick, big ones thud) and level
//! follows approach speed. The sound starts on the simulation step of the hit,
//! so it lines up with the picture to within one step (about 4 ms at 240 Hz).
//!
//! A pile of fifty balls landing at once would otherwise turn into white
//! noise, so impacts are thinned: in each 12 ms window only the loudest few
//! are kept.

use std::io::Write;
use std::path::Path;

use crate::physics::Impact;

pub const SAMPLE_RATE: u32 = 48_000;
const PARTIALS: [(f64, f64); 3] = [(1.0, 1.0), (2.76, 0.45), (5.40, 0.2)];
const CLICK_SECONDS: f64 = 0.0015;

#[derive(Debug, Clone, Copy)]
pub struct SfxSettings {
    /// Pitch of a ball of `reference_radius`, Hz.
    pub base_hz: f64,
    pub reference_radius: f64,
    /// Approach speed (px/s) that plays at full level.
    pub full_speed: f64,
    /// Thinning window, seconds.
    pub window: f64,
    /// Impacts kept per window.
    pub per_window: usize,
}

impl Default for SfxSettings {
    fn default() -> Self {
        SfxSettings { base_hz: 880.0, reference_radius: 14.0, full_speed: 1800.0, window: 0.012, per_window: 3 }
    }
}

/// The impacts that will be heard: per window, the fastest `per_window`, in time order.
pub fn thin(impacts: &[Impact], s: &SfxSettings) -> Vec<Impact> {
    let mut kept = Vec::new();
    let mut i = 0;
    while i < impacts.len() {
        let bucket = (impacts[i].time / s.window).floor();
        let mut j = i;
        while j < impacts.len() && (impacts[j].time / s.window).floor() == bucket {
            j += 1;
        }
        let mut group: Vec<Impact> = impacts[i..j].to_vec();
        group.sort_by(|a, b| b.speed.total_cmp(&a.speed).then(a.time.total_cmp(&b.time)));
        group.truncate(s.per_window);
        group.sort_by(|a, b| a.time.total_cmp(&b.time));
        kept.extend(group);
        i = j;
    }
    kept
}

/// Pitch for a body radius, kept in a comfortable band.
pub fn pitch(radius: f64, s: &SfxSettings) -> f64 {
    (s.base_hz * (s.reference_radius / radius.max(1.0)).powf(0.8)).clamp(180.0, 4200.0)
}

/// Mono track of `duration` seconds with one knock per kept impact, peak-normalized to 0.9.
pub fn track(impacts: &[Impact], duration: f64, s: &SfxSettings) -> Vec<f32> {
    let rate = f64::from(SAMPLE_RATE);
    let len = (duration.max(0.0) * rate).round() as usize;
    let mut out = vec![0.0f64; len];
    let mut noise = 0x2545_F491_4F6C_DD1Du64;
    for (n, hit) in thin(impacts, s).iter().enumerate() {
        let start = (hit.time * rate).round() as usize;
        if start >= len {
            continue;
        }
        // A small, deterministic detune per hit keeps repeated knocks from sounding mechanical.
        let detune = 1.0 + 0.03 * (((n as u64).wrapping_mul(2_654_435_761) % 1000) as f64 / 500.0 - 1.0);
        let f0 = pitch(hit.radius, s) * detune;
        let level = (hit.speed / s.full_speed).clamp(0.0, 1.0).powf(0.6);
        // Bigger bodies ring longer.
        let tau = 0.012 + 0.0011 * hit.radius;
        let samples = ((tau * 7.0) * rate) as usize;
        for k in 0..samples {
            let Some(slot) = out.get_mut(start + k) else { break };
            let t = k as f64 / rate;
            let env = (-t / tau).exp();
            let mut v: f64 = PARTIALS.iter().map(|(ratio, amp)| amp * (std::f64::consts::TAU * f0 * ratio * t).sin() * (-t * ratio.sqrt() / tau).exp()).sum();
            if t < CLICK_SECONDS {
                noise ^= noise << 13;
                noise ^= noise >> 7;
                noise ^= noise << 17;
                let white = (noise >> 11) as f64 / (1u64 << 53) as f64 * 2.0 - 1.0;
                v += 0.6 * white * (1.0 - t / CLICK_SECONDS);
            }
            *slot += level * env * v;
        }
    }
    let peak = out.iter().fold(0.0f64, |m, v| m.max(v.abs()));
    let gain = if peak > 0.0 { 0.9 / peak } else { 0.0 };
    out.iter().map(|v| (v * gain) as f32).collect()
}

/// Write mono 16-bit PCM WAV.
pub fn write_wav(path: &Path, samples: &[f32]) -> std::io::Result<()> {
    let data_len = (samples.len() * 2) as u32;
    let mut out = Vec::with_capacity(44 + samples.len() * 2);
    out.extend_from_slice(b"RIFF");
    out.extend_from_slice(&(36 + data_len).to_le_bytes());
    out.extend_from_slice(b"WAVEfmt ");
    out.extend_from_slice(&16u32.to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&SAMPLE_RATE.to_le_bytes());
    out.extend_from_slice(&(SAMPLE_RATE * 2).to_le_bytes());
    out.extend_from_slice(&2u16.to_le_bytes());
    out.extend_from_slice(&16u16.to_le_bytes());
    out.extend_from_slice(b"data");
    out.extend_from_slice(&data_len.to_le_bytes());
    for s in samples {
        let v = (s.clamp(-1.0, 1.0) * f32::from(i16::MAX)).round() as i16;
        out.extend_from_slice(&v.to_le_bytes());
    }
    std::fs::File::create(path)?.write_all(&out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hit(time: f64, speed: f64, radius: f64) -> Impact {
        Impact { time, speed, x: 0.0, y: 0.0, radius, on_static: true }
    }

    fn energy(t: &[f32], from: f64, to: f64) -> f64 {
        let r = f64::from(SAMPLE_RATE);
        let (a, b) = ((from * r) as usize, ((to * r) as usize).min(t.len()));
        t[a..b].iter().map(|s| f64::from(*s).powi(2)).sum()
    }

    #[test]
    fn knocks_start_on_their_impact_and_are_silent_elsewhere() {
        let t = track(&[hit(0.5, 1500.0, 14.0), hit(1.5, 1500.0, 14.0)], 2.0, &SfxSettings::default());
        assert_eq!(t.len(), 96_000);
        assert_eq!(energy(&t, 0.0, 0.4999), 0.0);
        assert!(energy(&t, 0.5, 0.52) > 1.0);
        assert_eq!(energy(&t, 0.8, 1.4999), 0.0);
        assert!(energy(&t, 1.5, 1.52) > 1.0);
        let peak = t.iter().fold(0.0f32, |m, s| m.max(s.abs()));
        assert!((peak - 0.9).abs() < 1e-6);
    }

    #[test]
    fn faster_hits_are_louder_and_small_balls_are_higher() {
        let s = SfxSettings::default();
        let t = track(&[hit(0.1, 1800.0, 14.0), hit(1.1, 300.0, 14.0)], 2.0, &s);
        assert!(energy(&t, 0.1, 0.2) > 4.0 * energy(&t, 1.1, 1.2));
        assert!(pitch(6.0, &s) > pitch(14.0, &s));
        assert!(pitch(40.0, &s) < pitch(14.0, &s));
        assert_eq!(pitch(0.001, &s), 4200.0);
    }

    #[test]
    fn crowded_windows_keep_only_the_loudest_hits() {
        let s = SfxSettings::default();
        let crowd: Vec<Impact> = (0..10).map(|k| hit(1.0 + k as f64 * 0.0005, 100.0 * (k + 1) as f64, 10.0)).collect();
        let kept = thin(&crowd, &s);
        assert_eq!(kept.len(), 3);
        let speeds: Vec<f64> = kept.iter().map(|h| h.speed).collect();
        assert_eq!(speeds, vec![800.0, 900.0, 1000.0]);
        assert!(kept.windows(2).all(|w| w[0].time <= w[1].time));
    }

    #[test]
    fn empty_and_out_of_range_impacts_give_silence() {
        let s = SfxSettings::default();
        assert!(track(&[], 1.0, &s).iter().all(|v| *v == 0.0));
        assert!(track(&[hit(5.0, 1000.0, 10.0)], 1.0, &s).iter().all(|v| *v == 0.0));
    }

    #[test]
    fn output_is_deterministic() {
        let s = SfxSettings::default();
        let hits = [hit(0.2, 900.0, 9.0), hit(0.21, 1200.0, 20.0)];
        assert_eq!(track(&hits, 1.0, &s), track(&hits, 1.0, &s));
    }
}
