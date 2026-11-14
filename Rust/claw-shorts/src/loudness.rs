//! Loudness: every Short leaves at -14 LUFS integrated with a -1 dBTP ceiling.
//!
//! Phones play at the platform's reference level, and a narration that sounds
//! fine on studio headphones can be quiet or clip on a phone speaker (ginyboi's
//! "don't trust your ears" tip; -14 LUFS is YouTube's reference). Normalization
//! measures the joined narration, then applies `loudnorm` in linear mode with
//! those measurements, so the voice keeps its dynamics instead of being pumped
//! by the single-pass dynamic mode.
//!
//! Linear gain cannot reach -14 LUFS when the voice's peaks would pass -1 dBTP
//! first (TTS such as macOS `say` measured -19.4 LUFS with peaks at -1.2 dBTP).
//! Then `loudnorm` silently falls back to dynamic mode and lands about 1 LU
//! short. For that case [`plan`] adds ginyboi's limiter step: boost into a hard
//! limiter, re-measure, and finish with linear `loudnorm`.

use std::path::Path;
use std::process::Command;

use serde::{Deserialize, Serialize};

pub const TARGET_LUFS: f64 = -14.0;
pub const TARGET_TRUE_PEAK_DBTP: f64 = -1.0;
pub const TARGET_LRA: f64 = 11.0;
/// How far the final file may sit from the target before a render fails.
pub const TOLERANCE_LU: f64 = 1.0;

/// One `loudnorm` measurement of a signal.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct Loudness {
    pub integrated_lufs: f64,
    pub true_peak_dbtp: f64,
    pub lra_lu: f64,
    pub threshold_lufs: f64,
    pub target_offset_lu: f64,
}

#[derive(Deserialize)]
struct LoudnormJson {
    input_i: String,
    input_tp: String,
    input_lra: String,
    input_thresh: String,
    target_offset: String,
}

fn targets() -> String {
    format!("I={TARGET_LUFS}:TP={TARGET_TRUE_PEAK_DBTP}:LRA={TARGET_LRA}")
}

/// First pass: measure only. The JSON goes to stderr.
pub fn measure_filter() -> String {
    format!("loudnorm={}:print_format=json", targets())
}

/// Second pass: linear gain from a measurement, resampled back to 48 kHz
/// (loudnorm works at 192 kHz internally). `None` when the measurement is not
/// usable, e.g. silence measures as -inf.
pub fn apply_filter(m: &Loudness) -> Option<String> {
    let finite = [m.integrated_lufs, m.true_peak_dbtp, m.lra_lu, m.threshold_lufs, m.target_offset_lu]
        .iter()
        .all(|v| v.is_finite());
    finite.then(|| {
        format!(
            "loudnorm={}:measured_I={:.2}:measured_TP={:.2}:measured_LRA={:.2}:measured_thresh={:.2}:offset={:.2}:linear=true:print_format=none,aresample=48000",
            targets(),
            m.integrated_lufs,
            m.true_peak_dbtp,
            m.lra_lu,
            m.threshold_lufs,
            m.target_offset_lu
        )
    })
}

/// Sample-peak ceiling of the limiter step. True peak lands a few tenths
/// higher, still under the -1 dBTP target.
pub const LIMITER_CEILING_DBFS: f64 = -2.0;
/// Boost beyond the target into the limiter. Limiting removes some loudness;
/// 2 dB of drive put `say` narration at -14.0 LUFS before the final pass.
pub const LIMITER_DRIVE_DB: f64 = 2.0;

/// True when plain linear gain reaches the target without passing the peak ceiling.
pub fn linear_fits(m: &Loudness) -> bool {
    m.true_peak_dbtp + (TARGET_LUFS - m.integrated_lufs) <= TARGET_TRUE_PEAK_DBTP
}

/// Gain into a hard limiter (no auto-level), sized from a raw measurement.
pub fn limiter_filter(m: &Loudness) -> String {
    format!(
        "volume={:.2}dB,alimiter=limit={:.4}:attack=5:release=50:level=disabled",
        TARGET_LUFS - m.integrated_lufs + LIMITER_DRIVE_DB,
        10f64.powf(LIMITER_CEILING_DBFS / 20.0)
    )
}

/// The normalization chosen for one signal.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Normalization {
    /// Filter chain to append to the signal.
    pub filter: String,
    /// The signal as it came in.
    pub measured: Loudness,
    /// Whether the limiter step was needed.
    pub limited: bool,
}

