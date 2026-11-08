//! voxcpm-tts: macOS voice-cloning text-to-speech around VoxCPM2.
//!
//! The application layer (chunking, reference preparation, stitching,
//! progress/ETA, cleanup) is engine-independent; see [`engine`].

pub mod audio;
pub mod config;
pub mod engine;
pub mod eta;
pub mod hub;
pub mod job;
pub mod native;
pub mod text;
pub mod voice;
