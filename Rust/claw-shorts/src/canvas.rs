//! Canvas path: turn a spec plus measured narration into one self-contained
//! `scene.html` for `skills/canvas-explainer`'s renderer.
//!
//! The scene draws every frame from `window.renderFrame(t)` using data computed
//! here: real narration loudness per frame, the reference clip's waveform, and
//! word timings for burned-in captions. The HTML is a template with the data
//! embedded as JSON, so it needs no network and no external files.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde::{Deserialize, Serialize};

use crate::filters;
use crate::media;
use crate::render::{self, RenderOptions};
use crate::spec::{self, Spec};
use crate::style::Style;
use crate::timeline::{self, SceneTiming};

const TEMPLATE: &str = include_str!("../canvas/scene.template.html");
const MARKER: &str = "/*__SHORT_DATA__*/null";
/// Sample rate used to analyze loudness; plenty for an amplitude envelope.
const ANALYSIS_RATE: u32 = 8000;
const REF_BINS: usize = 160;
/// A scene held longer than this gets a retention warning.
pub const LONG_SCENE_MS: u64 = 10_000;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Word {
    pub text: String,
    pub start_ms: u64,
    pub end_ms: u64,
}

#[derive(Debug, Serialize)]
pub struct Colors {
    pub bg: String,
    pub fg: String,
    pub accent: String,
    pub muted: String,
    pub panel: String,
    pub pass: String,
    pub warn: String,
    pub fail: String,
}

#[derive(Debug, Serialize)]
pub struct RowData {
    pub mark: String,
    pub tone: String,
    pub text: String,
}

