//! `voxcpm-tts` command-line interface.

use std::io::{BufRead, IsTerminal, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::time::{Duration, Instant};

use anyhow::{Context, Result, bail};
use clap::{Args, Parser, Subcommand};
use indicatif::{ProgressBar, ProgressStyle};

use voxcpm_tts::audio::{StitchOptions, WavFormat};
use voxcpm_tts::config::{self, MODEL_ID};
use voxcpm_tts::engine::{self, Cancelled, Engine, EngineKind, GenerationParams};
use voxcpm_tts::eta::{Calibration, Estimator, format_duration};
use voxcpm_tts::job::{self, ChunkReport, JobReport, JobSpec, ProgressSink, RetryPolicy};
use voxcpm_tts::text::{Chunk, ChunkOptions, chunk_text};

static CANCEL: AtomicBool = AtomicBool::new(false);
static INTERRUPTS: AtomicUsize = AtomicUsize::new(0);

#[derive(Parser)]
#[command(
    name = "voxcpm-tts",
    version,
    about = "Clone a voice and read text aloud with VoxCPM2 on Apple Silicon"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Synthesize one script to a single WAV file (loads, generates, frees).
    Say(SayArgs),
    /// Keep the model loaded and synthesize several scripts; weights are freed on quit.
    Session(SessionArgs),
    /// Show how a script will be chunked and how long it should take, without loading the model.
    Plan(PlanArgs),
    /// Check the installation: engines, model weights, calibration.
    Doctor,
    /// Download the model weights into the Hugging Face cache.
    Fetch {
        #[arg(long, default_value = MODEL_ID)]
        model: String,
    },
}

#[derive(Args, Clone)]
struct VoiceArgs {
    /// Reference recording of ONE speaker (.wav, .mp3, .flac, .m4a); 5–20 s of clean speech is ideal.
    #[arg(long, short = 'v')]
    voice: Option<PathBuf>,
    /// Exact transcript of the reference recording (improves similarity).
    #[arg(long, conflicts_with = "transcript_file")]
    transcript: Option<String>,
    /// File containing the exact transcript of the reference recording.
    #[arg(long)]
    transcript_file: Option<PathBuf>,
}

#[derive(Args, Clone)]
struct GenArgs {
    /// Inference engine (default: native if built, otherwise mlx-python).
    #[arg(long, value_enum)]
    engine: Option<EngineKind>,
    #[arg(long, default_value = MODEL_ID)]
    model: String,
    /// Maximum characters per synthesis chunk.
    #[arg(long, default_value_t = config::DEFAULT_MAX_CHARS)]
    max_chars: usize,
    /// Diffusion steps per frame (higher = slower, slightly cleaner).
    #[arg(long, default_value_t = config::DEFAULT_INFERENCE_TIMESTEPS)]
    timesteps: usize,
    /// Classifier-free guidance strength.
    #[arg(long, default_value_t = config::DEFAULT_CFG_VALUE)]
    cfg: f32,
    /// Seed for reproducible output.
    #[arg(long)]
    seed: Option<u64>,
    /// Write 32-bit float WAV instead of 16-bit PCM.
    #[arg(long)]
    float: bool,
    /// Retries for a chunk whose length is implausible for its text.
    #[arg(long, default_value_t = 2)]
    retries: usize,
    /// Copy the per-chunk WAVs into this directory instead of discarding them.
    #[arg(long)]
    keep_parts: Option<PathBuf>,
    /// Write a JSON report of the job here.
    #[arg(long)]
    report: Option<PathBuf>,
    /// Pause after a sentence boundary between chunks (seconds).
    #[arg(long, default_value_t = 0.30)]
    sentence_gap: f64,
    /// Pause after a paragraph boundary between chunks (seconds).
    #[arg(long, default_value_t = 0.65)]
    paragraph_gap: f64,
}

