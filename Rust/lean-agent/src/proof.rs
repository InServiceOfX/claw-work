use crate::{
    Result,
    input::{create_private_dir, hash, read_source, write_private},
    process::{ProcessResult, run_bounded, run_cleanup},
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::{
    collections::HashSet,
    fs,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::atomic::{AtomicU64, Ordering},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

pub const DEFAULT_IMAGE: &str = "claw-work/lean-agent:lean4.35.0-mathlib";
pub const PERMITTED_AXIOMS: &[&str] = &["propext", "Classical.choice", "Quot.sound"];
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Manifest {
    pub schema_version: u32,
    pub challenge: PathBuf,
    pub solution: PathBuf,
    pub theorems: Vec<String>,
    pub intended_properties: Vec<String>,
    #[serde(default)]
    pub assumptions: Vec<String>,
    #[serde(default)]
    pub source_mapping: Vec<serde_json::Value>,
    #[serde(default)]
    pub specification_reviewed: bool,
}
impl Manifest {
    pub fn new(
        challenge: impl Into<PathBuf>,
        solution: impl Into<PathBuf>,
        theorems: Vec<String>,
    ) -> Self {
        Self {
            schema_version: 1,
            challenge: challenge.into(),
            solution: solution.into(),
            theorems,
            intended_properties: vec![],
            assumptions: vec![],
            source_mapping: vec![],
            specification_reviewed: false,
        }
    }
}
#[derive(Debug, Clone, Serialize)]
pub struct Limits {
    pub image: String,
    pub timeout_ms: u64,
    pub memory_mib: u32,
    pub cpus: u32,
    pub max_output_bytes: usize,
}
impl Default for Limits {
    fn default() -> Self {
        Self {
            image: DEFAULT_IMAGE.into(),
            timeout_ms: 120_000,
            memory_mib: 4096,
            cpus: 2,
            max_output_bytes: 64 * 1_048_576,
        }
    }
}
#[derive(Debug, Serialize)]
pub struct Phase {
    pub name: String,
    pub status: String,
    pub exit_code: Option<i32>,
    pub wall_ms: u128,
    pub stdout_bytes: usize,
    pub stderr_bytes: usize,
}
#[derive(Debug, Serialize)]
pub struct ProofReport {
    pub schema_version: u32,
    pub model_proof_status: String,
    pub source_correspondence_status: String,
    pub source_mapping_status: String,
    pub specification_review_status: String,
    pub trust_boundary: String,
    pub image_id: String,
    pub image_pins: serde_json::Value,
    pub permitted_axioms: Vec<String>,
    pub axiom_audit: String,
    pub manifest: Manifest,
    pub source_hashes: serde_json::Value,
    pub limits: Limits,
    pub phases: Vec<Phase>,
    pub finding: String,
}
pub fn validate_manifest(m: &Manifest) -> Result<()> {
    if m.schema_version != 1 || m.theorems.is_empty() || m.theorems.len() > 128 {
        return Err("Expected schema version 1 and 1..128 theorem names".into());
    }
    let mut names = HashSet::new();
    for n in &m.theorems {
        if n.len() > 512
            || !n.split('.').all(|part| {
                !part.is_empty()
                    && part.as_bytes()[0].is_ascii_alphabetic()
                    && part
                        .bytes()
                        .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'\'')
            })
            || !names.insert(n)
        {
            return Err("Theorem names must be distinct ASCII Lean names".into());
        }
    }
    if m.intended_properties.is_empty() || m.intended_properties.iter().any(|s| s.trim().is_empty())
    {
        return Err("Describe independent intended properties before checking a model".into());
    }
    if m.challenge.extension().and_then(|s| s.to_str()) != Some("lean")
        || m.solution.extension().and_then(|s| s.to_str()) != Some("lean")
    {
        return Err("Challenge and solution must be Lean source files".into());
    }
    Ok(())
}
fn local_docker() -> Result<()> {
    if std::env::var("DOCKER_HOST").is_ok_and(|s| !s.starts_with("unix://")) {
        return Err(
            "Only a local Unix Docker daemon is supported; no remote review uploads".into(),
        );
    }
    let r = run_bounded(
        Command::new("docker").args([
            "context",
            "inspect",
            "--format",
            "{{.Endpoints.docker.Host}}",
        ]),
        Duration::from_secs(10),
        4096,
    )?;
    if r.exit_code != Some(0)
        || !String::from_utf8_lossy(&r.stdout)
            .trim()
            .starts_with("unix://")
    {
        return Err("A local Unix-socket Docker context is required".into());
    }
    Ok(())
}
pub fn resolve_image(image: &str) -> Result<String> {
    local_docker()?;
    if image.starts_with('-') || image.chars().any(char::is_whitespace) {
        return Err("Invalid image reference".into());
    }
    let r = run_bounded(
        Command::new("docker").args(["image", "inspect", "--format", "{{.Id}}", image]),
        Duration::from_secs(10),
        4096,
    )?;
    let id = String::from_utf8(r.stdout)?.trim().to_owned();
    if r.exit_code != Some(0)
        || !id.starts_with("sha256:")
        || id.len() != 71
        || !id[7..].bytes().all(|b| b.is_ascii_hexdigit())
    {
        return Err(
            "Image is absent/unreadable. Build it explicitly; this command never pulls".into(),
        );
    }
    Ok(id)
}
static COUNTER: AtomicU64 = AtomicU64::new(0);
fn name() -> String {
    format!(
        "claw-lean-{}-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos(),
        COUNTER.fetch_add(1, Ordering::Relaxed)
    )
}
struct Container(String);
impl Drop for Container {
    fn drop(&mut self) {
        let _ = run_cleanup(
            Command::new("docker")
                .args(["rm", "-f", &self.0])
                .stdout(Stdio::null()),
        );
    }
}
pub fn docker_args(
    id: &str,
    name: &str,
    mount: Option<&Path>,
    limits: &Limits,
) -> Result<Vec<String>> {
    if limits.timeout_ms == 0
        || limits.memory_mib < 128
        || limits.cpus == 0
        || limits.cpus > 64
        || limits.max_output_bytes == 0
        || limits.max_output_bytes > 512 * 1_048_576
    {
        return Err("Invalid resource limits".into());
    }
    #[cfg(unix)]
    let uid = unsafe { libc::geteuid() };
    #[cfg(not(unix))]
    let uid = 65532;
    #[cfg(unix)]
    let gid = unsafe { libc::getegid() };
    #[cfg(not(unix))]
    let gid = 65532;
    let (uid, gid) = if uid == 0 { (65532, 65532) } else { (uid, gid) };
    let mut args: Vec<String> = [
        "run",
        "--rm",
        "--pull=never",
        "--network=none",
        "--read-only",
        "--cap-drop=ALL",
        "--security-opt=no-new-privileges",
        "--pids-limit=256",
        "--log-driver=none",
        "--label=io.claw-work.lean-agent=true",
        "--tmpfs=/tmp:rw,noexec,nosuid,nodev,size=768m",
    ]
    .into_iter()
    .map(str::to_owned)
    .collect();
    args.extend([
        "--name".into(),
        name.into(),
        "--user".into(),
        format!("{uid}:{gid}"),
        format!("--memory={}m", limits.memory_mib),
        format!("--memory-swap={}m", limits.memory_mib),
        format!("--cpus={}", limits.cpus),
    ]);
    if let Some(path) = mount {
        let p = path.to_str().ok_or("Non-UTF8 mount path")?;
        if p.chars().any(|c| c == ',' || c == '\n' || c == '\r') {
            return Err("Unsafe Docker mount delimiter".into());
        }
        args.extend([
            "--mount".into(),
            format!("type=bind,src={p},dst=/input,readonly"),
        ]);
    }
    args.push(id.into());
    Ok(args)
}
fn run_container(
    id: &str,
    mount: Option<&Path>,
    command: &[&str],
    limits: &Limits,
) -> Result<ProcessResult> {
    let n = name();
    let _cleanup = Container(n.clone());
    let mut args = docker_args(id, &n, mount, limits)?;
    args.pop(); // place the fixed entrypoint before the immutable image ID
    args.extend([
        "--entrypoint=/usr/bin/timeout".into(),
        id.into(),
        "--signal=TERM".into(),
        "--kill-after=2".into(),
        (limits.timeout_ms.div_ceil(1000) + 2).to_string(),
        "/usr/local/bin/lean-container".into(),
    ]);
    run_bounded(
        Command::new("docker").args(args).args(command),
        Duration::from_millis(limits.timeout_ms),
        limits.max_output_bytes,
    )
}
pub fn image_metadata(id: &str) -> Result<serde_json::Value> {
    let limits = Limits {
        timeout_ms: 15_000,
        max_output_bytes: 65_536,
        ..Default::default()
    };
    let r = run_container(id, None, &["metadata"], &limits)?;
    if phase_status(&r) != "success" {
        return Err("Image metadata preflight failed".into());
    }
    let pins: serde_json::Value = serde_json::from_slice(&r.stdout)?;
    validate_image_pins(&pins)?;
    Ok(pins)
}
pub fn validate_image_pins(pins: &serde_json::Value) -> Result<()> {
    let profile = match pins.get("platform").and_then(serde_json::Value::as_str) {
        Some("linux/amd64") => include_str!("../docker/pins.json"),
        Some("linux/arm64") => include_str!("../docker/pins-arm64.json"),
        _ => return Err("Unsupported pinned image platform".into()),
    };
    let expected: serde_json::Value = serde_json::from_str(profile)?;
    if *pins != expected {
        return Err(
            "Image pins differ from this checker build; rebuild the compatible image/tool".into(),
        );
    }
    Ok(())
}
fn snapshot_dir(out: &Path, name: &str) -> Result<PathBuf> {
    let dir = out.join(name);
    fs::create_dir(&dir)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&dir, fs::Permissions::from_mode(0o700))?;
        // Root callers run the worker as 65532, never as root. Keep snapshots
        // owner-only, under the original caller\'s private outer directory.
        if unsafe { libc::geteuid() } == 0 {
            std::os::unix::fs::chown(&dir, Some(65532), Some(65532))?;
        }
    }
    Ok(dir)
}
fn snapshot_file(dir: &Path, name: &str, bytes: &[u8]) -> Result<()> {
    let path = dir.join(name);
    write_private(&path, bytes)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&path, fs::Permissions::from_mode(0o600))?;
        if unsafe { libc::geteuid() } == 0 {
            std::os::unix::fs::chown(&path, Some(65532), Some(65532))?;
        }
    }
    Ok(())
}
fn phase_status(r: &ProcessResult) -> &'static str {
    if r.cancelled {
        "cancelled"
    } else if r.timed_out {
        "timeout"
    } else if r.output_limited {
        "output_limit"
    } else if r.exit_code == Some(0) {
        "success"
    } else if matches!(r.exit_code, Some(124 | 125 | 126 | 127 | 137 | 143) | None) {
        "resource_or_runtime_error"
    } else {
        "rejected"
    }
}
fn record(out: &Path, name: &str, r: &ProcessResult) -> Result<Phase> {
    write_private(&out.join(format!("{name}.stdout")), &r.stdout)?;
    write_private(&out.join(format!("{name}.stderr")), &r.stderr)?;
    Ok(Phase {
        name: name.into(),
        status: phase_status(r).into(),
        exit_code: r.exit_code,
        wall_ms: r.wall_ms,
        stdout_bytes: r.stdout.len(),
        stderr_bytes: r.stderr.len(),
    })
}
fn finish(out: &Path, report: &ProofReport) -> Result<()> {
    write_private(
        &out.join("report.json"),
        &serde_json::to_vec_pretty(report)?,
    )?;
    let text = format!(
        "# Lean model proof report\n\n- Model proof: **{}**\n- Source correspondence: **unproved**\n- Specification review: **{}** (caller attestation only)\n- Independent statement/axiom/kernel audit: {}\n- Expected theorems: {}\n\n{}\n\nSee report.json for hashes, source mappings, assumptions and immutable image identity. Phase diagnostics are private adjacent .stdout/.stderr files. A rejected/unknown proof is not proof of a source bug. This report is private and is not an authorization to publish it.\n",
        report.model_proof_status,
        report.specification_review_status,
        report.axiom_audit,
        report.manifest.theorems.join(", "),
        report.finding
    );
    write_private(&out.join("report.md"), text.as_bytes())
}
pub fn check_proofs(
    m: &Manifest,
    base: &Path,
    output: &Path,
    limits: &Limits,
) -> Result<ProofReport> {
    validate_manifest(m)?;
    let challenge = read_source(&base.join(&m.challenge), 2 * 1_048_576)?;
    let solution = read_source(&base.join(&m.solution), 2 * 1_048_576)?;
    let image = resolve_image(&limits.image)?;
    let image_pins = image_metadata(&image)?;
    let out = create_private_dir(output)?;
    let mut report = ProofReport {
        schema_version: 1, model_proof_status: "unknown".into(), source_correspondence_status: "unproved".into(),
        source_mapping_status: "caller_supplied_not_validated".into(),
        specification_review_status: if m.specification_reviewed { "caller_attested" } else { "not_attested" }.into(),
        trust_boundary: "Immutable selected image, pinned upstream imports/exporter/comparator/kernel, local Docker daemon and host OS; independent challenge meaning is not established by this tool".into(),
        image_id: image.clone(), image_pins,
        permitted_axioms: PERMITTED_AXIOMS.iter().map(|s| (*s).into()).collect(), axiom_audit: "not_completed".into(),
        manifest: m.clone(), source_hashes: json!({"challenge_sha256":hash(&challenge), "solution_sha256":hash(&solution)}),
        limits: limits.clone(), phases: vec![], finding: "No source verification has been established".into(),
    };
    let judge = snapshot_dir(&out, "judge-input")?;
    for (module, name, bytes) in [
        ("Challenge", "challenge", &challenge),
        ("Solution", "solution", &solution),
    ] {
        let dir = snapshot_dir(&out, &format!("{name}-source"))?;
        snapshot_file(&dir, &format!("{module}.lean"), bytes)?;
        let mut export_args = vec!["export", module];
        export_args.extend(m.theorems.iter().map(String::as_str));
        let r = match run_container(&image, Some(&dir), &export_args, limits) {
            Ok(r) => r,
            Err(e) => {
                report.finding = format!("Could not execute {name} export: {e}");
                finish(&out, &report)?;
                return Ok(report);
            }
        };
        let status = phase_status(&r);
        report.phases.push(record(&out, name, &r)?);
        if status != "success" {
            report.model_proof_status = if status == "rejected" {
                "rejected"
            } else {
                "unknown"
            }
            .into();
            report.finding = format!(
                "{name} export did not complete; inspect private diagnostics, not a source-bug verdict"
            );
            finish(&out, &report)?;
            return Ok(report);
        }
        snapshot_file(&judge, &format!("{name}.ndjson"), &r.stdout)?;
    }
    snapshot_file(
        &judge,
        "comparator.json",
        &serde_json::to_vec(&json!({
            "challenge_module":"Challenge", "solution_module":"Solution", "theorem_names":m.theorems,
            "permitted_axioms":PERMITTED_AXIOMS
        }))?,
    )?;
    match run_container(&image, Some(&judge), &["judge"], limits) {
        Ok(r) => {
            let status = phase_status(&r);
            report.phases.push(record(&out, "judge", &r)?);
            report.model_proof_status = match status {
                "success" => "checked",
                "rejected" => "rejected",
                _ => "unknown",
            }
            .into();
            if status == "success" {
                report.axiom_audit =
                    "pinned_primitive_guard_and_upstream_comparator_accepted_under_listed_axioms"
                        .into();
                report.finding = "Expected model statements accepted by independent comparison, axiom audit and kernel replay. Source correspondence remains unproved".into();
            } else {
                report.finding = "Independent judgement did not accept the model proof. See private diagnostics; do not infer a source bug from this alone".into();
            }
        }
        Err(e) => report.finding = format!("Could not execute independent judge: {e}"),
    }
    finish(&out, &report)?;
    Ok(report)
}
