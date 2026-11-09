use anyhow::{Result, ensure};
use clap::Parser;
use claw_lm::{
    Assets, Decoder, GenerationOptions, Mode, chat::Chat, cpu::CpuDecoder, generate,
    generation::Attention,
};
use serde_json::{Value, json};
use std::{path::PathBuf, sync::atomic::AtomicBool, time::Instant};
#[derive(Parser)]
struct Args {
    #[arg(long, env = "MODEL_DIR")]
    model_dir: PathBuf,
    #[arg(long, default_value = "cuda")]
    backend: String,
    #[arg(long)]
    device: Option<i32>,
    #[arg(long, default_value_t = 2048)]
    context: usize,
    #[arg(long, default_value = "autoregressive")]
    mode: String,
    #[arg(long, default_value = "What is 2 + 2? Answer briefly.")]
    prompt: String,
    #[arg(long)]
    messages: Option<PathBuf>,
    #[arg(long)]
    tools: Option<PathBuf>,
    #[arg(long, default_value_t = 32)]
    max_tokens: usize,
    #[arg(long, default_value_t = 32)]
    block_length: usize,
    #[arg(long, default_value_t = 0.9)]
    threshold: f32,
    #[arg(long)]
    render_only: bool,
    #[arg(long)]
    dump_logits: Option<PathBuf>,
}
fn main() -> Result<()> {
    let args = Args::parse();
    let started = Instant::now();
    let chat = Chat::load(&args.model_dir)?;
    let messages: Vec<Value> = if let Some(path) = args.messages {
        serde_json::from_slice(&std::fs::read(path)?)?
    } else {
        vec![json!({"role":"user","content":args.prompt})]
    };
    let tools: Vec<Value> = if let Some(path) = args.tools {
        serde_json::from_slice(&std::fs::read(path)?)?
    } else {
        Vec::new()
    };
    let prompt = chat.render(&messages, &tools)?;
    let ids = chat.encode(&prompt)?;
    if args.render_only {
        println!("{}", json!({"prompt":prompt,"token_ids":ids}));
        return Ok(());
    }
    let assets = Assets::open(&args.model_dir)?;
    let eos = assets.config.eos_token_id;
    let mask = assets.config.mask_token_id;
    let mut model: Box<dyn Decoder> = match args.backend.as_str() {
        "cpu" => Box::new(CpuDecoder::new(assets, args.context)?),
        "cuda" => {
            #[cfg(feature = "cuda")]
            {
                Box::new(claw_lm::cuda::CudaDecoder::new(
                    &assets,
                    args.device
                        .ok_or_else(|| anyhow::anyhow!("--device is required for CUDA"))?,
                    args.context,
                )?)
            }
            #[cfg(not(feature = "cuda"))]
            {
                anyhow::bail!("Build with --features cuda");
            }
        }
        _ => anyhow::bail!("Unknown backend"),
    };
    let load_seconds = started.elapsed().as_secs_f64();
    if let Some(path) = args.dump_logits {
        ensure!(
            ids.len() <= 128,
            "Logit dump currently accepts at most 128 prompt IDs"
        );
        let logits = model.forward(&ids, Attention::Causal, true, true)?;
        std::fs::write(
            path,
            serde_json::to_vec(&json!({"token_ids":ids,"logits":logits[0]}))?,
        )?;
        model.reset()?;
    }
    let mode: Mode = serde_json::from_value(json!(args.mode))?;
    let options = GenerationOptions {
        mode,
        max_new_tokens: args.max_tokens,
        block_length: args.block_length,
        threshold: Some(args.threshold),
        temperature: 0.,
        eos_token_id: eos,
        mask_token_id: mask,
    };
    let start = Instant::now();
    let result = generate(
        model.as_mut(),
        &ids,
        &options,
        &AtomicBool::new(false),
        |_| Ok(true),
    )?;
    println!(
        "{}",
        json!({"result":result,"text":chat.decode(&result.tokens)?,"prompt_tokens":ids.len(),"load_seconds":load_seconds,"generation_seconds":start.elapsed().as_secs_f64()})
    );
    Ok(())
}