#[derive(Args)]
struct SayArgs {
    /// Path to a UTF-8 text file to read aloud.
    #[arg(conflicts_with_all = ["text", "edit"])]
    text_file: Option<PathBuf>,
    /// Text to read aloud, given inline.
    #[arg(long, conflicts_with = "edit")]
    text: Option<String>,
    /// Type the text in your editor ($VISUAL/$EDITOR, default nano).
    #[arg(long)]
    edit: bool,
    /// Output WAV (default: next to the text file, or ./voxcpm-<time>.wav).
    #[arg(long, short = 'o')]
    output: Option<PathBuf>,
    /// Overwrite the output if it exists.
    #[arg(long)]
    force: bool,
    #[command(flatten)]
    voice: VoiceArgs,
    #[command(flatten)]
    gen_args: GenArgs,
}

#[derive(Args)]
struct SessionArgs {
    /// Directory for outputs (default: next to each text file / current directory).
    #[arg(long)]
    out_dir: Option<PathBuf>,
    #[command(flatten)]
    voice: VoiceArgs,
    #[command(flatten)]
    gen_args: GenArgs,
}

#[derive(Args)]
struct PlanArgs {
    text_file: PathBuf,
    #[arg(long, default_value_t = config::DEFAULT_MAX_CHARS)]
    max_chars: usize,
    #[arg(long, value_enum)]
    engine: Option<EngineKind>,
    #[arg(long, default_value_t = config::DEFAULT_INFERENCE_TIMESTEPS)]
    timesteps: usize,
    /// Print every chunk's text.
    #[arg(long)]
    verbose: bool,
}

fn main() {
    let _ = ctrlc::set_handler(|| {
        if INTERRUPTS.fetch_add(1, Ordering::SeqCst) == 0 {
            CANCEL.store(true, Ordering::SeqCst);
            eprintln!("\nStopping after cleanup… (press Ctrl-C again to quit immediately)");
        } else {
            std::process::exit(130);
        }
    });
    let cli = Cli::parse();
    let result = match cli.command {
        Command::Say(a) => cmd_say(a),
        Command::Session(a) => cmd_session(a),
        Command::Plan(a) => cmd_plan(a),
        Command::Doctor => cmd_doctor(),
        Command::Fetch { model } => cmd_fetch(&model),
    };
    if let Err(e) = result {
        if e.chain().any(|c| c.is::<Cancelled>()) {
            eprintln!("Cancelled. Intermediate files were removed and the model was unloaded.");
            std::process::exit(130);
        }
        eprintln!("error: {e:#}");
        std::process::exit(1);
    }
}

fn calibration_path() -> PathBuf {
    config::app_support_dir().join("calibration.json")
}

fn resolve_engine(kind: Option<EngineKind>) -> EngineKind {
    kind.unwrap_or(if voxcpm_tts::native::available() {
        EngineKind::Native
    } else {
        EngineKind::MlxPython
    })
}

fn engine_name(kind: EngineKind) -> &'static str {
    match kind {
        EngineKind::Native => "native",
        EngineKind::MlxPython => "mlx-python",
    }
}

fn read_text_file(path: &Path) -> Result<String> {
    let bytes = std::fs::read(path).with_context(|| format!("reading {}", path.display()))?;
    String::from_utf8(bytes).with_context(|| format!("{} is not UTF-8 text", path.display()))
}

fn read_transcript(v: &VoiceArgs) -> Result<Option<String>> {
    match (&v.transcript, &v.transcript_file) {
        (Some(t), _) => Ok(Some(t.clone())),
        (None, Some(p)) => Ok(Some(read_text_file(p)?)),
        (None, None) => Ok(None),
    }
}

