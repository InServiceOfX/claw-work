#!/usr/bin/env python3
"""
claw-shorts: Vertical short-form video creation CLI.

Creates 9:16 (1080x1920) 30fps H.264/AAC videos from raw takes, screen
recordings, or synthetic content. Non-destructive workflow; never
overwrites source files.

Templates:
  hook_reveal     - Hook card (0-2s), content, CTA card (end)
  step_breakdown  - Numbered step overlays with paced captions
  demo_zoom       - Keyframed zoom/pan on a screen recording
  before_after    - Split screen or sequential comparison
  readme_scroll   - Scrolling README capture (requires Playwright)
  terminal_typer  - Asciinema cast rendered to frames

Commands:
  render   - Render a video from template
  launch   - Generate 5 launch Shorts from a repo URL
  doctor   - Check tool dependencies
  info     - Get video information
  templates - List available templates

Usage:
  claw-shorts render --input ./input/ --output ./output/ --template hook_reveal
  claw-shorts launch --repo https://github.com/user/project --output ./output/
  claw-shorts doctor
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
from typing import Optional, List, Tuple


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


def render_demo_zoom(
    input_video: Path,
    output_dir: Path,
    hook_text: str,
    cta_text: str,
    title: str,
    zoom_factor: float = 1.5,
    seed: Optional[int] = None,
) -> Tuple[Path, Path]:
    """
    Render a demo video with keyframed zoom/pan effect.
    
    Zooms into the center of the video, then pans, then zooms out.
    Good for screen recordings where you want to highlight a feature.
    """
    output_name = generate_output_name("demo_zoom", seed)
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

        scaled_content = work_path / "content_scaled.mp4"
        scale_and_pad_video(input_video, scaled_content)
        content_duration = get_video_duration(scaled_content)

        zoomed_content = work_path / "content_zoomed.mp4"
        zoom_filter = (
            f"zoompan=z='if(lt(on,{OUTPUT_FPS}),1+(on/{OUTPUT_FPS})*{zoom_factor-1},"
            f"if(lt(on,{int(content_duration*OUTPUT_FPS)-OUTPUT_FPS}),"
            f"{zoom_factor},{zoom_factor}-(on-{int(content_duration*OUTPUT_FPS)-OUTPUT_FPS})/{OUTPUT_FPS}*{zoom_factor-1}))':"
            f"d=1:x='iw/2-(iw/zoom/2)':y='ih/2-(ih/zoom/2)':"
            f"s={OUTPUT_WIDTH}x{OUTPUT_HEIGHT}:fps={OUTPUT_FPS}"
        )
        subprocess.run(
            [
                "ffmpeg", "-y",
                "-i", str(scaled_content),
                "-vf", zoom_filter,
                "-c:v", "libx264",
                "-pix_fmt", "yuv420p",
                "-an",
                str(zoomed_content),
            ],
            capture_output=True,
            check=True,
        )

        audio_path = work_path / "audio.aac"
        subprocess.run(
            [
                "ffmpeg", "-y",
                "-i", str(scaled_content),
                "-vn", "-c:a", "aac",
                str(audio_path),
            ],
            capture_output=True,
        )

        if audio_path.exists():
            zoomed_with_audio = work_path / "content_final.mp4"
            subprocess.run(
                [
                    "ffmpeg", "-y",
                    "-i", str(zoomed_content),
                    "-i", str(audio_path),
                    "-c:v", "copy",
                    "-c:a", "aac",
                    "-shortest",
                    str(zoomed_with_audio),
                ],
                capture_output=True,
                check=True,
            )
            zoomed_content = zoomed_with_audio

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

        concat_videos([hook_clip, zoomed_content, cta_clip], output_video)

    total_duration = 2.0 + content_duration + 3.0

    metadata = {
        "title": title,
        "template": "demo_zoom",
        "hook_text": hook_text,
        "cta_text": cta_text,
        "zoom_factor": zoom_factor,
        "duration_seconds": round(total_duration, 2),
        "resolution": f"{OUTPUT_WIDTH}x{OUTPUT_HEIGHT}",
        "fps": OUTPUT_FPS,
        "codec": "H.264/AAC",
        "suggested_caption": f"{title} {hook_text}",
        "suggested_hashtags": ["#shorts", "#demo", "#devtools"],
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


def render_before_after(
    before_video: Path,
    after_video: Path,
    output_dir: Path,
    hook_text: str,
    cta_text: str,
    title: str,
    split_mode: str = "sequential",
    seed: Optional[int] = None,
) -> Tuple[Path, Path]:
    """
    Render a before/after comparison video.
    
    split_mode: "sequential" (before then after) or "side_by_side"
    """
    output_name = generate_output_name("before_after", seed)
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

        if split_mode == "side_by_side":
            scaled_before = work_path / "before_scaled.mp4"
            scaled_after = work_path / "after_scaled.mp4"
            
            half_height = OUTPUT_HEIGHT // 2
            scale_filter_half = (
                f"scale={OUTPUT_WIDTH}:{half_height}:force_original_aspect_ratio=decrease,"
                f"pad={OUTPUT_WIDTH}:{half_height}:(ow-iw)/2:(oh-ih)/2:black"
            )
            
            subprocess.run(
                ["ffmpeg", "-y", "-i", str(before_video), "-vf", scale_filter_half,
                 "-c:v", "libx264", "-pix_fmt", "yuv420p", "-r", str(OUTPUT_FPS),
                 "-t", "5", str(scaled_before)],
                capture_output=True, check=True,
            )
            subprocess.run(
                ["ffmpeg", "-y", "-i", str(after_video), "-vf", scale_filter_half,
                 "-c:v", "libx264", "-pix_fmt", "yuv420p", "-r", str(OUTPUT_FPS),
                 "-t", "5", str(scaled_after)],
                capture_output=True, check=True,
            )
            
            stacked = work_path / "stacked.mp4"
            subprocess.run(
                ["ffmpeg", "-y",
                 "-i", str(scaled_before), "-i", str(scaled_after),
                 "-filter_complex", "[0:v][1:v]vstack=inputs=2[v]",
                 "-map", "[v]",
                 "-c:v", "libx264", "-pix_fmt", "yuv420p",
                 str(stacked)],
                capture_output=True, check=True,
            )
            content_clip = stacked
            content_duration = get_video_duration(stacked)
        else:
            scaled_before = work_path / "before_scaled.mp4"
            scaled_after = work_path / "after_scaled.mp4"
            
            scale_and_pad_video(before_video, scaled_before)
            scale_and_pad_video(after_video, scaled_after)
            
            before_trimmed = work_path / "before_trimmed.mp4"
            after_trimmed = work_path / "after_trimmed.mp4"
            subprocess.run(
                ["ffmpeg", "-y", "-i", str(scaled_before), "-t", "4",
                 "-c:v", "libx264", "-c:a", "aac", "-pix_fmt", "yuv420p",
                 str(before_trimmed)],
                capture_output=True, check=True,
            )
            subprocess.run(
                ["ffmpeg", "-y", "-i", str(scaled_after), "-t", "4",
                 "-c:v", "libx264", "-c:a", "aac", "-pix_fmt", "yuv420p",
                 str(after_trimmed)],
                capture_output=True, check=True,
            )
            
            before_label = work_path / "before_label.mp4"
            after_label = work_path / "after_label.mp4"
            label_bg = work_path / "label_bg.mp4"
            create_solid_color_clip(label_bg, 1.0, OUTPUT_WIDTH, OUTPUT_HEIGHT, "0x1a1a2e")
            create_text_overlay_clip(label_bg, before_label, "BEFORE", font_size=80)
            create_text_overlay_clip(label_bg, after_label, "AFTER", font_size=80)
            
            sequential = work_path / "sequential.mp4"
            concat_videos([before_label, before_trimmed, after_label, after_trimmed], sequential)
            content_clip = sequential
            content_duration = get_video_duration(sequential)

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

        concat_videos([hook_clip, content_clip, cta_clip], output_video)

    total_duration = 2.0 + content_duration + 3.0

    metadata = {
        "title": title,
        "template": "before_after",
        "split_mode": split_mode,
        "hook_text": hook_text,
        "cta_text": cta_text,
        "duration_seconds": round(total_duration, 2),
        "resolution": f"{OUTPUT_WIDTH}x{OUTPUT_HEIGHT}",
        "fps": OUTPUT_FPS,
        "codec": "H.264/AAC",
        "suggested_caption": f"{title} {hook_text}",
        "suggested_hashtags": ["#shorts", "#beforeafter", "#transformation"],
        "audio_rights": {
            "status": "check_required",
            "notes": "Verify audio licensing before monetized upload",
        },
        "rendered_at": datetime.now().isoformat(),
    }

    with open(output_json, "w") as f:
        json.dump(metadata, f, indent=2)

    return output_video, output_json


def render_readme_scroll(
    readme_url: str,
    output_dir: Path,
    hook_text: str,
    cta_text: str,
    title: str,
    scroll_duration: float = 10.0,
    seed: Optional[int] = None,
) -> Tuple[Path, Path]:
    """
    Render a scrolling README capture using Playwright.
    
    Captures the README page and creates a smooth scroll animation.
    Requires Playwright to be installed.
    """
    output_name = generate_output_name("readme_scroll", seed)
    output_video = output_dir / f"{output_name}.mp4"
    output_json = output_dir / f"{output_name}.json"

    if not check_playwright():
        raise RuntimeError("Playwright not installed. Run: pip install playwright && playwright install chromium")

    with tempfile.TemporaryDirectory() as work_dir:
        work_path = Path(work_dir)
        frames_dir = work_path / "frames"
        frames_dir.mkdir()

        capture_script = work_path / "capture.js"
        capture_script.write_text(f'''
const {{ chromium }} = require('playwright');

(async () => {{
    const browser = await chromium.launch();
    const page = await browser.newPage();
    await page.setViewportSize({{ width: 1080, height: 1920 }});
    await page.goto('{readme_url}');
    await page.waitForLoadState('networkidle');
    
    const scrollHeight = await page.evaluate(() => document.body.scrollHeight);
    const viewportHeight = 1920;
    const scrollDistance = Math.max(0, scrollHeight - viewportHeight);
    
    const fps = {OUTPUT_FPS};
    const duration = {scroll_duration};
    const totalFrames = Math.floor(fps * duration);
    
    for (let i = 0; i < totalFrames; i++) {{
        const progress = i / totalFrames;
        const scrollY = Math.floor(progress * scrollDistance);
        await page.evaluate((y) => window.scrollTo(0, y), scrollY);
        await page.screenshot({{
            path: `{frames_dir}/frame_${{String(i).padStart(6, '0')}}.png`
        }});
    }}
    
    await browser.close();
}})();
''')

        try:
            subprocess.run(
                ["node", str(capture_script)],
                capture_output=True,
                check=True,
                timeout=120,
            )
        except (subprocess.CalledProcessError, FileNotFoundError) as e:
            raise RuntimeError(f"Failed to capture README: {e}")

        scroll_video = work_path / "scroll.mp4"
        subprocess.run(
            [
                "ffmpeg", "-y",
                "-framerate", str(OUTPUT_FPS),
                "-i", str(frames_dir / "frame_%06d.png"),
                "-c:v", "libx264",
                "-pix_fmt", "yuv420p",
                "-r", str(OUTPUT_FPS),
                str(scroll_video),
            ],
            capture_output=True,
            check=True,
        )

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

        concat_videos([hook_clip, scroll_video, cta_clip], output_video)

    total_duration = 2.0 + scroll_duration + 3.0

    metadata = {
        "title": title,
        "template": "readme_scroll",
        "hook_text": hook_text,
        "cta_text": cta_text,
        "scroll_duration": scroll_duration,
        "source_url": readme_url,
        "duration_seconds": round(total_duration, 2),
        "resolution": f"{OUTPUT_WIDTH}x{OUTPUT_HEIGHT}",
        "fps": OUTPUT_FPS,
        "codec": "H.264/AAC",
        "suggested_caption": f"{title} {hook_text}",
        "suggested_hashtags": ["#shorts", "#readme", "#opensource"],
        "audio_rights": {
            "status": "silent",
            "notes": "No audio in this render",
        },
        "rendered_at": datetime.now().isoformat(),
    }

    with open(output_json, "w") as f:
        json.dump(metadata, f, indent=2)

    return output_video, output_json


def render_terminal_typer(
    cast_file: Path,
    output_dir: Path,
    hook_text: str,
    cta_text: str,
    title: str,
    seed: Optional[int] = None,
) -> Tuple[Path, Path]:
    """
    Render an asciinema cast to a vertical video.
    
    Uses agg (asciinema gif generator) or falls back to frame-by-frame rendering.
    """
    output_name = generate_output_name("terminal_typer", seed)
    output_video = output_dir / f"{output_name}.mp4"
    output_json = output_dir / f"{output_name}.json"

    if not check_agg():
        raise RuntimeError("agg not installed. Install from: https://github.com/asciinema/agg")

    with tempfile.TemporaryDirectory() as work_dir:
        work_path = Path(work_dir)

        terminal_gif = work_path / "terminal.gif"
        subprocess.run(
            [
                "agg",
                "--cols", "60",
                "--rows", "40",
                "--font-size", "24",
                str(cast_file),
                str(terminal_gif),
            ],
            capture_output=True,
            check=True,
        )

        terminal_video = work_path / "terminal.mp4"
        subprocess.run(
            [
                "ffmpeg", "-y",
                "-i", str(terminal_gif),
                "-movflags", "faststart",
                "-pix_fmt", "yuv420p",
                "-vf", f"scale={OUTPUT_WIDTH}:{OUTPUT_HEIGHT}:force_original_aspect_ratio=decrease,"
                       f"pad={OUTPUT_WIDTH}:{OUTPUT_HEIGHT}:(ow-iw)/2:(oh-ih)/2:black",
                "-r", str(OUTPUT_FPS),
                str(terminal_video),
            ],
            capture_output=True,
            check=True,
        )

        content_duration = get_video_duration(terminal_video)

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

        concat_videos([hook_clip, terminal_video, cta_clip], output_video)

    total_duration = 2.0 + content_duration + 3.0

    metadata = {
        "title": title,
        "template": "terminal_typer",
        "hook_text": hook_text,
        "cta_text": cta_text,
        "duration_seconds": round(total_duration, 2),
        "resolution": f"{OUTPUT_WIDTH}x{OUTPUT_HEIGHT}",
        "fps": OUTPUT_FPS,
        "codec": "H.264/AAC",
        "suggested_caption": f"{title} {hook_text}",
        "suggested_hashtags": ["#shorts", "#terminal", "#cli", "#coding"],
        "audio_rights": {
            "status": "silent",
            "notes": "No audio in this render",
        },
        "rendered_at": datetime.now().isoformat(),
        "source_file": str(cast_file.name),
    }

    with open(output_json, "w") as f:
        json.dump(metadata, f, indent=2)

    return output_video, output_json


def check_playwright() -> bool:
    """Check if Playwright is available."""
    try:
        subprocess.run(
            ["node", "-e", "require('playwright')"],
            capture_output=True,
            check=True,
        )
        return True
    except (subprocess.CalledProcessError, FileNotFoundError):
        return False


def check_agg() -> bool:
    """Check if agg (asciinema gif generator) is available."""
    try:
        subprocess.run(
            ["agg", "--version"],
            capture_output=True,
            check=True,
        )
        return True
    except (subprocess.CalledProcessError, FileNotFoundError):
        return False


def check_whisper() -> bool:
    """Check if whisper is available for captions."""
    try:
        subprocess.run(
            ["whisper", "--help"],
            capture_output=True,
            check=True,
        )
        return True
    except (subprocess.CalledProcessError, FileNotFoundError):
        return False


def check_piper() -> bool:
    """Check if piper TTS is available."""
    try:
        subprocess.run(
            ["piper", "--help"],
            capture_output=True,
            check=True,
        )
        return True
    except (subprocess.CalledProcessError, FileNotFoundError):
        return False


def cmd_render(args: argparse.Namespace) -> int:
    if not check_ffmpeg():
        print("Error: FFmpeg not found. Install with: brew install ffmpeg", file=sys.stderr)
        return 1

    if not check_ffprobe():
        print("Error: ffprobe not found. Install FFmpeg.", file=sys.stderr)
        return 1

    template = args.template
    title = args.title or "Untitled"
    hook = args.hook or "Watch this"
    cta = args.cta or "Follow for more"
    seed = args.seed
    output_path = Path(args.output).resolve()
    output_path.mkdir(parents=True, exist_ok=True)

    if template == "readme_scroll":
        if not args.url:
            print("Error: --url required for readme_scroll template", file=sys.stderr)
            return 1
        print(f"URL: {args.url}")
        print(f"Template: {template}")
        print(f"Output dir: {output_path}")
        try:
            video, metadata = render_readme_scroll(
                args.url,
                output_path,
                hook_text=hook,
                cta_text=cta,
                title=title,
                scroll_duration=args.scroll_duration,
                seed=seed,
            )
            print(f"Output video: {video}")
            print(f"Output metadata: {metadata}")
            return 0
        except Exception as e:
            print(f"Error: {e}", file=sys.stderr)
            return 1

    if template == "terminal_typer":
        if not args.cast:
            print("Error: --cast required for terminal_typer template", file=sys.stderr)
            return 1
        cast_path = Path(args.cast).resolve()
        if not cast_path.exists():
            print(f"Error: Cast file not found: {cast_path}", file=sys.stderr)
            return 1
        print(f"Cast: {cast_path}")
        print(f"Template: {template}")
        print(f"Output dir: {output_path}")
        try:
            video, metadata = render_terminal_typer(
                cast_path,
                output_path,
                hook_text=hook,
                cta_text=cta,
                title=title,
                seed=seed,
            )
            print(f"Output video: {video}")
            print(f"Output metadata: {metadata}")
            return 0
        except Exception as e:
            print(f"Error: {e}", file=sys.stderr)
            return 1

    if template == "before_after":
        before_path = Path(args.before).resolve() if args.before else None
        after_path = Path(args.after).resolve() if args.after else None
        if not before_path or not after_path:
            input_path = Path(args.input).resolve()
            input_video = find_input_video(input_path)
            if input_video is None:
                print("Error: --before and --after required, or --input for same video", file=sys.stderr)
                return 1
            before_path = before_path or input_video
            after_path = after_path or input_video
        print(f"Before: {before_path}")
        print(f"After: {after_path}")
        print(f"Template: {template}")
        print(f"Output dir: {output_path}")
        try:
            video, metadata = render_before_after(
                before_path,
                after_path,
                output_path,
                hook_text=hook,
                cta_text=cta,
                title=title,
                split_mode=args.split,
                seed=seed,
            )
            print(f"Output video: {video}")
            print(f"Output metadata: {metadata}")
            return 0
        except Exception as e:
            print(f"Error: {e}", file=sys.stderr)
            return 1

    input_path = Path(args.input).resolve()
    input_video = find_input_video(input_path)
    if input_video is None:
        print(f"Error: No video file found in {input_path}", file=sys.stderr)
        return 1

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
        elif template == "demo_zoom":
            video, metadata = render_demo_zoom(
                input_video,
                output_path,
                hook_text=hook,
                cta_text=cta,
                title=title,
                zoom_factor=args.zoom,
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
        "demo_zoom": {
            "description": "Keyframed zoom/pan effect on screen recording",
            "options": ["--hook", "--cta", "--title", "--zoom"],
            "use_case": "Demo highlight with zoom emphasis",
        },
        "before_after": {
            "description": "Sequential or side-by-side comparison",
            "options": ["--before", "--after", "--hook", "--cta", "--title", "--split"],
            "use_case": "Before/after transformation showcase",
        },
        "readme_scroll": {
            "description": "Scrolling README page capture",
            "options": ["--url", "--hook", "--cta", "--title", "--scroll-duration"],
            "use_case": "README walkthrough (requires Playwright)",
        },
        "terminal_typer": {
            "description": "Asciinema cast rendered to video",
            "options": ["--cast", "--hook", "--cta", "--title"],
            "use_case": "Terminal demo (requires agg)",
        },
    }
    print(json.dumps(templates, indent=2))
    return 0


def cmd_doctor(args: argparse.Namespace) -> int:
    """Check tool dependencies and report status."""
    print("claw-shorts dependency check")
    print("=" * 40)
    
    checks = [
        ("FFmpeg", check_ffmpeg, True, "brew install ffmpeg / apt install ffmpeg"),
        ("ffprobe", check_ffprobe, True, "Included with FFmpeg"),
        ("Playwright", check_playwright, False, "pip install playwright && playwright install chromium"),
        ("agg", check_agg, False, "https://github.com/asciinema/agg"),
        ("Whisper", check_whisper, False, "pip install openai-whisper"),
        ("Piper TTS", check_piper, False, "https://github.com/rhasspy/piper"),
    ]
    
    all_required_ok = True
    
    for name, check_fn, required, install_hint in checks:
        status = check_fn()
        status_str = "OK" if status else "MISSING"
        req_str = "(required)" if required else "(optional)"
        
        if required and not status:
            all_required_ok = False
        
        icon = "✓" if status else ("✗" if required else "○")
        print(f"  {icon} {name}: {status_str} {req_str}")
        if not status:
            print(f"      Install: {install_hint}")
    
    print()
    if all_required_ok:
        print("All required dependencies are installed.")
        return 0
    else:
        print("Some required dependencies are missing.")
        return 1


def cmd_launch(args: argparse.Namespace) -> int:
    """Generate 5 launch Shorts from a repository."""
    try:
        from launch_kit import (
            fetch_github_repo_metadata,
            generate_launch_plan,
            save_launch_plan,
            generate_posting_kit,
        )
    except ImportError:
        scripts_dir = Path(__file__).parent
        sys.path.insert(0, str(scripts_dir))
        from launch_kit import (
            fetch_github_repo_metadata,
            generate_launch_plan,
            save_launch_plan,
            generate_posting_kit,
        )
    
    if not check_ffmpeg():
        print("Error: FFmpeg not found. Run: claw-shorts doctor", file=sys.stderr)
        return 1
    
    repo_url = args.repo
    output_dir = Path(args.output).resolve()
    recording_path = Path(args.recording).resolve() if args.recording else None
    n_clips = args.n if hasattr(args, 'n') else 5
    sponsored = args.sponsored if hasattr(args, 'sponsored') else False
    affiliate = args.affiliate if hasattr(args, 'affiliate') else False
    
    output_dir.mkdir(parents=True, exist_ok=True)
    
    print(f"Fetching metadata from: {repo_url}")
    try:
        metadata = fetch_github_repo_metadata(repo_url)
    except Exception as e:
        print(f"Error fetching repo metadata: {e}", file=sys.stderr)
        return 1
    
    print(f"Project: {metadata.name}")
    print(f"Description: {metadata.description[:100]}..." if len(metadata.description) > 100 else f"Description: {metadata.description}")
    print(f"Language: {metadata.language}")
    print(f"Stars: {metadata.stars}")
    
    print("\nGenerating launch plan...")
    plan = generate_launch_plan(
        metadata,
        recording_path=recording_path,
        sponsored=sponsored,
        affiliate=affiliate,
    )
    
    plan_path = output_dir / "launch_plan.json"
    save_launch_plan(plan, plan_path)
    print(f"Launch plan saved: {plan_path}")
    
    kit_path = generate_posting_kit(plan, output_dir)
    print(f"Posting kit saved: {kit_path}")
    
    if args.render:
        print("\nRendering clips...")
        clips_dir = output_dir / "clips"
        clips_dir.mkdir(exist_ok=True)
        
        rendered_clips = []
        
        for clip in plan.clips[:n_clips]:
            print(f"  Rendering clip {clip.clip_number}: {clip.template}")
            
            try:
                if recording_path and recording_path.exists():
                    input_video = recording_path
                else:
                    placeholder = clips_dir / f"placeholder_{clip.clip_number}.mp4"
                    create_solid_color_clip(
                        placeholder, 5.0, OUTPUT_WIDTH, OUTPUT_HEIGHT, "0x2d2d44"
                    )
                    input_video = placeholder
                
                if clip.template == "hook_reveal":
                    video, meta = render_hook_reveal(
                        input_video, clips_dir,
                        hook_text=clip.hook_text,
                        cta_text=clip.cta_text,
                        title=plan.project_name,
                        seed=clip.clip_number,
                    )
                elif clip.template == "step_breakdown":
                    video, meta = render_step_breakdown(
                        input_video, clips_dir,
                        steps=clip.body_beats[:3],
                        title=plan.project_name,
                        cta_text=clip.cta_text,
                        seed=clip.clip_number,
                    )
                elif clip.template == "demo_zoom":
                    video, meta = render_demo_zoom(
                        input_video, clips_dir,
                        hook_text=clip.hook_text,
                        cta_text=clip.cta_text,
                        title=plan.project_name,
                        seed=clip.clip_number,
                    )
                elif clip.template == "before_after":
                    video, meta = render_before_after(
                        input_video, input_video, clips_dir,
                        hook_text=clip.hook_text,
                        cta_text=clip.cta_text,
                        title=plan.project_name,
                        seed=clip.clip_number,
                    )
                else:
                    video, meta = render_hook_reveal(
                        input_video, clips_dir,
                        hook_text=clip.hook_text,
                        cta_text=clip.cta_text,
                        title=plan.project_name,
                        seed=clip.clip_number,
                    )
                
                rendered_clips.append((video, meta))
                print(f"    Created: {video.name}")
                
            except Exception as e:
                print(f"    Warning: Failed to render clip {clip.clip_number}: {e}")
        
        print(f"\nRendered {len(rendered_clips)} clips to {clips_dir}")
    
    print("\n" + "=" * 40)
    print("Launch kit generated successfully!")
    print(f"Output directory: {output_dir}")
    print("\nNext steps:")
    print("1. Review launch_plan.json and edit hooks/CTAs as needed")
    print("2. Add your screen recording with --recording")
    print("3. Re-run with --render to generate final clips")
    print("4. Use posting_kit.json for titles, captions, and hashtags")
    
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
        choices=["hook_reveal", "step_breakdown", "demo_zoom", "before_after",
                 "readme_scroll", "terminal_typer"],
        default="hook_reveal",
        help="Template to use",
    )
    render_parser.add_argument("--title", help="Video title")
    render_parser.add_argument("--hook", help="Hook text (for hook_reveal)")
    render_parser.add_argument("--cta", help="Call-to-action text")
    render_parser.add_argument("--steps", help="Comma-separated step names (for step_breakdown)")
    render_parser.add_argument("--seed", type=int, help="Random seed for deterministic output names")
    render_parser.add_argument("--zoom", type=float, default=1.5, help="Zoom factor (for demo_zoom)")
    render_parser.add_argument("--before", help="Before video path (for before_after)")
    render_parser.add_argument("--after", help="After video path (for before_after)")
    render_parser.add_argument("--split", choices=["sequential", "side_by_side"],
                               default="sequential", help="Split mode (for before_after)")
    render_parser.add_argument("--url", help="README URL (for readme_scroll)")
    render_parser.add_argument("--scroll-duration", type=float, default=10.0,
                               help="Scroll duration in seconds (for readme_scroll)")
    render_parser.add_argument("--cast", help="Asciinema cast file (for terminal_typer)")

    info_parser = subparsers.add_parser("info", help="Get video information")
    info_parser.add_argument("--input", "-i", required=True, help="Input video file")

    subparsers.add_parser("templates", help="List available templates")
    
    subparsers.add_parser("doctor", help="Check tool dependencies")
    
    launch_parser = subparsers.add_parser("launch", help="Generate 5 launch Shorts from a repo")
    launch_parser.add_argument("--repo", "-r", required=True,
                               help="GitHub repo URL or local path")
    launch_parser.add_argument("--output", "-o", required=True,
                               help="Output directory")
    launch_parser.add_argument("--recording", help="Screen or terminal recording to use")
    launch_parser.add_argument("--url", help="Product website URL (for additional context)")
    launch_parser.add_argument("--n", type=int, default=5,
                               help="Number of clips to generate (default: 5)")
    launch_parser.add_argument("--render", action="store_true",
                               help="Render clips immediately (otherwise just generate plan)")
    launch_parser.add_argument("--sponsored", action="store_true",
                               help="Add sponsored disclosure")
    launch_parser.add_argument("--affiliate", action="store_true",
                               help="Add affiliate disclosure")

    args = parser.parse_args()

    if args.command == "render":
        return cmd_render(args)
    elif args.command == "info":
        return cmd_info(args)
    elif args.command == "templates":
        return cmd_templates(args)
    elif args.command == "doctor":
        return cmd_doctor(args)
    elif args.command == "launch":
        return cmd_launch(args)
    else:
        parser.print_help()
        return 1


if __name__ == "__main__":
    sys.exit(main())
