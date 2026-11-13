//! claw-shorts: a local, reviewable pipeline for short promotional videos.
//!
//! A human or agent writes a [`spec::Spec`]. This crate validates it (no
//! placeholders, no unsupported superlatives, no glyphs the font cannot draw),
//! narrates every scene with a local TTS engine, builds an FFmpeg filter graph
//! and renders the video plus a contact sheet and a posting kit. It never
//! uploads or posts anything.

pub mod brief;
pub mod filters;
pub mod kit;
pub mod media;
pub mod render;
pub mod spec;
pub mod style;
pub mod text;
pub mod timeline;
