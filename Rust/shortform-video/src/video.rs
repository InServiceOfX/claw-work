use crate::caption::CaptionOverlay;
use crate::config::RenderConfig;
use crate::error::{Error, Result};
use crate::material::Aspect;
use claw_shorts::loudness::{self, Loudness};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::process::Command;

/// Extra visual runtime beyond the narration so the last frame never freezes
/// early (ported from MoneyPrinterTurbo's safety margin).
pub const DURATION_SAFETY_MARGIN: f64 = 0.1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, clap::ValueEnum)]
#[serde(rename_all = "lowercase")]
pub enum FitMode {
    /// Fill the canvas, cropping overflow symmetrically.
    Cover,
    /// Letterbox: show the whole frame over black bars.
    Contain,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, clap::ValueEnum)]
#[serde(rename_all = "lowercase")]
pub enum ConcatMode {
    /// One leading chunk per source, in download order (keeps visuals aligned
    /// with script order).
    Sequential,
    /// All chunks, round-robin across sources for early variety.
    Random,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, clap::ValueEnum)]
#[serde(rename_all = "lowercase")]
pub enum Transition {
    None,
    /// 0.5s fade-in on every segment.
    Fade,
}

#[derive(Debug, Clone)]
pub struct SourceClip {
    pub path: PathBuf,
    pub duration: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SegmentPlan {
    pub source: PathBuf,
    pub start: f64,
    pub duration: f64,
}

/// Local still images become Ken Burns segments instead of video cuts.
pub const STILL_EXTENSIONS: &[&str] = &["jpg", "jpeg", "png", "webp"];
pub const VIDEO_EXTENSIONS: &[&str] = &["mp4", "mov", "m4v", "webm", "mkv"];

/// Total zoom over one still: 8% (ginyboi: even 4-8% over 3-5 s keeps a
/// photo from reading as static).
pub const KEN_BURNS_ZOOM: f64 = 0.08;

fn has_extension(path: &Path, list: &[&str]) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| list.iter().any(|known| known.eq_ignore_ascii_case(ext)))
}

pub fn is_still(path: &Path) -> bool {
    has_extension(path, STILL_EXTENSIONS)
}

pub fn is_video(path: &Path) -> bool {
    has_extension(path, VIDEO_EXTENSIONS)
}

/// Frames in a still segment: at least 2, so the move has a start and an end.
pub fn still_frames(duration: f64, fps: u32) -> u32 {
    ((duration * f64::from(fps)).round() as u32).max(2)
}

/// Slow zoom plus a sideways drift over one still (Ken Burns). `variant`
/// picks one of four moves (zoom in/out x drift right/left) so consecutive
/// stills alternate. The still is first cover-fitted at twice the canvas size,
/// which keeps zoompan's whole-pixel crop steps from showing as jitter.
pub fn ken_burns_filter(width: u32, height: u32, fps: u32, frames: u32, variant: usize) -> String {
    format!(
        "{},{}",
        fit_filter(FitMode::Cover, width * 2, height * 2),
        ken_burns_motion(width, height, fps, frames, variant)
    )
}

/// The zoompan half of [`ken_burns_filter`], applied to a 2x-canvas image.
fn ken_burns_motion(width: u32, height: u32, fps: u32, frames: u32, variant: usize) -> String {
    let last = frames.saturating_sub(1).max(1);
    let progress = format!("on/{last}");
    let zoom = if variant.is_multiple_of(2) {
        format!("1+{KEN_BURNS_ZOOM}*{progress}")
    } else {
        format!("{}-{KEN_BURNS_ZOOM}*{progress}", 1.0 + KEN_BURNS_ZOOM)
    };
    let drift = if (variant / 2).is_multiple_of(2) {
        format!("0.25+0.5*{progress}")
    } else {
        format!("0.75-0.5*{progress}")
    };
    format!(
        "zoompan=z='{zoom}':x='(iw-iw/zoom)*({drift})':y='(ih-ih/zoom)/2':d={frames}:s={width}x{height}:fps={fps},setsar=1"
    )
}

