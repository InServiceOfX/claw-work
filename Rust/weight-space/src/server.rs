use anyhow::{Context, Result, bail, ensure};
use serde::{Deserialize, Serialize};
use std::{
    fs::{self, File},
    path::Path,
    process::{Child, Command, Stdio},
    time::{Duration, Instant},
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    pub image: String,
    pub gpu: String,
    pub port: u16,
    pub context: usize,
}
pub struct OwnedServer {
    child: Child,
    name: String,
    pub url: String,
}

pub fn local_client(seconds: u64) -> Result<reqwest::blocking::Client> {
    Ok(reqwest::blocking::Client::builder()
        .no_proxy()
        .redirect(reqwest::redirect::Policy::none())
        .connect_timeout(Duration::from_secs(3))
        .timeout(Duration::from_secs(seconds))
        .build()?)
}

pub fn decode_json_response(status: u16, bytes: &[u8]) -> Result<serde_json::Value> {
    ensure!(
        (200..300).contains(&status),
        "local backend HTTP {status}: {}",
        String::from_utf8_lossy(&bytes[..bytes.len().min(65536)])
    );
    Ok(serde_json::from_slice(bytes)?)
}

pub fn read_json_response(response: reqwest::blocking::Response) -> Result<serde_json::Value> {
    use std::io::Read;
    let status = response.status().as_u16();
    let mut bytes = Vec::new();
    response
        .take(64 * 1024 * 1024 + 1)
        .read_to_end(&mut bytes)?;
    ensure!(
        bytes.len() <= 64 * 1024 * 1024,
        "backend response exceeds limit"
    );
    decode_json_response(status, &bytes)
}

impl OwnedServer {
    pub fn start(model: &Path, log: &Path, cfg: &ServerConfig) -> Result<Self> {
        crate::ensure_external_output(log)?;
        ensure!(
            cfg.port > 1024 && cfg.context >= 1024,
            "invalid port/context"
        );
        ensure!(
            !cfg.gpu.is_empty()
                && cfg
                    .gpu
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b == b'-'),
            "GPU must be a single index or UUID"
        );
        ensure!(
            cfg.image.starts_with("sha256:")
                && cfg.image.len() == 71
                && cfg.image[7..].bytes().all(|b| b.is_ascii_hexdigit()),
            "pin a local Docker image by full sha256 ID"
        );
        // A bind reservation check makes accidental reuse of another local service fail.
        let reservation = std::net::TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, cfg.port))
            .context("experiment port is occupied")?;
        let model = fs::canonicalize(model)?;
        ensure!(model.is_file(), "model must be regular file");
        let parent = model.parent().context("model parent")?;
        ensure!(
            !parent.to_string_lossy().contains([',', ':'])
                && !model
                    .file_name()
                    .context("model filename")?
                    .to_string_lossy()
                    .contains([',', ':']),
            "unsupported Docker mount path"
        );
        let name = format!("claw-wsp-{}-{}", std::process::id(), cfg.port);
        let output = File::options().write(true).create_new(true).open(log)?;
        let filename = model.file_name().context("model name")?.to_string_lossy();
        drop(reservation);
        let child = Command::new("docker")
            .args([
                "run",
                "--rm",
                "--pull=never",
                "--name",
                &name,
                "--gpus",
                &format!("device={}", cfg.gpu),
                "-p",
                &format!("127.0.0.1:{}:{}", cfg.port, cfg.port),
                "--mount",
                &format!("type=bind,src={},dst=/models,readonly", parent.display()),
                &cfg.image,
                "-m",
                &format!("/models/{filename}"),
                "--host",
                "0.0.0.0",
                "--port",
                &cfg.port.to_string(),
                "-ngl",
                "99",
                "-c",
                &cfg.context.to_string(),
                "-np",
                "1",
                "-b",
                "512",
                "-ub",
                "256",
                "-t",
                "8",
                "-fa",
                "on",
                "--jinja",
                "--reasoning-format",
                "none",
                "--no-context-shift",
            ])
            .stdin(Stdio::null())
            .stdout(Stdio::from(output.try_clone()?))
            .stderr(Stdio::from(output))
            .spawn()
            .context("launch Docker backend")?;
        let mut server = Self {
            child,
            name,
            url: format!("http://127.0.0.1:{}", cfg.port),
        };
        let client = local_client(3)?;
        let start = Instant::now();
        while start.elapsed() < Duration::from_secs(120) {
            if let Some(status) = server.child.try_wait()? {
                bail!(
                    "experiment backend exited {status}; inspect {}",
                    log.display()
                );
            }
            if client
                .get(format!("{}/health", server.url))
                .send()
                .is_ok_and(|r| r.status().is_success())
            {
                return Ok(server);
            }
            std::thread::sleep(Duration::from_millis(250));
        }
        bail!("backend did not become healthy; inspect {}", log.display())
    }
}
impl Drop for OwnedServer {
    fn drop(&mut self) {
        let _ = Command::new("docker")
            .args(["stop", "--time", "3", &self.name])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
        let _ = self.child.wait();
    }
}
