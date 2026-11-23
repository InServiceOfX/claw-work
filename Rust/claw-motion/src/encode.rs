//! FFmpeg plumbing: raw frames in, H.264 out; audio mixed and normalized in
//! its own pass (as in `shortform-video`, which found a shared video+audio
//! graph made the audio differ between identical runs), then muxed.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, Command, Stdio};

use claw_shorts::loudness::{self, Loudness};

use crate::scene::Video;

pub struct VideoEncoder {
    child: Child,
    stdin: Option<ChildStdin>,
    path: PathBuf,
}

impl VideoEncoder {
    /// Start an encoder writing a silent MP4 to `path`.
    pub fn start(path: &Path, v: &Video) -> Result<VideoEncoder, String> {
        let mut child = Command::new("ffmpeg")
            .args(["-hide_banner", "-loglevel", "error", "-y", "-f", "rawvideo", "-pix_fmt", "rgba", "-s"])
            .arg(format!("{}x{}", v.width, v.height))
            .args(["-r", &v.fps.to_string(), "-i", "-"])
            // A fixed thread count keeps x264's output byte-identical between runs.
            .args(["-c:v", "libx264", "-preset", "medium", "-crf", "18", "-threads", "4", "-pix_fmt", "yuv420p", "-movflags", "+faststart", "-an"])
            .arg(path)
            .stdin(Stdio::piped())
            .spawn()
            .map_err(|e| format!("ffmpeg: could not start ({e}); is it installed?"))?;
        let stdin = child.stdin.take();
        Ok(VideoEncoder { child, stdin, path: path.to_path_buf() })
    }

    pub fn write(&mut self, rgba: &[u8]) -> Result<(), String> {
        self.stdin.as_mut().ok_or("encoder already closed")?.write_all(rgba).map_err(|e| format!("ffmpeg stopped accepting frames: {e}"))
    }

    pub fn finish(mut self) -> Result<PathBuf, String> {
        drop(self.stdin.take());
        let status = self.child.wait().map_err(|e| format!("ffmpeg: {e}"))?;
        if !status.success() {
            return Err(format!("ffmpeg failed to encode {}", self.path.display()));
        }
        Ok(self.path)
    }
}

/// The audio sources for a render.
#[derive(Debug, Clone)]
pub struct AudioPlan<'a> {
    pub narration: Option<&'a Path>,
    pub sfx: Option<&'a Path>,
    /// SFX level relative to the narration when both are present, dB.
    pub sfx_db: f64,
    pub duration: f64,
}

impl AudioPlan<'_> {
    pub fn inputs(&self) -> Vec<&Path> {
        self.narration.into_iter().chain(self.sfx).collect()
    }

    /// Filter graph ending in `[aout]`: every source padded or cut to the video length.
    pub fn graph(&self) -> Option<String> {
        let fit = |label: &str, extra: &str| {
            format!("[{label}]aformat=sample_rates=48000:channel_layouts=mono{extra},apad,atrim=0:{:.3}", self.duration)
        };
        match (self.narration, self.sfx) {
            (None, None) => None,
            (Some(_), None) | (None, Some(_)) => Some(format!("{}[aout]", fit("0:a", ""))),
            (Some(_), Some(_)) => Some(format!(
                "{}[n];{}[s];[n][s]amix=inputs=2:normalize=0:duration=first[aout]",
                fit("0:a", ""),
                fit("1:a", &format!(",volume={:.2}dB", self.sfx_db))
            )),
        }
    }
}

fn ffmpeg_inputs(cmd: &mut Command, inputs: &[&Path]) {
    for i in inputs {
        cmd.arg("-i").arg(i);
    }
}

/// Mix and normalize to -14 LUFS / -1 dBTP into a 48 kHz WAV. Returns the
/// measurement of the mix before normalization, or `None` when there is no audio.
pub fn render_audio(plan: &AudioPlan, out: &Path) -> Result<Option<loudness::Normalization>, String> {
    let Some(graph) = plan.graph() else { return Ok(None) };
    let inputs = plan.inputs();
    let normalization = loudness::plan(|pre| {
        let chain = match pre {
            Some(pre) => format!("{pre},{}", loudness::measure_filter()),
            None => loudness::measure_filter(),
        };
        let mut cmd = Command::new("ffmpeg");
        cmd.args(["-hide_banner", "-nostats"]);
        ffmpeg_inputs(&mut cmd, &inputs);
        cmd.arg("-filter_complex").arg(format!("{graph};[aout]{chain}[measured]")).args(["-map", "[measured]", "-f", "null", "-"]);
        loudness::measure_with(&mut cmd)
    })?;
    let mut cmd = Command::new("ffmpeg");
    cmd.args(["-hide_banner", "-loglevel", "error", "-y"]);
    ffmpeg_inputs(&mut cmd, &inputs);
    cmd.arg("-filter_complex")
        .arg(format!("{graph};[aout]{}[final]", normalization.filter))
        .args(["-map", "[final]", "-c:a", "pcm_s16le"])
        .arg(out);
    claw_shorts::media::run(&mut cmd, "normalize audio")?;
    Ok(Some(normalization))
}

/// Join the silent video with the normalized audio, then check the result's loudness.
pub fn mux(video: &Path, audio: &Path, out: &Path) -> Result<Loudness, String> {
    let mut cmd = Command::new("ffmpeg");
    cmd.args(["-hide_banner", "-loglevel", "error", "-y", "-i"])
        .arg(video)
        .arg("-i")
        .arg(audio)
        .args(["-map", "0:v", "-map", "1:a", "-c:v", "copy", "-c:a", "aac", "-b:a", "192k", "-ac", "2", "-shortest", "-movflags", "+faststart"])
        .arg(out);
    claw_shorts::media::run(&mut cmd, "mux")?;
    let finished = loudness::measure_file(out)?;
    let problems = loudness::problems(&finished);
    if !problems.is_empty() {
        return Err(format!("final loudness is off target: {}", problems.join("; ")));
    }
    Ok(finished)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn graphs_pad_every_source_to_the_video_length() {
        let n = Path::new("n.wav");
        let s = Path::new("s.wav");
        let none = AudioPlan { narration: None, sfx: None, sfx_db: -14.0, duration: 5.0 };
        assert!(none.graph().is_none());
        let sfx_only = AudioPlan { sfx: Some(s), ..none.clone() };
        assert_eq!(sfx_only.graph().unwrap(), "[0:a]aformat=sample_rates=48000:channel_layouts=mono,apad,atrim=0:5.000[aout]");
        assert_eq!(sfx_only.inputs(), vec![s]);
        let both = AudioPlan { narration: Some(n), sfx: Some(s), ..none };
        let g = both.graph().unwrap();
        assert!(g.contains("[1:a]aformat=sample_rates=48000:channel_layouts=mono,volume=-14.00dB,apad,atrim=0:5.000[s]"), "{g}");
        assert!(g.ends_with("amix=inputs=2:normalize=0:duration=first[aout]"));
        assert_eq!(both.inputs(), vec![n, s]);
    }
}
