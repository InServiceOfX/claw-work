//! `VoxCPM TTS.app`: a native macOS window with the same features as the CLI.
//!
//! The model loads automatically on launch in a background thread and is
//! dropped (weights freed) when the window closes.

use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, Sender, channel};
use std::time::Instant;

use eframe::egui;
use voxcpm_tts::audio::{StitchOptions, WavFormat};
use voxcpm_tts::config::{self, MODEL_ID};
use voxcpm_tts::engine::{self, Cancelled, EngineKind, GenerationParams};
use voxcpm_tts::eta::{Calibration, Estimator, format_duration};
use voxcpm_tts::job::{self, ChunkReport, JobReport, JobSpec, ProgressSink, RetryPolicy};
use voxcpm_tts::text::{Chunk, ChunkOptions, chunk_text};

enum Command {
    Load,
    Run(Box<JobSpec>),
    Unload,
}

enum Event {
    Loading(f64),
    Loaded(f64),
    LoadFailed(String),
    Unloaded,
    Planned {
        chunks: usize,
        chars: usize,
        estimate: f64,
    },
    Tick {
        done_chars: usize,
        label: String,
        eta: f64,
    },
    Log(String),
    Done(Result<JobReport, String>),
}

struct ChannelSink {
    tx: Sender<Event>,
    total_chunks: std::sync::atomic::AtomicUsize,
}

impl ProgressSink for ChannelSink {
    fn loading(&self, estimate: f64) {
        let _ = self.tx.send(Event::Loading(estimate));
    }
    fn loaded(&self, secs: f64) {
        let _ = self.tx.send(Event::Loaded(secs));
    }
    fn planned(&self, chunks: &[Chunk], estimate: f64) {
        self.total_chunks.store(chunks.len(), Ordering::SeqCst);
        let chars = chunks.iter().map(Chunk::char_len).sum();
        let _ = self.tx.send(Event::Planned {
            chunks: chunks.len(),
            chars,
            estimate,
        });
    }
    fn chunk_started(&self, chunk: &Chunk, attempt: usize) {
        let retry = if attempt > 1 {
            format!(" (retry {})", attempt - 1)
        } else {
            String::new()
        };
        let _ = self.tx.send(Event::Log(format!(
            "Chunk {}/{}{retry}…",
            chunk.index,
            self.total_chunks.load(Ordering::SeqCst)
        )));
    }
    fn tick(&self, chunk: &Chunk, done_chars: usize, audio_seconds: f64, eta: f64) {
        let label = format!(
            "chunk {}/{} · {:.1}s audio",
            chunk.index,
            self.total_chunks.load(Ordering::SeqCst),
            audio_seconds
        );
        let _ = self.tx.send(Event::Tick {
            done_chars,
            label,
            eta,
        });
    }
    fn chunk_finished(&self, r: &ChunkReport, done_chars: usize, eta: f64) {
        let _ = self.tx.send(Event::Tick {
            done_chars,
            label: format!(
                "chunk {}/{} done",
                r.index,
                self.total_chunks.load(Ordering::SeqCst)
            ),
            eta,
        });
        let _ = self.tx.send(Event::Log(format!(
            "  ✓ {:.1}s of speech in {}",
            r.audio_seconds,
            format_duration(r.compute_seconds)
        )));
    }
    fn retrying(&self, chunk: &Chunk, reason: &str) {
        let _ = self
            .tx
            .send(Event::Log(format!("  ↻ chunk {}: {reason}", chunk.index)));
    }
    fn stitching(&self) {
        let _ = self
            .tx
            .send(Event::Log("Stitching chunks into one WAV…".into()));
    }
    fn warning(&self, message: &str) {
        let _ = self.tx.send(Event::Log(format!("⚠ {message}")));
    }
}

