use crate::error::{Error, Result};
use serde::{Deserialize, Serialize};
use std::path::Path;

/// One subtitle cue, in seconds.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Cue {
    pub start: f64,
    pub end: f64,
    pub text: String,
}

/// A single spoken word with its timeline position, as reported by a TTS
/// engine (Edge TTS WordBoundary events).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WordBoundary {
    pub start: f64,
    pub end: f64,
    pub text: String,
}

/// Split a narration script into subtitle-sized lines at sentence punctuation.
/// Handles both Latin and CJK punctuation so zh/ja scripts stay readable
/// instead of degrading into per-word cues.
pub fn split_script_lines(script: &str) -> Vec<String> {
    const BREAK_CHARS: &[char] = &[
        '.', '?', '!', ';', ':', ',', '\n', '。', '？', '！', '；', '：', '，', '…',
    ];
    let mut lines = Vec::new();
    let mut current = String::new();
    for ch in script.chars() {
        if BREAK_CHARS.contains(&ch) {
            if !current.trim().is_empty() {
                lines.push(current.trim().to_string());
            }
            current.clear();
        } else {
            current.push(ch);
        }
    }
    if !current.trim().is_empty() {
        lines.push(current.trim().to_string());
    }
    lines
}

/// Comparison form used to match accumulated TTS words against a script line:
/// lowercase with all whitespace and punctuation removed. TTS engines drop or
/// re-space punctuation, so exact string equality would never converge.
fn normalized(text: &str) -> String {
    text.chars()
        .filter(|ch| ch.is_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect()
}

/// Aggregate fine-grained word boundaries into script-line cues.
///
/// Ported from MoneyPrinterTurbo's `_build_subtitle_items_from_edge_cues`:
/// consume word cues in order, accumulating text until it matches the next
/// script line; the cue then spans first-word start to last-word end, keeping
/// the timeline continuous. Returns None when the accumulation never lines up
/// (e.g. the engine skipped words), so callers can fall back to proportional
/// timing instead of writing a broken timeline.
pub fn cues_from_word_boundaries(words: &[WordBoundary], lines: &[String]) -> Option<Vec<Cue>> {
    let mut cues = Vec::new();
    let mut line_index = 0;
    let mut accumulated = String::new();
    let mut cue_start: Option<f64> = None;

    for word in words {
        if cue_start.is_none() {
            cue_start = Some(word.start);
        }
        accumulated.push_str(&word.text);

        let Some(line) = lines.get(line_index) else {
            break;
        };
        if normalized(&accumulated) == normalized(line) {
            cues.push(Cue {
                start: cue_start.take().unwrap_or(0.0),
                end: word.end,
                text: line.clone(),
            });
            accumulated.clear();
            line_index += 1;
        }
    }

    (line_index == lines.len() && !cues.is_empty()).then_some(cues)
}

/// Distribute `total_duration` across script lines proportionally to their
/// character count. Used when the TTS backend reports no word boundaries
/// (macOS `say`, custom audio files).
pub fn cues_proportional(lines: &[String], total_duration: f64) -> Vec<Cue> {
    let weights: Vec<f64> = lines
        .iter()
        .map(|line| line.chars().count().max(1) as f64)
        .collect();
    let total_weight: f64 = weights.iter().sum();
    if total_weight <= 0.0 || total_duration <= 0.0 {
        return Vec::new();
    }

    let mut cues = Vec::new();
    let mut elapsed = 0.0;
    for (line, weight) in lines.iter().zip(&weights) {
        let duration = total_duration * weight / total_weight;
        cues.push(Cue {
            start: elapsed,
            end: elapsed + duration,
            text: line.clone(),
        });
        elapsed += duration;
    }
    if let Some(last) = cues.last_mut() {
        last.end = total_duration;
    }
    cues
}

pub fn format_srt_timestamp(seconds: f64) -> String {
    let total_millis = (seconds.max(0.0) * 1000.0).round() as u64;
    let millis = total_millis % 1000;
    let total_secs = total_millis / 1000;
    format!(
        "{:02}:{:02}:{:02},{:03}",
        total_secs / 3600,
        (total_secs % 3600) / 60,
        total_secs % 60,
        millis
    )
}

pub fn write_srt(cues: &[Cue], path: &Path) -> Result<()> {
    if cues.is_empty() {
        return Err(Error::Subtitle("no cues to write".to_string()));
    }
    let mut out = String::new();
    for (index, cue) in cues.iter().enumerate() {
        out.push_str(&format!(
            "{}\n{} --> {}\n{}\n\n",
            index + 1,
            format_srt_timestamp(cue.start),
            format_srt_timestamp(cue.end),
            cue.text.trim()
        ));
    }
    std::fs::write(path, out)?;
    Ok(())
}

pub fn read_srt(path: &Path) -> Result<Vec<Cue>> {
    let text = std::fs::read_to_string(path)?;
    let mut cues = Vec::new();
    for block in text.replace("\r\n", "\n").split("\n\n") {
        let mut lines = block.lines().filter(|line| !line.trim().is_empty());
        let Some(first) = lines.next() else { continue };
        // The index line is optional in the wild; the timing line is not.
        let timing = if first.contains("-->") {
            first
        } else {
            match lines.next() {
                Some(line) if line.contains("-->") => line,
                _ => continue,
            }
        };
        let Some((start, end)) = parse_timing_line(timing) else {
            continue;
        };
        let text: String = lines.collect::<Vec<_>>().join("\n");
        if !text.trim().is_empty() {
            cues.push(Cue {
                start,
                end,
                text: text.trim().to_string(),
            });
        }
    }
    Ok(cues)
}

fn parse_timing_line(line: &str) -> Option<(f64, f64)> {
    let (start, end) = line.split_once("-->")?;
    Some((parse_srt_timestamp(start)?, parse_srt_timestamp(end)?))
}

fn parse_srt_timestamp(text: &str) -> Option<f64> {
    let text = text.trim().replace(',', ".");
    let parts: Vec<&str> = text.split(':').collect();
    let [hours, minutes, seconds] = parts.as_slice() else {
        return None;
    };
    Some(
        hours.parse::<f64>().ok()? * 3600.0
            + minutes.parse::<f64>().ok()? * 60.0
            + seconds.parse::<f64>().ok()?,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_latin_and_cjk_punctuation() {
        let lines = split_script_lines("Money is a tool. It stores value, and moves it.");
        assert_eq!(
            lines,
            vec!["Money is a tool", "It stores value", "and moves it"]
        );
        let zh = split_script_lines("金钱是工具。它储存价值，也转移价值。");
        assert_eq!(zh, vec!["金钱是工具", "它储存价值", "也转移价值"]);
    }

    #[test]
    fn aggregates_word_boundaries_to_lines() {
        let words = [
            ("Money", 0.0, 0.4),
            ("is", 0.4, 0.6),
            ("a", 0.6, 0.7),
            ("tool.", 0.7, 1.1),
            ("It", 1.3, 1.5),
            ("works.", 1.5, 2.0),
        ]
        .map(|(text, start, end)| WordBoundary {
            start,
            end,
            text: text.to_string(),
        });
        let lines = split_script_lines("Money is a tool. It works.");
        let cues = cues_from_word_boundaries(&words, &lines).unwrap();
        assert_eq!(cues.len(), 2);
        assert_eq!(cues[0].text, "Money is a tool");
        assert!((cues[0].start - 0.0).abs() < 1e-9);
        assert!((cues[0].end - 1.1).abs() < 1e-9);
        assert!((cues[1].start - 1.3).abs() < 1e-9);
    }

    #[test]
    fn word_aggregation_fails_soft_on_mismatch() {
        let words = [WordBoundary {
            start: 0.0,
            end: 1.0,
            text: "completely different words".to_string(),
        }];
        let lines = split_script_lines("Money is a tool.");
        assert!(cues_from_word_boundaries(&words, &lines).is_none());
    }

    #[test]
    fn proportional_cues_cover_full_duration() {
        let lines = vec!["short".to_string(), "a much longer line here".to_string()];
        let cues = cues_proportional(&lines, 10.0);
        assert_eq!(cues.len(), 2);
        assert!((cues[0].start - 0.0).abs() < 1e-9);
        assert!((cues[1].end - 10.0).abs() < 1e-9);
        assert!(cues[1].end - cues[1].start > cues[0].end - cues[0].start);
    }

    #[test]
    fn srt_round_trip() {
        let dir = std::env::temp_dir().join("shortform-video-srt-test");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("test.srt");
        let cues = vec![
            Cue {
                start: 0.0,
                end: 1.25,
                text: "Hello there".to_string(),
            },
            Cue {
                start: 1.5,
                end: 3.0,
                text: "General Kenobi".to_string(),
            },
        ];
        write_srt(&cues, &path).unwrap();
        let read_back = read_srt(&path).unwrap();
        assert_eq!(read_back, cues);
    }

    #[test]
    fn timestamp_formatting() {
        assert_eq!(format_srt_timestamp(0.0), "00:00:00,000");
        assert_eq!(format_srt_timestamp(3661.5), "01:01:01,500");
    }
}
