use crate::config::{Config, TtsConfig};
use crate::error::{Error, Result};
use crate::subtitle::WordBoundary;
use crate::video;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug, Clone)]
pub struct TtsOutput {
    pub audio_file: PathBuf,
    /// Real duration of the written audio file. Deliberately measured from the
    /// file, not the last word boundary: Edge TTS leaves a fixed ~0.9s tail
    /// past the final word, which matters most on short scripts.
    pub duration: f64,
    /// Empty when the backend cannot report word timings (macOS `say`).
    pub word_boundaries: Vec<WordBoundary>,
}

pub fn synthesize(config: &Config, text: &str, output: &Path) -> Result<TtsOutput> {
    let text = text.trim();
    if text.is_empty() {
        return Err(Error::Tts("cannot synthesize empty text".to_string()));
    }
    match config.tts.backend.trim().to_lowercase().as_str() {
        "edge" => edge_tts(&config.tts, text, output, config),
        "say" => macos_say(&config.tts, text, output, config),
        other => Err(Error::Tts(format!(
            "unknown TTS backend '{other}' (expected 'edge' or 'say')"
        ))),
    }
}

/// Microsoft Edge neural TTS over its public WebSocket endpoint. Word
/// boundaries arrive as metadata with offsets in 100-nanosecond ticks.
fn edge_tts(tts: &TtsConfig, text: &str, output: &Path, config: &Config) -> Result<TtsOutput> {
    use msedge_tts::tts::SpeechConfig;
    use msedge_tts::tts::client::connect;

    let rate_percent = ((tts.rate - 1.0) * 100.0).round() as i32;
    let speech_config = SpeechConfig {
        voice_name: tts.voice.clone(),
        audio_format: "audio-24khz-48kbitrate-mono-mp3".to_string(),
        pitch: 0,
        rate: rate_percent,
        volume: 0,
    };

    let mut client =
        connect().map_err(|e| Error::Tts(format!("edge TTS connection failed: {e:?}")))?;
    let synthesized = client
        .synthesize(text, &speech_config)
        .map_err(|e| Error::Tts(format!("edge TTS synthesis failed: {e:?}")))?;
    if synthesized.audio_bytes.is_empty() {
        return Err(Error::Tts(
            "edge TTS returned no audio; check the voice name and network".to_string(),
        ));
    }
    std::fs::write(output, &synthesized.audio_bytes)?;

    const TICKS_PER_SECOND: f64 = 10_000_000.0;
    let word_boundaries = synthesized
        .audio_metadata
        .iter()
        .filter(|meta| meta.metadata_type.as_deref() == Some("WordBoundary"))
        .filter_map(|meta| {
            let start = meta.offset as f64 / TICKS_PER_SECOND;
            let end = start + meta.duration as f64 / TICKS_PER_SECOND;
            Some(WordBoundary {
                start,
                end,
                text: meta.text.clone()?,
            })
        })
        .collect();

    let duration = video::probe_duration(&config.render.ffprobe_path, output)?;
    Ok(TtsOutput {
        audio_file: output.to_path_buf(),
        duration,
        word_boundaries,
    })
}

/// Offline fallback via the macOS `say` command. Produces an AIFF that is
/// converted to mp3 with ffmpeg. `say` reports no word timings, so subtitle
/// timing falls back to proportional allocation.
fn macos_say(tts: &TtsConfig, text: &str, output: &Path, config: &Config) -> Result<TtsOutput> {
    let aiff = output.with_extension("aiff");
    let mut command = Command::new("say");
    // `say -r` is words per minute; ~175 wpm reads naturally at rate 1.0.
    let wpm = (175.0 * tts.rate).round() as i64;
    command.arg("-r").arg(wpm.to_string());
    if !tts.voice.trim().is_empty() && !tts.voice.contains("Neural") {
        command.arg("-v").arg(tts.voice.trim());
    }
    let status = command
        .arg("-o")
        .arg(&aiff)
        .arg(text)
        .status()
        .map_err(|e| Error::Tts(format!("failed to run `say`: {e}")))?;
    if !status.success() {
        return Err(Error::Tts(format!("`say` exited with {status}")));
    }

    let convert = Command::new(&config.render.ffmpeg_path)
        .args(["-y", "-hide_banner", "-loglevel", "error", "-i"])
        .arg(&aiff)
        .args(["-codec:a", "libmp3lame", "-qscale:a", "4"])
        .arg(output)
        .status()
        .map_err(|e| Error::Tts(format!("ffmpeg mp3 conversion failed to start: {e}")))?;
    let _ = std::fs::remove_file(&aiff);
    if !convert.success() {
        return Err(Error::Tts(format!(
            "ffmpeg mp3 conversion exited with {convert}"
        )));
    }

    let duration = video::probe_duration(&config.render.ffprobe_path, output)?;
    Ok(TtsOutput {
        audio_file: output.to_path_buf(),
        duration,
        word_boundaries: Vec::new(),
    })
}

/// List available Edge voices (network call), optionally filtered by locale
/// prefix such as "en-US" or "zh-CN".
pub fn list_edge_voices(locale_filter: Option<&str>) -> Result<Vec<String>> {
    let voices = msedge_tts::voice::get_voices_list()
        .map_err(|e| Error::Tts(format!("failed to list edge voices: {e:?}")))?;
    let mut names: Vec<String> = voices
        .into_iter()
        .map(|voice| voice.short_name.unwrap_or(voice.name))
        .filter(|name| {
            locale_filter
                .map(|prefix| name.to_lowercase().starts_with(&prefix.to_lowercase()))
                .unwrap_or(true)
        })
        .collect();
    names.sort();
    Ok(names)
}
