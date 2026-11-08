//! One text-to-speech job: plan chunks, synthesize each (retrying runaway or
//! truncated generations), stitch, write the final WAV atomically, and always
//! remove intermediate files.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;

use anyhow::{Context, Result, bail};
use serde::Serialize;

use crate::audio::{Audio, StitchOptions, WavFormat, read_wav, stitch, write_wav};
use crate::engine::{Cancelled, Engine, EngineEvent, GenerationParams};
use crate::eta::{Calibration, Estimator};
use crate::text::{Chunk, ChunkOptions, Pause, chunk_text};
use crate::voice::{PreparedVoice, prepare_reference};

/// Where the text comes from is the caller's concern; the job gets the text.
#[derive(Debug, Clone)]
pub struct JobSpec {
    pub text: String,
    pub voice: Option<PathBuf>,
    pub voice_transcript: Option<String>,
    pub output: PathBuf,
    pub overwrite: bool,
    pub chunking: ChunkOptions,
    pub params: GenerationParams,
    pub stitching: StitchOptions,
    pub wav_format: WavFormat,
    /// Copy chunk WAVs here before cleanup (debugging); `None` discards them.
    pub keep_parts: Option<PathBuf>,
    pub retry: RetryPolicy,
    /// Parent of the private work directory (default: the system temp dir).
    pub work_root: Option<PathBuf>,
}

#[derive(Debug, Clone, Copy)]
pub struct RetryPolicy {
    /// Extra attempts for a chunk whose duration is implausible for its text.
    pub max_retries: usize,
    /// Audio longer than `max_ratio` × the expected duration is a runaway.
    pub max_ratio: f64,
    /// Audio shorter than `min_ratio` × the expected duration is truncated.
    pub min_ratio: f64,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            max_retries: 2,
            max_ratio: 2.6,
            min_ratio: 0.3,
        }
    }
}

/// Expected speaking time for `text` at a natural pace (seconds).
///
/// Latin-script text is ~15 characters/second; CJK/Thai-like scripts that pack
/// a syllable into one character are ~4.5 characters/second.
pub fn expected_speech_seconds(text: &str) -> f64 {
    let mut secs = 0.0f64;
    for c in text.chars() {
        if c.is_whitespace() {
            secs += 1.0 / 15.0;
        } else if is_dense_script(c) {
            secs += 1.0 / 4.5;
        } else if c.is_alphanumeric() {
            secs += 1.0 / 15.0;
        }
    }
    secs.max(0.6)
}

fn is_dense_script(c: char) -> bool {
    matches!(c as u32,
        0x3040..=0x30FF   // kana
        | 0x3400..=0x4DBF | 0x4E00..=0x9FFF | 0xF900..=0xFAFF // CJK
        | 0xAC00..=0xD7AF // Hangul
        | 0x0E00..=0x0E7F // Thai
    )
}

/// Upper bound on generated audio per chunk passed to the engine.
pub fn max_audio_seconds_for(text: &str, policy: &RetryPolicy) -> f64 {
    expected_speech_seconds(text) * (policy.max_ratio + 1.0) + 6.0
}

#[derive(Debug, Clone, Serialize)]
pub struct ChunkReport {
    pub index: usize,
    pub chars: usize,
    pub attempts: usize,
    pub audio_seconds: f64,
    pub compute_seconds: f64,
    pub accepted_ratio: f64,
    pub warning: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct JobReport {
    pub output: PathBuf,
    pub engine: String,
    pub chunks: Vec<ChunkReport>,
    pub audio_seconds: f64,
    pub compute_seconds: f64,
    pub load_seconds: f64,
    pub real_time_factor: f64,
    pub voice_warnings: Vec<String>,
    pub parts_directory: Option<PathBuf>,
}

/// Progress callbacks; implemented by the CLI progress bar and the GUI.
pub trait ProgressSink {
    fn loading(&self, _estimate_secs: f64) {}
    fn loaded(&self, _secs: f64) {}
    fn planned(&self, _chunks: &[Chunk], _estimate_secs: f64) {}
    fn chunk_started(&self, _chunk: &Chunk, _attempt: usize) {}
    /// Called frequently during a chunk.
    fn tick(&self, _chunk: &Chunk, _done_chars: usize, _audio_seconds: f64, _eta_secs: f64) {}
    fn chunk_finished(&self, _report: &ChunkReport, _done_chars: usize, _eta_secs: f64) {}
    fn retrying(&self, _chunk: &Chunk, _reason: &str) {}
    fn stitching(&self) {}
    fn warning(&self, _message: &str) {}
}

pub struct NoProgress;
impl ProgressSink for NoProgress {}

const WORK_PREFIX: &str = "voxcpm-tts-job-";

/// Remove job directories left behind by processes that no longer exist
/// (e.g. after a crash or SIGKILL).
pub fn sweep_stale_work_dirs(tmp_root: &Path) {
    let Ok(entries) = std::fs::read_dir(tmp_root) else {
        return;
    };
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        if !name.starts_with(WORK_PREFIX) {
            continue;
        }
        let pid = std::fs::read_to_string(entry.path().join("owner.pid"))
            .ok()
            .and_then(|s| s.trim().parse::<i32>().ok());
        let alive = pid.is_some_and(process_alive);
        if !alive {
            let _ = std::fs::remove_dir_all(entry.path());
        }
    }
}