/// Choose the normalization. `measure(pre)` must measure the signal after the
/// optional `pre` filter chain. Two measurements when linear gain fits, three
/// otherwise (raw, then after the limiter).
pub fn plan<F>(mut measure: F) -> Result<Normalization, String>
where
    F: FnMut(Option<&str>) -> Result<Loudness, String>,
{
    let measured = measure(None)?;
    let silent = || "the narration is silent; loudness cannot be normalized".to_string();
    if !measured.integrated_lufs.is_finite() {
        return Err(silent());
    }
    if linear_fits(&measured) {
        let filter = apply_filter(&measured).ok_or_else(silent)?;
        return Ok(Normalization { filter, measured, limited: false });
    }
    let limiter = limiter_filter(&measured);
    let limited = measure(Some(&limiter))?;
    let finish = apply_filter(&limited).ok_or_else(silent)?;
    Ok(Normalization { filter: format!("{limiter},{finish}"), measured, limited: true })
}

/// Parse the last JSON object `loudnorm` printed to stderr.
pub fn parse(stderr: &str) -> Result<Loudness, String> {
    let start = stderr.rfind('{').ok_or("loudnorm printed no measurement")?;
    let end = stderr[start..].find('}').ok_or("loudnorm measurement is truncated")? + start;
    let raw: LoudnormJson =
        serde_json::from_str(&stderr[start..=end]).map_err(|e| format!("bad loudnorm output: {e}"))?;
    let num = |name: &str, v: &str| -> Result<f64, String> {
        v.trim().parse::<f64>().map_err(|e| format!("loudnorm {name} {v:?}: {e}"))
    };
    Ok(Loudness {
        integrated_lufs: num("input_i", &raw.input_i)?,
        true_peak_dbtp: num("input_tp", &raw.input_tp)?,
        lra_lu: num("input_lra", &raw.input_lra)?,
        threshold_lufs: num("input_thresh", &raw.input_thresh)?,
        target_offset_lu: num("target_offset", &raw.target_offset)?,
    })
}

/// Measure a filter graph whose `[aout]` is the signal to measure. `inputs`
/// are the graph's audio inputs in order.
pub fn measure_graph(inputs: &[&Path], graph: &str) -> Result<Loudness, String> {
    let mut cmd = Command::new("ffmpeg");
    cmd.args(["-hide_banner", "-nostats"]);
    for input in inputs {
        cmd.arg("-i").arg(input);
    }
    cmd.arg("-filter_complex")
        .arg(format!("{graph};[aout]{}[measured]", measure_filter()))
        .args(["-map", "[measured]", "-f", "null", "-"]);
    measure_with(&mut cmd)
}

/// Measure the first audio stream of a finished file.
pub fn measure_file(path: &Path) -> Result<Loudness, String> {
    let mut cmd = Command::new("ffmpeg");
    cmd.args(["-hide_banner", "-nostats", "-i"])
        .arg(path)
        .args(["-map", "0:a:0", "-af", &measure_filter(), "-f", "null", "-"]);
    measure_with(&mut cmd)
}

/// Run a command that ends in `measure_filter()` writing to `-f null -`, and
/// parse its measurement. For graphs [`measure_graph`] cannot express, such as
/// inputs that need their own options (`-stream_loop`).
pub fn measure_with(cmd: &mut Command) -> Result<Loudness, String> {
    let output = cmd.output().map_err(|e| format!("ffmpeg loudness: could not start: {e}"))?;
    let stderr = String::from_utf8_lossy(&output.stderr);
    if !output.status.success() {
        let tail: Vec<&str> = stderr.lines().rev().take(8).collect();
        return Err(format!("ffmpeg loudness measurement failed:\n{}", tail.into_iter().rev().collect::<Vec<_>>().join("\n")));
    }
    parse(&stderr)
}

