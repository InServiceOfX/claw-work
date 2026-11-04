//! Local, provider-independent Lean tools. A checked model is not source verification.
pub mod artifact;
pub mod input;
pub mod privacy;
pub mod process;
pub mod proof;

pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;