/// Owns the engine; the only thread that touches model weights.
fn engine_thread(
    kind: EngineKind,
    rx: Receiver<Command>,
    tx: Sender<Event>,
    cancel: Arc<AtomicBool>,
) {
    let mut engine = match engine::create(kind, MODEL_ID) {
        Ok(e) => e,
        Err(e) => {
            let _ = tx.send(Event::LoadFailed(format!("{e:#}")));
            return;
        }
    };
    let cal_path = config::app_support_dir().join("calibration.json");
    while let Ok(cmd) = rx.recv() {
        match cmd {
            Command::Load => {
                let mut cal = Calibration::load(&cal_path);
                let key = Calibration::key(engine.name(), config::DEFAULT_INFERENCE_TIMESTEPS);
                let _ = tx.send(Event::Loading(cal.get(&key).load_secs));
                match engine.load_cancellable(&cancel) {
                    Ok(secs) => {
                        cal.record_load(&key, secs);
                        cal.save();
                        let _ = tx.send(Event::Loaded(secs));
                    }
                    Err(e) => {
                        let _ = tx.send(Event::LoadFailed(format!("{e:#}")));
                    }
                }
            }
            Command::Run(spec) => {
                let mut cal = Calibration::load(&cal_path);
                let sink = ChannelSink {
                    tx: tx.clone(),
                    total_chunks: Default::default(),
                };
                let result = job::run_job(engine.as_mut(), &spec, &sink, &cancel, &mut cal)
                    .map_err(|e| {
                        if e.chain().any(|c| c.is::<Cancelled>()) {
                            "Cancelled — intermediate files removed.".to_string()
                        } else {
                            format!("{e:#}")
                        }
                    });
                let _ = tx.send(Event::Done(result));
            }
            Command::Unload => {
                let _ = engine.unload();
                let _ = tx.send(Event::Unloaded);
            }
        }
    }
    // Channel closed: the window is gone. Dropping the engine frees the weights.
    let _ = engine.unload();
}

#[derive(PartialEq)]
enum ModelState {
    Loading { since: Instant, estimate: f64 },
    Ready,
    Unloaded,
    Failed(String),
}

struct App {
    engine_thread: Option<std::thread::JoinHandle<()>>,
    tx: Sender<Command>,
    rx: Receiver<Event>,
    cancel: Arc<AtomicBool>,
    engine_kind: EngineKind,
    model: ModelState,
    voice: Option<PathBuf>,
    transcript: String,
    script: String,
    output: Option<PathBuf>,
    max_chars: usize,
    timesteps: usize,
    cfg: f32,
    seed: String,
    float: bool,
    retries: usize,
    sentence_gap: f64,
    paragraph_gap: f64,
    keep_parts: Option<PathBuf>,
    report: Option<PathBuf>,
    running: bool,
    started: Option<Instant>,
    total_chars: usize,
    done_chars: usize,
    status: String,
    eta: f64,
    log: Vec<String>,
    last_output: Option<PathBuf>,
}

impl App {
    fn new(engine_kind: EngineKind) -> Self {
        let (cmd_tx, cmd_rx) = channel();
        let (ev_tx, ev_rx) = channel();
        let cancel = Arc::new(AtomicBool::new(false));
        let c = cancel.clone();
        let engine_thread = Some(std::thread::spawn(move || {
            engine_thread(engine_kind, cmd_rx, ev_tx, c)
        }));
        let _ = cmd_tx.send(Command::Load); // load weights automatically
        Self {
            engine_thread,
            tx: cmd_tx,
            rx: ev_rx,
            cancel,
            engine_kind,
            model: ModelState::Loading {
                since: Instant::now(),
                estimate: 0.0,
            },
            voice: None,
            transcript: String::new(),
            script: String::new(),
            output: None,
            max_chars: config::DEFAULT_MAX_CHARS,
            timesteps: config::DEFAULT_INFERENCE_TIMESTEPS,
            cfg: config::DEFAULT_CFG_VALUE,
            seed: String::new(),
            float: false,
            retries: 2,
            sentence_gap: 0.30,
            paragraph_gap: 0.65,
            keep_parts: None,
            report: None,
            running: false,
            started: None,
            total_chars: 0,
            done_chars: 0,
            status: String::new(),
            eta: 0.0,
            log: Vec::new(),
            last_output: None,
        }
    }

