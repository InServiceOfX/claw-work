//! Faceless short-form video assembly pipeline.
//!
//! Core stages, each usable on its own: LLM script + search terms, stock
//! material sourcing (Pexels/Pixabay), TTS narration (Edge neural voices or
//! macOS `say`), script-aligned subtitles, and ffmpeg-only composition with
//! caption overlays rendered in-process.
//!
//! Algorithms for duration budgeting, cover/contain fitting, and word-cue
//! aggregation are ported from MoneyPrinterTurbo (harry0703/MoneyPrinterTurbo,
//! MIT license); the implementation is original to this crate.

pub mod caption;
pub mod config;
pub mod error;
pub mod llm;
pub mod material;
pub mod pipeline;
pub mod subtitle;
pub mod tts;
pub mod video;
