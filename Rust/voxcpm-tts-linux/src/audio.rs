//! Audio I/O and assembly: decode WAV/MP3 (and FLAC/AAC/M4A) to mono f32,
//! band-limited resampling, WAV writing, and stitching chunk audio into one
//! seamless take.

use std::fs::File;
use std::path::Path;

use anyhow::{Context, Result, anyhow, bail, ensure};
use symphonia::core::audio::SampleBuffer;
use symphonia::core::codecs::{CODEC_TYPE_NULL, DecoderOptions};
use symphonia::core::errors::Error as SymphoniaError;
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;

use crate::text::Pause;

/// Mono audio at a known sample rate.
#[derive(Debug, Clone, PartialEq)]
pub struct Audio {
    pub samples: Vec<f32>,
    pub sample_rate: u32,
}

impl Audio {
    pub fn new(samples: Vec<f32>, sample_rate: u32) -> Self {
        Self {
            samples,
            sample_rate,
        }
    }

    pub fn duration_seconds(&self) -> f64 {
        self.samples.len() as f64 / self.sample_rate as f64
    }

    pub fn silence(seconds: f64, sample_rate: u32) -> Self {
        Self {
            samples: vec![0.0; (seconds * sample_rate as f64).round() as usize],
            sample_rate,
        }
    }
}

/// Decode an audio file to mono f32 (channels averaged).
pub fn decode_file(path: &Path) -> Result<Audio> {
    let file =
        File::open(path).with_context(|| format!("opening audio file {}", path.display()))?;
    let mss = MediaSourceStream::new(Box::new(file), Default::default());
    let mut hint = Hint::new();
    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        hint.with_extension(ext);
    }
    let probed = symphonia::default::get_probe()
        .format(
            &hint,
            mss,
            &FormatOptions::default(),
            &MetadataOptions::default(),
        )
        .with_context(|| format!("unsupported or corrupt audio file {}", path.display()))?;
    let mut format = probed.format;
    let track = format
        .tracks()
        .iter()
        .find(|t| t.codec_params.codec != CODEC_TYPE_NULL)
        .ok_or_else(|| anyhow!("no audio track in {}", path.display()))?;
    let track_id = track.id;
    let sample_rate = track
        .codec_params
        .sample_rate
        .ok_or_else(|| anyhow!("unknown sample rate in {}", path.display()))?;
    let mut decoder = symphonia::default::get_codecs()
        .make(&track.codec_params, &DecoderOptions::default())
        .with_context(|| format!("no decoder for {}", path.display()))?;

    let mut mono = Vec::new();
    let mut buf: Option<SampleBuffer<f32>> = None;
    loop {
        let packet = match format.next_packet() {
            Ok(p) => p,
            Err(SymphoniaError::IoError(e)) if e.kind() == std::io::ErrorKind::UnexpectedEof => {
                break;
            }
            Err(SymphoniaError::ResetRequired) => break,
            Err(e) => return Err(e).context("reading audio packets"),
        };
        if packet.track_id() != track_id {
            continue;
        }
        let decoded = match decoder.decode(&packet) {
            Ok(d) => d,
            Err(SymphoniaError::DecodeError(_)) => continue, // skip a corrupt frame
            Err(e) => return Err(e).context("decoding audio"),
        };
        let spec = *decoded.spec();
        let channels = spec.channels.count().max(1);
        let sb = buf.get_or_insert_with(|| SampleBuffer::new(decoded.capacity() as u64, spec));
        if sb.capacity() < decoded.capacity() * channels {
            *sb = SampleBuffer::new(decoded.capacity() as u64, spec);
        }
        sb.copy_interleaved_ref(decoded);
        for frame in sb.samples().chunks(channels) {
            mono.push(frame.iter().sum::<f32>() / channels as f32);
        }
    }
    if mono.is_empty() {
        bail!("{} decoded to zero samples", path.display());
    }
    Ok(Audio::new(mono, sample_rate))
}

fn bessel_i0(x: f64) -> f64 {
    let mut sum = 1.0;
    let mut term = 1.0;
    let half = x / 2.0;
    for k in 1..50 {
        term *= (half / k as f64) * (half / k as f64);
        sum += term;
        if term < 1e-12 * sum {
            break;
        }
    }
    sum
}