    fn drain_events(&mut self) {
        while let Ok(ev) = self.rx.try_recv() {
            match ev {
                Event::Loading(estimate) => {
                    self.model = ModelState::Loading {
                        since: Instant::now(),
                        estimate,
                    }
                }
                Event::Loaded(secs) => {
                    self.model = ModelState::Ready;
                    self.log
                        .push(format!("Model loaded in {}", format_duration(secs)));
                }
                Event::LoadFailed(e) => {
                    self.model = ModelState::Failed(e.clone());
                    self.running = false;
                    self.log.push(format!("Model failed to load: {e}"));
                }
                Event::Unloaded => self.model = ModelState::Unloaded,
                Event::Planned {
                    chunks,
                    chars,
                    estimate,
                } => {
                    self.total_chars = chars;
                    self.done_chars = 0;
                    self.eta = estimate;
                    self.log.push(format!(
                        "{chars} characters in {chunks} chunk(s); about {}",
                        format_duration(estimate)
                    ));
                }
                Event::Tick {
                    done_chars,
                    label,
                    eta,
                } => {
                    self.done_chars = done_chars;
                    self.status = label;
                    self.eta = eta;
                }
                Event::Log(line) => self.log.push(line),
                Event::Done(result) => {
                    self.running = false;
                    match result {
                        Ok(r) => {
                            if let Some(path) = &self.report {
                                let saved = job::write_report(path, &r);
                                if let Err(e) = saved {
                                    self.log.push(format!("Could not save report: {e}"));
                                }
                            }
                            self.log.push(format!(
                                "✓ Saved {} ({} of audio in {})",
                                r.output.display(),
                                format_duration(r.audio_seconds),
                                format_duration(r.compute_seconds)
                            ));
                            self.last_output = Some(r.output);
                            self.model = ModelState::Ready;
                        }
                        Err(e) => self.log.push(e),
                    }
                }
            }
        }
    }

    fn start_job(&mut self) {
        if let Some(path) = &self.report
            && let Err(e) = job::check_report(path)
        {
            self.log.push(e.to_string());
            return;
        }
        let Some(output) = self.output.clone().or_else(|| {
            rfd::FileDialog::new()
                .add_filter("WAV audio", &["wav"])
                .set_file_name("speech.wav")
                .save_file()
        }) else {
            return;
        };
        let seed = if self.seed.trim().is_empty() {
            None
        } else {
            match self.seed.trim().parse::<u64>() {
                Ok(seed) => Some(seed),
                Err(_) => {
                    self.log
                        .push("Seed must be a nonnegative whole number.".into());
                    return;
                }
            }
        };
        if output.exists()
            && rfd::MessageDialog::new()
                .set_title("Replace WAV?")
                .set_description(format!("{} already exists. Replace it?", output.display()))
                .set_buttons(rfd::MessageButtons::YesNo)
                .show()
                != rfd::MessageDialogResult::Yes
        {
            return;
        }
        self.output = Some(output.clone());
        let transcript = self.transcript.trim();
        let spec = JobSpec {
            text: self.script.clone(),
            voice: self.voice.clone(),
            voice_transcript: (!transcript.is_empty()).then(|| transcript.to_string()),
            output,
            overwrite: true, // the save dialog already confirmed replacement
            chunking: ChunkOptions {
                max_chars: self.max_chars,
                ..ChunkOptions::default()
            },
            params: GenerationParams {
                cfg_value: self.cfg,
                inference_timesteps: self.timesteps,
                seed,
                max_audio_seconds: 0.0,
            },
            stitching: StitchOptions {
                sentence_gap: self.sentence_gap,
                paragraph_gap: self.paragraph_gap,
                ..StitchOptions::default()
            },
            wav_format: if self.float {
                WavFormat::Float32
            } else {
                WavFormat::Pcm16
            },
            keep_parts: self.keep_parts.clone(),
            retry: RetryPolicy {
                max_retries: self.retries,
                ..RetryPolicy::default()
            },
            work_root: None,
        };
        self.running = true;
        self.started = Some(Instant::now());
        self.done_chars = 0;
        self.status = "starting…".into();
        self.log.clear();
        self.cancel.store(false, Ordering::SeqCst);
        let _ = self.tx.send(Command::Run(Box::new(spec)));
    }

