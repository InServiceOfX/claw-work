use claw_lean_agent::privacy::{scan_context, scan_index};
use std::path::Path;
fn main() {
    let args: Vec<String> = std::env::args().collect();
    let mode = args.get(1).map(String::as_str).unwrap_or("index");
    let path = args.get(2).map(String::as_str).unwrap_or(".");
    if mode == "artifact-canary" {
        match claw_lean_agent::artifact::image_archive_contains(
            std::io::stdin().lock(),
            path.as_bytes(),
        ) {
            Ok(false) => return,
            Ok(true) => {
                eprintln!("Synthetic canary found in image archive/layer; publication blocked");
                std::process::exit(1);
            }
            Err(e) => {
                eprintln!("Archive gate failed closed: {e}");
                std::process::exit(2);
            }
        }
    }
    let result = match mode {
        "index" => scan_index(Path::new(path)),
        "context" => scan_context(Path::new(path)),
        _ => Err("Use claw-public-check index [repo] or context [Lean project]".into()),
    };
    match result {
        Ok(v) if v.is_empty() => (),
        Ok(v) => {
            eprintln!(
                "{}",
                serde_json::to_string_pretty(&v).expect("diagnostic serialization")
            );
            std::process::exit(1);
        }
        Err(e) => {
            eprintln!("Privacy gate failed closed: {e}");
            std::process::exit(2);
        }
    }
}
