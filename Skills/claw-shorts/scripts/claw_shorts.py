#!/usr/bin/env python3
"""
claw-shorts: Vertical short-form video creation CLI.

Creates 9:16 (1080x1920) 30fps H.264/AAC videos from raw takes, screen
recordings, or synthetic content. Non-destructive workflow; never
overwrites source files.

Templates:
  hook_reveal     - Hook card (0-2s), content, CTA card (end)
  step_breakdown  - Numbered step overlays with paced captions

Usage:
  claw-shorts render --input ./input/ --output ./output/ --template hook_reveal
  claw-shorts info --input video.mp4
"""

import argparse
import hashlib
import json
import os
import shutil
import subprocess
import sys
import tempfile
import time
from datetime import datetime
from pathlib import Path
from typing import Optional


OUTPUT_WIDTH = 1080
OUTPUT_HEIGHT = 1920
OUTPUT_FPS = 30

SAFE_TOP = 150
SAFE_BOTTOM = 300
SAFE_SIDE = 50

FONT_SANS = "sans-serif"


def check_ffmpeg() -> bool:
    try:
        subprocess.run(
            ["ffmpeg", "-version"],
            capture_output=True,
            check=True,
        )
        return True
    except (subprocess.CalledProcessError, FileNotFoundError):
        return False


def check_ffprobe() -> bool:
    try:
        subprocess.run(
            ["ffprobe", "-version"],
            capture_output=True,
            check=True,
        )
        return True
    except (subprocess.CalledProcessError, FileNotFoundError):
        return False


def get_video_duration(path: Path) -> float:
    result = subprocess.run(
        [
            "ffprobe",
            "-v", "error",
            "-show_entries", "format=duration",
            "-of", "default=noprint_wrappers=1:nokey=1",
            str(path),
        ],
        capture_output=True,
        text=True,
    )
    return float(result.stdout.strip())


def get_video_info(path: Path) -> dict:
    result = subprocess.run(
        [
            "ffprobe",
            "-v", "error",
            "-show_entries", "stream=width,height,codec_name,duration,r_frame_rate",
            "-show_entries", "format=duration,size,bit_rate",
            "-of", "json",
            str(path),
        ],
        capture_output=True,
        text=True,
    )
    return json.loads(result.stdout)


def generate_output_name(template: str, seed: Optional[int] = None) -> str:
    timestamp = datetime.now().strftime("%Y%m%d_%H%M%S")
    if seed is None:
        seed = int(time.time() * 1000) % 10000
    hash_input = f"{template}_{timestamp}_{seed}"
    hash_suffix = hashlib.sha256(hash_input.encode()).hexdigest()[:8]
    return f"{timestamp}_{hash_suffix}"


def create_solid_color_clip(
    output_path: Path,
    duration: float,
    width: int,
    height: int,
    color: str = "0x1a1a2e",
) -> None:
    subprocess.run(
        [
            "ffmpeg", "-y",
            "-f", "lavfi",
            "-i", f"color=c={color}:s={width}x{height}:d={duration}:r={OUTPUT_FPS}",
            "-c:v", "libx264",
            "-pix_fmt", "yuv420p",
            str(output_path),
        ],
        capture_output=True,
        check=True,
    )


def create_text_overlay_clip(
    background_path: Path,
    output_path: Path,
    text: str,
    font_size: int = 72,
    font_color: str = "white",
    y_position: Optional[int] = None,
) -> None:
    if y_position is None:
        y_expr = "(h-text_h)/2"
    else:
        y_expr = str(y_position)

    safe_text = text.replace("'", "'\\''").replace(":", "\\:")

    drawtext_filter = (
        f"drawtext=text='{safe_text}':"
        f"fontsize={font_size}:"
        f"fontcolor={font_color}:"
        f"x=(w-text_w)/2:"
        f"y={y_expr}"
    )

    subprocess.run(
        [
            "ffmpeg", "-y",
            "-i", str(background_path),
            "-vf", drawtext_filter,
            "-c:v", "libx264",
            "-pix_fmt", "yuv420p",
            "-an",
            str(output_path),
        ],
        capture_output=True,
        check=True,
    )


def scale_and_pad_video(
    input_path: Path,
    output_path: Path,
    target_width: int = OUTPUT_WIDTH,
    target_height: int = OUTPUT_HEIGHT,
) -> None:
    scale_filter = (
        f"scale={target_width}:{target_height}:force_original_aspect_ratio=decrease,"
        f"pad={target_width}:{target_height}:(ow-iw)/2:(oh-ih)/2:black"
    )

    subprocess.run(
        [
            "ffmpeg", "-y",
            "-i", str(input_path),
            "-vf", scale_filter,
            "-c:v", "libx264",
            "-c:a", "aac",
            "-pix_fmt", "yuv420p",
            "-r", str(OUTPUT_FPS),
            str(output_path),
        ],
        capture_output=True,
        check=True,
    )