/// Open the user's editor on a scratch file and return what they typed.
fn text_from_editor(initial: &str) -> Result<String> {
    let file = tempfile::Builder::new()
        .prefix("voxcpm-tts-script-")
        .suffix(".txt")
        .tempfile()?;
    std::fs::write(file.path(), initial)?;
    let editor = std::env::var("VISUAL")
        .or_else(|_| std::env::var("EDITOR"))
        .unwrap_or_else(|_| "nano".to_string());
    let mut parts = editor.split_whitespace();
    let program = parts.next().unwrap_or("nano");
    let status = std::process::Command::new(program)
        .args(parts)
        .arg(file.path())
        .status()
        .with_context(|| format!("launching editor '{editor}'"))?;
    if !status.success() {
        bail!("editor exited with {status}");
    }
    Ok(std::fs::read_to_string(file.path())?)
}

fn default_output(text_file: Option<&Path>, out_dir: Option<&Path>) -> PathBuf {
    let name = match text_file.and_then(|p| p.file_stem()) {
        Some(stem) => format!("{}.wav", stem.to_string_lossy()),
        None => {
            let secs = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0);
            format!("voxcpm-{secs}.wav")
        }
    };
    match (out_dir, text_file.and_then(|p| p.parent())) {
        (Some(d), _) => d.join(name),
        (None, Some(parent)) if !parent.as_os_str().is_empty() => parent.join(name),
        _ => PathBuf::from(name),
    }
}

fn job_spec(
    text: String,
    output: PathBuf,
    force: bool,
    voice: &VoiceArgs,
    g: &GenArgs,
) -> Result<JobSpec> {
    Ok(JobSpec {
        text,
        voice: voice.voice.clone(),
        voice_transcript: read_transcript(voice)?,
        output,
        overwrite: force,
        chunking: ChunkOptions {
            max_chars: g.max_chars,
            ..ChunkOptions::default()
        },
        params: GenerationParams {
            cfg_value: g.cfg,
            inference_timesteps: g.timesteps,
            seed: g.seed,
            max_audio_seconds: 0.0,
        },
        stitching: StitchOptions {
            sentence_gap: g.sentence_gap,
            paragraph_gap: g.paragraph_gap,
            ..StitchOptions::default()
        },
        wav_format: if g.float {
            WavFormat::Float32
        } else {
            WavFormat::Pcm16
        },
        keep_parts: g.keep_parts.clone(),
        retry: RetryPolicy {
            max_retries: g.retries,
            ..RetryPolicy::default()
        },
        work_root: None,
    })
}

/// Terminal progress: a bar over characters with chunk, audio and ETA.
struct TerminalProgress {
    bar: ProgressBar,
    total_chunks: AtomicUsize,
    started: Instant,
}

impl TerminalProgress {
    fn new() -> Self {
        let bar = ProgressBar::new_spinner();
        bar.enable_steady_tick(Duration::from_millis(120));
        if !std::io::stderr().is_terminal() {
            bar.set_draw_target(indicatif::ProgressDrawTarget::hidden());
        }
        Self {
            bar,
            total_chunks: AtomicUsize::new(0),
            started: Instant::now(),
        }
    }
}