/// How a still fills the canvas.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, clap::ValueEnum)]
#[serde(rename_all = "lowercase")]
pub enum StillLayout {
    /// Crop to fill the frame (good for photos).
    Cover,
    /// "B-roll space" (ginyboi): the whole image as a bordered card on a
    /// brand-colored background, above the caption area. Good for app
    /// screenshots and wide images that a 9:16 crop would cut apart.
    Frame,
}

/// Still-image look: layout plus the brand colors the frame layout uses.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StillStyle {
    pub layout: StillLayout,
    pub background: String,
    pub accent: String,
}

impl StillStyle {
    /// Colors must be `#rrggbb`; they are pasted into an ffmpeg filter graph.
    pub fn new(layout: StillLayout, background: &str, accent: &str) -> Result<Self> {
        for (name, value) in [("frame_background", background), ("frame_accent", accent)] {
            let hex = value.strip_prefix('#').unwrap_or("");
            if hex.len() != 6 || !hex.chars().all(|c| c.is_ascii_hexdigit()) {
                return Err(Error::Config(format!("render.{name} must be #rrggbb, got {value:?}")));
            }
        }
        Ok(Self { layout, background: background.to_string(), accent: accent.to_string() })
    }
}

/// One still's look and its position-dependent Ken Burns move.
#[derive(Debug, Clone, Copy)]
pub struct StillMove<'a> {
    pub style: &'a StillStyle,
    pub variant: usize,
}

fn even(value: f64) -> u32 {
    ((value / 2.0).floor() as u32) * 2
}

/// Full `-filter_complex` graph for one still: input `[0:v]`, output `[v]`.
pub fn still_graph(style: &StillStyle, width: u32, height: u32, fps: u32, frames: u32, variant: usize) -> String {
    let motion = ken_burns_motion(width, height, fps, frames, variant);
    match style.layout {
        StillLayout::Cover => format!("[0:v]{},{motion}[v]", fit_filter(FitMode::Cover, width * 2, height * 2)),
        StillLayout::Frame => {
            // Work at 2x like the cover path. The card sits in the band from
            // 12% to 70% of the height, clear of the platform UI on top and
            // the burned-in captions at the bottom, with 8% side margins.
            let (w2, h2) = (f64::from(width * 2), f64::from(height * 2));
            let border = even(w2 / 180.0).max(4);
            let card_w = even(w2 * 0.84) - 2 * border;
            let card_h = even(h2 * 0.52) - 2 * border;
            let top = even(h2 * 0.12);
            let band = even(h2 * 0.58);
            format!(
                "color=c={bg}:s={w}x{h}:r=1:d=1[base];\
                 [0:v]scale=w={card_w}:h={card_h}:force_original_aspect_ratio=decrease:force_divisible_by=2,setsar=1,\
                 pad=iw+{b2}:ih+{b2}:{border}:{border}:color={accent}[card];\
                 [base][card]overlay=x=(W-w)/2:y={top}+({band}-h)/2,trim=end_frame=1,{motion}[v]",
                bg = style.background,
                accent = style.accent,
                w = width * 2,
                h = height * 2,
                b2 = 2 * border,
            )
        }
    }
}

/// Every segment leaves as limited-range ("tv") yuv420p. JPEGs decode as
/// full-range yuvj420p; segments are joined with stream copy, so one
/// full-range segment would make the decoder misread the colors of the
/// segments after it (seen as a washed-out background after a JPEG still).
pub const SEGMENT_PIXELS: &str = "scale=out_range=tv,format=yuv420p";

/// ffmpeg video filter that fits any source frame onto the target canvas.
pub fn fit_filter(fit: FitMode, width: u32, height: u32) -> String {
    match fit {
        FitMode::Cover => format!(
            "scale={width}:{height}:force_original_aspect_ratio=increase,crop={width}:{height}"
        ),
        FitMode::Contain => format!(
            "scale={width}:{height}:force_original_aspect_ratio=decrease,\
             pad={width}:{height}:(ow-iw)/2:(oh-ih)/2:color=black"
        ),
    }
}

