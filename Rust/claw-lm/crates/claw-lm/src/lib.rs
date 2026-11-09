//! Model-specific native Nemotron inference. HTTP is a separate crate.
pub mod assets;
pub mod chat;
pub mod cpu;
#[cfg(feature = "cuda")]
pub mod cuda;
pub mod generation;
pub use assets::{Assets, Config};
pub use generation::{Decoder, GenerationOptions, GenerationResult, Mode, generate};