impl ProgressSink for TerminalProgress {
    fn loading(&self, estimate: f64) {
        self.bar.set_message(format!(
            "Loading VoxCPM2 weights (≈{})…",
            format_duration(estimate)
        ));
    }
    fn loaded(&self, secs: f64) {
        self.bar
            .println(format!("✓ Model loaded in {}", format_duration(secs)));
    }
    fn planned(&self, chunks: &[Chunk], estimate: f64) {
        let total: usize = chunks.iter().map(Chunk::char_len).sum();
        self.total_chunks.store(chunks.len(), Ordering::SeqCst);
        self.bar.println(format!(
            "Script: {total} characters in {} chunk(s). Estimated time: {}",
            chunks.len(),
            format_duration(estimate)
        ));
        self.bar.set_style(
            ProgressStyle::with_template("{spinner} [{bar:32.cyan/blue}] {percent:>3}% {msg}")
                .expect("valid template")
                .progress_chars("█▉▊▋▌▍▎▏ "),
        );
        self.bar.set_length(total as u64);
        self.bar.set_position(0);
    }
    fn chunk_started(&self, chunk: &Chunk, attempt: usize) {
        let retry = if attempt > 1 {
            format!(" (retry {})", attempt - 1)
        } else {
            String::new()
        };
        self.bar.set_message(format!(
            "chunk {}/{}{retry}",
            chunk.index,
            self.total_chunks.load(Ordering::SeqCst)
        ));
    }
    fn tick(&self, chunk: &Chunk, done_chars: usize, audio_seconds: f64, eta: f64) {
        self.bar.set_position(done_chars as u64);
        self.bar.set_message(format!(
            "chunk {}/{} · {:.1}s audio · elapsed {} · ETA {}",
            chunk.index,
            self.total_chunks.load(Ordering::SeqCst),
            audio_seconds,
            format_duration(self.started.elapsed().as_secs_f64()),
            format_duration(eta)
        ));
    }
    fn chunk_finished(&self, r: &ChunkReport, done_chars: usize, eta: f64) {
        self.bar.set_position(done_chars as u64);
        self.bar.println(format!(
            "  chunk {}/{}: {:.1}s of speech in {} · remaining ≈ {}",
            r.index,
            self.total_chunks.load(Ordering::SeqCst),
            r.audio_seconds,
            format_duration(r.compute_seconds),
            format_duration(eta)
        ));
    }
    fn retrying(&self, chunk: &Chunk, reason: &str) {
        self.bar.println(format!(
            "  ↻ chunk {}: {reason}; retrying with a new seed",
            chunk.index
        ));
    }
    fn stitching(&self) {
        self.bar.set_message("stitching chunks into one WAV…");
    }
    fn warning(&self, message: &str) {
        self.bar.println(format!("  ⚠ {message}"));
    }
}

fn print_summary(report: &JobReport) {
    println!(
        "✓ Wrote {} ({} of audio, {} chunk(s), generated in {}, {:.2}× real time)",
        report.output.display(),
        format_duration(report.audio_seconds),
        report.chunks.len(),
        format_duration(report.compute_seconds),
        report.real_time_factor
    );
}

fn run_with_progress(
    engine: &mut dyn Engine,
    spec: &JobSpec,
    report_path: Option<&Path>,
) -> Result<JobReport> {
    if let Some(path) = report_path {
        job::check_report(path)?;
    }
    let mut calibration = Calibration::load(&calibration_path());
    let progress = TerminalProgress::new();
    let result = job::run_job(engine, spec, &progress, &CANCEL, &mut calibration);
    progress.bar.finish_and_clear();
    let report = result?;
    if let Some(p) = report_path {
        job::write_report(p, &report)?;
    }
    print_summary(&report);
    Ok(report)
}

fn cmd_say(a: SayArgs) -> Result<()> {
    let text = match (&a.text_file, &a.text, a.edit) {
        (Some(p), _, _) => read_text_file(p)?,
        (None, Some(t), _) => t.clone(),
        (None, None, true) => text_from_editor("")?,
        (None, None, false) => bail!("give a text file path, --text \"…\", or --edit"),
    };
    let output = a
        .output
        .clone()
        .unwrap_or_else(|| default_output(a.text_file.as_deref(), None));
    let spec = job_spec(text, output, a.force, &a.voice, &a.gen_args)?;
    job::check_output(&spec.output, spec.overwrite)?;
    if spec.voice.is_none() {
        eprintln!("note: no --voice given; VoxCPM2 will invent a voice");
    }
    let mut engine = engine::create(resolve_engine(a.gen_args.engine), &a.gen_args.model)?;
    let result = run_with_progress(engine.as_mut(), &spec, a.gen_args.report.as_deref());
    engine.unload()?; // weights are freed before the process exits
    result.map(|_| ())
}

const SESSION_HELP: &str = "\
Type a path to a .txt file to read it aloud, or one of:
  :edit              write text in your editor, then synthesize it
  :text <words>      synthesize the words on this line
  :voice <path>      switch the reference voice (:voice none to clear)
  :transcript <txt>  set the reference transcript (:transcript none to clear)
  :out <dir>         directory for outputs
  :unload            free the model now (it reloads on the next job)
  :help              this help
  :quit              free the model and exit (also Ctrl-D)";