/// Split sources into subclips of at most `max_clip` seconds, order them for
/// variety, and take enough to cover `required` seconds, looping if the pool
/// is too small. Ported from MoneyPrinterTurbo's combine_videos.
pub fn plan_segments(
    sources: &[SourceClip],
    max_clip: f64,
    required: f64,
    mode: ConcatMode,
) -> Vec<SegmentPlan> {
    // Chunk every source on its own timeline.
    let mut per_source: Vec<Vec<SegmentPlan>> = Vec::new();
    for source in sources {
        let mut chunks = Vec::new();
        let mut start = 0.0;
        while start < source.duration {
            let duration = (source.duration - start).min(max_clip);
            if duration > 0.25 {
                chunks.push(SegmentPlan {
                    source: source.path.clone(),
                    start,
                    duration,
                });
            }
            start += max_clip;
            if mode == ConcatMode::Sequential {
                break;
            }
        }
        if !chunks.is_empty() {
            per_source.push(chunks);
        }
    }
    if per_source.is_empty() {
        return Vec::new();
    }

    // Round-robin across sources so early output cycles through distinct
    // footage instead of exhausting one source first.
    let mut ordered = Vec::new();
    let mut depth = 0;
    loop {
        let mut any = false;
        for chunks in &per_source {
            if let Some(chunk) = chunks.get(depth) {
                ordered.push(chunk.clone());
                any = true;
            }
        }
        if !any {
            break;
        }
        depth += 1;
    }

    // Budget, looping over the pool when the material is shorter than the
    // narration.
    let mut planned = Vec::new();
    let mut covered = 0.0;
    let mut index = 0;
    while covered < required && !ordered.is_empty() {
        let chunk = ordered[index % ordered.len()].clone();
        covered += chunk.duration;
        planned.push(chunk);
        index += 1;
        // Hard stop against degenerate inputs (e.g. all chunks ~0.25s).
        if index > 10_000 {
            break;
        }
    }
    planned
}

fn run_ffmpeg(binary: &str, args: &[String]) -> Result<()> {
    let output = Command::new(binary)
        .args(["-y", "-hide_banner", "-loglevel", "error"])
        .args(args)
        .output()
        .map_err(|e| Error::Ffmpeg(format!("failed to run {binary}: {e}")))?;
    if !output.status.success() {
        return Err(Error::Ffmpeg(format!(
            "{binary} {} failed: {}",
            args.join(" "),
            String::from_utf8_lossy(&output.stderr)
        )));
    }
    Ok(())
}

pub fn probe_duration(ffprobe: &str, path: &Path) -> Result<f64> {
    let output = Command::new(ffprobe)
        .args([
            "-v",
            "error",
            "-show_entries",
            "format=duration",
            "-of",
            "default=noprint_wrappers=1:nokey=1",
        ])
        .arg(path)
        .output()
        .map_err(|e| Error::Ffmpeg(format!("failed to run {ffprobe}: {e}")))?;
    if !output.status.success() {
        return Err(Error::Ffmpeg(format!(
            "ffprobe failed for {}: {}",
            path.display(),
            String::from_utf8_lossy(&output.stderr)
        )));
    }
    String::from_utf8_lossy(&output.stdout)
        .trim()
        .parse::<f64>()
        .map_err(|e| Error::Ffmpeg(format!("unparseable duration for {}: {e}", path.display())))
}

