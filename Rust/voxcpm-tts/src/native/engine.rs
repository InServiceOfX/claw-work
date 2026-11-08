//! Local candle/Metal adapter. Dropping the whole loaded state also drops the
//! device allocator; dropping model tensors alone can leave its buffer pool alive.

use std::ops::ControlFlow;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result, ensure};
use candle_core::{DType, Tensor};

use crate::audio::{Audio, read_wav, resample};
use crate::config::{OUTPUT_SAMPLE_RATE, model_snapshot_dir};
use crate::engine::{BadGeneration, Cancelled, Engine, EngineEvent, GenerationParams, VoicePrompt};

use super::model::{Conditioning, GenOptions, SECONDS_PER_PATCH, SeededNoise, VoxCpm2};

struct CachedVoice {
    path: PathBuf,
    modified: Option<SystemTime>,
    len: u64,
    transcript: Option<String>,
    reference: Tensor,
    prompt: Option<Tensor>,
}

pub struct NativeEngine {
    model_id: String,
    model: Option<VoxCpm2>,
    voice: Option<CachedVoice>,
}

impl NativeEngine {
    pub fn new(model_id: &str) -> Self {
        Self {
            model_id: model_id.into(),
            model: None,
            voice: None,
        }
    }

    fn check_cancel(cancel: &AtomicBool) -> Result<()> {
        if cancel.load(Ordering::SeqCst) {
            Err(Cancelled.into())
        } else {
            Ok(())
        }
    }

    fn prepare_voice(&mut self, voice: Option<&VoicePrompt>) -> Result<()> {
        let Some(voice) = voice else {
            self.voice = None;
            return Ok(());
        };
        let metadata = std::fs::metadata(&voice.audio_path)?;
        let modified = metadata.modified().ok();
        if self.voice.as_ref().is_some_and(|c| {
            c.path == voice.audio_path
                && c.modified == modified
                && c.len == metadata.len()
                && c.transcript == voice.transcript
        }) {
            return Ok(());
        }
        let audio = resample(&read_wav(&voice.audio_path)?, 16_000);
        let model = self.model.as_ref().context("model is not loaded")?;
        self.voice = Some(CachedVoice {
            path: voice.audio_path.clone(),
            modified,
            len: metadata.len(),
            transcript: voice.transcript.clone(),
            reference: model.encode_audio(&audio.samples, false)?,
            prompt: voice
                .transcript
                .as_ref()
                .map(|_| model.encode_audio(&audio.samples, true))
                .transpose()?,
        });
        Ok(())
    }
}

