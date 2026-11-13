use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use clap::{Parser, Subcommand};

use claw_shorts::render::{self, RenderOptions};
use claw_shorts::spec::{self, Severity, Spec};
use claw_shorts::style::Style;
use claw_shorts::{brief, kit};

#[derive(Parser)]
#[command(name = "claw-shorts", version, about = "Plan, validate and render voiced vertical Shorts. Never uploads.")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Read a project and write a spec template plus evidence candidates.
    Brief {
        /// Project directory (README, Cargo.toml, docs/).
        #[arg(long)]
        repo: PathBuf,
        /// New directory for short.template.json and evidence.md.
        #[arg(long)]
        output: PathBuf,
    },
    /// Validate a spec: placeholders, unsupported claims, glyphs, structure.
    Check {
        #[arg(long)]
        spec: PathBuf,
        /// Standalone brand recipe that replaces the spec's own style.
        #[arg(long)]
        style: Option<PathBuf>,
        /// Treat warnings as errors.
        #[arg(long)]
        strict: bool,
    },
    /// Generate (or reuse) one narration WAV per scene.
    Narrate {
        #[arg(long)]
        spec: PathBuf,
        #[arg(long)]
        work: PathBuf,
        #[arg(long)]
        style: Option<PathBuf>,
        #[arg(long)]
        force_tts: bool,
    },
    /// Narrate, render the video, verify it, and write a contact sheet and posting kit.
    Render {
        #[arg(long)]
        spec: PathBuf,
        /// Scratch directory for audio, text files and frames (reusable).
        #[arg(long)]
        work: PathBuf,
        /// New directory for short.mp4 and its companions.
        #[arg(long)]
        output: PathBuf,
        #[arg(long)]
        style: Option<PathBuf>,
        #[arg(long)]
        force_tts: bool,
        #[arg(long)]
        strict: bool,
    },
    /// Write only the posting kit (no render).
    Kit {
        #[arg(long)]
        spec: PathBuf,
        #[arg(long)]
        output: PathBuf,
    },
}

fn load(spec_path: &Path, style_path: Option<&Path>) -> Result<(Spec, PathBuf), String> {
    let json = fs::read_to_string(spec_path).map_err(|e| format!("cannot read {}: {e}", spec_path.display()))?;
    let mut spec = spec::parse(&json)?;
    if let Some(path) = style_path {
        let style_json = fs::read_to_string(path).map_err(|e| format!("cannot read {}: {e}", path.display()))?;
        spec.style = serde_json::from_str::<Style>(&style_json).map_err(|e| format!("style is not valid: {e}"))?;
    }
    let dir = spec_path.parent().filter(|p| !p.as_os_str().is_empty()).unwrap_or(Path::new(".")).to_path_buf();
    Ok((spec, dir))
}

/// Print every issue; return an error when the spec must not proceed.
fn report_issues(spec: &Spec, strict: bool) -> Result<(), String> {
    let issues = spec.validate();
    for issue in &issues {
        let tag = match issue.severity {
            Severity::Error => "error",
            Severity::Warn => "warning",
        };
        eprintln!("{tag}: {}", issue.message);
    }
    let errors = issues.iter().filter(|i| i.severity == Severity::Error).count();
    let warnings = issues.len() - errors;
    if errors > 0 {
        return Err(format!("{errors} error(s), {warnings} warning(s)"));
    }
    if strict && warnings > 0 {
        return Err(format!("{warnings} warning(s) under --strict"));
    }
    Ok(())
}

fn run(cli: Cli) -> Result<(), String> {
    match cli.command {
        Command::Brief { repo, output } => {
            let (spec_path, evidence, facts) = brief::write_brief(&repo, &output)?;
            println!("Wrote {}", spec_path.display());
            println!("Wrote {}", evidence.display());
            if !facts.license_file {
                println!("warning: no LICENSE file found near the project; add one before linking the repo");
            }
            println!("\nThe template is intentionally unrenderable. Replace every TODO with copy");
            println!("backed by evidence.md, then run `claw-shorts check`.");
            Ok(())
        }
        Command::Check { spec: path, style, strict } => {
            let (spec, _) = load(&path, style.as_deref())?;
            report_issues(&spec, strict)?;
            let words: usize = spec.scenes.iter().map(|s| s.narration.split_whitespace().count()).sum();
            println!(
                "ok: {} scene(s), about {} s of narration (estimate from {words} words)",
                spec.scenes.len(),
                words * 60 / 150
            );
            Ok(())
        }
        Command::Narrate { spec: path, work, style, force_tts } => {
            let (spec, dir) = load(&path, style.as_deref())?;
            render::blocking_errors(&spec)?;
            let opts = RenderOptions { spec_dir: dir, work_dir: work, output_dir: PathBuf::new(), force_tts };
            for (i, wav) in render::narrate(&spec, &opts)?.iter().enumerate() {
                println!("scene {}: {}", i + 1, wav.display());
            }
            Ok(())
        }
        Command::Render { spec: path, work, output, style, force_tts, strict } => {
            let (spec, dir) = load(&path, style.as_deref())?;
            report_issues(&spec, strict)?;
            let opts = RenderOptions { spec_dir: dir, work_dir: work, output_dir: output, force_tts };
            let report = render::render(&spec, &opts)?;
            println!("video:        {}", report.video.display());
            println!("contact:      {}", report.contact_sheet.display());
            println!("captions:     {}", report.captions_srt.display());
            println!("timeline:     {}", report.timeline_json.display());
            println!("posting kit:  {}", report.posting_kit_md.display());
            println!("length:       {:.1} s", report.total_ms as f64 / 1000.0);
            println!("\nStructure verified. Listen to the audio and view contact.png before posting.");
            Ok(())
        }
        Command::Kit { spec: path, output } => {
            let (spec, _) = load(&path, None)?;
            render::blocking_errors(&spec)?;
            let (json, md) = kit::write_kit(&kit::build(&spec, None), &output)?;
            println!("Wrote {}\nWrote {}", json.display(), md.display());
            Ok(())
        }
    }
}

fn main() -> ExitCode {
    match run(Cli::parse()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("claw-shorts: {message}");
            ExitCode::FAILURE
        }
    }
}
