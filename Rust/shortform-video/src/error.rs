use std::path::PathBuf;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("config error: {0}")]
    Config(String),

    #[error("LLM request failed: {0}")]
    Llm(String),

    #[error("material search/download failed: {0}")]
    Material(String),

    #[error("TTS failed: {0}")]
    Tts(String),

    #[error("subtitle error: {0}")]
    Subtitle(String),

    #[error("caption rendering failed: {0}")]
    Caption(String),

    #[error("ffmpeg/ffprobe failed: {0}")]
    Ffmpeg(String),

    #[error("file not found: {0}")]
    NotFound(PathBuf),

    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    Http(#[from] reqwest::Error),

    #[error(transparent)]
    Json(#[from] serde_json::Error),
}
