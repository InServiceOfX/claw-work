//! Native Rust VoxCPM2 engine (candle + Metal). See docs/PORTING_SPEC.md.
//!
//! Module ownership (one file per component, shared interfaces below):
//! `minicpm` transformer stack + KV cache, `locenc` patch encoder, `locdit`
//! diffusion transformer, `cfm` Euler/CFG sampler, `fsq` scalar quantizer,
//! `vae` AudioVAE V2 codec, `tokenizer` text → ids, `model` the full
//! generation loop, `engine` the [`crate::engine::Engine`] implementation.

pub mod cfm;
pub mod config;
pub mod engine;
pub mod fsq;
pub mod locdit;
pub mod locenc;
pub mod minicpm;
pub mod model;
pub mod tokenizer;
pub mod vae;
pub mod weights;

#[cfg(test)]
pub mod golden;

use anyhow::Result;

use crate::engine::Engine;

pub fn create_engine(model_id: &str) -> Result<Box<dyn Engine>> {
    Ok(Box::new(engine::NativeEngine::new(model_id)))
}

/// Whether the native engine can be constructed in this build.
pub fn available() -> bool {
    true
}