#[derive(Debug, Serialize)]
pub struct SceneData {
    pub kind: String,
    pub start_ms: u64,
    pub end_ms: u64,
    pub audio_ms: u64,
    pub narration: String,
    pub lines: Vec<String>,
    pub command: Option<String>,
    pub output: Vec<String>,
    pub rows: Vec<RowData>,
    pub url: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct CanvasData {
    pub width: u32,
    pub height: u32,
    pub fps: u32,
    pub total_ms: u64,
    pub colors: Colors,
    pub scenes: Vec<SceneData>,
    pub scoreboard: Vec<String>,
    pub engagement_prompt: Option<String>,
    pub footer: Option<String>,
    /// Normalized loudness (0..1), one value per video frame.
    pub peaks: Vec<f32>,
    /// Normalized waveform of the reference clip, `REF_BINS` values.
    pub ref_peaks: Vec<f32>,
    pub ref_ms: u64,
    pub words: Vec<Word>,
}

/// Root-mean-square of consecutive windows (the last, partial one included).
pub fn rms_windows(samples: &[f32], window: usize) -> Vec<f32> {
    let window = window.max(1);
    samples
        .chunks(window)
        .map(|c| (c.iter().map(|x| x * x).sum::<f32>() / c.len() as f32).sqrt())
        .collect()
}

/// Scale to 0..1 by the 95th percentile, so one click cannot flatten the rest.
pub fn normalize(values: &[f32]) -> Vec<f32> {
    if values.is_empty() {
        return Vec::new();
    }
    let mut sorted = values.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let p95 = sorted[((sorted.len() - 1) as f64 * 0.95) as usize];
    if p95 <= 1e-6 {
        return vec![0.0; values.len()];
    }
    values.iter().map(|v| (v / p95).clamp(0.0, 1.0)).collect()
}

/// Resize to exactly `len` values by truncating or repeating silence.
pub fn fit_len(mut values: Vec<f32>, len: usize) -> Vec<f32> {
    values.resize(len, 0.0);
    values
}

/// Average `values` down to `bins` bins.
pub fn downsample(values: &[f32], bins: usize) -> Vec<f32> {
    if values.is_empty() || bins == 0 {
        return vec![0.0; bins];
    }
    (0..bins)
        .map(|b| {
            let lo = b * values.len() / bins;
            let hi = ((b + 1) * values.len() / bins).max(lo + 1).min(values.len());
            values[lo..hi].iter().sum::<f32>() / (hi - lo) as f32
        })
        .collect()
}

fn decode_f32(path: &Path) -> Result<Vec<f32>, String> {
    let output = Command::new("ffmpeg")
        .args(["-v", "error", "-i"])
        .arg(path)
        .args(["-ac", "1", "-ar", &ANALYSIS_RATE.to_string(), "-f", "f32le", "-"])
        .output()
        .map_err(|e| format!("ffmpeg: could not start: {e}"))?;
    if !output.status.success() {
        return Err(format!("could not decode {}: {}", path.display(), String::from_utf8_lossy(&output.stderr)));
    }
    Ok(output
        .stdout
        .chunks_exact(4)
        .map(|b| f32::from_le_bytes([b[0], b[1], b[2], b[3]]))
        .collect())
}

/// Parse and validate word timings: ordered, non-empty text, inside the video.
pub fn parse_words(json: &str, total_ms: u64) -> Result<Vec<Word>, String> {
    let words: Vec<Word> = serde_json::from_str(json).map_err(|e| format!("words file is not valid: {e}"))?;
    let mut last_start = 0;
    for w in &words {
        if w.text.trim().is_empty() || w.end_ms < w.start_ms {
            return Err(format!("bad word entry {w:?}"));
        }
        if w.start_ms < last_start {
            return Err(format!("words are out of order at {:?}", w.text));
        }
        if w.start_ms > total_ms + 500 {
            return Err(format!("word {:?} starts after the video ends", w.text));
        }
        last_start = w.start_ms;
    }
    Ok(words)
}

fn norm(word: &str) -> String {
    word.chars().filter(|c| c.is_alphanumeric()).flat_map(char::to_lowercase).collect()
}

/// Show the narration's own words (spelling, punctuation, "ElevenLabs") with
/// the speech recognizer's timings. Whisper writes "11 labs" and "21"; viewers
/// should read what the script says.
///
/// The two word lists are aligned by edit distance. A narration word takes the
/// span of the recognized words matched to it; leftover recognized words extend
/// the previous narration word; narration words with no match are spread evenly
/// between their neighbors. The result is always ordered.
pub fn align_words(narration: &[String], asr: &[Word]) -> Vec<Word> {
    let (n, m) = (narration.len(), asr.len());
    if n == 0 || m == 0 {
        return Vec::new();
    }
    let a: Vec<String> = narration.iter().map(|w| norm(w)).collect();
    let b: Vec<String> = asr.iter().map(|w| norm(&w.text)).collect();
    let mut cost = vec![vec![0u32; m + 1]; n + 1];
    for (i, row) in cost.iter_mut().enumerate() {
        row[0] = i as u32;
    }
    for (j, slot) in cost[0].iter_mut().enumerate() {
        *slot = j as u32;
    }
    for i in 1..=n {
        for j in 1..=m {
            let sub = cost[i - 1][j - 1] + u32::from(a[i - 1] != b[j - 1]);
            cost[i][j] = sub.min(cost[i - 1][j] + 1).min(cost[i][j - 1] + 1);
        }
    }
    // Trace back; `matched[i]` collects recognized indices for narration word i.
    let mut matched: Vec<Vec<usize>> = vec![Vec::new(); n];
    let mut exact = vec![false; n];
    // (recognized index, narration word before it, narration word after it)
    let mut orphans: Vec<(usize, Option<usize>, Option<usize>)> = Vec::new();
    let (mut i, mut j) = (n, m);
    while i > 0 || j > 0 {
        if i > 0 && j > 0 && cost[i][j] == cost[i - 1][j - 1] + u32::from(a[i - 1] != b[j - 1]) {
            matched[i - 1].push(j - 1);
            exact[i - 1] = a[i - 1] == b[j - 1];
            i -= 1;
            j -= 1;
        } else if i > 0 && cost[i][j] == cost[i - 1][j] + 1 {
            i -= 1;
        } else {
            orphans.push((j - 1, i.checked_sub(1), (i < n).then_some(i)));
            j -= 1;
        }
    }
    // A leftover recognized word ("labs" after "11") belongs to a neighboring
    // narration word that did not match exactly ("ElevenLabs"), preferring the
    // following one; otherwise it extends the previous word.
    for (asr_index, prev, next) in orphans {
        let target = [next, prev]
            .into_iter()
            .flatten()
            .find(|&w| !exact[w])
            .or(prev)
            .or(next);
        if let Some(t) = target {
            matched[t].push(asr_index);
        }
    }
    let mut spans: Vec<Option<(u64, u64)>> = matched
        .iter()
        .map(|idx| {
            let start = idx.iter().map(|&k| asr[k].start_ms).min()?;
            let end = idx.iter().map(|&k| asr[k].end_ms).max()?;
            Some((start, end))
        })
        .collect();
    // Spread unmatched narration words evenly between their matched neighbors.
    let first_start = asr[0].start_ms;
    let last_end = asr[m - 1].end_ms;
    let mut k = 0;
    while k < n {
        if spans[k].is_some() {
            k += 1;
            continue;
        }
        let run_start = k;
        while k < n && spans[k].is_none() {
            k += 1;
        }
        let from = if run_start == 0 { first_start } else { spans[run_start - 1].map_or(first_start, |s| s.1) };
        let to = if k == n { last_end } else { spans[k].map_or(last_end, |s| s.0) };
        let to = to.max(from);
        let count = (k - run_start) as u64;
        for (r, slot) in spans[run_start..k].iter_mut().enumerate() {
            let r = r as u64;
            *slot = Some((from + (to - from) * r / count, from + (to - from) * (r + 1) / count));
        }
    }
    // Enforce order: a word never starts before the previous one starts.
    let mut out = Vec::with_capacity(n);
    let mut floor = 0u64;
    for (word, span) in narration.iter().zip(spans) {
        let (start, end) = span.expect("every narration word has a span");
        let start = start.max(floor);
        let end = end.max(start);
        floor = start;
        out.push(Word { text: word.clone(), start_ms: start, end_ms: end });
    }
    out
}

fn style_colors(style: &Style) -> Colors {
    Colors {
        bg: style.bg.clone(),
        fg: style.fg.clone(),
        accent: style.accent.clone(),
        muted: style.muted.clone(),
        panel: style.panel.clone(),
        pass: style.pass.clone(),
        warn: style.warn.clone(),
        fail: style.fail.clone(),
    }
}

fn kind_name(kind: spec::SceneKind) -> &'static str {
    match kind {
        spec::SceneKind::Title => "title",
        spec::SceneKind::Cloning => "clone",
        spec::SceneKind::Terminal => "terminal",
        spec::SceneKind::Verdict => "verdict",
        spec::SceneKind::Endcard => "endcard",
    }
}