fn process_alive(pid: i32) -> bool {
    std::process::Command::new("kill")
        .args(["-0", &pid.to_string()])
        .stderr(std::process::Stdio::null())
        .status()
        .is_ok_and(|s| s.success())
}

fn check_cancel(cancel: &AtomicBool) -> Result<()> {
    if cancel.load(Ordering::SeqCst) {
        Err(Cancelled.into())
    } else {
        Ok(())
    }
}

/// Validate the destination before any expensive work.
pub fn check_output(output: &Path, overwrite: bool) -> Result<()> {
    if output
        .extension()
        .and_then(|e| e.to_str())
        .map(str::to_ascii_lowercase)
        .as_deref()
        != Some("wav")
    {
        bail!("output must be a .wav file: {}", output.display());
    }
    if output.exists() && !overwrite {
        bail!(
            "{} already exists (pass --force to overwrite)",
            output.display()
        );
    }
    if let Some(parent) = output.parent().filter(|p| !p.as_os_str().is_empty())
        && !parent.is_dir()
    {
        bail!("output directory does not exist: {}", parent.display());
    }
    Ok(())
}

/// JSON reports use their own extension and never replace pre-existing files.
/// This prevents a report path from destroying the WAV or a reference clip.
pub fn check_report(path: &Path) -> Result<()> {
    if path
        .extension()
        .and_then(|s| s.to_str())
        .is_none_or(|s| !s.eq_ignore_ascii_case("json"))
    {
        bail!("report destination must be a .json file");
    }
    if path.exists() {
        bail!(
            "report already exists; choose a new report destination: {}",
            path.display()
        );
    }
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    if !parent.is_dir() {
        bail!("report directory does not exist: {}", parent.display());
    }
    Ok(())
}

pub fn write_report(path: &Path, report: &JobReport) -> Result<()> {
    use std::io::Write;
    check_report(path)?;
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let mut tmp = tempfile::Builder::new()
        .prefix(".voxcpm-report-")
        .tempfile_in(parent)?;
    tmp.write_all(serde_json::to_string_pretty(report)?.as_bytes())?;
    tmp.persist_noclobber(path).map_err(|e| e.error)?;
    Ok(())
}