/// Cut one planned segment from its source and normalize it onto the target
/// canvas: exact resolution, constant fps, square pixels, no audio.
pub fn normalize_segment(
    render: &RenderConfig,
    plan: &SegmentPlan,
    aspect: Aspect,
    fit: FitMode,
    transition: Transition,
    still: StillMove,
    output: &Path,
) -> Result<()> {
    let (width, height) = aspect.resolution();
    if is_still(&plan.source) {
        return normalize_still(render, plan, (width, height), transition, still, output);
    }
    let mut filter = format!(
        "{},fps={},setsar=1",
        fit_filter(fit, width, height),
        render.fps
    );
    filter.push_str(&format!(",{SEGMENT_PIXELS}"));
    if transition == Transition::Fade {
        filter.push_str(",fade=t=in:st=0:d=0.5");
    }
    let args = vec![
        "-ss".to_string(),
        format!("{:.3}", plan.start),
        "-t".to_string(),
        format!("{:.3}", plan.duration),
        "-i".to_string(),
        plan.source.display().to_string(),
        "-vf".to_string(),
        filter,
        "-an".to_string(),
        "-c:v".to_string(),
        "libx264".to_string(),
        "-preset".to_string(),
        "veryfast".to_string(),
        "-crf".to_string(),
        render.crf.to_string(),
        "-pix_fmt".to_string(),
        "yuv420p".to_string(),
        "-color_range".to_string(),
        "tv".to_string(),
        output.display().to_string(),
    ];
    run_ffmpeg(&render.ffmpeg_path, &args)
}

/// A still becomes a moving segment of exactly `plan.duration` seconds.
fn normalize_still(
    render: &RenderConfig,
    plan: &SegmentPlan,
    (width, height): (u32, u32),
    transition: Transition,
    still: StillMove,
    output: &Path,
) -> Result<()> {
    let frames = still_frames(plan.duration, render.fps);
    let mut tail = String::new();
    if transition == Transition::Fade {
        tail.push_str(",fade=t=in:st=0:d=0.5");
    }
    tail.push_str(&format!(",{SEGMENT_PIXELS}[v]"));
    // still_graph has exactly one "[v]": its output label.
    let graph = still_graph(still.style, width, height, render.fps, frames, still.variant)
        .replacen("[v]", &tail, 1);
    let args = vec![
        "-i".to_string(),
        plan.source.display().to_string(),
        "-filter_complex".to_string(),
        graph,
        "-map".to_string(),
        "[v]".to_string(),
        "-frames:v".to_string(),
        frames.to_string(),
        "-an".to_string(),
        "-c:v".to_string(),
        "libx264".to_string(),
        "-preset".to_string(),
        "veryfast".to_string(),
        "-crf".to_string(),
        render.crf.to_string(),
        "-pix_fmt".to_string(),
        "yuv420p".to_string(),
        "-color_range".to_string(),
        "tv".to_string(),
        output.display().to_string(),
    ];
    run_ffmpeg(&render.ffmpeg_path, &args)
}

/// Concatenate normalized segments losslessly via the concat demuxer.
pub fn concat_segments(render: &RenderConfig, segments: &[PathBuf], output: &Path) -> Result<()> {
    if segments.is_empty() {
        return Err(Error::Ffmpeg("no segments to concatenate".to_string()));
    }
    let list_path = output.with_extension("concat.txt");
    let mut list = String::new();
    for segment in segments {
        // The concat demuxer resolves entries relative to the list file, so
        // paths must be absolute. Quoting: single-quote wrap, escape quotes.
        let absolute = segment.canonicalize().unwrap_or_else(|_| segment.clone());
        let escaped = absolute.display().to_string().replace('\'', "'\\''");
        list.push_str(&format!("file '{escaped}'\n"));
    }
    std::fs::write(&list_path, list)?;
    let args = vec![
        "-f".to_string(),
        "concat".to_string(),
        "-safe".to_string(),
        "0".to_string(),
        "-i".to_string(),
        list_path.display().to_string(),
        "-c".to_string(),
        "copy".to_string(),
        output.display().to_string(),
    ];
    let result = run_ffmpeg(&render.ffmpeg_path, &args);
    let _ = std::fs::remove_file(&list_path);
    result
}

#[derive(Debug, Clone, Copy)]
pub enum CaptionPosition {
    Bottom,
    Top,
    Center,
    /// 0 = top, 100 = bottom.
    Percent(f64),
}

impl CaptionPosition {
    fn y(self, canvas_height: u32, overlay_height: u32) -> f64 {
        let canvas = canvas_height as f64;
        let overlay = overlay_height as f64;
        match self {
            CaptionPosition::Bottom => canvas * 0.95 - overlay,
            CaptionPosition::Top => canvas * 0.05,
            CaptionPosition::Center => (canvas - overlay) / 2.0,
            CaptionPosition::Percent(pct) => {
                ((canvas - overlay) * pct / 100.0).clamp(10.0, canvas - overlay - 10.0)
            }
        }
    }
}