fn tone_name(tone: spec::Tone) -> &'static str {
    match tone {
        spec::Tone::Pass => "pass",
        spec::Tone::Warn => "warn",
        spec::Tone::Fail => "fail",
    }
}

#[allow(clippy::too_many_arguments)]
pub fn build_data(
    spec: &Spec,
    timings: &[SceneTiming],
    peaks: Vec<f32>,
    ref_peaks: Vec<f32>,
    ref_ms: u64,
    words: Vec<Word>,
) -> CanvasData {
    CanvasData {
        width: spec.style.width,
        height: spec.style.height,
        fps: spec.style.fps,
        total_ms: timeline::total_ms(timings),
        colors: style_colors(&spec.style),
        scenes: spec
            .scenes
            .iter()
            .zip(timings)
            .map(|(s, t)| SceneData {
                kind: kind_name(s.kind).into(),
                start_ms: t.start_ms,
                end_ms: t.end_ms,
                audio_ms: t.audio_ms,
                narration: s.narration.clone(),
                lines: s.lines.clone(),
                command: s.command.clone(),
                output: s.output.clone(),
                rows: s
                    .rows
                    .iter()
                    .map(|r| RowData { mark: r.mark.clone(), tone: tone_name(r.tone).into(), text: r.text.clone() })
                    .collect(),
                url: s.url.clone(),
            })
            .collect(),
        scoreboard: spec.scoreboard.clone(),
        engagement_prompt: spec.engagement.as_ref().map(|e| e.prompt.clone()),
        footer: spec.footer_text(),
        peaks,
        ref_peaks,
        ref_ms,
        words,
    }
}

/// Embed the data in the template. `</` is escaped so no string can close the tag.
pub fn embed(template: &str, data: &CanvasData) -> Result<String, String> {
    if !template.contains(MARKER) {
        return Err("scene template has no data marker".into());
    }
    let json = serde_json::to_string(data).map_err(|e| e.to_string())?.replace("</", "<\\/");
    Ok(template.replacen(MARKER, &json, 1))
}

#[derive(Debug)]
pub struct CanvasReport {
    pub scene_html: PathBuf,
    pub scene_data: PathBuf,
    pub narration_wav: PathBuf,
    pub timeline_json: PathBuf,
    pub total_ms: u64,
    pub fps: u32,
    pub warnings: Vec<String>,
}

