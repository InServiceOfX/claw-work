//! Reference-voice preparation: validate the user's clip and write a clean,
//! mono WAV copy the engines can consume.

use std::path::{Path, PathBuf};

use anyhow::{Result, bail};

use crate::audio::{Audio, WavFormat, decode_file, trim_silence, write_wav};
use crate::config::{MAX_REFERENCE_SECONDS, MIN_REFERENCE_SECONDS};
use crate::engine::VoicePrompt;

pub const SUPPORTED_EXTENSIONS: &[&str] = &["wav", "mp3", "flac", "m4a", "aac"];

#[derive(Debug, Clone)]
pub struct PreparedVoice {
    pub prompt: VoicePrompt,
    pub original_seconds: f64,
    pub used_seconds: f64,
    pub warnings: Vec<String>,
}

/// Decode, downmix, trim edge silence, cap the length, peak-normalize and
/// write a mono 32-bit float WAV into `work_dir`.
///
/// A transcript, when given, must describe the whole clip; a clip with a
/// transcript is therefore never truncated (it is rejected if too long).
pub fn prepare_reference(
    source: &Path,
    transcript: Option<String>,
    work_dir: &Path,
) -> Result<PreparedVoice> {
    let ext = source
        .extension()
        .and_then(|e| e.to_str())
        .map(str::to_ascii_lowercase)
        .unwrap_or_default();
    if !SUPPORTED_EXTENSIONS.contains(&ext.as_str()) {
        bail!(
            "unsupported reference voice format '{}': use one of {}",
            source.display(),
            SUPPORTED_EXTENSIONS.join(", ")
        );
    }
    if !source.is_file() {
        bail!("reference voice file not found: {}", source.display());
    }
    let decoded = decode_file(source)?;
    let original_seconds = decoded.duration_seconds();
    let mut audio = trim_silence(&decoded, -50.0, 0.10);
    let mut warnings = Vec::new();
    if audio.duration_seconds() < MIN_REFERENCE_SECONDS {
        bail!(
            "reference voice has only {:.1}s of audible speech; provide at least {MIN_REFERENCE_SECONDS:.0}s \
             (5–20s of clean speech works best)",
            audio.duration_seconds()
        );
    }
    let transcript = transcript
        .map(|t| t.trim().to_string())
        .filter(|t| !t.is_empty());
    if audio.duration_seconds() > MAX_REFERENCE_SECONDS {
        if transcript.is_some() {
            bail!(
                "reference voice is {:.1}s long; with a transcript it must be at most {MAX_REFERENCE_SECONDS:.0}s \
                 (cut the clip and its transcript to match)",
                audio.duration_seconds()
            );
        }
        let keep = (MAX_REFERENCE_SECONDS * audio.sample_rate as f64) as usize;
        audio = Audio::new(audio.samples[..keep].to_vec(), audio.sample_rate);
        warnings.push(format!(
            "reference voice trimmed to its first {MAX_REFERENCE_SECONDS:.0}s (was {original_seconds:.1}s)"
        ));
    }
    let peak = audio.samples.iter().fold(0.0f32, |m, s| m.max(s.abs()));
    if peak < 1e-3 {
        bail!("reference voice is silent");
    }
    if peak >= 0.999 {
        warnings.push("reference voice appears clipped; cloning quality may suffer".into());
    }
    let gain = 0.89 / peak; // ≈ -1 dBFS
    audio.samples.iter_mut().for_each(|s| *s *= gain);
    let out: PathBuf = work_dir.join("reference.wav");
    write_wav(&out, &audio, WavFormat::Float32)?;
    Ok(PreparedVoice {
        used_seconds: audio.duration_seconds(),
        original_seconds,
        prompt: VoicePrompt {
            audio_path: out,
            transcript,
        },
        warnings,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn speechlike(seconds: f64, sr: u32) -> Audio {
        let n = (seconds * sr as f64) as usize;
        Audio::new(
            (0..n)
                .map(|i| {
                    let t = i as f32 / sr as f32;
                    0.3 * (2.0 * std::f32::consts::PI * 150.0 * t).sin()
                        * (0.5 + 0.5 * (6.0 * t).sin().abs())
                })
                .collect(),
            sr,
        )
    }

    fn write(dir: &Path, name: &str, a: &Audio) -> PathBuf {
        let p = dir.join(name);
        write_wav(&p, a, WavFormat::Pcm16).unwrap();
        p
    }

    #[test]
    fn prepares_mono_normalized_copy() {
        let dir = tempfile::tempdir().unwrap();
        let src = write(dir.path(), "voice.wav", &speechlike(6.0, 44_100));
        let work = tempfile::tempdir().unwrap();
        let v = prepare_reference(&src, Some("  hello  ".into()), work.path()).unwrap();
        assert_eq!(v.prompt.transcript.as_deref(), Some("hello"));
        let a = crate::audio::read_wav(&v.prompt.audio_path).unwrap();
        assert_eq!(a.sample_rate, 44_100);
        let peak = a.samples.iter().fold(0.0f32, |m, s| m.max(s.abs()));
        assert!((peak - 0.89).abs() < 0.01);
        assert!(v.warnings.is_empty());
        // The user's file is untouched.
        assert!(src.exists());
    }

    #[test]
    fn rejects_bad_inputs() {
        let dir = tempfile::tempdir().unwrap();
        let work = tempfile::tempdir().unwrap();
        let short = write(dir.path(), "short.wav", &speechlike(1.0, 16_000));
        assert!(
            prepare_reference(&short, None, work.path())
                .unwrap_err()
                .to_string()
                .contains("at least")
        );
        let txt = dir.path().join("voice.txt");
        std::fs::write(&txt, "x").unwrap();
        assert!(
            prepare_reference(&txt, None, work.path())
                .unwrap_err()
                .to_string()
                .contains("unsupported")
        );
        assert!(prepare_reference(&dir.path().join("missing.wav"), None, work.path()).is_err());
        let silent = write(dir.path(), "silent.wav", &Audio::silence(5.0, 16_000));
        assert!(prepare_reference(&silent, None, work.path()).is_err());
    }

    #[test]
    fn long_clip_trimmed_without_transcript_rejected_with_one() {
        let dir = tempfile::tempdir().unwrap();
        let work = tempfile::tempdir().unwrap();
        let long = write(dir.path(), "long.wav", &speechlike(40.0, 16_000));
        let v = prepare_reference(&long, None, work.path()).unwrap();
        assert!((v.used_seconds - MAX_REFERENCE_SECONDS).abs() < 0.01);
        assert_eq!(v.warnings.len(), 1);
        assert!(prepare_reference(&long, Some("words".into()), work.path()).is_err());
    }
}