/// Build the overlay half of the filter graph: caption PNG inputs start at
/// ffmpeg input index `first_input`, chained onto [0:v].
pub fn build_overlay_filter(
    overlays: &[CaptionOverlay],
    first_input: usize,
    canvas_height: u32,
    position: CaptionPosition,
) -> (String, String) {
    if overlays.is_empty() {
        return (String::new(), "0:v".to_string());
    }
    let mut chains = Vec::new();
    let mut current = "0:v".to_string();
    for (index, overlay) in overlays.iter().enumerate() {
        let input = first_input + index;
        let label = format!("cap{index}");
        let y = position.y(canvas_height, overlay.height);
        chains.push(format!(
            "[{current}][{input}:v]overlay=0:{y:.0}:enable='between(t,{:.3},{:.3})':eof_action=repeat[{label}]",
            overlay.start, overlay.end
        ));
        current = label;
    }
    (chains.join(";"), current)
}

/// Build the audio half of the filter graph. Voice is input 1; BGM (if any)
/// is the input at `bgm_input`. BGM is looped upstream via -stream_loop,
/// volume-scaled, and faded out over the last 3 seconds.
pub fn build_audio_filter(
    voice_volume: f64,
    bgm_input: Option<usize>,
    bgm_volume: f64,
    sfx_input: Option<usize>,
    sfx_volume: f64,
    total_duration: f64,
) -> (String, String) {
    if bgm_input.is_none() && sfx_input.is_none() {
        return (format!("[1:a]volume={voice_volume:.3}[aout]"), "aout".to_string());
    }
    let mut chains = vec![format!("[1:a]volume={voice_volume:.3}[voice]")];
    let mut labels = String::from("[voice]");
    if let Some(input) = bgm_input {
        let fade_start = (total_duration - 3.0).max(0.0);
        chains.push(format!(
            "[{input}:a]volume={bgm_volume:.3},afade=t=out:st={fade_start:.3}:d=3[bgm]"
        ));
        labels.push_str("[bgm]");
    }
    if let Some(input) = sfx_input {
        chains.push(format!("[{input}:a]volume={sfx_volume:.3}[sfx]"));
        labels.push_str("[sfx]");
    }
    let count = labels.matches('[').count();
    chains.push(format!("{labels}amix=inputs={count}:duration=first:normalize=0[aout]"));
    (chains.join(";"), "aout".to_string())
}

pub struct RenderJob<'a> {
    pub combined_video: &'a Path,
    pub voice_audio: &'a Path,
    pub bgm_audio: Option<&'a Path>,
    /// Synthesized shutter track (see `sfx`), mixed at `render.sfx_volume`.
    pub sfx_audio: Option<&'a Path>,
    pub overlays: &'a [CaptionOverlay],
    pub output: &'a Path,
    pub duration: f64,
    pub aspect: Aspect,
    pub position: CaptionPosition,
}

/// Final composition, in two ffmpeg runs:
///
/// 1. Audio: narration, optional BGM and optional shutter track mixed and
///    normalized to -14 LUFS / -1 dBTP (measure, optional limiter step, linear
///    `loudnorm`; see `claw_shorts::loudness::plan`) into `<output>.audio.wav`.
/// 2. Video: captions overlaid on the combined video, muxed with that audio.
///
/// The audio has its own run because sharing one filter graph with the caption
/// overlays made the mixed audio differ from run to run (identical inputs gave
/// a different track each time, sometimes shifted ~50 ms and losing a shutter
/// click), while the audio-only graph is deterministic. Returns the loudness
/// measured on the finished file.
pub fn render_final(render: &RenderConfig, job: &RenderJob) -> Result<Loudness> {
    let mix = job.output.with_extension("audio.wav");
    render_audio(render, job, &mix)?;
    let result = render_video(render, job, &mix);
    let _ = std::fs::remove_file(&mix);
    result?;

    // Check the finished file, not the plan.
    let finished = loudness::measure_file(job.output).map_err(Error::Ffmpeg)?;
    let problems = loudness::problems(&finished);
    if !problems.is_empty() {
        return Err(Error::Ffmpeg(format!("final loudness is off target: {}", problems.join("; "))));
    }
    Ok(finished)
}