def concat_videos(input_paths: list, output_path: Path) -> None:
    with tempfile.NamedTemporaryFile(mode="w", suffix=".txt", delete=False) as f:
        for p in input_paths:
            f.write(f"file '{p}'\n")
        concat_file = f.name

    try:
        subprocess.run(
            [
                "ffmpeg", "-y",
                "-f", "concat",
                "-safe", "0",
                "-i", concat_file,
                "-c:v", "libx264",
                "-c:a", "aac",
                "-pix_fmt", "yuv420p",
                str(output_path),
            ],
            capture_output=True,
            check=True,
        )
    finally:
        os.unlink(concat_file)


def render_hook_reveal(
    input_video: Path,
    output_dir: Path,
    hook_text: str,
    cta_text: str,
    title: str,
    seed: Optional[int] = None,
) -> tuple:
    output_name = generate_output_name("hook_reveal", seed)
    output_video = output_dir / f"{output_name}.mp4"
    output_json = output_dir / f"{output_name}.json"

    with tempfile.TemporaryDirectory() as work_dir:
        work_path = Path(work_dir)

        hook_bg = work_path / "hook_bg.mp4"
        create_solid_color_clip(hook_bg, 2.0, OUTPUT_WIDTH, OUTPUT_HEIGHT, "0x1a1a2e")

        hook_clip = work_path / "hook.mp4"
        create_text_overlay_clip(
            hook_bg,
            hook_clip,
            hook_text,
            font_size=64,
            font_color="white",
        )

        scaled_content = work_path / "content.mp4"
        scale_and_pad_video(input_video, scaled_content)

        content_duration = get_video_duration(scaled_content)

        cta_bg = work_path / "cta_bg.mp4"
        create_solid_color_clip(cta_bg, 3.0, OUTPUT_WIDTH, OUTPUT_HEIGHT, "0x1a1a2e")

        cta_clip = work_path / "cta.mp4"
        create_text_overlay_clip(
            cta_bg,
            cta_clip,
            cta_text,
            font_size=56,
            font_color="white",
        )

        concat_videos([hook_clip, scaled_content, cta_clip], output_video)

    total_duration = 2.0 + content_duration + 3.0

    metadata = {
        "title": title,
        "template": "hook_reveal",
        "hook_text": hook_text,
        "cta_text": cta_text,
        "duration_seconds": round(total_duration, 2),
        "resolution": f"{OUTPUT_WIDTH}x{OUTPUT_HEIGHT}",
        "fps": OUTPUT_FPS,
        "codec": "H.264/AAC",
        "suggested_caption": f"{title} {hook_text}",
        "suggested_hashtags": ["#shorts", "#tutorial"],
        "audio_rights": {
            "status": "check_required",
            "notes": "Verify audio licensing before monetized upload",
        },
        "rendered_at": datetime.now().isoformat(),
        "source_file": str(input_video.name),
    }

    with open(output_json, "w") as f:
        json.dump(metadata, f, indent=2)

    return output_video, output_json


def render_step_breakdown(
    input_video: Path,
    output_dir: Path,
    steps: list,
    title: str,
    cta_text: str = "Follow for more",
    seed: Optional[int] = None,
) -> tuple:
    output_name = generate_output_name("step_breakdown", seed)
    output_video = output_dir / f"{output_name}.mp4"
    output_json = output_dir / f"{output_name}.json"

    with tempfile.TemporaryDirectory() as work_dir:
        work_path = Path(work_dir)

        title_bg = work_path / "title_bg.mp4"
        create_solid_color_clip(title_bg, 2.0, OUTPUT_WIDTH, OUTPUT_HEIGHT, "0x1a1a2e")

        num_steps = len(steps)
        title_full = f"{num_steps} steps: {title}"
        title_clip = work_path / "title.mp4"
        create_text_overlay_clip(
            title_bg,
            title_clip,
            title_full,
            font_size=56,
            font_color="white",
        )

        scaled_content = work_path / "content.mp4"
        scale_and_pad_video(input_video, scaled_content)

        content_duration = get_video_duration(scaled_content)

        cta_bg = work_path / "cta_bg.mp4"
        create_solid_color_clip(cta_bg, 3.0, OUTPUT_WIDTH, OUTPUT_HEIGHT, "0x1a1a2e")

        cta_clip = work_path / "cta.mp4"
        create_text_overlay_clip(
            cta_bg,
            cta_clip,
            cta_text,
            font_size=56,
            font_color="white",
        )

        concat_videos([title_clip, scaled_content, cta_clip], output_video)

    total_duration = 2.0 + content_duration + 3.0

    metadata = {
        "title": title,
        "template": "step_breakdown",
        "steps": steps,
        "cta_text": cta_text,
        "duration_seconds": round(total_duration, 2),
        "resolution": f"{OUTPUT_WIDTH}x{OUTPUT_HEIGHT}",
        "fps": OUTPUT_FPS,
        "codec": "H.264/AAC",
        "suggested_caption": f"{num_steps} steps to {title}",
        "suggested_hashtags": ["#tutorial", "#howto", "#shorts"],
        "audio_rights": {
            "status": "check_required",
            "notes": "Verify audio licensing before monetized upload",
        },
        "rendered_at": datetime.now().isoformat(),
        "source_file": str(input_video.name),
    }

    with open(output_json, "w") as f:
        json.dump(metadata, f, indent=2)

    return output_video, output_json


