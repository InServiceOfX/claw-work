use clap::{Parser, Subcommand};
use shortform_video::config::Config;
use shortform_video::error::Result;
use shortform_video::llm::LlmClient;
use shortform_video::material::Aspect;
use shortform_video::pipeline::{MaterialSource, Pipeline, RunOptions, default_task_dir};
use shortform_video::video::{ConcatMode, FitMode, Transition};
use shortform_video::{subtitle, tts};
use std::path::PathBuf;

#[derive(Parser)]
#[command(
    name = "shortform-video",
    about = "Faceless short-form video assembly: LLM script, TTS narration, stock b-roll, captioned vertical render",
    version
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Generate a narration script for a subject
    Script {
        #[arg(long)]
        subject: String,
        #[arg(long, default_value = "")]
        language: String,
        #[arg(long, default_value_t = 1)]
        paragraphs: u32,
    },
    /// Generate stock-footage search terms for a script
    Terms {
        #[arg(long)]
        subject: String,
        #[arg(long)]
        script_file: PathBuf,
        #[arg(long, default_value_t = 5)]
        amount: usize,
        /// Keep terms in script narrative order
        #[arg(long)]
        ordered: bool,
    },
    /// Synthesize narration audio (writes audio plus cues JSON)
    Tts {
        #[arg(long)]
        script_file: PathBuf,
        #[arg(long, default_value = "narration.mp3")]
        output: PathBuf,
        /// Also write an SRT next to the audio
        #[arg(long)]
        srt: bool,
    },
    /// List Edge TTS voices (network)
    Voices {
        /// Locale prefix filter, e.g. en-US or zh-CN
        #[arg(long)]
        locale: Option<String>,
    },
    /// Generate title/caption/hashtags for a platform
    Social {
        #[arg(long)]
        subject: String,
        #[arg(long)]
        script_file: PathBuf,
        #[arg(long, default_value = "tiktok")]
        platform: String,
    },
    /// Run the full pipeline: script, narration, subtitles, materials, render
    Run {
        #[arg(long)]
        subject: String,
        #[arg(long, default_value = "")]
        language: String,
        #[arg(long, default_value_t = 1)]
        paragraphs: u32,
        /// Use an existing script instead of generating one
        #[arg(long)]
        script_file: Option<PathBuf>,
        /// pexels | pixabay | a local directory of video files
        #[arg(long, default_value = "pexels")]
        source: String,
        #[arg(long, value_enum, default_value_t = Aspect::Portrait)]
        aspect: Aspect,
        #[arg(long, value_enum, default_value_t = FitMode::Cover)]
        fit: FitMode,
        #[arg(long, value_enum, default_value_t = ConcatMode::Random)]
        concat: ConcatMode,
        #[arg(long, value_enum, default_value_t = Transition::None)]
        transition: Transition,
        #[arg(long, default_value_t = 5.0)]
        max_clip_duration: f64,
        #[arg(long, default_value_t = 5)]
        terms_amount: usize,
        /// Loopable background music file (mixed at render.bgm_volume)
        #[arg(long)]
        bgm: Option<PathBuf>,
        #[arg(long)]
        no_subtitles: bool,
        /// Task directory for all artifacts (default: ./shortform-tasks/<ts>)
        #[arg(long)]
        task_dir: Option<PathBuf>,
    },
}

fn main() {
    // Both reqwest and msedge-tts link rustls; with two crypto backends in the
    // tree, rustls needs the process-level provider chosen explicitly.
    let _ = rustls::crypto::ring::default_provider().install_default();
    if let Err(error) = run() {
        eprintln!("error: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let cli = Cli::parse();
    let config = Config::load()?;

    match cli.command {
        Command::Script {
            subject,
            language,
            paragraphs,
        } => {
            let llm = LlmClient::new(config.llm)?;
            println!("{}", llm.generate_script(&subject, &language, paragraphs)?);
        }
        Command::Terms {
            subject,
            script_file,
            amount,
            ordered,
        } => {
            let script = std::fs::read_to_string(script_file)?;
            let llm = LlmClient::new(config.llm)?;
            let terms = llm.generate_terms(&subject, &script, amount, ordered)?;
            println!("{}", serde_json::to_string_pretty(&terms)?);
        }
        Command::Tts {
            script_file,
            output,
            srt,
        } => {
            let script = std::fs::read_to_string(script_file)?;
            let result = tts::synthesize(&config, &script, &output)?;
            eprintln!(
                "wrote {} ({:.2}s, {} word boundaries)",
                result.audio_file.display(),
                result.duration,
                result.word_boundaries.len()
            );
            if srt {
                let lines = subtitle::split_script_lines(&script);
                let cues =
                    subtitle::cues_from_word_boundaries(&result.word_boundaries, &lines)
                        .unwrap_or_else(|| subtitle::cues_proportional(&lines, result.duration));
                let srt_path = output.with_extension("srt");
                subtitle::write_srt(&cues, &srt_path)?;
                eprintln!("wrote {}", srt_path.display());
            }
        }
        Command::Voices { locale } => {
            for voice in tts::list_edge_voices(locale.as_deref())? {
                println!("{voice}");
            }
        }
        Command::Social {
            subject,
            script_file,
            platform,
        } => {
            let script = std::fs::read_to_string(script_file)?;
            let llm = LlmClient::new(config.llm)?;
            let metadata = llm.generate_social_metadata(&subject, &script, &platform)?;
            println!("{}", serde_json::to_string_pretty(&metadata)?);
        }
        Command::Run {
            subject,
            language,
            paragraphs,
            script_file,
            source,
            aspect,
            fit,
            concat,
            transition,
            max_clip_duration,
            terms_amount,
            bgm,
            no_subtitles,
            task_dir,
        } => {
            let material_source = match source.as_str() {
                "pexels" => MaterialSource::Pexels,
                "pixabay" => MaterialSource::Pixabay,
                dir => MaterialSource::LocalDir(PathBuf::from(dir)),
            };
            let task_dir = task_dir.unwrap_or_else(default_task_dir);
            eprintln!("task directory: {}", task_dir.display());
            let pipeline = Pipeline::new(config, task_dir)?;
            let options = RunOptions {
                subject,
                language,
                paragraphs,
                aspect,
                fit,
                concat_mode: concat,
                transition,
                max_clip_duration,
                terms_amount,
                source: material_source,
                script_file,
                bgm_file: bgm,
                subtitles_enabled: !no_subtitles,
            };
            let output = pipeline.run(&options)?;
            println!("{}", output.display());
        }
    }
    Ok(())
}
