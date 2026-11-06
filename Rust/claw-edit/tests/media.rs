use serde_json::json;
use std::{
    fs,
    path::Path,
    process::{Command, Output},
};

fn run(program: &str, args: &[&str]) -> Output {
    let out = Command::new(program).args(args).output().unwrap();
    assert!(
        out.status.success(),
        "{program}: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    out
}
fn cli(plan: &Path, dest: &Path) -> Output {
    Command::new(env!("CARGO_BIN_EXE_claw-edit"))
        .arg("render")
        .arg("--plan")
        .arg(plan)
        .arg("--output-dir")
        .arg(dest)
        .output()
        .unwrap()
}
fn pixel(video: &Path, time: &str) -> Vec<u8> {
    run(
        "ffmpeg",
        &[
            "-v",
            "error",
            "-ss",
            time,
            "-i",
            video.to_str().unwrap(),
            "-frames:v",
            "1",
            "-vf",
            "scale=1:1",
            "-pix_fmt",
            "rgb24",
            "-f",
            "rawvideo",
            "-",
        ],
    )
    .stdout
}
fn frequency(video: &Path, time: &str) -> f64 {
    let data = run(
        "ffmpeg",
        &[
            "-v",
            "error",
            "-ss",
            time,
            "-i",
            video.to_str().unwrap(),
            "-t",
            "0.2",
            "-vn",
            "-ac",
            "1",
            "-ar",
            "48000",
            "-f",
            "s16le",
            "-",
        ],
    )
    .stdout;
    let samples: Vec<i16> = data
        .chunks_exact(2)
        .map(|b| i16::from_le_bytes([b[0], b[1]]))
        .collect();
    let crossings = samples.windows(2).filter(|p| p[0] <= 0 && p[1] > 0).count();
    crossings as f64 * 48000.0 / samples.len() as f64
}

#[test]
#[ignore = "requires local FFmpeg with libx264/AAC; synthetic media only"]
fn renders_reordered_video_audio_and_captions_without_overwriting() {
    let dir = tempfile::tempdir().unwrap();
    let source = dir.path().join("raw ' ; touch SHOULD_NOT_EXIST ; '.mp4");
    run(
        "ffmpeg",
        &[
            "-v",
            "error",
            "-nostdin",
            "-n",
            "-f",
            "lavfi",
            "-i",
            "color=red:s=160x240:r=30:d=2",
            "-f",
            "lavfi",
            "-i",
            "sine=frequency=440:sample_rate=48000:duration=2",
            "-f",
            "lavfi",
            "-i",
            "color=blue:s=160x240:r=30:d=2",
            "-f",
            "lavfi",
            "-i",
            "sine=frequency=880:sample_rate=48000:duration=2",
            "-filter_complex",
            "[0:v][1:a][2:v][3:a]concat=n=2:v=1:a=1[v][a]",
            "-map",
            "[v]",
            "-map",
            "[a]",
            "-c:v",
            "libx264",
            "-pix_fmt",
            "yuv420p",
            "-c:a",
            "aac",
            source.to_str().unwrap(),
        ],
    );
    let original = fs::read(&source).unwrap();
    let mut plan = json!({"version":1,"source":source.file_name().unwrap().to_str().unwrap(),"width":160,"height":284,"fps":30,
        "clips":[{"start_ms":2500,"end_ms":3500,"text":"Blue, later take"},{"start_ms":500,"end_ms":1500,"text":"Red, earlier line"}]});
    let path = dir.path().join("edit.json");
    fs::write(&path, plan.to_string()).unwrap();
    let dest = dir.path().join("output");
    let check = Command::new(env!("CARGO_BIN_EXE_claw-edit"))
        .arg("check")
        .arg("--plan")
        .arg(&path)
        .output()
        .unwrap();
    assert!(
        check.status.success(),
        "{}",
        String::from_utf8_lossy(&check.stderr)
    );
    assert!(!dest.exists());
    let result = cli(&path, &dest);
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let video = dest.join("cut.mp4");
    let data = run(
        "ffprobe",
        &[
            "-v",
            "error",
            "-show_streams",
            "-show_format",
            "-of",
            "json",
            video.to_str().unwrap(),
        ],
    );
    let info: serde_json::Value = serde_json::from_slice(&data.stdout).unwrap();
    let duration: f64 = info["format"]["duration"]
        .as_str()
        .unwrap()
        .parse()
        .unwrap();
    assert!((duration - 2.0).abs() < 0.12);
    assert!(
        info["streams"]
            .as_array()
            .unwrap()
            .iter()
            .any(|s| s["codec_type"] == "audio")
    );
    assert_eq!(info["streams"][0]["width"], 160);
    assert_eq!(info["streams"][0]["height"], 284);
    let blue = pixel(&video, "0.25");
    let red = pixel(&video, "1.25");
    assert!(
        blue[2] > 200 && blue[0] < 30,
        "first clip must be blue: {blue:?}"
    );
    assert!(
        red[0] > 200 && red[2] < 30,
        "second clip must be red: {red:?}"
    );
    assert!((frequency(&video, "0.25") - 880.0).abs() < 15.0);
    assert!((frequency(&video, "1.25") - 440.0).abs() < 15.0);
    run(
        "ffmpeg",
        &[
            "-v",
            "error",
            "-i",
            video.to_str().unwrap(),
            "-f",
            "null",
            "-",
        ],
    );
    let srt = fs::read_to_string(dest.join("captions.srt")).unwrap();
    assert!(srt.contains("00:00:01,000 --> 00:00:02,000\nRed, earlier line"));
    let output_bytes = fs::read(&video).unwrap();
    assert!(!cli(&path, &dest).status.success());
    assert_eq!(fs::read(&video).unwrap(), output_bytes);
    let file_dest = dir.path().join("existing-file");
    fs::write(&file_dest, "keep").unwrap();
    assert!(!cli(&path, &file_dest).status.success());
    assert_eq!(fs::read_to_string(file_dest).unwrap(), "keep");
    #[cfg(unix)]
    {
        let link = dir.path().join("linked-output");
        std::os::unix::fs::symlink(&dest, &link).unwrap();
        assert!(!cli(&path, &link).status.success());
    }
    plan["clips"][0]["end_ms"] = json!(9000);
    fs::write(&path, plan.to_string()).unwrap();
    let bad_output = dir.path().join("invalid-range");
    assert!(!cli(&path, &bad_output).status.success());
    assert!(!bad_output.exists());
    let offset = dir.path().join("offset.mp4");
    run(
        "ffmpeg",
        &[
            "-v",
            "error",
            "-i",
            source.to_str().unwrap(),
            "-itsoffset",
            "0.2",
            "-i",
            source.to_str().unwrap(),
            "-map",
            "0:v:0",
            "-map",
            "1:a:0",
            "-c",
            "copy",
            offset.to_str().unwrap(),
        ],
    );
    plan["source"] = json!("offset.mp4");
    plan["clips"][0]["end_ms"] = json!(3500);
    fs::write(&path, plan.to_string()).unwrap();
    let bad_output = dir.path().join("offset-output");
    let rejected = cli(&path, &bad_output);
    assert!(!rejected.status.success());
    assert!(String::from_utf8_lossy(&rejected.stderr).contains("starts differ"));
    assert!(!bad_output.exists());
    assert_eq!(fs::read(source).unwrap(), original);
    assert!(!Path::new("SHOULD_NOT_EXIST").exists());
}