/// Audio inputs in the order `build_audio_filter` expects: input 0 is an
/// unused placeholder so the voice stays input 1, as in the filter tests.
fn audio_inputs(job: &RenderJob) -> (Vec<String>, Option<usize>, Option<usize>) {
    let mut args: Vec<String> = vec![
        "-f".to_string(),
        "lavfi".to_string(),
        "-i".to_string(),
        "anullsrc=r=48000:cl=mono".to_string(),
        "-i".to_string(),
        job.voice_audio.display().to_string(),
    ];
    let mut next_input = 2;
    let bgm_input = job.bgm_audio.map(|bgm| {
        args.extend([
            "-stream_loop".to_string(),
            "-1".to_string(),
            "-i".to_string(),
            bgm.display().to_string(),
        ]);
        let index = next_input;
        next_input += 1;
        index
    });
    let sfx_input = job.sfx_audio.map(|sfx| {
        args.extend(["-i".to_string(), sfx.display().to_string()]);
        next_input
    });
    (args, bgm_input, sfx_input)
}

fn render_audio(render: &RenderConfig, job: &RenderJob, mix: &Path) -> Result<()> {
    let (inputs, bgm_input, sfx_input) = audio_inputs(job);
    let (audio_filter, audio_label) = build_audio_filter(
        render.voice_volume,
        bgm_input,
        render.bgm_volume,
        sfx_input,
        render.sfx_volume,
        job.duration,
    );
    let duration = format!("{:.3}", job.duration);
    let normalization = loudness::plan(|pre| {
        let chain = match pre {
            Some(pre) => format!("{pre},{}", loudness::measure_filter()),
            None => loudness::measure_filter(),
        };
        let mut measure = Command::new(&render.ffmpeg_path);
        measure
            .args(["-hide_banner", "-nostats"])
            .args(&inputs)
            .arg("-filter_complex")
            .arg(format!("{audio_filter};[{audio_label}]{chain}[measured]"))
            .args(["-map", "[measured]", "-t", &duration, "-f", "null", "-"]);
        loudness::measure_with(&mut measure)
    })
    .map_err(Error::Ffmpeg)?;

    let mut args = inputs;
    args.extend([
        "-filter_complex".to_string(),
        format!("{audio_filter};[{audio_label}]{}[anorm]", normalization.filter),
        "-map".to_string(),
        "[anorm]".to_string(),
        "-t".to_string(),
        duration,
        "-c:a".to_string(),
        "pcm_s16le".to_string(),
        mix.display().to_string(),
    ]);
    run_ffmpeg(&render.ffmpeg_path, &args)
}

