use anyhow::{Result, ensure};
use clap::{Args, Parser, Subcommand};
use claw_weight_space::{
    assets,
    edit::Patch,
    evaluation,
    gguf::Gguf,
    projection,
    server::{OwnedServer, ServerConfig},
    write_json_new,
};
use std::{fs::File, path::PathBuf};

#[derive(Parser)]
#[command(
    name = "claw-wsp",
    version,
    about = "Copy-only BF16 editing and controlled local weight-space experiments"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}
#[derive(Args)]
struct Backend {
    /// Full local Docker image ID (sha256:...), never an automatically pulled tag.
    #[arg(long)]
    image: String,
    /// Explicit GPU UUID or index, as shown by nvidia-smi.
    #[arg(long)]
    gpu: String,
    #[arg(long, default_value_t = 8097)]
    port: u16,
    #[arg(long, default_value_t = 40960)]
    context: usize,
}
impl Backend {
    fn config(self) -> ServerConfig {
        ServerConfig {
            image: self.image,
            gpu: self.gpu,
            port: self.port,
            context: self.context,
        }
    }
}
#[derive(Subcommand)]
enum Command {
    Inspect {
        #[arg(long)]
        model: PathBuf,
    },
    Fetch {
        #[arg(long)]
        asset: String,
        #[arg(long)]
        output: PathBuf,
    },
    Patch {
        #[arg(long)]
        model: PathBuf,
        #[arg(long)]
        manifest: PathBuf,
        #[arg(long)]
        patch: PathBuf,
        #[arg(long)]
        output: PathBuf,
    },
    Audit {
        #[arg(long)]
        model: PathBuf,
        #[arg(long)]
        edited: PathBuf,
        #[arg(long)]
        manifest: PathBuf,
        #[arg(long)]
        patch: PathBuf,
    },
    Cases {
        #[arg(long)]
        seed: u64,
        #[arg(long, default_value_t = 24)]
        count: usize,
        #[arg(long)]
        output: PathBuf,
    },
    Evaluate {
        #[arg(long)]
        model: PathBuf,
        #[arg(long)]
        cases: PathBuf,
        #[arg(long)]
        output: PathBuf,
        #[arg(long)]
        arm: String,
        #[arg(long, default_value_t = 8192)]
        max_tokens: usize,
        #[arg(long)]
        no_thinking: bool,
        #[command(flatten)]
        backend: Backend,
    },
    Compare {
        #[arg(long)]
        left: PathBuf,
        #[arg(long)]
        right: PathBuf,
        #[arg(long)]
        output: Option<PathBuf>,
    },
    Project {
        #[arg(long)]
        input: PathBuf,
        #[arg(long)]
        output: PathBuf,
    },
    /// Start only an isolated server; Enter/EOF stops the owned container.
    Serve {
        #[arg(long)]
        model: PathBuf,
        #[arg(long)]
        log: PathBuf,
        #[command(flatten)]
        backend: Backend,
    },
}

#[derive(serde::Deserialize)]
struct ProjectionInput {
    weights: Vec<f32>,
    rows: usize,
    cols: usize,
    direction: Vec<f32>,
    selected: Vec<usize>,
    alpha: f32,
}
fn print(value: &impl serde::Serialize) -> Result<()> {
    println!("{}", serde_json::to_string_pretty(value)?);
    Ok(())
}
fn main() -> Result<()> {
    match Cli::parse().command {
        Command::Inspect { model } => print(&Gguf::inspect(&model)?)?,
        Command::Fetch { asset, output } => {
            assets::fetch(&asset, &output)?;
            print(
                &serde_json::json!({"asset":asset,"sha256":claw_weight_space::hash_file(&output)?}),
            )?;
        }
        Command::Patch {
            model,
            manifest,
            patch,
            output,
        } => {
            let (_, r) = Patch::load(&manifest, &patch)?.apply(&model, &output)?;
            print(&r)?;
        }
        Command::Audit {
            model,
            edited,
            manifest,
            patch,
        } => print(&Patch::load(&manifest, &patch)?.audit(&model, &edited)?)?,
        Command::Cases {
            seed,
            count,
            output,
        } => write_json_new(&output, &evaluation::synthetic_cases(seed, count)?)?,
        Command::Evaluate {
            model,
            cases,
            output,
            arm,
            max_tokens,
            no_thinking,
            backend,
        } => {
            let summary = evaluation::run(
                &model,
                &cases,
                &output,
                &arm,
                max_tokens,
                !no_thinking,
                backend.config(),
            )?;
            print(&summary)?;
            ensure!(
                summary["errors"].as_u64() == Some(0),
                "evaluation contains infrastructure/protocol errors; inspect retained results"
            );
        }
        Command::Compare {
            left,
            right,
            output,
        } => {
            let result = evaluation::compare(&left, &right)?;
            if let Some(out) = output {
                write_json_new(&out, &result)?;
            }
            print(&result)?;
        }
        Command::Project { input, output } => {
            ensure!(
                std::fs::metadata(&input)?.len() <= 64 * 1024 * 1024,
                "projection input too large"
            );
            let p: ProjectionInput = serde_json::from_reader(File::open(input)?)?;
            write_json_new(
                &output,
                &projection::project(
                    &p.weights,
                    p.rows,
                    p.cols,
                    &p.direction,
                    &p.selected,
                    p.alpha,
                )?,
            )?;
        }
        Command::Serve {
            model,
            log,
            backend,
        } => {
            let s = OwnedServer::start(&model, &log, &backend.config())?;
            println!("{} ready; Enter or EOF stops this experiment server", s.url);
            let mut line = String::new();
            std::io::stdin().read_line(&mut line)?;
        }
    }
    Ok(())
}
