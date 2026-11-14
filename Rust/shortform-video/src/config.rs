use crate::error::{Error, Result};
use serde::Deserialize;
use std::path::{Path, PathBuf};

/// Runtime configuration, merged from (lowest to highest precedence):
/// built-in defaults -> ~/.config/shortform-video/config.toml -> environment.
#[derive(Debug, Clone, Deserialize, Default)]
#[serde(default)]
pub struct Config {
    pub llm: LlmConfig,
    pub materials: MaterialsConfig,
    pub tts: TtsConfig,
    pub render: RenderConfig,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct LlmConfig {
    /// OpenAI-compatible chat-completions base URL (llama-server, mlx_lm.server,
    /// LiteLLM proxy, or a hosted provider).
    pub base_url: String,
    pub api_key: String,
    pub model: String,
    pub timeout_secs: u64,
}

impl Default for LlmConfig {
    fn default() -> Self {
        Self {
            base_url: "http://127.0.0.1:8080/v1".to_string(),
            api_key: String::new(),
            model: "default".to_string(),
            timeout_secs: 180,
        }
    }
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(default)]
pub struct MaterialsConfig {
    pub pexels_api_key: String,
    pub pixabay_api_key: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct TtsConfig {
    /// "edge" (Microsoft Edge neural voices, network) or "say" (macOS, offline).
    pub backend: String,
    pub voice: String,
    /// Playback rate multiplier; 1.0 is normal speed.
    pub rate: f64,
}

impl Default for TtsConfig {
    fn default() -> Self {
        Self {
            backend: "edge".to_string(),
            voice: "en-US-JennyNeural".to_string(),
            rate: 1.0,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct RenderConfig {
    pub ffmpeg_path: String,
    pub ffprobe_path: String,
    pub font_path: String,
    pub font_size: u32,
    pub fps: u32,
    /// x264 CRF quality (lower is better/larger).
    pub crf: u32,
    pub voice_volume: f64,
    pub bgm_volume: f64,
}

impl Default for RenderConfig {
    fn default() -> Self {
        Self {
            ffmpeg_path: "ffmpeg".to_string(),
            ffprobe_path: "ffprobe".to_string(),
            font_path: String::new(),
            font_size: 64,
            fps: 30,
            crf: 20,
            voice_volume: 1.0,
            bgm_volume: 0.2,
        }
    }
}

/// macOS system fonts probed when render.font_path is not configured.
const DEFAULT_FONT_CANDIDATES: &[&str] = &[
    "/System/Library/Fonts/Supplemental/Arial Bold.ttf",
    "/System/Library/Fonts/Supplemental/Arial.ttf",
    "/System/Library/Fonts/Supplemental/Verdana Bold.ttf",
    "/System/Library/Fonts/Supplemental/Tahoma Bold.ttf",
    "/Library/Fonts/Arial.ttf",
];

impl Config {
    pub fn load() -> Result<Self> {
        let mut config = match Self::config_file_path() {
            Some(path) if path.is_file() => {
                let text = std::fs::read_to_string(&path)?;
                toml::from_str(&text)
                    .map_err(|e| Error::Config(format!("{}: {e}", path.display())))?
            }
            _ => Self::default(),
        };
        config.apply_env();
        Ok(config)
    }

    pub fn config_file_path() -> Option<PathBuf> {
        std::env::var_os("HOME")
            .map(|home| Path::new(&home).join(".config/shortform-video/config.toml"))
    }

    fn apply_env(&mut self) {
        fn env_override(slot: &mut String, keys: &[&str]) {
            for key in keys {
                if let Ok(value) = std::env::var(key)
                    && !value.trim().is_empty()
                {
                    *slot = value;
                    return;
                }
            }
        }
        env_override(&mut self.llm.base_url, &["SHORTFORM_LLM_BASE_URL"]);
        env_override(&mut self.llm.api_key, &["SHORTFORM_LLM_API_KEY"]);
        env_override(&mut self.llm.model, &["SHORTFORM_LLM_MODEL"]);
        env_override(
            &mut self.materials.pexels_api_key,
            &["SHORTFORM_PEXELS_API_KEY", "PEXELS_API_KEY"],
        );
        env_override(
            &mut self.materials.pixabay_api_key,
            &["SHORTFORM_PIXABAY_API_KEY", "PIXABAY_API_KEY"],
        );
        env_override(&mut self.tts.backend, &["SHORTFORM_TTS_BACKEND"]);
        env_override(&mut self.tts.voice, &["SHORTFORM_TTS_VOICE"]);
        env_override(&mut self.render.font_path, &["SHORTFORM_FONT_PATH"]);
        env_override(&mut self.render.ffmpeg_path, &["SHORTFORM_FFMPEG_PATH"]);
        env_override(&mut self.render.ffprobe_path, &["SHORTFORM_FFPROBE_PATH"]);
    }

    /// Resolve a usable subtitle font, falling back to known macOS locations.
    pub fn resolve_font_path(&self) -> Result<PathBuf> {
        if !self.render.font_path.trim().is_empty() {
            let path = PathBuf::from(self.render.font_path.trim());
            if path.is_file() {
                return Ok(path);
            }
            return Err(Error::NotFound(path));
        }
        for candidate in DEFAULT_FONT_CANDIDATES {
            let path = PathBuf::from(candidate);
            if path.is_file() {
                return Ok(path);
            }
        }
        Err(Error::Config(
            "no subtitle font found; set render.font_path in config.toml or \
             SHORTFORM_FONT_PATH to a .ttf file"
                .to_string(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_sensible() {
        let config = Config::default();
        assert_eq!(config.render.fps, 30);
        assert_eq!(config.tts.backend, "edge");
        assert!(config.llm.base_url.starts_with("http"));
    }

    #[test]
    fn toml_partial_override() {
        let config: Config = toml::from_str(
            r#"
            [llm]
            model = "qwen3.5-9b"
            [render]
            font_size = 72
            "#,
        )
        .unwrap();
        assert_eq!(config.llm.model, "qwen3.5-9b");
        assert_eq!(config.render.font_size, 72);
        // Untouched sections keep defaults.
        assert_eq!(config.render.fps, 30);
        assert_eq!(config.tts.voice, "en-US-JennyNeural");
    }
}