fn cmd_session(a: SessionArgs) -> Result<()> {
    let mut voice = a.voice.clone();
    let mut out_dir = a.out_dir.clone();
    let kind = resolve_engine(a.gen_args.engine);
    let mut engine = engine::create(kind, &a.gen_args.model)?;
    println!(
        "voxcpm-tts session · engine {} · model {}",
        engine_name(kind),
        a.gen_args.model
    );
    if let Some(v) = &voice.voice {
        println!("voice: {}", v.display());
    }
    // Load up front so later jobs start immediately.
    let mut calibration = Calibration::load(&calibration_path());
    let key = Calibration::key(engine.name(), a.gen_args.timesteps);
    let spinner = ProgressBar::new_spinner();
    spinner.enable_steady_tick(Duration::from_millis(120));
    spinner.set_message(format!(
        "Loading VoxCPM2 weights (≈{})…",
        format_duration(calibration.get(&key).load_secs)
    ));
    let secs = engine.load_cancellable(&CANCEL)?;
    spinner.finish_and_clear();
    calibration.record_load(&key, secs);
    calibration.save();
    println!(
        "✓ Model loaded in {}. {SESSION_HELP}",
        format_duration(secs)
    );

    let stdin = std::io::stdin();
    loop {
        CANCEL.store(false, Ordering::SeqCst);
        INTERRUPTS.store(0, Ordering::SeqCst);
        print!("\nvoxcpm> ");
        std::io::stdout().flush()?;
        let mut line = String::new();
        if stdin.lock().read_line(&mut line)? == 0 {
            break;
        }
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let (cmd, rest) = line
            .split_once(' ')
            .map(|(c, r)| (c, r.trim()))
            .unwrap_or((line, ""));
        let job_input: Option<(String, Option<PathBuf>)> = match cmd {
            ":quit" | ":q" | ":exit" => break,
            ":help" | ":h" => {
                println!("{SESSION_HELP}");
                None
            }
            ":unload" => {
                engine.unload()?;
                println!("Model unloaded; memory released.");
                None
            }
            ":voice" => {
                voice.voice = (rest != "none" && !rest.is_empty()).then(|| PathBuf::from(rest));
                println!(
                    "voice: {}",
                    voice
                        .voice
                        .as_ref()
                        .map(|p| p.display().to_string())
                        .unwrap_or("none".into())
                );
                None
            }
            ":transcript" => {
                voice.transcript = (rest != "none" && !rest.is_empty()).then(|| rest.to_string());
                voice.transcript_file = None;
                None
            }
            ":out" => {
                out_dir = Some(PathBuf::from(rest));
                None
            }
            ":edit" => match text_from_editor("") {
                Ok(t) => Some((t, None)),
                Err(e) => {
                    eprintln!("error: {e:#}");
                    None
                }
            },
            ":text" => Some((rest.to_string(), None)),
            _ if cmd.starts_with(':') => {
                println!("unknown command; :help lists commands");
                None
            }
            _ => {
                let path = PathBuf::from(line.trim_matches(|c| c == '\'' || c == '"'));
                match read_text_file(&path) {
                    Ok(t) => Some((t, Some(path))),
                    Err(e) => {
                        eprintln!("error: {e:#}");
                        None
                    }
                }
            }
        };
        let Some((text, source)) = job_input else {
            continue;
        };
        let mut output = default_output(source.as_deref(), out_dir.as_deref());
        let mut n = 1;
        while output.exists() {
            n += 1;
            let stem = output
                .file_stem()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string();
            let base = stem
                .rsplit_once('-')
                .filter(|(_, d)| d.parse::<u32>().is_ok())
                .map(|(b, _)| b.to_string())
                .unwrap_or(stem);
            output.set_file_name(format!("{base}-{n}.wav"));
        }
        let result = job_spec(text, output, false, &voice, &a.gen_args).and_then(|spec| {
            run_with_progress(engine.as_mut(), &spec, a.gen_args.report.as_deref())
        });
        if let Err(e) = result {
            if e.chain().any(|c| c.is::<Cancelled>()) {
                println!("Job cancelled; intermediate files removed. Model stays loaded.");
            } else {
                eprintln!("error: {e:#}");
            }
        }
    }
    engine.unload()?;
    println!("Model unloaded; memory released. Bye.");
    Ok(())
}

