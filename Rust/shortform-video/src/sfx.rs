//! Synthesized sound effects, so no licensed audio file is needed.
//!
//! A camera shutter on every cut to a photo reads as "looking through
//! memories" (ginyboi, editing without B-roll). The click is generated here:
//! two short bursts of deterministic noise (shutter open, then close), each
//! with a fast exponential decay and a first-difference high-pass so it
//! sounds crisp rather than like hiss.

use std::io::Write;
use std::path::Path;

pub const SAMPLE_RATE: u32 = 48_000;
/// Seconds between the "open" and "close" clicks of one shutter.
const SECOND_CLICK_AT: f64 = 0.055;
/// Decay rate of each click, per second (about 7 ms to 1/e).
const DECAY: f64 = 140.0;
/// How long each click is rendered before it is treated as silent.
const CLICK_LENGTH: f64 = 0.04;
/// Cuts closer than this to the start are not cuts; the video opens there.
pub const MIN_CUT_TIME: f64 = 0.05;

/// Deterministic white noise in [-1, 1] (64-bit LCG), so renders repeat exactly.
struct Noise(u64);

impl Noise {
    fn next(&mut self) -> f64 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        ((self.0 >> 11) as f64 / (1u64 << 53) as f64) * 2.0 - 1.0
    }
}

/// A mono track `duration` seconds long with one shutter (peak about 1.0)
/// at each time in `cuts`. Times before `MIN_CUT_TIME` or past the end are skipped.
pub fn shutter_track(cuts: &[f64], duration: f64) -> Vec<f32> {
    let len = (duration.max(0.0) * f64::from(SAMPLE_RATE)).round() as usize;
    let mut track = vec![0.0f32; len];
    let mut noise = Noise(0x5eed_cafe);
    for &cut in cuts.iter().filter(|t| **t >= MIN_CUT_TIME && **t < duration) {
        for (offset, gain) in [(0.0, 1.0), (SECOND_CLICK_AT, 0.7)] {
            let start = ((cut + offset) * f64::from(SAMPLE_RATE)).round() as usize;
            let samples = (CLICK_LENGTH * f64::from(SAMPLE_RATE)) as usize;
            let mut previous = 0.0;
            for i in 0..samples {
                let Some(slot) = track.get_mut(start + i) else { break };
                let t = i as f64 / f64::from(SAMPLE_RATE);
                let raw = noise.next();
                // First difference: a cheap high-pass that removes the low rumble.
                let crisp = (raw - previous) * 0.5;
                previous = raw;
                *slot += (gain * crisp * (-t * DECAY).exp()) as f32;
            }
        }
    }
    let peak = track.iter().fold(0.0f32, |m, s| m.max(s.abs()));
    if peak > 0.0 {
        for s in &mut track {
            *s /= peak;
        }
    }
    track
}

/// Write mono 16-bit PCM WAV.
pub fn write_wav(path: &Path, samples: &[f32]) -> std::io::Result<()> {
    let data_len = (samples.len() * 2) as u32;
    let mut out = Vec::with_capacity(44 + samples.len() * 2);
    out.extend_from_slice(b"RIFF");
    out.extend_from_slice(&(36 + data_len).to_le_bytes());
    out.extend_from_slice(b"WAVEfmt ");
    out.extend_from_slice(&16u32.to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes()); // PCM
    out.extend_from_slice(&1u16.to_le_bytes()); // mono
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

    fn energy(track: &[f32], from: f64, to: f64) -> f32 {
        let a = (from * f64::from(SAMPLE_RATE)) as usize;
        let b = ((to * f64::from(SAMPLE_RATE)) as usize).min(track.len());
        track[a..b].iter().map(|s| s * s).sum()
    }

    #[test]
    fn clicks_land_on_the_cuts_and_nowhere_else() {
        let track = shutter_track(&[1.0, 2.5], 4.0);
        assert_eq!(track.len(), 4 * SAMPLE_RATE as usize);
        assert!(energy(&track, 1.0, 1.04) > 1.0);
        assert!(energy(&track, 1.055, 1.095) > 0.5, "second (closing) click");
        assert!(energy(&track, 2.5, 2.54) > 1.0);
        assert_eq!(energy(&track, 0.0, 0.99), 0.0);
        assert_eq!(energy(&track, 1.2, 2.49), 0.0);
        assert_eq!(energy(&track, 2.7, 4.0), 0.0);
        let peak = track.iter().fold(0.0f32, |m, s| m.max(s.abs()));
        assert!((peak - 1.0).abs() < 1e-6);
    }

    #[test]
    fn the_opening_frame_and_out_of_range_times_get_no_click() {
        let track = shutter_track(&[0.0, 0.01, 9.0], 2.0);
        assert!(track.iter().all(|s| *s == 0.0));
    }

    #[test]
    fn output_is_deterministic() {
        assert_eq!(shutter_track(&[0.5], 1.0), shutter_track(&[0.5], 1.0));
    }

    #[test]
    fn wav_header_and_length_are_valid() {
        let dir = std::env::temp_dir().join(format!("sfx-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("t.wav");
        write_wav(&path, &shutter_track(&[0.5], 1.0)).unwrap();
        let bytes = std::fs::read(&path).unwrap();
        assert_eq!(&bytes[0..4], b"RIFF");
        assert_eq!(&bytes[8..12], b"WAVE");
        assert_eq!(bytes.len(), 44 + 2 * SAMPLE_RATE as usize);
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