/// Band-limited resampling with a Kaiser-windowed sinc kernel.
pub fn resample(audio: &Audio, target_rate: u32) -> Audio {
    if audio.sample_rate == target_rate || audio.samples.is_empty() {
        return Audio::new(audio.samples.clone(), target_rate);
    }
    let ratio = target_rate as f64 / audio.sample_rate as f64;
    let cutoff = ratio.min(1.0) * 0.95; // relative to the input Nyquist
    let half_width = 32.0 / cutoff.min(1.0); // taps on each side, in input samples
    let beta = 8.6;
    let i0_beta = bessel_i0(beta);
    let out_len = (audio.samples.len() as f64 * ratio).round() as usize;
    let input = &audio.samples;
    let mut out = Vec::with_capacity(out_len);
    for n in 0..out_len {
        let t = n as f64 / ratio; // position in input samples
        let lo = (t - half_width).ceil().max(0.0) as usize;
        let hi = ((t + half_width).floor() as usize).min(input.len() - 1);
        let mut acc = 0.0;
        let mut norm = 0.0;
        for (k, &x) in input.iter().enumerate().take(hi + 1).skip(lo) {
            let d = k as f64 - t;
            let w_arg = 1.0 - (d / half_width).powi(2);
            if w_arg <= 0.0 {
                continue;
            }
            let window = bessel_i0(beta * w_arg.sqrt()) / i0_beta;
            let xs = std::f64::consts::PI * d * cutoff;
            let sinc = if xs.abs() < 1e-9 { 1.0 } else { xs.sin() / xs };
            let w = sinc * window;
            acc += x as f64 * w;
            norm += w;
        }
        out.push(if norm.abs() > 1e-12 {
            (acc / norm) as f32
        } else {
            0.0
        });
    }
    Audio::new(out, target_rate)
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum, serde::Serialize, serde::Deserialize,
)]
pub enum WavFormat {
    /// 16-bit integer PCM (most compatible).
    Pcm16,
    /// 32-bit IEEE float.
    Float32,
}

/// Write mono audio as WAV, via a temporary sibling file renamed into place.
pub fn write_wav(path: &Path, audio: &Audio, format: WavFormat) -> Result<()> {
    write_wav_output(path, audio, format, true)
}

/// Atomically publish a complete WAV; no-clobber is enforced at publication,
/// including if another writer creates the destination during synthesis.
pub fn write_wav_output(
    path: &Path,
    audio: &Audio,
    format: WavFormat,
    overwrite: bool,
) -> Result<()> {
    let spec = hound::WavSpec {
        channels: 1,
        sample_rate: audio.sample_rate,
        bits_per_sample: if format == WavFormat::Pcm16 { 16 } else { 32 },
        sample_format: if format == WavFormat::Pcm16 {
            hound::SampleFormat::Int
        } else {
            hound::SampleFormat::Float
        },
    };
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let mut tmp = tempfile::Builder::new()
        .prefix(".voxcpm-")
        .suffix(".wav.partial")
        .tempfile_in(parent)?;
    {
        let mut w = hound::WavWriter::new(tmp.as_file_mut(), spec)
            .with_context(|| format!("creating output for {}", path.display()))?;
        for &s in &audio.samples {
            let s = if s.is_finite() {
                s.clamp(-1.0, 1.0)
            } else {
                0.0
            };
            match format {
                WavFormat::Pcm16 => w.write_sample((s * i16::MAX as f32).round() as i16)?,
                WavFormat::Float32 => w.write_sample(s)?,
            }
        }
        w.finalize()?;
    }
    if overwrite {
        tmp.persist(path)
    } else {
        tmp.persist_noclobber(path)
    }
    .map_err(|e| e.error)
    .with_context(|| format!("moving output into {}", path.display()))?;
    Ok(())
}

/// Read a WAV (any bit depth, any channel count) as mono f32.
pub fn read_wav(path: &Path) -> Result<Audio> {
    let mut r =
        hound::WavReader::open(path).with_context(|| format!("opening {}", path.display()))?;
    let spec = r.spec();
    let channels = spec.channels.max(1) as usize;
    let interleaved: Vec<f32> = match spec.sample_format {
        hound::SampleFormat::Float => r.samples::<f32>().collect::<Result<_, _>>()?,
        hound::SampleFormat::Int => {
            let scale = (1i64 << (spec.bits_per_sample - 1)) as f32;
            r.samples::<i32>()
                .map(|s| s.map(|v| v as f32 / scale))
                .collect::<Result<_, _>>()?
        }
    };
    let mono = interleaved
        .chunks(channels)
        .map(|f| f.iter().sum::<f32>() / channels as f32)
        .collect();
    Ok(Audio::new(mono, spec.sample_rate))
}

