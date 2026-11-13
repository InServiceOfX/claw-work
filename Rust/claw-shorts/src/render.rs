//! Narrate, render, verify. Nothing is uploaded; the output directory is new.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde::Serialize;

use crate::filters;
use crate::kit;
use crate::media::{self, VideoInfo};
use crate::spec::{self, Severity, Spec};
use crate::text;
use crate::timeline::{self, SceneTiming};

#[derive(Debug, Clone)]
pub struct RenderOptions {
    pub spec_dir: PathBuf,
    pub work_dir: PathBuf,
    pub output_dir: PathBuf,
    /// Regenerate narration even when a cached WAV exists.
    pub force_tts: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct Report {
    pub video: PathBuf,
    pub contact_sheet: PathBuf,
    pub captions_srt: PathBuf,
    pub timeline_json: PathBuf,
    pub posting_kit_json: PathBuf,
    pub posting_kit_md: PathBuf,
    pub total_ms: u64,
    pub scene_audio: Vec<PathBuf>,
}

#[derive(Debug, Serialize)]
struct Verification {
    width: u32,
    height: u32,
    duration_ms: u64,
    expected_duration_ms: u64,
    has_audio: bool,
    full_decode_ok: bool,
    note: &'static str,
}

/// Errors from validation, joined for display. Warnings never block.
pub fn blocking_errors(spec: &Spec) -> Result<(), String> {
    let errors: Vec<String> = spec
        .validate()
        .into_iter()
        .filter(|i| i.severity == Severity::Error)
        .map(|i| format!("  - {}", i.message))
        .collect();
    if errors.is_empty() {
        Ok(())
    } else {
        Err(format!("spec has {} error(s):\n{}", errors.len(), errors.join("\n")))
    }
}

/// Produce one narration WAV per scene, reusing cached files by content hash.
pub fn narrate(spec: &Spec, opts: &RenderOptions) -> Result<Vec<PathBuf>, String> {
    let audio_dir = opts.work_dir.join("audio");
    fs::create_dir_all(&audio_dir).map_err(|e| format!("cannot create {}: {e}", audio_dir.display()))?;
    let mut wavs = Vec::new();
    for (index, scene) in spec.scenes.iter().enumerate() {
        if let Some(audio) = &scene.audio {
            let path = spec::resolve(&opts.spec_dir, audio);
            if !path.is_file() {
                return Err(format!("scene {}: audio file {} not found", index + 1, path.display()));
            }
            wavs.push(path);
            continue;
        }
        let voice = spec
            .voice
            .as_ref()
            .ok_or_else(|| format!("scene {} has no audio file and the spec has no voice", index + 1))?;
        let reference = spec::resolve(&opts.spec_dir, &voice.reference);
        if !reference.is_file() {
            return Err(format!("voice reference {} not found", reference.display()));
        }
        let binary = voice.binary.as_deref().unwrap_or("voxcpm-tts");
        let seed = voice.seed.map(|s| s.to_string()).unwrap_or_default();
        let hash = text::fnv64(&[
            &scene.narration,
            &reference.to_string_lossy(),
            voice.transcript.as_deref().unwrap_or(""),
            &seed,
            binary,
        ]);
        let wav = audio_dir.join(format!("scene_{:02}_{hash:016x}.wav", index + 1));
        if opts.force_tts || !wav.is_file() {
            if !media::have(binary) {
                return Err(format!("TTS binary {binary:?} not found on PATH"));
            }
            let txt = audio_dir.join(format!("scene_{:02}.txt", index + 1));
            fs::write(&txt, &scene.narration).map_err(|e| format!("cannot write {}: {e}", txt.display()))?;
            media::tts(binary, &txt, &wav, &reference, voice.transcript.as_deref(), voice.seed)?;
        }
        wavs.push(wav);
    }
    Ok(wavs)
}

fn write(path: &Path, content: &str) -> Result<(), String> {
    fs::write(path, content).map_err(|e| format!("cannot write {}: {e}", path.display()))
}

fn srt(spec: &Spec, timings: &[SceneTiming]) -> String {
    let mut out = String::new();
    for (i, (scene, t)) in spec.scenes.iter().zip(timings).enumerate() {
        let end = t.start_ms + t.audio_ms.max(1);
        out.push_str(&format!(
            "{}\n{} --> {}\n{}\n\n",
            i + 1,
            timeline::srt_time(t.start_ms),
            timeline::srt_time(end),
            text::wrap(&scene.narration, 42).join("\n")
        ));
    }
    out
}

/// Render the Short. Fails before writing anything if validation fails.
pub fn render(spec: &Spec, opts: &RenderOptions) -> Result<Report, String> {
    blocking_errors(spec)?;
    let missing = spec.style.missing_fonts();
    if !missing.is_empty() {
        return Err(format!("font file(s) not found: {}; set style.font / style.mono_font", missing.join(", ")));
    }
    for tool in ["ffmpeg", "ffprobe"] {
        if !media::have(tool) {
            return Err(format!("{tool} not found on PATH"));
        }
    }
    if opts.output_dir.exists() {
        return Err(format!("{} already exists; choose a new output directory", opts.output_dir.display()));
    }
    fs::create_dir_all(&opts.work_dir).map_err(|e| format!("cannot create work dir: {e}"))?;

    let wavs = narrate(spec, opts)?;
    let audio_ms: Vec<u64> = wavs.iter().map(|w| media::probe_duration_ms(w)).collect::<Result<_, _>>()?;
    let timings = timeline::build(&audio_ms, spec.style.pad_ms);
    let plan = filters::build(spec, &timings);

    let text_dir = opts.work_dir.join("text");
    if text_dir.exists() {
        fs::remove_dir_all(&text_dir).map_err(|e| format!("cannot reset {}: {e}", text_dir.display()))?;
    }
    fs::create_dir_all(&text_dir).map_err(|e| format!("cannot create {}: {e}", text_dir.display()))?;
    for file in &plan.text_files {
        write(&opts.work_dir.join(&file.name), &file.content)?;
    }

    fs::create_dir_all(&opts.output_dir).map_err(|e| format!("cannot create output dir: {e}"))?;
    // FFmpeg runs inside the work directory, so every path it sees must be absolute.
    let out_dir = fs::canonicalize(&opts.output_dir).map_err(|e| format!("cannot resolve output dir: {e}"))?;
    let video = out_dir.join("short.mp4");
    let style = &spec.style;
    let mut ffmpeg = Command::new("ffmpeg");
    ffmpeg
        .current_dir(&opts.work_dir)
        .args(["-y", "-v", "error", "-f", "lavfi", "-i"])
        .arg(format!(
            "color=c={}:s={}x{}:r={}:d={}",
            crate::style::ff_color(&style.bg)?,
            style.width,
            style.height,
            style.fps,
            timeline::secs(plan.total_ms)
        ));
    for wav in &wavs {
        ffmpeg.arg("-i").arg(fs::canonicalize(wav).map_err(|e| format!("{}: {e}", wav.display()))?);
    }
    ffmpeg
        .arg("-filter_complex")
        .arg(&plan.filter_complex)
        .args(["-map", "[v]", "-map", "[aout]", "-c:v", "libx264", "-pix_fmt", "yuv420p", "-r"])
        .arg(style.fps.to_string())
        .args(["-c:a", "aac", "-b:a", "192k", "-movflags", "+faststart"])
        .arg(&video);
    media::run(&mut ffmpeg, "ffmpeg render")?;

    let info: VideoInfo = media::probe(&video)?;
    let decode = media::decode_check(&video);
    let verification = Verification {
        width: info.width,
        height: info.height,
        duration_ms: info.duration_ms,
        expected_duration_ms: plan.total_ms,
        has_audio: info.has_audio,
        full_decode_ok: decode.is_ok(),
        note: "Structure only. Listen to the audio and view the contact sheet before posting.",
    };
    write(
        &out_dir.join("verification.json"),
        &serde_json::to_string_pretty(&verification).map_err(|e| e.to_string())?,
    )?;
    decode?;
    if info.width != style.width || info.height != style.height {
        return Err(format!("output is {}x{}, expected {}x{}", info.width, info.height, style.width, style.height));
    }
    if info.duration_ms.abs_diff(plan.total_ms) > 150 {
        return Err(format!("output is {} ms, expected about {} ms", info.duration_ms, plan.total_ms));
    }
    if !info.has_audio {
        return Err("output has no audio stream".into());
    }

    let contact_sheet = contact_sheet(&video, &timings, &opts.work_dir, &out_dir)?;
    let captions_srt = out_dir.join("captions.srt");
    write(&captions_srt, &srt(spec, &timings))?;
    let timeline_json = out_dir.join("timeline.json");
    write(&timeline_json, &serde_json::to_string_pretty(&timings).map_err(|e| e.to_string())?)?;
    let (posting_kit_json, posting_kit_md) = kit::write(spec, plan.total_ms, &out_dir)?;

    Ok(Report {
        video,
        contact_sheet,
        captions_srt,
        timeline_json,
        posting_kit_json,
        posting_kit_md,
        total_ms: plan.total_ms,
        scene_audio: wavs,
    })
}

/// One frame from the middle of each scene, side by side. A reviewer's first
/// check: clipping, contrast, safe zones and text hold time.
fn contact_sheet(video: &Path, timings: &[SceneTiming], work: &Path, out_dir: &Path) -> Result<PathBuf, String> {
    let frames = work.join("contact");
    fs::create_dir_all(&frames).map_err(|e| format!("cannot create {}: {e}", frames.display()))?;
    let mut paths = Vec::new();
    for t in timings {
        let mid = t.start_ms + t.duration_ms() * 7 / 10;
        let path = frames.join(format!("scene_{:02}.png", t.index + 1));
        media::run(
            Command::new("ffmpeg")
                .args(["-y", "-v", "error", "-ss", &timeline::secs(mid), "-i"])
                .arg(video)
                .args(["-frames:v", "1", "-vf", "scale=360:-1"])
                .arg(&path),
            "contact frame",
        )?;
        paths.push(path);
    }
    let sheet = out_dir.join("contact.png");
    let mut cmd = Command::new("ffmpeg");
    cmd.args(["-y", "-v", "error"]);
    for p in &paths {
        cmd.arg("-i").arg(p);
    }
    cmd.arg("-filter_complex").arg(format!("hstack=inputs={}", paths.len())).arg(&sheet);
    if paths.len() == 1 {
        fs::copy(&paths[0], &sheet).map_err(|e| e.to_string())?;
    } else {
        media::run(&mut cmd, "contact sheet")?;
    }
    Ok(sheet)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spec::parse;

    #[test]
    fn srt_has_one_cue_per_scene_with_wrapped_text() {
        let spec = parse(
            r##"{"version":1,"project":"p","title":"t","caption":"c","scenes":[
              {"kind":"title","narration":"One two three four five six seven eight nine ten eleven twelve thirteen","lines":["a"]},
              {"kind":"title","narration":"Second.","lines":["b"]}]}"##,
        )
        .unwrap();
        let timings = timeline::build(&[2000, 1000], 500);
        let out = srt(&spec, &timings);
        assert!(out.starts_with("1\n00:00:00,000 --> 00:00:02,000\n"), "{out}");
        assert!(out.contains("2\n00:00:02,500 --> 00:00:03,500\nSecond."), "{out}");
        assert!(out.lines().all(|l| l.chars().count() <= 42));
    }

    #[test]
    fn blocking_errors_lists_every_error() {
        let spec = parse(
            r##"{"version":1,"project":"p","title":"TODO","caption":"c","scenes":[
              {"kind":"title","narration":"n","lines":["fine"]}]}"##,
        )
        .unwrap();
        let err = blocking_errors(&spec).unwrap_err();
        assert!(err.contains("1 error") && err.contains("todo"), "{err}");
    }

    #[test]
    fn narrate_reports_missing_audio_and_missing_voice() {
        let dir = tempfile::tempdir().unwrap();
        let opts = RenderOptions {
            spec_dir: dir.path().into(),
            work_dir: dir.path().join("work"),
            output_dir: dir.path().join("out"),
            force_tts: false,
        };
        let no_voice = parse(
            r##"{"version":1,"project":"p","title":"t","caption":"c","scenes":[
              {"kind":"title","narration":"n","lines":["a"]}]}"##,
        )
        .unwrap();
        assert!(narrate(&no_voice, &opts).unwrap_err().contains("no voice"));
        let missing = parse(
            r##"{"version":1,"project":"p","title":"t","caption":"c","scenes":[
              {"kind":"title","narration":"n","lines":["a"],"audio":"nope.wav"}]}"##,
        )
        .unwrap();
        assert!(narrate(&missing, &opts).unwrap_err().contains("not found"));
    }
}