impl Engine for NativeEngine {
    fn name(&self) -> &'static str {
        "native"
    }

    fn load(&mut self) -> Result<f64> {
        objc2::rc::autoreleasepool(|_| {
        if self.model.is_some() {
            return Ok(0.0);
        }
        let start = Instant::now();
        let snapshot = match model_snapshot_dir(&self.model_id) {
            Some(path)
                if super::weights::shard_paths(&path)?
                    .iter()
                    .all(|p| p.is_file()) =>
            {
                path
            }
            _ => crate::hub::fetch_model(&self.model_id)?,
        };
        let device = super::weights::device()?;
        let model = VoxCpm2::load(&snapshot, DType::F32, &device)?;
        device.synchronize()?;
        self.model = Some(model);
        Ok(start.elapsed().as_secs_f64())
        })
    }

    fn is_loaded(&self) -> bool {
        self.model.is_some()
    }

    fn synthesize(
        &mut self,
        text: &str,
        voice: Option<&VoicePrompt>,
        params: &GenerationParams,
        on_event: &mut dyn FnMut(EngineEvent),
        cancel: &AtomicBool,
    ) -> Result<Audio> {
        objc2::rc::autoreleasepool(|_| {
        Self::check_cancel(cancel)?;
        ensure!(!text.trim().is_empty(), "the text is empty");
        ensure!(
            params.inference_timesteps > 0 && params.inference_timesteps <= 100,
            "diffusion steps must be in 1..=100"
        );
        ensure!(
            params.cfg_value.is_finite() && (1.0..=4.0).contains(&params.cfg_value),
            "cfg must be finite and in 1..=4"
        );
        ensure!(
            params.max_audio_seconds.is_finite() && params.max_audio_seconds >= 0.0,
            "max audio duration must be finite and nonnegative"
        );
        self.load()?;
        Self::check_cancel(cancel)?;
        self.prepare_voice(voice)?;
        Self::check_cancel(cancel)?;
        let model = self.model.as_ref().context("model is not loaded")?;
        let cond = Conditioning {
            reference: self.voice.as_ref().map(|v| &v.reference),
            prompt: self
                .voice
                .as_ref()
                .and_then(|v| v.prompt.as_ref().zip(v.transcript.as_deref())),
        };
        let max_patches = (params.max_audio_seconds > 0.0)
            .then(|| (params.max_audio_seconds / SECONDS_PER_PATCH).floor() as usize);
        if let Some(cap) = max_patches {
            ensure!(
                cap >= 4,
                "max audio duration must allow at least 0.64 seconds"
            );
        }
        let opts = GenOptions {
            inference_timesteps: params.inference_timesteps,
            cfg_value: params.cfg_value,
            max_patches,
            ..GenOptions::default()
        };
        let seed = params.seed.unwrap_or_else(|| {
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos() as u64
        });
        let mut noise = SeededNoise::new(seed);
        let generation = model.generate(text, &cond, &opts, &mut noise, &mut |patches| {
            on_event(EngineEvent::Generated {
                audio_seconds: patches as f64 * SECONDS_PER_PATCH,
            });
            if cancel.load(Ordering::SeqCst) {
                ControlFlow::Break(())
            } else {
                ControlFlow::Continue(())
            }
        })?;
        if !generation.stopped {
            return Err(BadGeneration(format!(
                "generation reached the safety cap after {} patches",
                generation.patches
            ))
            .into());
        }
        Self::check_cancel(cancel)?;
        let samples = model.decode(&generation)?;
        Self::check_cancel(cancel)?;
        ensure!(
            !samples.is_empty() && samples.iter().all(|s| s.is_finite()),
            "model produced empty or nonfinite audio"
        );
        Ok(Audio::new(samples, OUTPUT_SAMPLE_RATE))
        })
    }

    fn unload(&mut self) -> Result<()> {
        objc2::rc::autoreleasepool(|_| {
        let device = self.model.as_ref().map(|m| m.device().clone());
        if let Some(device) = &device { device.synchronize()?; }
        self.voice = None;
        self.model = None;
        // Flush once more after tensors drop to purge candle's unused pool.
        if let Some(device) = &device { device.synchronize()?; }
        Ok(())
        })
    }
}

impl Drop for NativeEngine {
    fn drop(&mut self) {
        let _ = self.unload();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cancellation_before_load_and_idempotent_unload() {
        let mut engine = NativeEngine::new(crate::config::MODEL_ID);
        assert!(!engine.is_loaded());
        let params = GenerationParams {
            cfg_value: 2.0,
            inference_timesteps: 10,
            seed: Some(1),
            max_audio_seconds: 5.0,
        };
        let err = engine
            .synthesize("Hello.", None, &params, &mut |_| {}, &AtomicBool::new(true))
            .unwrap_err();
        assert!(err.is::<Cancelled>());
        assert!(!engine.is_loaded());
        engine.unload().unwrap();
        engine.unload().unwrap();
    }

    #[test]
    fn invalid_settings_do_not_load_weights() {
        let mut engine = NativeEngine::new(crate::config::MODEL_ID);
        for (text, cfg, steps) in [("  ", 2.0, 10), ("Hello", f32::NAN, 10), ("Hello", 2.0, 0)] {
            let params = GenerationParams {
                cfg_value: cfg,
                inference_timesteps: steps,
                seed: None,
                max_audio_seconds: 5.0,
            };
            assert!(
                engine
                    .synthesize(text, None, &params, &mut |_| {}, &AtomicBool::new(false))
                    .is_err()
            );
            assert!(!engine.is_loaded());
        }
    }
}
