use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::Instant;

use clap::{Parser, Subcommand, ValueEnum};
use serde::Serialize;

use claw_motion::encode::{self, AudioPlan, VideoEncoder};
use claw_motion::raster::Frame;
use claw_motion::scene::{MorphRun, MorphScene, PhysicsRun, PhysicsScene, Video};
use claw_motion::sfx;

#[derive(Parser)]
#[command(name = "claw-motion", version, about = "Deterministic physics and shape-morph motion graphics for vertical Shorts, rendered headless to MP4")]
struct Cli {
    #[command(subcommand)]
    command: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Simulate balls falling through walls and pegs; collisions become timed impact sounds.
    Physics {
        scene: PathBuf,
        #[command(flatten)]
        out: OutputArgs,
        /// Leave out the impact sounds.
        #[arg(long)]
        no_sfx: bool,
        /// Impact-sound level relative to the narration, dB (only with --narration).
        #[arg(long, default_value_t = -14.0, allow_negative_numbers = true)]
        sfx_db: f64,
    },
    /// Morph through a sequence of shapes.
    Morph {
        scene: PathBuf,
        #[command(flatten)]
        out: OutputArgs,
    },
    /// Print an example scene file to stdout.
    Example { name: ExampleName },
}

#[derive(clap::Args)]
struct OutputArgs {
    /// Output: an .mp4, or a .png with --still.
    #[arg(short, long)]
    output: PathBuf,
    /// Render only the frame at this time (seconds) to a PNG, to check a scene quickly.
    #[arg(long)]
    still: Option<f64>,
    /// Narration to mix under the picture (normalized with the rest to -14 LUFS).
    #[arg(long)]
    narration: Option<PathBuf>,
    /// Replace the output if it exists.
    #[arg(long)]
    force: bool,
}

#[derive(Clone, Copy, ValueEnum)]
enum ExampleName {
    Plinko,
    Morph,
}

#[derive(Serialize)]
struct Report {
    output: String,
    frames: u64,
    fps: u32,
    duration: f64,
    render_seconds: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    bodies_spawned: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    impacts: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    impacts_heard: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    audio_before_normalization: Option<claw_shorts::loudness::Loudness>,
    #[serde(skip_serializing_if = "Option::is_none")]
    limiter_used: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    final_loudness: Option<claw_shorts::loudness::Loudness>,
}

fn sibling(out: &Path, suffix: &str) -> PathBuf {
    let stem = out.file_stem().and_then(|s| s.to_str()).unwrap_or("out");
    out.with_file_name(format!("{stem}.{suffix}"))
}

fn check_output(o: &OutputArgs) -> Result<(), String> {
    if o.output.exists() && !o.force {
        return Err(format!("{} exists; pass --force to replace it", o.output.display()));
    }
    let want = if o.still.is_some() { "png" } else { "mp4" };
    if o.output.extension().and_then(|e| e.to_str()) != Some(want) {
        return Err(format!("output must be a .{want} file"));
    }
    if let Some(n) = &o.narration {
        if !n.exists() {
            return Err(format!("narration {} not found", n.display()));
        }
    }
    Ok(())
}

fn frame_for(video: &Video, t: f64) -> Result<u64, String> {
    if !(0.0..=video.duration).contains(&t) {
        return Err(format!("--still {t} is outside 0..{}", video.duration));
    }
    Ok((t * f64::from(video.fps)).round() as u64)
}

/// Encode frames, then add audio if any. Returns the report fields it knows.
fn finish_video(
    video: &Video,
    o: &OutputArgs,
    sfx_wav: Option<&Path>,
    sfx_db: f64,
    mut draw: impl FnMut(u64, &mut Frame),
) -> Result<Report, String> {
    let started = Instant::now();
    let silent = sibling(&o.output, "silent.mp4");
    let mut enc = VideoEncoder::start(&silent, video)?;
    let mut frame = Frame::new(video.width, video.height)?;
    for i in 0..video.frames() {
        draw(i, &mut frame);
        enc.write(frame.rgba())?;
    }
    enc.finish()?;

    let plan = AudioPlan { narration: o.narration.as_deref(), sfx: sfx_wav, sfx_db, duration: video.duration };
    let audio = sibling(&o.output, "audio.wav");
    let normalization = encode::render_audio(&plan, &audio)?;
    let final_loudness = match &normalization {
        Some(_) => {
            let l = encode::mux(&silent, &audio, &o.output)?;
            std::fs::remove_file(&audio).ok();
            std::fs::remove_file(&silent).ok();
            Some(l)
        }
        None => {
            std::fs::rename(&silent, &o.output).map_err(|e| format!("move {}: {e}", silent.display()))?;
            None
        }
    };
    Ok(Report {
        output: o.output.display().to_string(),
        frames: video.frames(),
        fps: video.fps,
        duration: video.duration,
        render_seconds: (started.elapsed().as_secs_f64() * 10.0).round() / 10.0,
        bodies_spawned: None,
        impacts: None,
        impacts_heard: None,
        audio_before_normalization: normalization.as_ref().map(|n| n.measured),
        limiter_used: normalization.as_ref().map(|n| n.limited),
        final_loudness,
    })
}

