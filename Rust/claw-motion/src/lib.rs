//! claw-motion: headless, deterministic motion graphics for vertical Shorts.
//!
//! - [`physics`]: 2D circles falling and colliding with walls and pegs (the
//!   Matter.js idea, rewritten small), with every collision timed for sound.
//! - [`morph`]: one shape turning smoothly into another (the Flubber idea,
//!   rewritten).
//! - [`sfx`]: impact knocks synthesized on the exact step of each collision.
//! - [`scene`]: TOML scene files and the per-frame drivers.
//! - [`raster`] and [`encode`]: tiny-skia frames piped to FFmpeg, audio mixed
//!   and normalized to -14 LUFS with `claw-shorts`' loudness module.

pub mod encode;
pub mod geom;
pub mod morph;
pub mod physics;
pub mod raster;
pub mod scene;
pub mod sfx;