fn db_to_amp(db: f32) -> f32 {
    10f32.powf(db / 20.0)
}

/// Settings for joining chunk audio.
#[derive(Debug, Clone, Copy)]
pub struct StitchOptions {
    /// Leading/trailing audio quieter than this (dBFS, 10 ms frames) is trimmed.
    pub silence_threshold_db: f32,
    /// Audio kept around detected speech when trimming, in seconds.
    pub trim_padding: f64,
    pub sentence_gap: f64,
    pub paragraph_gap: f64,
    pub clause_gap: f64,
    /// Fade length applied at both ends of every chunk, in seconds.
    pub fade: f64,
    /// Max gain adjustment (dB) when matching chunk loudness to the median.
    pub max_gain_db: f32,
    /// Peak ceiling (dBFS) for the assembled result.
    pub peak_ceiling_db: f32,
}

impl StitchOptions {
    /// Reject invalid or unbounded durations before audio generation or
    /// allocation. Both the CLI and GUI use this same validation contract.
    pub fn validate(&self) -> Result<()> {
        for (name, seconds, maximum) in [
            ("sentence gap", self.sentence_gap, 60.0),
            ("paragraph gap", self.paragraph_gap, 60.0),
            ("clause gap", self.clause_gap, 60.0),
            ("fade", self.fade, 1.0),
            ("trim padding", self.trim_padding, 1.0),
        ] {
            ensure!(
                seconds.is_finite() && (0.0..=maximum).contains(&seconds),
                "{name} must be finite and in 0..={maximum} seconds"
            );
        }
        ensure!(
            self.max_gain_db.is_finite() && (0.0..=30.0).contains(&self.max_gain_db),
            "maximum gain adjustment must be finite and in 0..=30 dB"
        );
        ensure!(
            self.peak_ceiling_db.is_finite() && self.peak_ceiling_db <= 0.0,
            "peak ceiling must be finite and at most 0 dBFS"
        );
        ensure!(
            self.silence_threshold_db.is_finite() && self.silence_threshold_db <= 0.0,
            "silence threshold must be finite and at most 0 dBFS"
        );
        Ok(())
    }
}

impl Default for StitchOptions {
    fn default() -> Self {
        Self {
            silence_threshold_db: -45.0,
            trim_padding: 0.04,
            sentence_gap: 0.30,
            paragraph_gap: 0.65,
            clause_gap: 0.12,
            fade: 0.008,
            max_gain_db: 6.0,
            peak_ceiling_db: -1.0,
        }
    }
}

/// Trim leading and trailing silence, keeping `padding` seconds around speech.
pub fn trim_silence(audio: &Audio, threshold_db: f32, padding: f64) -> Audio {
    let frame = (audio.sample_rate as usize / 100).max(1);
    let threshold = db_to_amp(threshold_db);
    let loud = |chunk: &[f32]| {
        let rms = (chunk.iter().map(|s| s * s).sum::<f32>() / chunk.len() as f32).sqrt();
        rms > threshold
    };
    let frames: Vec<&[f32]> = audio.samples.chunks(frame).collect();
    let Some(first) = frames.iter().position(|f| loud(f)) else {
        return Audio::new(Vec::new(), audio.sample_rate);
    };
    let last = frames.iter().rposition(|f| loud(f)).unwrap_or(first);
    let pad = (padding * audio.sample_rate as f64) as usize;
    let start = (first * frame).saturating_sub(pad);
    let end = ((last + 1) * frame + pad).min(audio.samples.len());
    Audio::new(audio.samples[start..end].to_vec(), audio.sample_rate)
}

/// RMS of frames louder than the silence threshold (speech-only loudness).
pub fn active_rms(audio: &Audio, threshold_db: f32) -> Option<f32> {
    let frame = (audio.sample_rate as usize / 100).max(1);
    let threshold = db_to_amp(threshold_db);
    let mut sum = 0.0f64;
    let mut n = 0usize;
    for f in audio.samples.chunks(frame) {
        let e = f.iter().map(|s| (*s as f64).powi(2)).sum::<f64>();
        if (e / f.len() as f64).sqrt() as f32 > threshold {
            sum += e;
            n += f.len();
        }
    }
    (n > 0).then(|| (sum / n as f64).sqrt() as f32)
}