/// Run a job to completion. Intermediate files live in a private temporary
/// directory that is removed on success, error and cancellation alike.
pub fn run_job(
    engine: &mut dyn Engine,
    spec: &JobSpec,
    progress: &dyn ProgressSink,
    cancel: &AtomicBool,
    calibration: &mut Calibration,
) -> Result<JobReport> {
    spec.stitching.validate()?;
    if spec.chunking.max_chars == 0
        || !spec.params.cfg_value.is_finite()
        || !(1.0..=4.0).contains(&spec.params.cfg_value)
        || spec.params.inference_timesteps == 0
        || spec.params.inference_timesteps > 100
    {
        bail!(
            "invalid generation settings: max chars > 0, cfg in 1..=4, and timesteps in 1..=100 required"
        );
    }
    check_output(&spec.output, spec.overwrite)?;
    if let Some(source) = &spec.voice {
        use std::os::unix::fs::MetadataExt;
        if let (Ok(input), Ok(output)) =
            (std::fs::metadata(source), std::fs::metadata(&spec.output))
            && input.dev() == output.dev()
            && input.ino() == output.ino()
        {
            bail!("output must not replace the original reference voice");
        }
    }
    let chunks = chunk_text(&spec.text, spec.chunking);
    if chunks.is_empty() {
        bail!("the text is empty");
    }

    let tmp_root = spec.work_root.clone().unwrap_or_else(std::env::temp_dir);
    std::fs::create_dir_all(&tmp_root)?;
    sweep_stale_work_dirs(&tmp_root);
    let work = tempfile::Builder::new()
        .prefix(WORK_PREFIX)
        .tempdir_in(&tmp_root)?;
    std::fs::write(
        work.path().join("owner.pid"),
        std::process::id().to_string(),
    )?;

    let voice: Option<PreparedVoice> = match &spec.voice {
        Some(path) => Some(prepare_reference(
            path,
            spec.voice_transcript.clone(),
            work.path(),
        )?),
        None => None,
    };
    let voice_warnings = voice
        .as_ref()
        .map(|v| v.warnings.clone())
        .unwrap_or_default();
    for w in &voice_warnings {
        progress.warning(w);
    }

    let key = Calibration::key(engine.name(), spec.params.inference_timesteps);
    let prior = calibration.get(&key);
    let total_chars: usize = chunks.iter().map(Chunk::char_len).sum();
    let mut estimator = Estimator::new(prior, total_chars);

    let mut load_seconds = 0.0;
    if !engine.is_loaded() {
        progress.loading(prior.load_secs);
        check_cancel(cancel)?;
        load_seconds = engine.load_cancellable(cancel)?;
        calibration.record_load(&key, load_seconds);
        progress.loaded(load_seconds);
    }
    progress.planned(&chunks, estimator.remaining_secs(0, 0.0));

    let job_start = Instant::now();
    let mut reports = Vec::new();
    let mut part_paths: Vec<(PathBuf, Pause)> = Vec::new();
    let mut done_chars = 0usize;
    for chunk in &chunks {
        check_cancel(cancel)?;
        let chunk_start = Instant::now();
        let expected = expected_speech_seconds(&chunk.text);
        let mut best: Option<(Audio, f64)> = None;
        let mut attempts = 0;
        let mut warning = None;
        for attempt in 0..=spec.retry.max_retries {
            attempts = attempt + 1;
            progress.chunk_started(chunk, attempts);
            let params = GenerationParams {
                seed: spec
                    .params
                    .seed
                    .map(|s| s.wrapping_add((chunk.index as u64) * 1000 + attempt as u64)),
                max_audio_seconds: max_audio_seconds_for(&chunk.text, &spec.retry),
                ..spec.params
            };
            let mut on_event = |e: EngineEvent| match e {
                EngineEvent::Generated { audio_seconds } => {
                    let elapsed = chunk_start.elapsed().as_secs_f64();
                    let eta = estimator.remaining_secs(chunk.char_len(), elapsed);
                    let within =
                        ((audio_seconds / expected).min(0.99) * chunk.char_len() as f64) as usize;
                    progress.tick(chunk, done_chars + within, audio_seconds, eta);
                }
            };
            let result = engine.synthesize(
                &chunk.text,
                voice.as_ref().map(|v| &v.prompt),
                &params,
                &mut on_event,
                cancel,
            );
            let result = result.and_then(|audio| {
                if audio.samples.is_empty()
                    || audio.sample_rate != 48_000
                    || audio.samples.iter().any(|s| !s.is_finite())
                    || crate::audio::active_rms(&audio, -45.0).is_none()
                {
                    return Err(crate::engine::BadGeneration(
                        "engine produced empty, silent, nonfinite, or non-48-kHz audio".into(),
                    )
                    .into());
                }
                Ok(audio)
            });
            let audio = match result {
                Err(e)
                    if e.is::<crate::engine::BadGeneration>()
                        && attempt < spec.retry.max_retries =>
                {
                    progress.retrying(chunk, &e.to_string());
                    continue;
                }
                other => other.with_context(|| {
                    format!("synthesizing chunk {} of {}", chunk.index, chunks.len())
                })?,
            };
            let ratio =
                crate::audio::trim_silence(&audio, -45.0, 0.0).duration_seconds() / expected;
            let plausible = ratio >= spec.retry.min_ratio && ratio <= spec.retry.max_ratio;
            let better = best
                .as_ref()
                .is_none_or(|(_, r)| (ratio.ln()).abs() < (r.ln()).abs());
            if better {
                best = Some((audio, ratio));
            }
            if plausible {
                break;
            }
            let reason = if ratio > spec.retry.max_ratio {
                format!("audio {ratio:.1}× longer than expected (possible runaway)")
            } else {
                format!("audio only {ratio:.2}× the expected length (possible truncation)")
            };
            if attempt < spec.retry.max_retries {
                progress.retrying(chunk, &reason);
            } else {
                let msg = format!(
                    "chunk {}: kept best of {attempts} attempts; {reason}",
                    chunk.index
                );
                progress.warning(&msg);
                warning = Some(msg);
            }
        }
        let (audio, ratio) = best.expect("at least one attempt");
        let part = work.path().join(format!("part-{:04}.wav", chunk.index));
        write_wav(&part, &audio, WavFormat::Float32)?;
        part_paths.push((part, chunk.pause_after));
        let compute = chunk_start.elapsed().as_secs_f64();
        estimator.record_chunk(chunk.char_len(), compute);
        done_chars += chunk.char_len();
        let report = ChunkReport {
            index: chunk.index,
            chars: chunk.char_len(),
            attempts,
            audio_seconds: audio.duration_seconds(),
            compute_seconds: compute,
            accepted_ratio: ratio,
            warning,
        };
        progress.chunk_finished(&report, done_chars, estimator.remaining_secs(0, 0.0));
        reports.push(report);
    }

    check_cancel(cancel)?;
    progress.stitching();
    let parts: Vec<(Audio, Pause)> = part_paths
        .iter()
        .map(|(p, pause)| Ok((read_wav(p)?, *pause)))
        .collect::<Result<_>>()?;
    let joined = stitch(&parts, &spec.stitching)?;
    crate::audio::write_wav_output(&spec.output, &joined, spec.wav_format, spec.overwrite)?;

    let mut parts_directory = None;
    if let Some(dir) = &spec.keep_parts {
        std::fs::create_dir_all(dir)?;
        let run = tempfile::Builder::new()
            .prefix("voxcpm-parts-")
            .tempdir_in(dir)?;
        for (p, _) in &part_paths {
            std::fs::copy(p, run.path().join(p.file_name().expect("part file name")))?;
        }
        parts_directory = Some(run.keep());
    }
    work.close().context("removing intermediate files")?;

    if let Some(rate) = estimator.observed_secs_per_char() {
        calibration.record_rate(&key, rate);
    }
    calibration.save();

    let compute_seconds = job_start.elapsed().as_secs_f64();
    let audio_seconds = joined.duration_seconds();
    Ok(JobReport {
        output: spec.output.clone(),
        engine: engine.name().to_string(),
        chunks: reports,
        audio_seconds,
        compute_seconds,
        load_seconds,
        real_time_factor: compute_seconds / audio_seconds.max(1e-9),
        voice_warnings,
        parts_directory,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audio::Audio;
    use crate::engine::VoicePrompt;
    use std::cell::RefCell;

    /// Produces a tone whose length is proportional to the text, optionally
    /// misbehaving on chosen calls.
    struct MockEngine {
        loaded: bool,
        calls: RefCell<Vec<(String, Option<u64>, bool)>>,
        runaway_calls: Vec<usize>,
        cancel_on_call: Option<usize>,
        bad_calls: Vec<usize>,
    }

    impl MockEngine {
        fn new() -> Self {
            Self {
                loaded: false,
                calls: RefCell::new(vec![]),
                runaway_calls: vec![],
                cancel_on_call: None,
                bad_calls: vec![],
            }
        }
    }

    impl Engine for MockEngine {
        fn name(&self) -> &'static str {
            "mock"
        }
        fn load(&mut self) -> Result<f64> {
            self.loaded = true;
            Ok(0.01)
        }
        fn is_loaded(&self) -> bool {
            self.loaded
        }
        fn synthesize(
            &mut self,
            text: &str,
            voice: Option<&VoicePrompt>,
            params: &GenerationParams,
            on_event: &mut dyn FnMut(EngineEvent),
            cancel: &AtomicBool,
        ) -> Result<Audio> {
            let n = self.calls.borrow().len();
            self.calls
                .borrow_mut()
                .push((text.to_string(), params.seed, voice.is_some()));
            if self.bad_calls.contains(&n) {
                return Err(crate::engine::BadGeneration("reached cap".into()).into());
            }
            if self.cancel_on_call == Some(n) {
                cancel.store(true, Ordering::SeqCst);
                return Err(Cancelled.into());
            }
            let mut secs = expected_speech_seconds(text);
            if self.runaway_calls.contains(&n) {
                secs *= 5.0;
            }
            assert!(secs <= params.max_audio_seconds + 1e-9 || self.runaway_calls.contains(&n));
            on_event(EngineEvent::Generated {
                audio_seconds: secs / 2.0,
            });
            let sr = 48_000;
            let len = (secs * sr as f64) as usize;
            Ok(Audio::new(
                (0..len).map(|i| 0.3 * (i as f32 * 0.05).sin()).collect(),
                sr,
            ))
        }
        fn unload(&mut self) -> Result<()> {
            self.loaded = false;
            Ok(())
        }
    }

    fn spec(dir: &Path, text: &str) -> JobSpec {
        JobSpec {
            text: text.to_string(),
            voice: None,
            voice_transcript: None,
            output: dir.join("out.wav"),
            overwrite: false,
            chunking: ChunkOptions {
                max_chars: 120,
                min_chars: 40,
            },
            params: GenerationParams {
                cfg_value: 2.0,
                inference_timesteps: 10,
                seed: Some(7),
                max_audio_seconds: 0.0,
            },
            stitching: StitchOptions::default(),
            wav_format: WavFormat::Pcm16,
            keep_parts: None,
            retry: RetryPolicy::default(),
            work_root: Some(dir.join("work")),
        }
    }

    fn job_dirs(s: &JobSpec) -> Vec<PathBuf> {
        std::fs::read_dir(s.work_root.as_ref().unwrap())
            .unwrap()
            .flatten()
            .filter(|e| e.file_name().to_string_lossy().starts_with(WORK_PREFIX))
            .map(|e| e.path())
            .collect()
    }

    const SCRIPT: &str = "This is the first sentence of the script. Here is a second sentence that adds more words. \
        A third one keeps going for a while longer.\n\nA new paragraph begins here with fresh content. And it ends.";

    #[test]
    fn multi_chunk_job_writes_one_wav_and_cleans_up() {
        let dir = tempfile::tempdir().unwrap();
        let mut engine = MockEngine::new();
        let mut cal = Calibration::load(&dir.path().join("cal.json"));
        let s = spec(dir.path(), SCRIPT);
        std::fs::create_dir(s.work_root.as_ref().unwrap()).unwrap();
        let report = run_job(
            &mut engine,
            &s,
            &NoProgress,
            &AtomicBool::new(false),
            &mut cal,
        )
        .unwrap();
        assert!(report.chunks.len() >= 3, "{report:?}");
        let out = read_wav(&s.output).unwrap();
        assert_eq!(out.sample_rate, 48_000);
        let speech: f64 = report.chunks.iter().map(|c| c.audio_seconds).sum();
        assert!(out.duration_seconds() > speech * 0.9);
        // Every chunk was sent once, in order, covering the whole text.
        let calls = engine.calls.borrow();
        let sent: Vec<&str> = calls
            .iter()
            .flat_map(|(t, _, _)| t.split_whitespace())
            .collect();
        let original: Vec<&str> = SCRIPT.split_whitespace().collect();
        assert_eq!(sent, original);
        assert!(job_dirs(&s).is_empty(), "intermediate files left behind");
        assert!(dir.path().join("cal.json").exists());
        assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 3); // out.wav, cal.json, empty work root
    }

    #[test]
    fn runaway_chunk_is_retried_with_a_new_seed() {
        let dir = tempfile::tempdir().unwrap();
        let mut engine = MockEngine::new();
        engine.runaway_calls = vec![0];
        let mut cal = Calibration::default();
        let s = spec(dir.path(), "Just one short sentence here.");
        let report = run_job(
            &mut engine,
            &s,
            &NoProgress,
            &AtomicBool::new(false),
            &mut cal,
        )
        .unwrap();
        assert_eq!(report.chunks[0].attempts, 2);
        let calls = engine.calls.borrow();
        assert_ne!(calls[0].1, calls[1].1);
        assert!(report.chunks[0].warning.is_none());
    }

    #[test]
    fn capped_generation_is_retried_and_exhaustion_leaves_no_output() {
        let dir = tempfile::tempdir().unwrap();
        let s = spec(dir.path(), "Just one short sentence here.");
        let mut engine = MockEngine::new();
        engine.bad_calls = vec![0];
        let mut cal = Calibration::default();
        let report = run_job(
            &mut engine,
            &s,
            &NoProgress,
            &AtomicBool::new(false),
            &mut cal,
        )
        .unwrap();
        assert_eq!(report.chunks[0].attempts, 2);
        let calls = engine.calls.borrow();
        assert_ne!(calls[0].1, calls[1].1);
        drop(calls);
        let mut s = spec(dir.path(), "Just one short sentence here.");
        s.output = dir.path().join("exhausted.wav");
        let mut engine = MockEngine::new();
        engine.bad_calls = vec![0, 1, 2];
        assert!(
            run_job(
                &mut engine,
                &s,
                &NoProgress,
                &AtomicBool::new(false),
                &mut cal
            )
            .is_err()
        );
        assert_eq!(engine.calls.borrow().len(), 3);
        assert!(!s.output.exists());
    }

    #[test]
    fn persistent_runaway_keeps_best_attempt_with_warning() {
        let dir = tempfile::tempdir().unwrap();
        let mut engine = MockEngine::new();
        engine.runaway_calls = vec![0, 1, 2];
        let mut cal = Calibration::default();
        let s = spec(dir.path(), "Just one short sentence here.");
        let report = run_job(
            &mut engine,
            &s,
            &NoProgress,
            &AtomicBool::new(false),
            &mut cal,
        )
        .unwrap();
        assert_eq!(report.chunks[0].attempts, 3);
        assert!(report.chunks[0].warning.is_some());
        assert!(s.output.exists());
    }

    #[test]
    fn cancellation_leaves_no_output_and_no_intermediates() {
        let dir = tempfile::tempdir().unwrap();
        let mut engine = MockEngine::new();
        engine.cancel_on_call = Some(1);
        let mut cal = Calibration::default();
        let s = spec(dir.path(), SCRIPT);
        std::fs::create_dir(s.work_root.as_ref().unwrap()).unwrap();
        let cancel = AtomicBool::new(false);
        let err = run_job(&mut engine, &s, &NoProgress, &cancel, &mut cal).unwrap_err();
        assert!(err.chain().any(|e| e.is::<Cancelled>()), "{err:?}");
        assert!(!s.output.exists());
        assert!(job_dirs(&s).is_empty());
    }

    #[test]
    fn existing_output_is_refused_unless_overwrite() {
        let dir = tempfile::tempdir().unwrap();
        let mut s = spec(dir.path(), "Hello world.");
        std::fs::write(&s.output, b"keep me").unwrap();
        let mut engine = MockEngine::new();
        let mut cal = Calibration::default();
        assert!(
            run_job(
                &mut engine,
                &s,
                &NoProgress,
                &AtomicBool::new(false),
                &mut cal
            )
            .is_err()
        );
        assert_eq!(std::fs::read(&s.output).unwrap(), b"keep me");
        assert!(
            engine.calls.borrow().is_empty(),
            "no synthesis before output validation"
        );
        s.overwrite = true;
        run_job(
            &mut engine,
            &s,
            &NoProgress,
            &AtomicBool::new(false),
            &mut cal,
        )
        .unwrap();
        assert!(read_wav(&s.output).is_ok());
    }

    #[test]
    fn reference_cannot_be_overwritten_even_with_force() {
        let dir = tempfile::tempdir().unwrap();
        let mut s = spec(dir.path(), "Hello world.");
        let original = Audio::new(
            (0..144_000)
                .map(|i| 0.2 * (i as f32 * 0.04).sin())
                .collect(),
            48_000,
        );
        write_wav(&s.output, &original, WavFormat::Pcm16).unwrap();
        let bytes = std::fs::read(&s.output).unwrap();
        s.voice = Some(s.output.clone());
        s.overwrite = true;
        let mut engine = MockEngine::new();
        assert!(
            run_job(
                &mut engine,
                &s,
                &NoProgress,
                &AtomicBool::new(false),
                &mut Calibration::default()
            )
            .is_err()
        );
        assert_eq!(std::fs::read(&s.output).unwrap(), bytes);
        assert!(engine.calls.borrow().is_empty());
    }

    #[test]
    fn rejects_non_wav_output_and_empty_text() {
        let dir = tempfile::tempdir().unwrap();
        let mut s = spec(dir.path(), "Hello.");
        s.output = dir.path().join("out.mp3");
        let mut engine = MockEngine::new();
        let mut cal = Calibration::default();
        assert!(
            run_job(
                &mut engine,
                &s,
                &NoProgress,
                &AtomicBool::new(false),
                &mut cal
            )
            .is_err()
        );
        let s = spec(dir.path(), "   \n\n ");
        assert!(
            run_job(
                &mut engine,
                &s,
                &NoProgress,
                &AtomicBool::new(false),
                &mut cal
            )
            .is_err()
        );
    }

    #[test]
    fn keep_parts_copies_chunk_wavs() {
        let dir = tempfile::tempdir().unwrap();
        let mut s = spec(dir.path(), SCRIPT);
        s.keep_parts = Some(dir.path().join("parts"));
        let mut engine = MockEngine::new();
        let mut cal = Calibration::default();
        let report = run_job(
            &mut engine,
            &s,
            &NoProgress,
            &AtomicBool::new(false),
            &mut cal,
        )
        .unwrap();
        assert_eq!(
            std::fs::read_dir(report.parts_directory.as_ref().unwrap())
                .unwrap()
                .count(),
            report.chunks.len()
        );
        assert_ne!(report.parts_directory.as_deref(), s.keep_parts.as_deref());
    }

    #[test]
    fn reports_and_retained_parts_do_not_clobber_user_files() {
        let dir = tempfile::tempdir().unwrap();
        let mut s = spec(dir.path(), "A short sentence with enough words to speak.");
        let parts = dir.path().join("parts");
        std::fs::create_dir(&parts).unwrap();
        let existing = parts.join("part-0001.wav");
        std::fs::write(&existing, b"original reference").unwrap();
        s.keep_parts = Some(parts);
        let mut engine = MockEngine::new();
        let report = run_job(
            &mut engine,
            &s,
            &NoProgress,
            &AtomicBool::new(false),
            &mut Calibration::default(),
        )
        .unwrap();
        assert_eq!(std::fs::read(existing).unwrap(), b"original reference");
        let output_before = std::fs::read(&s.output).unwrap();
        assert!(write_report(&s.output, &report).is_err());
        assert_eq!(std::fs::read(&s.output).unwrap(), output_before);
        let path = dir.path().join("report.json");
        write_report(&path, &report).unwrap();
        let json = std::fs::read(&path).unwrap();
        assert!(write_report(&path, &report).is_err());
        assert_eq!(std::fs::read(path).unwrap(), json);
    }

    #[test]
    fn invalid_job_settings_are_rejected_before_load() {
        let dir = tempfile::tempdir().unwrap();
        let mut s = spec(dir.path(), SCRIPT);
        let mut engine = MockEngine::new();
        s.stitching.sentence_gap = f64::INFINITY;
        assert!(
            run_job(
                &mut engine,
                &s,
                &NoProgress,
                &AtomicBool::new(false),
                &mut Calibration::default()
            )
            .is_err()
        );
        assert!(!engine.loaded);
        s.stitching = StitchOptions::default();
        s.params.cfg_value = f32::NAN;
        assert!(
            run_job(
                &mut engine,
                &s,
                &NoProgress,
                &AtomicBool::new(false),
                &mut Calibration::default()
            )
            .is_err()
        );
        assert!(!engine.loaded);
    }

    #[test]
    fn stale_work_dirs_of_dead_processes_are_swept() {
        let root = tempfile::tempdir().unwrap();
        let dead = root.path().join(format!("{WORK_PREFIX}dead"));
        std::fs::create_dir(&dead).unwrap();
        std::fs::write(dead.join("owner.pid"), "999999").unwrap();
        let live = root.path().join(format!("{WORK_PREFIX}live"));
        std::fs::create_dir(&live).unwrap();
        std::fs::write(live.join("owner.pid"), std::process::id().to_string()).unwrap();
        let other = root.path().join("unrelated");
        std::fs::create_dir(&other).unwrap();
        sweep_stale_work_dirs(root.path());
        assert!(!dead.exists());
        assert!(live.exists());
        assert!(other.exists());
    }

    #[test]
    fn expected_duration_accounts_for_dense_scripts() {
        let en = expected_speech_seconds("hello world this is a test");
        let zh = expected_speech_seconds("你好世界这是一个测试");
        assert!((en - 26.0 / 15.0).abs() < 1e-9);
        assert!((zh - 10.0 / 4.5).abs() < 1e-9);
    }
}
