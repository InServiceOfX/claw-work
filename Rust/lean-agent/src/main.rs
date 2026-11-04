use clap::{Parser, Subcommand};
use claw_lean_agent::{
    Result,
    input::{InventoryLimits, inventory_target, prepare_review, read_source},
    proof::{DEFAULT_IMAGE, Limits, Manifest, check_proofs, image_metadata, resolve_image},
};
use std::path::PathBuf;

#[derive(Parser)]
#[command(
    version,
    about = "Private local Lean/mathlib tools. Model proof is not source verification."
)]
struct Cli {
    #[arg(long, global=true, default_value=DEFAULT_IMAGE)]
    image: String,
    #[command(subcommand)]
    command: Action,
}
#[derive(Subcommand)]
enum Action {
    /// List bounded source hashes and exclusions. Does not invoke an AI model.
    Inventory { input: PathBuf },
    /// Create a new private source snapshot outside Git trees.
    Prepare {
        input: PathBuf,
        #[arg(long)]
        output: PathBuf,
    },
    /// Independently compare and kernel-check expected model statements.
    Check {
        #[arg(long)]
        manifest: PathBuf,
        #[arg(long)]
        output: PathBuf,
        #[arg(long, default_value_t = 120)]
        timeout_seconds: u64,
        #[arg(long, default_value_t = 4096)]
        memory_mib: u32,
    },
    /// Check local Docker/image prerequisites without pulling anything.
    Doctor,
}
fn run(cli: Cli) -> Result<i32> {
    let limits = Limits {
        image: cli.image,
        ..Default::default()
    };
    match cli.command {
        Action::Inventory { input } => {
            println!(
                "{}",
                serde_json::to_string_pretty(&inventory_target(
                    &input,
                    &InventoryLimits::default()
                )?)?
            );
        }
        Action::Prepare { input, output } => {
            println!(
                "{}",
                serde_json::to_string_pretty(&prepare_review(
                    &input,
                    &output,
                    &InventoryLimits::default()
                )?)?
            );
        }
        Action::Check {
            manifest,
            output,
            timeout_seconds,
            memory_mib,
        } => {
            claw_lean_agent::process::install_cli_signals()?;
            if !(1..=3600).contains(&timeout_seconds) {
                return Err("Deadline must be 1..3600 seconds per phase".into());
            }
            let m: Manifest = serde_json::from_slice(&read_source(&manifest, 1_048_576)?)?;
            let limits = Limits {
                timeout_ms: timeout_seconds * 1000,
                memory_mib,
                ..limits
            };
            let report = check_proofs(
                &m,
                manifest.parent().unwrap_or(std::path::Path::new(".")),
                &output,
                &limits,
            )?;
            let exit = match report.model_proof_status.as_str() {
                "checked" => 0,
                "rejected" => 1,
                _ => 2,
            };
            println!("{}", serde_json::to_string_pretty(&report)?);
            return Ok(exit);
        }
        Action::Doctor => {
            let id = resolve_image(&limits.image)?;
            let pins = image_metadata(&id)?;
            println!(
                "{}",
                serde_json::to_string_pretty(&serde_json::json!({
                    "schema_version":1,"image_id":id,"pins":pins,
                    "model_source_boundary":"Model checks do not prove source correspondence",
                    "network":"Local Docker daemon only; review containers have no network"
                }))?
            );
        }
    }
    Ok(0)
}
fn main() {
    match run(Cli::parse()) {
        Ok(code) => std::process::exit(code),
        Err(e) => {
            eprintln!(
                "{}",
                serde_json::json!({"schema_version":1,"status":"error","error":e.to_string()})
            );
            std::process::exit(2);
        }
    }
}