fn apply_fades(samples: &mut [f32], fade_len: usize) {
    let n = samples.len();
    let fade_len = fade_len.min(n / 2);
    for i in 0..fade_len {
        let g = (i as f32 + 0.5) / fade_len as f32;
        let g = (g * std::f32::consts::FRAC_PI_2).sin().powi(2);
        samples[i] *= g;
        samples[n - 1 - i] *= g;
    }
}

/// Join chunk audio in order: trim each chunk's edges, match loudness to the
/// median chunk, fade edges to avoid clicks, insert pause-appropriate gaps,
/// and keep peaks under the ceiling.
pub fn stitch(parts: &[(Audio, Pause)], opts: &StitchOptions) -> Result<Audio> {
    opts.validate()?;
    let Some((first, _)) = parts.first() else {
        bail!("nothing to stitch")
    };
    let sr = first.sample_rate;
    ensure!(
        sr > 0 && sr <= 384_000,
        "audio sample rate must be in 1..=384000 Hz"
    );
    if parts.iter().any(|(a, _)| a.sample_rate != sr) {
        bail!("chunks have different sample rates");
    }
    for (i, (audio, _)) in parts.iter().enumerate() {
        ensure!(
            !audio.samples.is_empty() && audio.samples.iter().all(|s| s.is_finite()),
            "chunk {} has empty or nonfinite audio",
            i + 1
        );
    }
    let trimmed: Vec<(Audio, Pause)> = parts
        .iter()
        .map(|(a, p)| {
            (
                trim_silence(a, opts.silence_threshold_db, opts.trim_padding),
                *p,
            )
        })
        .collect();
    ensure!(
        trimmed.iter().all(|(a, _)| !a.samples.is_empty()),
        "a chunk contains no audible speech"
    );
    let mut levels: Vec<f32> = trimmed
        .iter()
        .filter_map(|(a, _)| active_rms(a, opts.silence_threshold_db))
        .collect();
    levels.sort_by(|a, b| a.total_cmp(b));
    let target = levels.get(levels.len() / 2).copied();

    let fade_len = (opts.fade * sr as f64) as usize;
    let mut out: Vec<f32> = Vec::new();
    for (i, (audio, pause)) in trimmed.iter().enumerate() {
        let mut samples = audio.samples.clone();
        if let (Some(target), Some(level)) = (target, active_rms(audio, opts.silence_threshold_db))
        {
            let gain_db =
                (20.0 * (target / level).log10()).clamp(-opts.max_gain_db, opts.max_gain_db);
            let g = db_to_amp(gain_db);
            samples.iter_mut().for_each(|s| *s *= g);
        }
        apply_fades(&mut samples, fade_len);
        out.extend_from_slice(&samples);
        if i + 1 < trimmed.len() {
            let gap = match pause {
                Pause::Paragraph => opts.paragraph_gap,
                Pause::Clause => opts.clause_gap,
                Pause::Sentence | Pause::End => opts.sentence_gap,
            };
            out.extend(std::iter::repeat_n(0.0, (gap * sr as f64) as usize));
        }
    }
    let peak = out.iter().fold(0.0f32, |m, s| m.max(s.abs()));
    let ceiling = db_to_amp(opts.peak_ceiling_db);
    if peak > ceiling {
        let g = ceiling / peak;
        out.iter_mut().for_each(|s| *s *= g);
    }
    Ok(Audio::new(out, sr))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tone(freq: f32, seconds: f64, sr: u32, amp: f32) -> Audio {
        let n = (seconds * sr as f64) as usize;
        Audio::new(
            (0..n)
                .map(|i| amp * (2.0 * std::f32::consts::PI * freq * i as f32 / sr as f32).sin())
                .collect(),
            sr,
        )
    }

    fn padded(core: &Audio, lead: f64, tail: f64) -> Audio {
        let mut s = vec![0.0; (lead * core.sample_rate as f64) as usize];
        s.extend_from_slice(&core.samples);
        s.extend(std::iter::repeat_n(
            0.0,
            (tail * core.sample_rate as f64) as usize,
        ));
        Audio::new(s, core.sample_rate)
    }

    #[test]
    fn wav_round_trip_both_formats() {
        let dir = tempfile::tempdir().unwrap();
        let a = tone(440.0, 0.25, 48_000, 0.5);
        for (fmt, tol) in [(WavFormat::Pcm16, 1e-4), (WavFormat::Float32, 1e-7)] {
            let p = dir.path().join(format!("{fmt:?}.wav"));
            write_wav(&p, &a, fmt).unwrap();
            assert!(!p.with_extension("wav.partial").exists());
            let b = read_wav(&p).unwrap();
            assert_eq!(b.sample_rate, 48_000);
            assert_eq!(b.samples.len(), a.samples.len());
            let err = a
                .samples
                .iter()
                .zip(&b.samples)
                .map(|(x, y)| (x - y).abs())
                .fold(0.0, f32::max);
            assert!(err < tol, "{fmt:?} err {err}");
        }
    }

    #[test]
    fn decode_wav_via_symphonia_downmixes_stereo() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("stereo.wav");
        let spec = hound::WavSpec {
            channels: 2,
            sample_rate: 22_050,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };
        let mut w = hound::WavWriter::create(&p, spec).unwrap();
        for _ in 0..1000 {
            w.write_sample(16384i16).unwrap();
            w.write_sample(0i16).unwrap();
        }
        w.finalize().unwrap();
        let a = decode_file(&p).unwrap();
        assert_eq!(a.sample_rate, 22_050);
        assert_eq!(a.samples.len(), 1000);
        assert!((a.samples[10] - 0.25).abs() < 1e-3);
    }

    #[test]
    fn decode_rejects_non_audio() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("fake.mp3");
        std::fs::write(&p, b"definitely not audio").unwrap();
        assert!(decode_file(&p).is_err());
    }

    #[test]
    fn resample_preserves_duration_and_tone() {
        let a = tone(440.0, 1.0, 44_100, 0.5);
        let b = resample(&a, 16_000);
        assert_eq!(b.sample_rate, 16_000);
        assert!((b.samples.len() as i64 - 16_000).abs() <= 1);
        // Count zero crossings in the middle: 440 Hz → ~880 per second.
        let mid = &b.samples[2000..14000];
        let crossings = mid
            .windows(2)
            .filter(|w| w[0].signum() != w[1].signum())
            .count();
        let expected = 880.0 * (12000.0 / 16000.0);
        assert!(
            (crossings as f64 - expected).abs() < 6.0,
            "crossings {crossings}"
        );
        let peak = mid.iter().fold(0.0f32, |m, s| m.max(s.abs()));
        assert!((peak - 0.5).abs() < 0.02, "peak {peak}");
    }

    #[test]
    fn resample_removes_content_above_new_nyquist() {
        let a = tone(12_000.0, 0.5, 48_000, 0.5); // above 8 kHz Nyquist of 16 kHz
        let b = resample(&a, 16_000);
        let rms = (b.samples[500..7500].iter().map(|s| s * s).sum::<f32>() / 7000.0).sqrt();
        assert!(rms < 0.01, "aliased energy rms {rms}");
    }

    #[test]
    fn trim_silence_keeps_speech_with_padding() {
        let core = tone(200.0, 0.5, 48_000, 0.3);
        let a = padded(&core, 0.4, 0.6);
        let t = trim_silence(&a, -45.0, 0.04);
        let d = t.duration_seconds();
        assert!((d - 0.58).abs() < 0.03, "duration {d}");
        assert!(
            trim_silence(&Audio::silence(1.0, 48_000), -45.0, 0.04)
                .samples
                .is_empty()
        );
    }

    #[test]
    fn stitch_inserts_gaps_matches_loudness_and_limits_peaks() {
        let sr = 48_000;
        let loud = padded(&tone(200.0, 0.5, sr, 0.9), 0.2, 0.2);
        let quiet = padded(&tone(200.0, 0.5, sr, 0.45), 0.2, 0.2);
        let opts = StitchOptions::default();
        let out = stitch(&[(loud, Pause::Paragraph), (quiet, Pause::End)], &opts).unwrap();
        let expected = 2.0 * (0.5 + 2.0 * opts.trim_padding) + opts.paragraph_gap;
        assert!(
            (out.duration_seconds() - expected).abs() < 0.03,
            "dur {}",
            out.duration_seconds()
        );
        let peak = out.samples.iter().fold(0.0f32, |m, s| m.max(s.abs()));
        assert!(peak <= db_to_amp(-1.0) + 1e-4);
        // Both halves end up at similar loudness (6 dB apart originally).
        let half = out.samples.len() / 2;
        let l1 = active_rms(&Audio::new(out.samples[..half].to_vec(), sr), -45.0).unwrap();
        let l2 = active_rms(&Audio::new(out.samples[half..].to_vec(), sr), -45.0).unwrap();
        assert!((20.0 * (l1 / l2).log10()).abs() < 1.0, "levels {l1} {l2}");
        // No click: first and last samples are faded to ~0.
        assert!(out.samples[0].abs() < 1e-3 && out.samples.last().unwrap().abs() < 1e-3);
    }

    #[test]
    fn stitch_rejects_mismatched_rates_and_empty() {
        assert!(stitch(&[], &StitchOptions::default()).is_err());
        let a = tone(200.0, 0.1, 48_000, 0.3);
        let b = tone(200.0, 0.1, 16_000, 0.3);
        assert!(
            stitch(
                &[(a, Pause::Sentence), (b, Pause::End)],
                &StitchOptions::default()
            )
            .is_err()
        );
    }

    #[test]
    fn stitch_rejects_invalid_settings_before_processing() {
        // A single part avoids attempting an enormous allocation in the old
        // implementation while proving that even unused gaps need validation.
        let part = [(tone(200.0, 0.1, 48_000, 0.3), Pause::End)];
        let invalid = [
            StitchOptions {
                sentence_gap: f64::INFINITY,
                ..Default::default()
            },
            StitchOptions {
                paragraph_gap: f64::NAN,
                ..Default::default()
            },
            StitchOptions {
                clause_gap: -0.1,
                ..Default::default()
            },
            StitchOptions {
                sentence_gap: 60.1,
                ..Default::default()
            },
            StitchOptions {
                fade: 1.1,
                ..Default::default()
            },
            StitchOptions {
                fade: f64::NAN,
                ..Default::default()
            },
            StitchOptions {
                trim_padding: -1.0,
                ..Default::default()
            },
            StitchOptions {
                trim_padding: 1.1,
                ..Default::default()
            },
            StitchOptions {
                max_gain_db: f32::NAN,
                ..Default::default()
            },
            StitchOptions {
                max_gain_db: -1.0,
                ..Default::default()
            },
            StitchOptions {
                max_gain_db: 30.1,
                ..Default::default()
            },
            StitchOptions {
                peak_ceiling_db: 0.1,
                ..Default::default()
            },
            StitchOptions {
                peak_ceiling_db: f32::NEG_INFINITY,
                ..Default::default()
            },
            StitchOptions {
                silence_threshold_db: 1.0,
                ..Default::default()
            },
            StitchOptions {
                silence_threshold_db: f32::NAN,
                ..Default::default()
            },
        ];
        for options in invalid {
            assert!(
                stitch(&part, &options).is_err(),
                "accepted invalid options {options:?}"
            );
        }
    }

    #[test]
    fn stitch_rejects_invalid_audio() {
        for audio in [
            Audio::new(vec![0.25f32; 8], 0),
            Audio::new(vec![f32::NAN; 8], 48_000),
            Audio::new(vec![f32::INFINITY; 8], 48_000),
            Audio::new(vec![0.25f32; 8], u32::MAX),
            Audio::new(vec![], 48_000),
            Audio::new(vec![0.0; 8], 48_000),
        ] {
            assert!(stitch(&[(audio, Pause::End)], &StitchOptions::default()).is_err());
        }
    }

    #[test]
    fn stitch_accepts_valid_boundary_settings() {
        let options = StitchOptions {
            sentence_gap: 60.0,
            paragraph_gap: 60.0,
            clause_gap: 0.0,
            fade: 1.0,
            trim_padding: 1.0,
            max_gain_db: 30.0,
            peak_ceiling_db: 0.0,
            silence_threshold_db: -45.0,
        };
        options.validate().unwrap();
        let zero = StitchOptions {
            sentence_gap: 0.0,
            paragraph_gap: 0.0,
            fade: 0.0,
            trim_padding: 0.0,
            max_gain_db: 0.0,
            ..options
        };
        let out = stitch(&[(tone(200.0, 0.1, 48_000, 0.3), Pause::End)], &zero).unwrap();
        assert!(!out.samples.is_empty());
        assert!(out.samples.iter().all(|s| s.is_finite()));
    }
}
