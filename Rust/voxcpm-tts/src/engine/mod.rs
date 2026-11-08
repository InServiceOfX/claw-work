//! Inference engines behind one interface.
//!
//! * `native` — VoxCPM2 ported to Rust on candle/Metal (preferred).
//! * `mlx-python` — a persistent mlx-audio worker process (fallback and the
//!   numerical oracle for the native port).
//!
//! Lifecycle: an engine owns the weights between `load` and `unload`; dropping
//! the engine (including on Ctrl-C or window close) must release them.

pub mod worker;

use std::path::PathBuf;
use std::sync::atomic::AtomicBool;

use anyhow::Result;

use crate::audio::Audio;

/// A prepared reference voice.
#[derive(Debug, Clone)]
pub struct VoicePrompt {
    /// Mono WAV prepared by [`crate::voice::prepare_reference`].
    pub audio_path: PathBuf,
    /// Exact transcript of the reference clip. When present, generation
    /// continues from the clip ("ultimate cloning"); otherwise the clip
    /// conditions timbre only.
    pub transcript: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GenerationParams {
    pub cfg_value: f32,
    pub inference_timesteps: usize,
    pub seed: Option<u64>,
    /// Hard cap on audio produced for one chunk (runaway-generation guard).
    pub max_audio_seconds: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum EngineEvent {
    /// Audio generated so far for the current chunk.
    Generated { audio_seconds: f64 },
}

#[derive(thiserror::Error, Debug)]
#[error("cancelled")]
pub struct Cancelled;

/// A generation that reached its safety cap or failed to produce speech.
/// Unlike I/O/model errors, the job may retry it with a new noise seed.
#[derive(thiserror::Error, Debug)]
#[error("{0}")]
pub struct BadGeneration(pub String);

pub trait Engine: Send {
    /// Stable identifier used for calibration and reports.
    fn name(&self) -> &'static str;
    /// Load weights; returns wall-clock seconds spent. Idempotent.
    fn load(&mut self) -> Result<f64>;
    /// Cancellable front-end load; process engines override this to interrupt
    /// startup/import/download waits. Native local allocation finishes its
    /// current call, then drops weights if cancellation arrived meanwhile.
    fn load_cancellable(&mut self, cancel: &AtomicBool) -> Result<f64> {
        use std::sync::atomic::Ordering;
        if cancel.load(Ordering::SeqCst) {
            return Err(Cancelled.into());
        }
        let secs = self.load()?;
        if cancel.load(Ordering::SeqCst) {
            self.unload()?;
            return Err(Cancelled.into());
        }
        Ok(secs)
    }
    fn is_loaded(&self) -> bool;
    fn synthesize(
        &mut self,
        text: &str,
        voice: Option<&VoicePrompt>,
        params: &GenerationParams,
        on_event: &mut dyn FnMut(EngineEvent),
        cancel: &AtomicBool,
    ) -> Result<Audio>;
    /// Release weights and accelerator memory. Idempotent.
    fn unload(&mut self) -> Result<()>;
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum, serde::Serialize, serde::Deserialize,
)]
pub enum EngineKind {
    /// Native Rust (candle + Metal).
    Native,
    /// Python mlx-audio worker process.
    MlxPython,
}

/// Construct an engine of the requested kind.
pub fn create(kind: EngineKind, model_id: &str) -> Result<Box<dyn Engine>> {
    match kind {
        EngineKind::MlxPython => Ok(Box::new(worker::WorkerEngine::new(model_id)?)),
        EngineKind::Native => crate::native::create_engine(model_id),
    }
}