fn write_json(path: &Path, value: &impl Serialize) -> Result<(), String> {
    let text = serde_json::to_string_pretty(value).map_err(|e| e.to_string())?;
    std::fs::write(path, text + "\n").map_err(|e| format!("write {}: {e}", path.display()))
}

fn physics(scene_path: &Path, o: &OutputArgs, no_sfx: bool, sfx_db: f64) -> Result<(), String> {
    check_output(o)?;
    let scene = PhysicsScene::load(scene_path)?;
    let video = scene.video.clone();
    let sfx_settings = scene.sfx.settings();
    let sfx_on = scene.sfx.enabled && !no_sfx;
    let mut run = PhysicsRun::new(scene);

    if let Some(t) = o.still {
        let target = frame_for(&video, t)?;
        for _ in 0..=target {
            run.advance();
        }
        let mut frame = Frame::new(video.width, video.height)?;
        run.draw(&mut frame);
        frame.save_png(&o.output)?;
        println!("{} (frame {target}, {} bodies)", o.output.display(), run.world.alive().count());
        return Ok(());
    }

    // Simulate the whole scene first: the sound needs every impact before encoding.
    // Frames are drawn from a second, identical run, so nothing is held in memory.
    let mut sim = PhysicsRun::new(run.scene.clone());
    for _ in 0..video.frames() {
        sim.advance();
    }
    let impacts = sim.world.impacts.clone();
    let impacts_path = sibling(&o.output, "impacts.json");
    write_json(&impacts_path, &impacts)?;
    let sfx_path = sibling(&o.output, "sfx.wav");
    let heard = sfx::thin(&impacts, &sfx_settings).len();
    let have_sfx = sfx_on && heard > 0;
    if have_sfx {
        sfx::write_wav(&sfx_path, &sfx::track(&impacts, video.duration, &sfx_settings)).map_err(|e| format!("write {}: {e}", sfx_path.display()))?;
    }

    let mut report = finish_video(&video, o, have_sfx.then_some(sfx_path.as_path()), sfx_db, |_, frame| {
        run.advance();
        run.draw(frame);
    })?;
    if run.world.impacts.len() != impacts.len() {
        return Err("the drawing run diverged from the sound run (non-deterministic simulation)".into());
    }
    report.bodies_spawned = Some(run.spawned);
    report.impacts = Some(impacts.len());
    report.impacts_heard = Some(if have_sfx { heard } else { 0 });
    write_json(&sibling(&o.output, "report.json"), &report)?;
    println!("{}", serde_json::to_string_pretty(&report).unwrap_or_default());
    Ok(())
}

fn morph(scene_path: &Path, o: &OutputArgs) -> Result<(), String> {
    check_output(o)?;
    let run = MorphRun::new(MorphScene::load(scene_path)?)?;
    let video = run.scene.video.clone();
    let fps = f64::from(video.fps);
    if let Some(t) = o.still {
        frame_for(&video, t)?;
        let mut frame = Frame::new(video.width, video.height)?;
        run.draw(&mut frame, t);
        frame.save_png(&o.output)?;
        println!("{}", o.output.display());
        return Ok(());
    }
    let report = finish_video(&video, o, None, 0.0, |i, frame| run.draw(frame, i as f64 / fps))?;
    write_json(&sibling(&o.output, "report.json"), &report)?;
    println!("{}", serde_json::to_string_pretty(&report).unwrap_or_default());
    Ok(())
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    let result = match &cli.command {
        Cmd::Physics { scene, out, no_sfx, sfx_db } => physics(scene, out, *no_sfx, *sfx_db),
        Cmd::Morph { scene, out } => morph(scene, out),
        Cmd::Example { name } => {
            print!("{}", match name {
                ExampleName::Plinko => include_str!("../examples/plinko.toml"),
                ExampleName::Morph => include_str!("../examples/morph.toml"),
            });
            Ok(())
        }
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("claw-motion: {e}");
            ExitCode::FAILURE
        }
    }
}