/// Problems with a finished file's loudness, empty when it meets the target.
pub fn problems(m: &Loudness) -> Vec<String> {
    let mut out = Vec::new();
    if !m.integrated_lufs.is_finite() || (m.integrated_lufs - TARGET_LUFS).abs() > TOLERANCE_LU {
        out.push(format!(
            "integrated loudness is {:.1} LUFS; expected {TARGET_LUFS} ± {TOLERANCE_LU}",
            m.integrated_lufs
        ));
    }
    // AAC encoding can add a few tenths of a dB of overshoot above the -1 dBTP
    // ceiling; only a peak at or above full scale is a real clip.
    if m.true_peak_dbtp >= 0.0 {
        out.push(format!("true peak is {:.1} dBTP; the audio clips", m.true_peak_dbtp));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"[Parsed_loudnorm_0 @ 0x1]
{
	"input_i" : "-27.61",
	"input_tp" : "-4.47",
	"input_lra" : "18.06",
	"input_thresh" : "-39.20",
	"output_i" : "-16.58",
	"output_tp" : "-1.50",
	"output_lra" : "14.78",
	"output_thresh" : "-27.71",
	"normalization_type" : "dynamic",
	"target_offset" : "0.58"
}
"#;

    #[test]
    fn parses_the_loudnorm_json_block() {
        let m = parse(&format!("noise {{not json}}\n{SAMPLE}")).unwrap();
        assert_eq!(m.integrated_lufs, -27.61);
        assert_eq!(m.true_peak_dbtp, -4.47);
        assert_eq!(m.lra_lu, 18.06);
        assert_eq!(m.threshold_lufs, -39.20);
        assert_eq!(m.target_offset_lu, 0.58);
    }

    #[test]
    fn missing_or_bad_output_is_an_error() {
        assert!(parse("no json here").is_err());
        assert!(parse("{\"input_i\": \"x\"}").is_err());
    }

    #[test]
    fn apply_filter_is_linear_with_measurements_and_skips_silence() {
        let m = parse(SAMPLE).unwrap();
        let f = apply_filter(&m).unwrap();
        assert!(f.starts_with("loudnorm=I=-14:TP=-1:LRA=11:measured_I=-27.61:"), "{f}");
        assert!(f.contains("offset=0.58:linear=true") && f.ends_with("aresample=48000"), "{f}");
        let silent = Loudness { integrated_lufs: f64::NEG_INFINITY, ..m };
        assert!(apply_filter(&silent).is_none());
        assert_eq!(parse(&SAMPLE.replace("-27.61", "-inf")).unwrap().integrated_lufs, f64::NEG_INFINITY);
    }

    fn level(i: f64, tp: f64) -> Loudness {
        Loudness { integrated_lufs: i, true_peak_dbtp: tp, lra_lu: 5.0, threshold_lufs: i - 10.0, target_offset_lu: 0.0 }
    }

    #[test]
    fn linear_gain_is_used_when_the_peaks_allow_it() {
        let mut calls = Vec::new();
        let plan = plan(|pre| {
            calls.push(pre.map(str::to_string));
            Ok(level(-22.7, -12.0))
        })
        .unwrap();
        assert_eq!(calls, vec![None]);
        assert!(!plan.limited && plan.filter.starts_with("loudnorm=") && plan.filter.contains("linear=true"));
    }

    #[test]
    fn peaky_speech_goes_through_the_limiter_and_is_re_measured() {
        // macOS `say`: -19.38 LUFS, -1.21 dBTP. +5.38 dB of linear gain would peak at +4.2 dBTP.
        let raw = level(-19.38, -1.21);
        assert!(!linear_fits(&raw));
        let mut calls = Vec::new();
        let plan = plan(|pre| {
            calls.push(pre.map(str::to_string));
            Ok(if pre.is_some() { level(-13.99, -1.49) } else { raw })
        })
        .unwrap();
        assert_eq!(calls.len(), 2);
        let pre = calls[1].as_deref().unwrap();
        assert_eq!(pre, "volume=7.38dB,alimiter=limit=0.7943:attack=5:release=50:level=disabled");
        assert!(plan.limited && plan.filter.starts_with(pre) && plan.filter.contains("measured_I=-13.99"));
        assert_eq!(plan.measured, raw);
    }

    #[test]
    fn silence_cannot_be_normalized() {
        assert!(plan(|_| Ok(level(f64::NEG_INFINITY, f64::NEG_INFINITY))).unwrap_err().contains("silent"));
    }

    #[test]
    fn problems_flag_off_target_loudness_and_clipping() {
        let ok = Loudness { integrated_lufs: -14.3, true_peak_dbtp: -1.2, lra_lu: 5.0, threshold_lufs: -24.0, target_offset_lu: 0.0 };
        assert!(problems(&ok).is_empty());
        assert_eq!(problems(&Loudness { integrated_lufs: -19.5, ..ok }).len(), 1);
        assert_eq!(problems(&Loudness { true_peak_dbtp: 0.2, ..ok }).len(), 1);
        assert_eq!(problems(&Loudness { integrated_lufs: f64::NEG_INFINITY, ..ok }).len(), 1);
    }
}