/// Narrate (cached), measure, and write `scene.html`, `narration.wav`,
/// `scene_data.json` and `timeline.json` into a new directory.
pub fn prepare(spec: &Spec, opts: &RenderOptions, words_path: Option<&Path>) -> Result<CanvasReport, String> {
    render::blocking_errors(spec)?;
    for tool in ["ffmpeg", "ffprobe"] {
        if !media::have(tool) {
            return Err(format!("{tool} not found on PATH"));
        }
    }
    if opts.output_dir.exists() {
        return Err(format!("{} already exists; choose a new output directory", opts.output_dir.display()));
    }
    fs::create_dir_all(&opts.work_dir).map_err(|e| format!("cannot create work dir: {e}"))?;
    let wavs = render::narrate(spec, opts)?;
    let audio_ms: Vec<u64> = wavs.iter().map(|w| media::probe_duration_ms(w)).collect::<Result<_, _>>()?;
    let timings = timeline::build(&audio_ms, spec.style.pad_ms);
    let total = timeline::total_ms(&timings);
    let fps = spec.style.fps;

    fs::create_dir_all(&opts.output_dir).map_err(|e| format!("cannot create output dir: {e}"))?;
    let out_dir = fs::canonicalize(&opts.output_dir).map_err(|e| e.to_string())?;
    let narration_wav = out_dir.join("narration.wav");
    let mut ffmpeg = Command::new("ffmpeg");
    ffmpeg.args(["-y", "-v", "error"]);
    for wav in &wavs {
        ffmpeg.arg("-i").arg(fs::canonicalize(wav).map_err(|e| format!("{}: {e}", wav.display()))?);
    }
    ffmpeg
        .arg("-filter_complex")
        .arg(filters::audio_graph(&timings, 0))
        .args(["-map", "[aout]", "-c:a", "pcm_s16le"])
        .arg(&narration_wav);
    media::run(&mut ffmpeg, "ffmpeg narration concat")?;

    let frames = (total as usize * fps as usize).div_ceil(1000);
    let window = (ANALYSIS_RATE / fps) as usize;
    let peaks = fit_len(normalize(&rms_windows(&decode_f32(&narration_wav)?, window)), frames);

    let (ref_peaks, ref_ms) = match spec.voice.as_ref().map(|v| spec::resolve(&opts.spec_dir, &v.reference)) {
        Some(path) if path.is_file() => {
            let samples = decode_f32(&path)?;
            (downsample(&normalize(&rms_windows(&samples, 80)), REF_BINS), media::probe_duration_ms(&path)?)
        }
        _ => (Vec::new(), 0),
    };

    let mut warnings = Vec::new();
    let words = match words_path {
        Some(path) => {
            let json = fs::read_to_string(path).map_err(|e| format!("cannot read {}: {e}", path.display()))?;
            let asr = parse_words(&json, total)?;
            let spoken: Vec<String> =
                spec.scenes.iter().flat_map(|sc| sc.narration.split_whitespace().map(str::to_string)).collect();
            let aligned = align_words(&spoken, &asr);
            if aligned.is_empty() && !asr.is_empty() {
                warnings.push("could not align the words file to the narration".into());
            }
            aligned
        }
        None => {
            warnings.push("no --words file: the scene will have no burned-in captions".to_string());
            Vec::new()
        }
    };
    if spec.scenes.iter().any(|s| s.kind == spec::SceneKind::Cloning) && ref_peaks.is_empty() {
        warnings.push("a clone scene needs voice.reference to draw the reference waveform".into());
    }
    for index in timeline::long_scenes(&timings, LONG_SCENE_MS) {
        warnings.push(format!(
            "scene {} runs {:.1} s; long holds lose viewers, so split it or add a visual change",
            index + 1,
            timings[index].duration_ms() as f64 / 1000.0
        ));
    }

    let data = build_data(spec, &timings, peaks, ref_peaks, ref_ms, words);
    let scene_html = out_dir.join("scene.html");
    fs::write(&scene_html, embed(TEMPLATE, &data)?).map_err(|e| e.to_string())?;
    let scene_data = out_dir.join("scene_data.json");
    fs::write(&scene_data, serde_json::to_string_pretty(&data).map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;
    let timeline_json = out_dir.join("timeline.json");
    fs::write(&timeline_json, serde_json::to_string_pretty(&timings).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())?;

    Ok(CanvasReport { scene_html, scene_data, narration_wav, timeline_json, total_ms: total, fps, warnings })
}

/// Join the silent canvas video with the narration and verify the result.
pub fn mux(video: &Path, audio: &Path, output: &Path, timeline_path: Option<&Path>) -> Result<Vec<String>, String> {
    if output.exists() {
        return Err(format!("{} already exists", output.display()));
    }
    // Compare the inputs, not the output: FFmpeg stretches the output to the
    // longer stream, which would hide a video that ends before its narration.
    let video_ms = media::probe_duration_ms(video)?;
    let audio_ms = media::probe_duration_ms(audio)?;
    if video_ms.abs_diff(audio_ms) > 150 {
        return Err(format!(
            "video is {video_ms} ms but narration is {audio_ms} ms; the canvas render and the narration disagree"
        ));
    }
    let mut cmd = Command::new("ffmpeg");
    cmd.args(["-y", "-v", "error", "-i"])
        .arg(video)
        .arg("-i")
        .arg(audio)
        .args(["-map", "0:v", "-map", "1:a", "-c:v", "copy", "-c:a", "aac", "-b:a", "192k", "-movflags", "+faststart"])
        .arg(output);
    media::run(&mut cmd, "ffmpeg mux")?;
    let info = media::probe(output)?;
    media::decode_check(output)?;
    if !info.has_audio {
        return Err("muxed output has no audio".into());
    }
    let mut notes = vec![format!("{}x{}, {} ms, audio present, full decode ok", info.width, info.height, info.duration_ms)];
    if let Some(path) = timeline_path {
        let json = fs::read_to_string(path).map_err(|e| format!("cannot read {}: {e}", path.display()))?;
        let timings: Vec<SceneTiming> = parse_timings(&json)?;
        let parent = output.parent().filter(|p| !p.as_os_str().is_empty()).unwrap_or(Path::new("."));
        let scratch = parent.join(".contact-work");
        let sheet = render::contact_sheet(output, &timings, &scratch, parent);
        let _ = fs::remove_dir_all(&scratch);
        notes.push(format!("contact sheet: {}", sheet?.display()));
    }
    Ok(notes)
}

fn parse_timings(json: &str) -> Result<Vec<SceneTiming>, String> {
    #[derive(Deserialize)]
    struct Raw {
        index: usize,
        start_ms: u64,
        end_ms: u64,
        audio_ms: u64,
    }
    let raw: Vec<Raw> = serde_json::from_str(json).map_err(|e| format!("timeline is not valid: {e}"))?;
    Ok(raw
        .into_iter()
        .map(|r| SceneTiming { index: r.index, start_ms: r.start_ms, end_ms: r.end_ms, audio_ms: r.audio_ms })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rms_of_a_constant_signal_is_its_amplitude() {
        let out = rms_windows(&[0.5; 8], 4);
        assert_eq!(out.len(), 2);
        assert!(out.iter().all(|v| (v - 0.5).abs() < 1e-6));
        assert_eq!(rms_windows(&[1.0, 1.0, 1.0], 2).len(), 2, "partial window is kept");
        assert!(rms_windows(&[], 4).is_empty());
    }

    #[test]
    fn normalize_ignores_a_single_spike_and_handles_silence() {
        let mut v = vec![0.2f32; 99];
        v.push(50.0);
        let n = normalize(&v);
        assert!((n[0] - 1.0).abs() < 1e-6, "p95 is the scale, so ordinary values reach 1: {}", n[0]);
        assert!(n.iter().all(|x| (0.0..=1.0).contains(x)));
        assert_eq!(normalize(&[0.0, 0.0]), vec![0.0, 0.0]);
        assert!(normalize(&[]).is_empty());
    }

    #[test]
    fn fit_and_downsample_have_exact_lengths() {
        assert_eq!(fit_len(vec![1.0; 5], 3), vec![1.0; 3]);
        assert_eq!(fit_len(vec![1.0; 2], 4), vec![1.0, 1.0, 0.0, 0.0]);
        assert_eq!(downsample(&[1.0, 3.0, 5.0, 7.0], 2), vec![2.0, 6.0]);
        assert_eq!(downsample(&[], 3), vec![0.0; 3]);
        assert_eq!(downsample(&[1.0], 3).len(), 3);
    }

    #[test]
    fn words_are_validated() {
        let ok = r#"[{"text":"Hi","start_ms":0,"end_ms":300},{"text":"there","start_ms":300,"end_ms":700}]"#;
        assert_eq!(parse_words(ok, 1000).unwrap().len(), 2);
        let swapped = r#"[{"text":"a","start_ms":500,"end_ms":600},{"text":"b","start_ms":100,"end_ms":200}]"#;
        assert!(parse_words(swapped, 1000).unwrap_err().contains("out of order"));
        let late = r#"[{"text":"a","start_ms":9000,"end_ms":9100}]"#;
        assert!(parse_words(late, 1000).unwrap_err().contains("after the video"));
        assert!(parse_words(r#"[{"text":" ","start_ms":0,"end_ms":1}]"#, 1000).is_err());
        assert!(parse_words("nope", 1000).is_err());
    }

    fn asr(words: &[(&str, u64, u64)]) -> Vec<Word> {
        words.iter().map(|(t, s, e)| Word { text: t.to_string(), start_ms: *s, end_ms: *e }).collect()
    }
    fn spoken(text: &str) -> Vec<String> {
        text.split_whitespace().map(str::to_string).collect()
    }

    #[test]
    fn narration_spelling_replaces_the_recognizers() {
        let heard = asr(&[("How", 0, 200), ("does", 200, 400), ("it", 400, 500), ("compare", 500, 900),
            ("to", 900, 1000), ("11", 1000, 1300), ("labs?", 1300, 1700), ("I", 1900, 2000), ("haven't", 2000, 2400)]);
        let words = align_words(&spoken("How does it compare to ElevenLabs? I haven't"), &heard);
        let texts: Vec<&str> = words.iter().map(|w| w.text.as_str()).collect();
        assert_eq!(texts, ["How", "does", "it", "compare", "to", "ElevenLabs?", "I", "haven't"]);
        let eleven = &words[5];
        assert_eq!((eleven.start_ms, eleven.end_ms), (1000, 1700), "it spans both recognized words");
        assert_eq!(words[7].start_ms, 2000);
    }

    #[test]
    fn identical_text_keeps_the_timings() {
        let heard = asr(&[("Hi", 0, 300), ("there", 300, 700)]);
        let words = align_words(&spoken("Hi there"), &heard);
        assert_eq!(words, heard);
    }

    #[test]
    fn missing_recognized_words_are_interpolated_in_order() {
        let heard = asr(&[("one", 0, 400), ("four", 1200, 1600)]);
        let words = align_words(&spoken("one two three four"), &heard);
        assert_eq!(words.len(), 4);
        assert!(words[1].start_ms >= 400 && words[2].end_ms <= 1200, "{words:?}");
        assert!(words.windows(2).all(|p| p[0].start_ms <= p[1].start_ms), "ordered: {words:?}");
        assert!(words.iter().all(|w| w.end_ms >= w.start_ms));
    }

    #[test]
    fn extra_recognized_words_do_not_break_order() {
        let heard = asr(&[("uh", 0, 100), ("hello", 100, 500), ("um", 500, 600), ("world", 600, 1000)]);
        let words = align_words(&spoken("hello world"), &heard);
        assert_eq!(words.iter().map(|w| w.text.as_str()).collect::<Vec<_>>(), ["hello", "world"]);
        assert!(words[0].start_ms <= words[1].start_ms);
    }

    #[test]
    fn empty_inputs_give_no_captions() {
        assert!(align_words(&[], &asr(&[("a", 0, 1)])).is_empty());
        assert!(align_words(&spoken("a"), &[]).is_empty());
    }

    #[test]
    fn embedding_replaces_the_marker_and_escapes_script_closers() {
        let spec = crate::spec::parse(
            r##"{"version":1,"project":"p","title":"t","caption":"c","scenes":[
              {"kind":"title","narration":"n </script> sneaky","lines":["a"]}]}"##,
        )
        .unwrap();
        let timings = timeline::build(&[1000], 350);
        let data = build_data(&spec, &timings, vec![0.5], vec![], 0, vec![]);
        let html = embed("<script>window.SHORT = /*__SHORT_DATA__*/null;</script>", &data).unwrap();
        assert!(html.starts_with("<script>window.SHORT = {"));
        assert_eq!(html.matches("</script>").count(), 1, "only the template's own closer may remain");
        assert!(html.contains("<\\/script> sneaky"));
        assert!(embed("<script></script>", &data).is_err());
    }

    #[test]
    fn the_shipped_template_contains_the_marker_once() {
        assert_eq!(TEMPLATE.matches(MARKER).count(), 1);
    }

    #[test]
    fn timelines_round_trip() {
        let timings = timeline::build(&[1000, 2000], 350);
        let json = serde_json::to_string(&timings).unwrap();
        assert_eq!(parse_timings(&json).unwrap(), timings);
    }
}
