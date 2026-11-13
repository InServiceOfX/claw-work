//! Scene timing derived from measured narration lengths.

use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SceneTiming {
    pub index: usize,
    pub start_ms: u64,
    pub end_ms: u64,
    /// Measured length of the scene's narration audio.
    pub audio_ms: u64,
}

impl SceneTiming {
    pub fn duration_ms(&self) -> u64 {
        self.end_ms - self.start_ms
    }
}

/// Back-to-back scenes: each lasts its narration plus `pad_ms` of silence.
pub fn build(audio_ms: &[u64], pad_ms: u32) -> Vec<SceneTiming> {
    let mut start = 0u64;
    audio_ms
        .iter()
        .enumerate()
        .map(|(index, &audio)| {
            let end = start + audio + u64::from(pad_ms);
            let timing = SceneTiming { index, start_ms: start, end_ms: end, audio_ms: audio };
            start = end;
            timing
        })
        .collect()
}

pub fn total_ms(timings: &[SceneTiming]) -> u64 {
    timings.last().map_or(0, |t| t.end_ms)
}

/// Seconds with millisecond precision, for FFmpeg expressions.
pub fn secs(ms: u64) -> String {
    format!("{}.{:03}", ms / 1000, ms % 1000)
}

/// `HH:MM:SS,mmm` for SubRip captions.
pub fn srt_time(ms: u64) -> String {
    format!(
        "{:02}:{:02}:{:02},{:03}",
        ms / 3_600_000,
        ms / 60_000 % 60,
        ms / 1000 % 60,
        ms % 1000
    )
}

/// Scenes longer than `limit_ms`: a long hold with one idea is where viewers
/// leave, so callers warn about them.
pub fn long_scenes(timings: &[SceneTiming], limit_ms: u64) -> Vec<usize> {
    timings.iter().filter(|t| t.duration_ms() > limit_ms).map(|t| t.index).collect()
}

/// How long the typed command takes to appear: 40 ms per character, at least
/// 600 ms, and never more than 40% of the scene.
pub fn typing_ms(chars: usize, scene_ms: u64) -> u64 {
    let wanted = (chars as u64 * 40).max(600);
    wanted.min(scene_ms * 2 / 5).max(1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scenes_are_contiguous_and_padded() {
        let t = build(&[1000, 2500], 350);
        assert_eq!((t[0].start_ms, t[0].end_ms), (0, 1350));
        assert_eq!((t[1].start_ms, t[1].end_ms), (1350, 4200));
        assert_eq!(total_ms(&t), 4200);
        assert_eq!(t[1].duration_ms(), 2850);
        assert_eq!(total_ms(&[]), 0);
    }

    #[test]
    fn formats_times() {
        assert_eq!(secs(1005), "1.005");
        assert_eq!(secs(60_000), "60.000");
        assert_eq!(srt_time(3_723_456), "01:02:03,456");
    }

    #[test]
    fn finds_long_scenes() {
        let t = build(&[3000, 11_000, 9_000], 350);
        assert_eq!(long_scenes(&t, 10_000), vec![1]);
        assert!(long_scenes(&t, 20_000).is_empty());
    }

    #[test]
    fn typing_is_bounded() {
        assert_eq!(typing_ms(10, 10_000), 600);
        assert_eq!(typing_ms(43, 10_000), 1720);
        assert_eq!(typing_ms(200, 10_000), 4000);
        assert!(typing_ms(5, 1) >= 1);
    }
}
