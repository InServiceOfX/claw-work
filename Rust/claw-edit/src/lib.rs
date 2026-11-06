use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Plan {
    pub version: u32,
    pub source: String,
    pub width: u32,
    pub height: u32,
    pub fps: u32,
    pub clips: Vec<Clip>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Clip {
    pub start_ms: u64,
    pub end_ms: u64,
    pub text: String,
}

#[derive(Debug, Serialize)]
pub struct Timeline {
    pub duration_ms: u64,
    pub clips: Vec<TimedClip>,
}

#[derive(Debug, Serialize)]
pub struct TimedClip {
    pub source_start_ms: u64,
    pub source_end_ms: u64,
    pub output_start_ms: u64,
    pub output_end_ms: u64,
}

/// Millisecond cut lists are interpreted in supplied order, including repeats.
pub fn validate(plan: &Plan, source_duration_ms: u64) -> Result<Timeline, String> {
    if plan.version != 1 {
        return Err("unsupported plan version".into());
    }
    if plan.source.trim().is_empty() || plan.source.contains('\0') {
        return Err("source must name a local file".into());
    }
    if [plan.width, plan.height]
        .iter()
        .any(|v| *v < 2 || *v > 3840 || v % 2 != 0)
        || !(1..=60).contains(&plan.fps)
    {
        return Err("dimensions must be even, 2..3840; fps must be 1..60".into());
    }
    if plan.clips.is_empty() || plan.clips.len() > 128 {
        return Err("supply 1..128 clips".into());
    }
    let mut cursor: u64 = 0;
    let mut clips = Vec::new();
    for (index, clip) in plan.clips.iter().enumerate() {
        if clip.start_ms >= clip.end_ms || clip.end_ms > source_duration_ms {
            return Err(format!(
                "clip {} is outside source duration or empty",
                index + 1
            ));
        }
        if clip.text.trim().is_empty()
            || clip.text.chars().count() > 500
            || clip.text.chars().any(char::is_control)
            || clip.text.contains("-->")
        {
            return Err(format!(
                "clip {} needs one plain caption line (1..500 characters)",
                index + 1
            ));
        }
        let start = cursor;
        cursor = cursor
            .checked_add(clip.end_ms - clip.start_ms)
            .ok_or("duration overflow")?;
        if cursor > 600_000 {
            return Err("output exceeds ten-minute short-form limit".into());
        }
        clips.push(TimedClip {
            source_start_ms: clip.start_ms,
            source_end_ms: clip.end_ms,
            output_start_ms: start,
            output_end_ms: cursor,
        });
    }
    Ok(Timeline {
        duration_ms: cursor,
        clips,
    })
}

fn srt_time(ms: u64) -> String {
    format!(
        "{:02}:{:02}:{:02},{:03}",
        ms / 3_600_000,
        ms / 60_000 % 60,
        ms / 1000 % 60,
        ms % 1000
    )
}

pub fn captions(plan: &Plan, timeline: &Timeline) -> String {
    plan.clips
        .iter()
        .zip(&timeline.clips)
        .enumerate()
        .map(|(i, (clip, time))| {
            format!(
                "{}\n{} --> {}\n{}\n\n",
                i + 1,
                srt_time(time.output_start_ms),
                srt_time(time.output_end_ms),
                clip.text.trim()
            )
        })
        .collect()
}

fn seconds(ms: u64) -> String {
    format!("{}.{:03}", ms / 1000, ms % 1000)
}

/// Only validated numeric values enter filter syntax; paths/text never do.
pub fn filter_graph(plan: &Plan) -> String {
    let count = plan.clips.len();
    let mut graph = format!("[0:v:0]setpts=PTS-STARTPTS,split={count}");
    for i in 0..count {
        graph.push_str(&format!("[vi{i}]"));
    }
    graph.push_str(&format!(";[0:a:0]asetpts=PTS-STARTPTS,asplit={count}"));
    for i in 0..count {
        graph.push_str(&format!("[ai{i}]"));
    }
    for (i, clip) in plan.clips.iter().enumerate() {
        let start = seconds(clip.start_ms);
        let end = seconds(clip.end_ms);
        graph.push_str(&format!(
            ";[vi{i}]trim=start={start}:end={end},setpts=PTS-STARTPTS,scale={}:{}:force_original_aspect_ratio=decrease:force_divisible_by=2,pad={}:{}:(ow-iw)/2:(oh-ih)/2,setsar=1,fps={},format=yuv420p[v{i}];[ai{i}]atrim=start={start}:end={end},asetpts=PTS-STARTPTS,aresample=48000[a{i}]",
            plan.width, plan.height, plan.width, plan.height, plan.fps
        ));
    }
    graph.push(';');
    for i in 0..count {
        graph.push_str(&format!("[v{i}][a{i}]"));
    }
    graph.push_str(&format!("concat=n={count}:v=1:a=1[vout][aout]"));
    graph
}

#[cfg(test)]
mod tests {
    use super::*;
    fn plan() -> Plan {
        serde_json::from_str(r#"{"version":1,"source":"raw.mp4","width":360,"height":640,"fps":30,"clips":[{"start_ms":3000,"end_ms":4100,"text":"Second take"},{"start_ms":500,"end_ms":1600,"text":"Earlier line"}]}"#).unwrap()
    }
    #[test]
    fn reorders_and_retimes_instead_of_using_source_timestamps() {
        let p = plan();
        let t = validate(&p, 5000).unwrap();
        assert_eq!(t.duration_ms, 2200);
        assert_eq!(t.clips[1].output_start_ms, 1100);
        assert!(captions(&p, &t).contains("00:00:01,100 --> 00:00:02,200\nEarlier line"));
    }
    #[test]
    fn rejects_invalid_ranges() {
        for (a, b) in [(10, 10), (12, 11), (0, 5001), (u64::MAX, u64::MAX)] {
            let mut p = plan();
            p.clips[0].start_ms = a;
            p.clips[0].end_ms = b;
            assert!(validate(&p, 5000).is_err());
        }
    }
    #[test]
    fn rejects_caption_injection_and_empty_text() {
        for text in ["", "   ", "a\nb", "a\rb", "a\0b", "00 --> 01"] {
            let mut p = plan();
            p.clips[0].text = text.into();
            assert!(validate(&p, 5000).is_err());
        }
    }
    #[test]
    fn rejects_unsupported_geometry_versions_and_resource_limits() {
        let mut p = plan();
        p.width = 361;
        assert!(validate(&p, 5000).is_err());
        p.width = 360;
        p.fps = 0;
        assert!(validate(&p, 5000).is_err());
        p.fps = 30;
        p.version = 2;
        assert!(validate(&p, 5000).is_err());
        p.version = 1;
        p.clips.clear();
        assert!(validate(&p, 5000).is_err());
        p.clips = (0..129)
            .map(|_| Clip {
                start_ms: 0,
                end_ms: 1,
                text: "x".into(),
            })
            .collect();
        assert!(validate(&p, 5000).is_err());
        p.clips.truncate(1);
        p.clips[0].end_ms = 600_001;
        assert!(validate(&p, 700_000).is_err());
    }
    #[test]
    fn rejects_unknown_fields_instead_of_silently_ignoring_effects() {
        let p = r#"{"version":1,"source":"raw.mp4","width":360,"height":640,"fps":30,"clips":[],"zoom":1.5}"#;
        assert!(serde_json::from_str::<Plan>(p).is_err());
    }
    #[test]
    fn graph_keeps_audio_and_video_ranges_in_sync() {
        let p = plan();
        let g = filter_graph(&p);
        assert!(g.contains("trim=start=3.000:end=4.100,setpts=PTS-STARTPTS"));
        assert!(g.contains("atrim=start=3.000:end=4.100,asetpts=PTS-STARTPTS"));
        assert!(g.ends_with("[v0][a0][v1][a1]concat=n=2:v=1:a=1[vout][aout]"));
        assert!(!g.contains("Earlier line"));
    }
}