def find_input_video(input_path: Path) -> Optional[Path]:
    if input_path.is_file():
        return input_path

    video_extensions = [".mp4", ".mov", ".avi", ".mkv", ".webm"]
    for ext in video_extensions:
        for video_file in input_path.glob(f"*{ext}"):
            return video_file
        for video_file in input_path.glob(f"*{ext.upper()}"):
            return video_file

    return None


def cmd_render(args: argparse.Namespace) -> int:
    if not check_ffmpeg():
        print("Error: FFmpeg not found. Install with: brew install ffmpeg", file=sys.stderr)
        return 1

    if not check_ffprobe():
        print("Error: ffprobe not found. Install FFmpeg.", file=sys.stderr)
        return 1

    input_path = Path(args.input).resolve()
    output_path = Path(args.output).resolve()

    input_video = find_input_video(input_path)
    if input_video is None:
        print(f"Error: No video file found in {input_path}", file=sys.stderr)
        return 1

    output_path.mkdir(parents=True, exist_ok=True)

    template = args.template
    title = args.title or "Untitled"
    hook = args.hook or "Watch this"
    cta = args.cta or "Follow for more"
    seed = args.seed

    print(f"Input: {input_video}")
    print(f"Template: {template}")
    print(f"Output dir: {output_path}")

    try:
        if template == "hook_reveal":
            video, metadata = render_hook_reveal(
                input_video,
                output_path,
                hook_text=hook,
                cta_text=cta,
                title=title,
                seed=seed,
            )
        elif template == "step_breakdown":
            steps = args.steps.split(",") if args.steps else ["Step 1", "Step 2", "Step 3"]
            video, metadata = render_step_breakdown(
                input_video,
                output_path,
                steps=steps,
                title=title,
                cta_text=cta,
                seed=seed,
            )
        else:
            print(f"Error: Unknown template '{template}'", file=sys.stderr)
            return 1

        print(f"Output video: {video}")
        print(f"Output metadata: {metadata}")
        return 0

    except subprocess.CalledProcessError as e:
        print(f"Error during rendering: {e}", file=sys.stderr)
        if e.stderr:
            print(e.stderr.decode() if isinstance(e.stderr, bytes) else e.stderr, file=sys.stderr)
        return 1


def cmd_info(args: argparse.Namespace) -> int:
    if not check_ffprobe():
        print("Error: ffprobe not found. Install FFmpeg.", file=sys.stderr)
        return 1

    input_path = Path(args.input).resolve()

    if not input_path.exists():
        print(f"Error: File not found: {input_path}", file=sys.stderr)
        return 1

    info = get_video_info(input_path)
    print(json.dumps(info, indent=2))
    return 0


def cmd_templates(args: argparse.Namespace) -> int:
    templates = {
        "hook_reveal": {
            "description": "Hook card (0-2s), main content, CTA card (last 3s)",
            "options": ["--hook", "--cta", "--title"],
            "use_case": "Attention-grabbing reveal format",
        },
        "step_breakdown": {
            "description": "Numbered step overlays with title and CTA",
            "options": ["--steps", "--cta", "--title"],
            "use_case": "Tutorial and how-to content",
        },
    }
    print(json.dumps(templates, indent=2))
    return 0


def main() -> int:
    parser = argparse.ArgumentParser(
        prog="claw-shorts",
        description="Vertical short-form video creation CLI",
    )
    subparsers = parser.add_subparsers(dest="command", required=True)

    render_parser = subparsers.add_parser("render", help="Render a video from template")
    render_parser.add_argument("--input", "-i", required=True, help="Input video or directory")
    render_parser.add_argument("--output", "-o", required=True, help="Output directory")
    render_parser.add_argument(
        "--template", "-t",
        choices=["hook_reveal", "step_breakdown"],
        default="hook_reveal",
        help="Template to use",
    )
    render_parser.add_argument("--title", help="Video title")
    render_parser.add_argument("--hook", help="Hook text (for hook_reveal)")
    render_parser.add_argument("--cta", help="Call-to-action text")
    render_parser.add_argument("--steps", help="Comma-separated step names (for step_breakdown)")
    render_parser.add_argument("--seed", type=int, help="Random seed for deterministic output names")

    info_parser = subparsers.add_parser("info", help="Get video information")
    info_parser.add_argument("--input", "-i", required=True, help="Input video file")

    subparsers.add_parser("templates", help="List available templates")

    args = parser.parse_args()

    if args.command == "render":
        return cmd_render(args)
    elif args.command == "info":
        return cmd_info(args)
    elif args.command == "templates":
        return cmd_templates(args)
    else:
        parser.print_help()
        return 1


if __name__ == "__main__":
    sys.exit(main())