fn cmd_plan(a: PlanArgs) -> Result<()> {
    let text = read_text_file(&a.text_file)?;
    let chunks = chunk_text(
        &text,
        ChunkOptions {
            max_chars: a.max_chars,
            ..ChunkOptions::default()
        },
    );
    let kind = resolve_engine(a.engine);
    let calibration = Calibration::load(&calibration_path());
    let cal = calibration.get(&Calibration::key(engine_name(kind), a.timesteps));
    let total: usize = chunks.iter().map(Chunk::char_len).sum();
    let est = Estimator::new(cal, total).remaining_secs(0, 0.0);
    let speech: f64 = chunks
        .iter()
        .map(|c| job::expected_speech_seconds(&c.text))
        .sum();
    println!(
        "{} characters → {} chunk(s) (max {} chars)",
        total,
        chunks.len(),
        a.max_chars
    );
    println!("Expected speech: ≈{}", format_duration(speech));
    println!(
        "Estimated generation time with {}: ≈{} (+ ≈{} to load){}",
        engine_name(kind),
        format_duration(est),
        format_duration(cal.load_secs),
        if cal.samples == 0 {
            " — first-run guess; improves after a job"
        } else {
            ""
        }
    );
    for c in &chunks {
        if a.verbose {
            println!(
                "\n[{}] ({} chars, pause {:?})\n{}",
                c.index,
                c.char_len(),
                c.pause_after,
                c.text
            );
        } else {
            let preview: String = c.text.chars().take(70).collect();
            println!(
                "  [{:>3}] {:>4} chars  {}{}",
                c.index,
                c.char_len(),
                preview.replace('\n', " "),
                if c.char_len() > 70 { "…" } else { "" }
            );
        }
    }
    Ok(())
}

fn cmd_doctor() -> Result<()> {
    println!("voxcpm-tts {}", env!("CARGO_PKG_VERSION"));
    println!("app data:       {}", config::app_support_dir().display());
    match config::model_snapshot_dir(MODEL_ID) {
        Some(d) => println!("model weights:  ✓ {}", d.display()),
        None => println!("model weights:  ✗ not downloaded (run `voxcpm-tts fetch`)"),
    }
    println!(
        "native engine:  {}",
        if voxcpm_tts::native::available() {
            "✓ available"
        } else {
            "✗ not available in this build"
        }
    );
    match engine::worker::locate_worker() {
        Ok((py, script)) => println!("mlx-python:     ✓ {} {}", py.display(), script.display()),
        Err(e) => println!("mlx-python:     ✗ {e}"),
    }
    let cal = Calibration::load(&calibration_path());
    for kind in [EngineKind::Native, EngineKind::MlxPython] {
        let c = cal.get(&Calibration::key(
            engine_name(kind),
            config::DEFAULT_INFERENCE_TIMESTEPS,
        ));
        if c.samples > 0 {
            println!(
                "calibration:    {} {:.3}s/char, load {} ({} jobs)",
                engine_name(kind),
                c.secs_per_char,
                format_duration(c.load_secs),
                c.samples
            );
        }
    }
    Ok(())
}

fn cmd_fetch(model: &str) -> Result<()> {
    if let Some(d) = config::model_snapshot_dir(model) {
        println!("Already downloaded: {}", d.display());
        return Ok(());
    }
    let dir = voxcpm_tts::hub::fetch_model(model)?;
    println!("✓ {model} → {}", dir.display());
    Ok(())
}
