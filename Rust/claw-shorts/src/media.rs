//! Thin wrappers over `ffprobe`, `ffmpeg` and the local TTS binary. Nothing here
//! touches the network or uploads anything.

use std::path::Path;
use std::process::Command;

use serde::Deserialize;

/// True when `bin` resolves on `PATH` (or is an existing path).
pub fn have(bin: &str) -> bool {
    if bin.contains('/') {
        return Path::new(bin).is_file();
    }
    std::env::var_os("PATH")
        .map(|paths| std::env::split_paths(&paths).any(|dir| dir.join(bin).is_file()))
        .unwrap_or(false)
}

fn tail(bytes: &[u8]) -> String {
    let text = String::from_utf8_lossy(bytes);
    let lines: Vec<&str> = text.lines().collect();
    lines[lines.len().saturating_sub(12)..].join("\n")
}

/// Run a command, returning an error that includes the tail of its stderr.
pub fn run(command: &mut Command, what: &str) -> Result<(), String> {
    let output = command.output().map_err(|e| format!("{what}: could not start: {e}"))?;
    if output.status.success() {
        Ok(())
    } else {
        Err(format!("{what} failed ({}):\n{}", output.status, tail(&output.stderr)))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VideoInfo {
    pub width: u32,
    pub height: u32,
    pub duration_ms: u64,
    pub has_audio: bool,
}

#[derive(Deserialize)]
struct ProbeOut {
    #[serde(default)]
    streams: Vec<ProbeStream>,
    format: Option<ProbeFormat>,
}
#[derive(Deserialize)]
struct ProbeStream {
    codec_type: Option<String>,
    width: Option<u32>,
    height: Option<u32>,
}
#[derive(Deserialize)]
struct ProbeFormat {
    duration: Option<String>,
}

pub fn parse_probe(json: &str) -> Result<VideoInfo, String> {
    let probe: ProbeOut = serde_json::from_str(json).map_err(|e| format!("bad ffprobe output: {e}"))?;
    let seconds: f64 = probe
        .format
        .and_then(|f| f.duration)
        .ok_or("ffprobe reported no duration")?
        .parse()
        .map_err(|e| format!("bad duration: {e}"))?;
    let video = probe.streams.iter().find(|s| s.codec_type.as_deref() == Some("video"));
    Ok(VideoInfo {
        width: video.and_then(|s| s.width).unwrap_or(0),
        height: video.and_then(|s| s.height).unwrap_or(0),
        duration_ms: (seconds * 1000.0).round() as u64,
        has_audio: probe.streams.iter().any(|s| s.codec_type.as_deref() == Some("audio")),
    })
}

pub fn probe(path: &Path) -> Result<VideoInfo, String> {
    let output = Command::new("ffprobe")
        .args(["-v", "error", "-show_entries", "stream=codec_type,width,height:format=duration", "-of", "json"])
        .arg(path)
        .output()
        .map_err(|e| format!("ffprobe: could not start: {e}"))?;
    if !output.status.success() {
        return Err(format!("ffprobe failed on {}:\n{}", path.display(), tail(&output.stderr)));
    }
    parse_probe(&String::from_utf8_lossy(&output.stdout))
}

pub fn probe_duration_ms(path: &Path) -> Result<u64, String> {
    probe(path).map(|info| info.duration_ms)
}

/// Decode the whole file; any decoder error fails the check.
pub fn decode_check(path: &Path) -> Result<(), String> {
    let output = Command::new("ffmpeg")
        .args(["-v", "error", "-i"])
        .arg(path)
        .args(["-f", "null", "-"])
        .output()
        .map_err(|e| format!("ffmpeg: could not start: {e}"))?;
    if output.status.success() && output.stderr.is_empty() {
        Ok(())
    } else {
        Err(format!("decode check failed:\n{}", tail(&output.stderr)))
    }
}

/// Narrate `text` with the local TTS binary into `out_wav`.
pub fn tts(
    binary: &str,
    text_file: &Path,
    out_wav: &Path,
    reference: &Path,
    transcript: Option<&str>,
    seed: Option<u64>,
) -> Result<(), String> {
    let mut cmd = Command::new(binary);
    cmd.arg("say").arg(text_file).arg("-o").arg(out_wav).arg("--force").arg("--voice").arg(reference);
    if let Some(seed) = seed {
        cmd.arg("--seed").arg(seed.to_string());
    }
    if let Some(transcript) = transcript {
        cmd.arg("--transcript").arg(transcript);
    }
    run(&mut cmd, &format!("{binary} say"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_ffprobe_json() {
        let json = r#"{"streams":[{"codec_type":"video","width":1080,"height":1920},
            {"codec_type":"audio"}],"format":{"duration":"22.650000"}}"#;
        assert_eq!(
            parse_probe(json).unwrap(),
            VideoInfo { width: 1080, height: 1920, duration_ms: 22650, has_audio: true }
        );
    }

    #[test]
    fn missing_duration_is_an_error() {
        assert!(parse_probe(r#"{"streams":[],"format":{}}"#).is_err());
        assert!(parse_probe("not json").is_err());
    }

    #[test]
    fn have_checks_path_and_names() {
        assert!(!have("definitely-not-a-real-binary-xyz"));
        assert!(!have("/definitely/not/here"));
    }
}