    fn estimate(&self) -> (usize, usize, f64) {
        let chunks = chunk_text(
            &self.script,
            ChunkOptions {
                max_chars: self.max_chars,
                ..ChunkOptions::default()
            },
        );
        let chars: usize = chunks.iter().map(Chunk::char_len).sum();
        let name = match self.engine_kind {
            EngineKind::Native => "native",
            EngineKind::MlxPython => "mlx-python",
        };
        let cal = Calibration::load(&config::app_support_dir().join("calibration.json"))
            .get(&Calibration::key(name, self.timesteps));
        (
            chunks.len(),
            chars,
            Estimator::new(cal, chars).remaining_secs(0, 0.0),
        )
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.drain_events();
        if self.running || matches!(self.model, ModelState::Loading { .. }) {
            ctx.request_repaint_after(std::time::Duration::from_millis(150));
        }
        // Drag & drop: audio → voice, text → script.
        let dropped: Vec<PathBuf> = ctx.input(|i| {
            i.raw
                .dropped_files
                .iter()
                .filter_map(|f| f.path.clone())
                .collect()
        });
        for path in dropped {
            let ext = path
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("")
                .to_ascii_lowercase();
            if voxcpm_tts::voice::SUPPORTED_EXTENSIONS.contains(&ext.as_str()) {
                self.voice = Some(path);
            } else if let Ok(text) = std::fs::read_to_string(&path) {
                self.script = text;
            }
        }

        egui::TopBottomPanel::top("model").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.heading("VoxCPM TTS");
                ui.separator();
                match &self.model {
                    ModelState::Loading { since, estimate } => {
                        ui.spinner();
                        ui.label(format!(
                            "Loading model… {} (≈{})",
                            format_duration(since.elapsed().as_secs_f64()),
                            format_duration(*estimate)
                        ));
                    }
                    ModelState::Ready => {
                        ui.label("● Model loaded");
                        if ui
                            .add_enabled(!self.running, egui::Button::new("Free memory"))
                            .clicked()
                        {
                            let _ = self.tx.send(Command::Unload);
                        }
                    }
                    ModelState::Unloaded => {
                        ui.label("○ Model unloaded (reloads on Generate)");
                        if ui
                            .add_enabled(!self.running, egui::Button::new("Load now"))
                            .clicked()
                        {
                            let _ = self.tx.send(Command::Load);
                        }
                    }
                    ModelState::Failed(e) => {
                        ui.colored_label(egui::Color32::RED, format!("Model unavailable: {e}"));
                    }
                }
            });
        });

        egui::TopBottomPanel::bottom("progress")
            .min_height(140.0)
            .show(ctx, |ui| {
                if self.running {
                    let frac = if self.total_chars > 0 {
                        self.done_chars as f32 / self.total_chars as f32
                    } else {
                        0.0
                    };
                    ui.add(egui::ProgressBar::new(frac).show_percentage());
                    ui.label(format!(
                        "{} · elapsed {} · remaining ≈ {}",
                        self.status,
                        format_duration(
                            self.started
                                .map(|s| s.elapsed().as_secs_f64())
                                .unwrap_or(0.0)
                        ),
                        format_duration(self.eta)
                    ));
                }
                if let Some(out) = self.last_output.clone() {
                    ui.horizontal(|ui| {
                        if ui.button("▶ Play").clicked() {
                            let _ = std::process::Command::new("afplay").arg(&out).spawn();
                        }
                        if ui.button("Show in Finder").clicked() {
                            let _ = std::process::Command::new("open")
                                .arg("-R")
                                .arg(&out)
                                .spawn();
                        }
                        ui.label(out.display().to_string());
                    });
                }
                egui::ScrollArea::vertical()
                    .stick_to_bottom(true)
                    .max_height(90.0)
                    .show(ui, |ui| {
                        for line in &self.log {
                            ui.monospace(line);
                        }
                    });
            });

        egui::SidePanel::left("settings")
            .resizable(false)
            .default_width(300.0)
            .show(ctx, |ui| {
                ui.add_space(6.0);
                ui.strong("Voice to clone");
                ui.label(
                    self.voice
                        .as_ref()
                        .map(|p| p.display().to_string())
                        .unwrap_or("none — drop a .wav/.mp3 here".into()),
                );
                ui.horizontal(|ui| {
                    if ui.button("Choose…").clicked()
                        && let Some(p) = rfd::FileDialog::new()
                            .add_filter("Audio", voxcpm_tts::voice::SUPPORTED_EXTENSIONS)
                            .pick_file()
                    {
                        self.voice = Some(p);
                    }
                    if self.voice.is_some() && ui.button("Clear").clicked() {
                        self.voice = None;
                    }
                });
                ui.label("Transcript of the voice clip (optional, improves similarity):");
                ui.add(
                    egui::TextEdit::multiline(&mut self.transcript)
                        .desired_rows(3)
                        .desired_width(f32::INFINITY),
                );
                ui.separator();
                ui.strong("Output");
                ui.label(
                    self.output
                        .as_ref()
                        .map(|p| p.display().to_string())
                        .unwrap_or("asked on Generate".into()),
                );
                if ui.button("Save as…").clicked()
                    && let Some(p) = rfd::FileDialog::new()
                        .add_filter("WAV audio", &["wav"])
                        .set_file_name("speech.wav")
                        .save_file()
                {
                    self.output = Some(p);
                }
                ui.separator();
                egui::CollapsingHeader::new("Advanced").show(ui, |ui| {
                    ui.add(
                        egui::Slider::new(&mut self.max_chars, 80..=600).text("max chars / chunk"),
                    );
                    ui.add(egui::Slider::new(&mut self.timesteps, 4..=20).text("diffusion steps"));
                    ui.add(egui::Slider::new(&mut self.cfg, 1.0..=4.0).text("guidance (cfg)"));
                    ui.horizontal(|ui| {
                        ui.label("seed");
                        ui.text_edit_singleline(&mut self.seed);
                    });
                    ui.checkbox(&mut self.float, "32-bit float WAV");
                    ui.add(egui::Slider::new(&mut self.retries, 0..=5).text("retries"));
                    ui.add(
                        egui::Slider::new(&mut self.sentence_gap, 0.0..=2.0)
                            .text("sentence pause (s)"),
                    );
                    ui.add(
                        egui::Slider::new(&mut self.paragraph_gap, 0.0..=3.0)
                            .text("paragraph pause (s)"),
                    );
                    if ui.button("Keep parts in folder…").clicked() {
                        self.keep_parts = rfd::FileDialog::new().pick_folder();
                    }
                    if self.keep_parts.is_some() && ui.button("Discard parts").clicked() {
                        self.keep_parts = None;
                    }
                    if ui.button("Save JSON report…").clicked() {
                        self.report = rfd::FileDialog::new()
                            .add_filter("JSON", &["json"])
                            .set_file_name("speech-report.json")
                            .save_file();
                    }
                });
            });

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.strong("Script");
                if ui.button("Open text file…").clicked()
                    && let Some(p) = rfd::FileDialog::new()
                        .add_filter("Text", &["txt", "md"])
                        .pick_file()
                {
                    match std::fs::read_to_string(&p) {
                        Ok(t) => {
                            self.script = t;
                            if self.output.is_none() {
                                self.output = Some(p.with_extension("wav"));
                            }
                        }
                        Err(e) => self
                            .log
                            .push(format!("Could not read {}: {e}", p.display())),
                    }
                }
                let (chunks, chars, est) = self.estimate();
                if chars > 0 {
                    ui.label(format!(
                        "{chars} chars · {chunks} chunk(s) · ≈{}",
                        format_duration(est)
                    ));
                }
            });
            let can_run = !self.running
                && !self.script.trim().is_empty()
                && !matches!(self.model, ModelState::Failed(_));
            ui.horizontal(|ui| {
                if ui
                    .add_enabled(can_run, egui::Button::new("Generate speech"))
                    .clicked()
                {
                    self.start_job();
                }
                if self.running && ui.button("Cancel").clicked() {
                    self.cancel.store(true, Ordering::SeqCst);
                }
            });
            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.add(
                    egui::TextEdit::multiline(&mut self.script)
                        .hint_text("Type or paste what should be spoken, or drop a .txt file…")
                        .desired_width(f32::INFINITY)
                        .desired_rows(24),
                );
            });
        });
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        // Stop any job, then close the command channel so the engine thread
        // drops the model; process exit reclaims anything left.
        self.cancel.store(true, Ordering::SeqCst);
        let (dummy, _) = channel();
        self.tx = dummy;
        if let Some(thread) = self.engine_thread.take() {
            let deadline = Instant::now() + std::time::Duration::from_secs(5);
            while !thread.is_finished() && Instant::now() < deadline {
                std::thread::sleep(std::time::Duration::from_millis(20));
            }
            if thread.is_finished() {
                let _ = thread.join();
            }
        }
    }
}

fn main() -> eframe::Result<()> {
    use clap::Parser;
    #[derive(Parser)]
    struct Args {
        #[arg(long, value_enum)]
        engine: Option<EngineKind>,
    }
    let kind = Args::parse()
        .engine
        .unwrap_or(if voxcpm_tts::native::available() {
            EngineKind::Native
        } else {
            EngineKind::MlxPython
        });
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1100.0, 760.0])
            .with_title("VoxCPM TTS"),
        ..Default::default()
    };
    eframe::run_native(
        "VoxCPM TTS",
        options,
        Box::new(move |_cc| Ok(Box::new(App::new(kind)))),
    )
}
