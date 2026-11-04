//! Bounded public synthetic metrics; no filesystem cache reset or private source.
use claw_lean_agent::{
    Result,
    process::{run_bounded, run_cleanup},
    proof::{DEFAULT_IMAGE, Limits, docker_args, resolve_image},
};
use serde_json::json;
use std::{fs, process::Command, time::Duration};
struct Own(String);
impl Drop for Own {
    fn drop(&mut self) {
        let _ = run_cleanup(Command::new("docker").args(["rm", "-f", &self.0]));
    }
}
fn main() -> Result<()> {
    let id = resolve_image(DEFAULT_IMAGE)?;
    let temp = tempfile::tempdir()?;
    fs::write(
        temp.path().join("Bench.lean"),
        "import Mathlib.Data.Nat.Basic\nexample (n : Nat) : n + 0 = n := by omega\n",
    )?;
    let limits = Limits::default();
    let mut startup = Vec::new();
    let mut fresh = Vec::new();
    let timing = [
        "/usr/bin/time",
        "-f",
        "wall_s=%e user_s=%U sys_s=%S peak_rss_kib=%M",
        "lean",
        "/input/Bench.lean",
    ];
    for i in 0..5 {
        for version in [true, false] {
            let own = Own(format!("claw-metrics-{}-{i}-{version}", std::process::id()));
            let args = docker_args(&id, &own.0, Some(temp.path()), &limits)?;
            let command: Vec<&str> = if version {
                vec!["version"]
            } else {
                vec![
                    "shell",
                    "-c",
                    "/usr/bin/time -f 'wall_s=%e user_s=%U sys_s=%S peak_rss_kib=%M' lean /input/Bench.lean",
                ]
            };
            let r = run_bounded(
                Command::new("docker").args(args).args(command),
                Duration::from_secs(60),
                65536,
            )?;
            if r.exit_code != Some(0) || r.timed_out || r.output_limited || r.cancelled {
                return Err("Metric subprocess did not complete".into());
            }
            if version {
                startup.push(r.wall_ms);
            } else {
                fresh.push(
                    json!({"docker_wall_ms":r.wall_ms,"lean":String::from_utf8(r.stderr)?.trim()}),
                );
            }
        }
    }
    let own = Own(format!("claw-metrics-persistent-{}", std::process::id()));
    let mut args = docker_args(&id, &own.0, Some(temp.path()), &limits)?;
    args.insert(1, "-d".into());
    let run = run_bounded(
        Command::new("docker")
            .args(args)
            .args(["shell", "-c", "exec sleep 180"]),
        Duration::from_secs(30),
        4096,
    )?;
    if run.exit_code != Some(0) {
        return Err("Persistent metric container failed".into());
    }
    let stats = run_bounded(
        Command::new("docker").args([
            "stats",
            "--no-stream",
            "--format",
            "{{.CPUPerc}} {{.MemUsage}}",
            &own.0,
        ]),
        Duration::from_secs(10),
        4096,
    )?;
    let mut warm = Vec::new();
    for _ in 0..5 {
        let r = run_bounded(
            Command::new("docker").args(["exec", &own.0]).args(timing),
            Duration::from_secs(60),
            65536,
        )?;
        if r.exit_code != Some(0) {
            return Err(
                format!("Exec metric failed: {}", String::from_utf8_lossy(&r.stderr)).into(),
            );
        }
        warm.push(json!({"docker_wall_ms":r.wall_ms,"lean":String::from_utf8(r.stderr)?.trim()}));
    }
    println!(
        "{}",
        serde_json::to_string_pretty(&json!({
            "schema_version":1,"image_id":id,"version_startup_wall_ms":startup,
            "fresh_container_proof":fresh,"persistent_container_exec_proof":warm,
            "idle_docker_cpu_memory":String::from_utf8(stats.stdout)?.trim(),
            "context":"Linux x86_64; 2 CPU/4096MiB limits; five descriptive samples; uncontrolled warm host page cache; not a cold boot or a persistent elaborator; raw Lean elaboration, not strict proof-pipeline timing"
        }))?
    );
    Ok(())
}
