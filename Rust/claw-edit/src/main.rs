use clap::{Parser, Subcommand};
use claw_edit::{Plan, captions, filter_graph, validate};
use serde::Deserialize;
use std::{
    error::Error,
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    process::Command,
};

#[derive(Parser)]
#[command(
    version,
    about = "Render reviewed local cut lists; no AI or network calls"
)]
struct Cli {
    #[command(subcommand)]
    command: Action,
}

#[derive(Subcommand)]
enum Action {
    /// Probe local media, validate the plan and print the output timeline. No writes.
    Check {
        #[arg(long)]
        plan: PathBuf,
    },
    /// Create a NEW directory containing cut.mp4, captions.srt and timeline.json.
    Render {
        #[arg(long)]
        plan: PathBuf,
        #[arg(long)]
        output_dir: PathBuf,
    },
}

#[derive(Deserialize)]
struct Probe {
    streams: Vec<Stream>,
}
#[derive(Deserialize)]
struct Stream {
    codec_type: String,
    duration: Option<String>,
    start_time: Option<String>,
}

fn probe(source: &Path) -> Result<u64, Box<dyn Error>> {
    let output = Command::new("ffprobe")
        .args([
            "-v",
            "error",
            "-protocol_whitelist",
            "file",
            "-show_entries",
            "stream=codec_type,duration,start_time",
            "-of",
            "json",
        ])
        .arg(source)
        .output()?;
    if !output.status.success() {
        return Err("ffprobe failed; supply a readable local video".into());
    }
    let parsed: Probe = serde_json::from_slice(&output.stdout)?;
    // Require finite, known durations for BOTH first streams. Avoid silently
    // shortening a segment because one stream ends before the other.
    let mut durations = Vec::new();
    let mut starts = Vec::new();
    for kind in ["video", "audio"] {
        let stream = parsed
            .streams
            .iter()
            .find(|s| s.codec_type == kind)
            .ok_or_else(|| format!("source needs a {kind} stream"))?;
        let seconds: f64 = stream
            .duration
            .as_deref()
            .ok_or("stream duration unavailable; normalize media first")?
            .parse()?;
        if !seconds.is_finite() || seconds <= 0.0 || seconds > 86_400.0 {
            return Err("source duration must be finite and at most 24 hours".into());
        }
        durations.push((seconds * 1000.0).floor() as u64);
        let start: f64 = stream
            .start_time
            .as_deref()
            .ok_or("stream start unavailable; normalize media first")?
            .parse()?;
        if !start.is_finite() {
            return Err("invalid stream start time".into());
        }
        starts.push(start);
    }
    if (starts[0] - starts[1]).abs() > 0.001 {
        return Err("audio/video starts differ; normalize offset media first".into());
    }
    Ok(*durations.iter().min().unwrap())
}

fn write_new(path: &Path, data: &[u8]) -> Result<(), Box<dyn Error>> {
    let mut file = OpenOptions::new().write(true).create_new(true).open(path)?;
    file.write_all(data)?;
    Ok(())
}

fn run() -> Result<(), Box<dyn Error>> {
    let action = Cli::parse().command;
    let plan_path = match &action {
        Action::Check { plan } | Action::Render { plan, .. } => plan,
    };
    let plan_file = fs::canonicalize(plan_path)?;
    if fs::metadata(&plan_file)?.len() > 1_000_000 {
        return Err("plan exceeds 1 MB".into());
    }
    let plan: Plan = serde_json::from_slice(&fs::read(&plan_file)?)?;
    let source = fs::canonicalize(plan_file.parent().unwrap().join(&plan.source))?;
    if !source.is_file() {
        return Err("source must be a local regular file".into());
    }
    let timeline = validate(&plan, probe(&source)?).map_err(|e| format!("invalid plan: {e}"))?;
    let json = serde_json::to_vec_pretty(&timeline)?;
    if let Action::Render { output_dir, .. } = action {
        // Atomic reservation: existing directories/files/symlinks are refused.
        // A failed render may leave a partial directory; retry with a new name.
        fs::create_dir(&output_dir)?;
        let output = fs::canonicalize(output_dir)?;
        let status = Command::new("ffmpeg")
            .args([
                "-hide_banner",
                "-loglevel",
                "error",
                "-nostdin",
                "-n",
                "-protocol_whitelist",
                "file",
                "-i",
            ])
            .arg(&source)
            .args([
                "-filter_complex",
                &filter_graph(&plan),
                "-map",
                "[vout]",
                "-map",
                "[aout]",
                "-map_metadata",
                "-1",
                "-map_chapters",
                "-1",
                "-c:v",
                "libx264",
                "-crf",
                "20",
                "-preset",
                "fast",
                "-pix_fmt",
                "yuv420p",
                "-c:a",
                "aac",
                "-b:a",
                "192k",
                "-movflags",
                "+faststart",
            ])
            .arg(output.join("cut.mp4"))
            .status()?;
        if !status.success() {
            return Err(
                "FFmpeg failed; partial directory retained, no completion report written".into(),
            );
        }
        write_new(
            &output.join("captions.srt"),
            captions(&plan, &timeline).as_bytes(),
        )?;
        // Written last; records the intended edit timeline, not a quality verdict.
        write_new(&output.join("timeline.json"), &json)?;
        println!("{}", output.display());
    } else {
        println!("{}", String::from_utf8(json)?);
    }
    Ok(())
}

fn main() {
    if let Err(error) = run() {
        eprintln!("claw-edit: {error}");
        std::process::exit(1);
    }
}