fn render_video(render: &RenderConfig, job: &RenderJob, mix: &Path) -> Result<()> {
    let (_, canvas_height) = job.aspect.resolution();
    let mut args: Vec<String> = vec![
        "-i".to_string(),
        job.combined_video.display().to_string(),
        "-i".to_string(),
        mix.display().to_string(),
    ];
    for overlay in job.overlays {
        args.extend(["-i".to_string(), overlay.png_path.display().to_string()]);
    }
    let (overlay_filter, video_label) =
        build_overlay_filter(job.overlays, 2, canvas_height, job.position);
    if !overlay_filter.is_empty() {
        args.extend(["-filter_complex".to_string(), overlay_filter]);
    }
    args.extend([
        "-map".to_string(),
        format!("[{video_label}]").replace("[0:v]", "0:v"),
        "-map".to_string(),
        "1:a".to_string(),
        "-t".to_string(),
        format!("{:.3}", job.duration),
        "-c:v".to_string(),
        "libx264".to_string(),
        "-preset".to_string(),
        "veryfast".to_string(),
        "-crf".to_string(),
        render.crf.to_string(),
        "-pix_fmt".to_string(),
        "yuv420p".to_string(),
        "-c:a".to_string(),
        "aac".to_string(),
        "-b:a".to_string(),
        "192k".to_string(),
        "-movflags".to_string(),
        "+faststart".to_string(),
        job.output.display().to_string(),
    ]);
    run_ffmpeg(&render.ffmpeg_path, &args)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn clip(name: &str, duration: f64) -> SourceClip {
        SourceClip {
            path: PathBuf::from(name),
            duration,
        }
    }

    #[test]
    fn fit_filters_are_wellformed() {
        assert_eq!(
            fit_filter(FitMode::Cover, 1080, 1920),
            "scale=1080:1920:force_original_aspect_ratio=increase,crop=1080:1920"
        );
        assert!(fit_filter(FitMode::Contain, 1080, 1920).contains("pad=1080:1920"));
    }

    #[test]
    fn sequential_takes_one_chunk_per_source() {
        let sources = vec![clip("a.mp4", 12.0), clip("b.mp4", 12.0)];
        let plan = plan_segments(&sources, 5.0, 8.0, ConcatMode::Sequential);
        assert_eq!(plan.len(), 2);
        assert_eq!(plan[0].source, PathBuf::from("a.mp4"));
        assert!((plan[0].duration - 5.0).abs() < 1e-9);
        assert_eq!(plan[1].source, PathBuf::from("b.mp4"));
    }

    #[test]
    fn random_round_robins_across_sources() {
        let sources = vec![clip("a.mp4", 10.0), clip("b.mp4", 10.0)];
        let plan = plan_segments(&sources, 5.0, 18.0, ConcatMode::Random);
        let order: Vec<&str> = plan
            .iter()
            .map(|segment| segment.source.to_str().unwrap())
            .collect();
        assert_eq!(order, vec!["a.mp4", "b.mp4", "a.mp4", "b.mp4"]);
        // Second pass over source a starts at its second chunk.
        assert!((plan[2].start - 5.0).abs() < 1e-9);
    }

    #[test]
    fn plan_loops_when_material_is_short() {
        let sources = vec![clip("a.mp4", 4.0)];
        let plan = plan_segments(&sources, 5.0, 10.0, ConcatMode::Random);
        assert_eq!(plan.len(), 3); // 4s + 4s + 4s covers 10s
        assert_eq!(plan[1].source, plan[0].source);
    }

    #[test]
    fn plan_skips_sub_quarter_second_slivers() {
        let sources = vec![clip("a.mp4", 5.1)];
        let plan = plan_segments(&sources, 5.0, 5.0, ConcatMode::Random);
        assert_eq!(plan.len(), 1, "0.1s tail chunk must be dropped");
    }

    #[test]
    fn overlay_filter_chains_and_labels() {
        let overlays = vec![
            CaptionOverlay {
                png_path: PathBuf::from("c0.png"),
                start: 0.0,
                end: 1.5,
                height: 200,
            },
            CaptionOverlay {
                png_path: PathBuf::from("c1.png"),
                start: 1.5,
                end: 3.0,
                height: 100,
            },
        ];
        let (filter, label) =
            build_overlay_filter(&overlays, 2, 1920, CaptionPosition::Bottom);
        assert_eq!(label, "cap1");
        assert!(filter.contains("[0:v][2:v]overlay"));
        assert!(filter.contains("[cap0][3:v]overlay"));
        assert!(filter.contains("between(t,0.000,1.500)"));
        // Bottom position: 1920*0.95 - 200 = 1624.
        assert!(filter.contains("overlay=0:1624"));
    }

    #[test]
    fn stills_and_videos_are_told_apart_by_extension() {
        assert!(is_still(Path::new("a/photo.JPG")) && is_still(Path::new("b.webp")));
        assert!(!is_still(Path::new("clip.mp4")) && is_video(Path::new("clip.MOV")));
        assert!(!is_still(Path::new("notes.txt")) && !is_video(Path::new("notes.txt")));
    }

    #[test]
    fn still_frames_round_and_never_collapse() {
        assert_eq!(still_frames(4.0, 30), 120);
        assert_eq!(still_frames(0.01, 30), 2);
    }

    #[test]
    fn ken_burns_moves_alternate_and_stay_in_frame() {
        let a = ken_burns_filter(1080, 1920, 30, 120, 0);
        assert!(a.starts_with("scale=2160:3840:force_original_aspect_ratio=increase,crop=2160:3840,zoompan="), "{a}");
        assert!(a.contains("z='1+0.08*on/119'") && a.contains("(0.25+0.5*on/119)"), "{a}");
        assert!(a.contains(":d=120:s=1080x1920:fps=30,setsar=1"), "{a}");
        let b = ken_burns_filter(1080, 1920, 30, 120, 1);
        assert!(b.contains("z='1.08-0.08*on/119'"), "{b}");
        let c = ken_burns_filter(1080, 1920, 30, 120, 2);
        assert!(c.contains("(0.75-0.5*on/119)"), "{c}");
        // x = (iw - iw/zoom) * p with p in [0.25, 0.75] never leaves the image.
        assert_eq!(ken_burns_filter(1080, 1920, 30, 120, 4), a);
    }

    #[test]
    fn frame_layout_is_a_bordered_card_on_brand_color_above_captions() {
        let style = StillStyle::new(StillLayout::Frame, "#0f141b", "#f5b83d").unwrap();
        let g = still_graph(&style, 1080, 1920, 30, 120, 0);
        assert!(g.starts_with("color=c=#0f141b:s=2160x3840:r=1:d=1[base];[0:v]scale=w="), "{g}");
        // 12 px border (2160/180); card 84% of width minus borders; 52% of height.
        assert!(g.contains("scale=w=1790:h=1972"), "{g}");
        assert!(g.contains("pad=iw+24:ih+24:12:12:color=#f5b83d[card]"), "{g}");
        assert!(g.contains("overlay=x=(W-w)/2:y=460+(2226-h)/2,trim=end_frame=1,zoompan="), "{g}");
        assert!(g.ends_with(":d=120:s=1080x1920:fps=30,setsar=1[v]"), "{g}");
        assert_eq!(g.matches("[v]").count(), 1);

        let cover = StillStyle::new(StillLayout::Cover, "#000000", "#ffffff").unwrap();
        let c = still_graph(&cover, 1080, 1920, 30, 120, 0);
        assert_eq!(c, format!("[0:v]{}[v]", ken_burns_filter(1080, 1920, 30, 120, 0)));
    }

    #[test]
    fn frame_colors_must_be_plain_hex() {
        assert!(StillStyle::new(StillLayout::Frame, "#0f141b", "#F5B83D").is_ok());
        for bad in ["red", "#12345", "#0f141b[x]", "0f141b", "#gggggg"] {
            assert!(StillStyle::new(StillLayout::Frame, bad, "#ffffff").is_err(), "{bad}");
        }
    }

    #[test]
    fn audio_filter_with_and_without_bgm() {
        let (voice_only, label) = build_audio_filter(1.0, None, 0.2, None, 0.25, 30.0);
        assert_eq!(label, "aout");
        assert!(!voice_only.contains("amix"));

        let (mixed, _) = build_audio_filter(1.0, Some(2), 0.2, None, 0.25, 30.0);
        assert!(mixed.contains("amix=inputs=2"));
        assert!(mixed.contains("afade=t=out:st=27.000:d=3"));

        let (all, _) = build_audio_filter(1.0, Some(2), 0.2, Some(3), 0.25, 30.0);
        assert!(all.contains("[3:a]volume=0.250[sfx]"), "{all}");
        assert!(all.contains("[voice][bgm][sfx]amix=inputs=3:duration=first:normalize=0[aout]"), "{all}");
        let (sfx_only, _) = build_audio_filter(1.0, None, 0.2, Some(2), 0.25, 30.0);
        assert!(sfx_only.contains("[voice][sfx]amix=inputs=2"), "{sfx_only}");
    }
}
