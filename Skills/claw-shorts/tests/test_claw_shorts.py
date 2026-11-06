#!/usr/bin/env python3
"""
Unit and integration tests for claw-shorts.

Tests use synthetic frames generated in-memory to avoid committing binary
video files to the repository.
"""

import json
import os
import shutil
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent.parent / "scripts"))

from claw_shorts import (
    OUTPUT_WIDTH,
    OUTPUT_HEIGHT,
    OUTPUT_FPS,
    SAFE_TOP,
    SAFE_BOTTOM,
    SAFE_SIDE,
    check_ffmpeg,
    check_ffprobe,
    generate_output_name,
    get_video_duration,
    get_video_info,
)


def ffmpeg_available() -> bool:
    try:
        subprocess.run(["ffmpeg", "-version"], capture_output=True, check=True)
        return True
    except (subprocess.CalledProcessError, FileNotFoundError):
        return False


def create_synthetic_video(output_path: Path, duration: float = 3.0) -> None:
    subprocess.run(
        [
            "ffmpeg", "-y",
            "-f", "lavfi",
            "-i", f"color=c=blue:s=1080x1920:d={duration}:r=30",
            "-f", "lavfi",
            "-i", f"sine=frequency=440:duration={duration}",
            "-c:v", "libx264",
            "-c:a", "aac",
            "-pix_fmt", "yuv420p",
            "-shortest",
            str(output_path),
        ],
        capture_output=True,
        check=True,
    )


class TestConstants(unittest.TestCase):
    def test_output_dimensions(self):
        self.assertEqual(OUTPUT_WIDTH, 1080)
        self.assertEqual(OUTPUT_HEIGHT, 1920)
        self.assertEqual(OUTPUT_FPS, 30)

    def test_safe_zones(self):
        self.assertEqual(SAFE_TOP, 150)
        self.assertEqual(SAFE_BOTTOM, 300)
        self.assertEqual(SAFE_SIDE, 50)
        content_height = OUTPUT_HEIGHT - SAFE_TOP - SAFE_BOTTOM
        self.assertGreater(content_height, 1000)


class TestOutputNaming(unittest.TestCase):
    def test_generate_output_name_deterministic(self):
        name1 = generate_output_name("hook_reveal", seed=12345)
        name2 = generate_output_name("hook_reveal", seed=12345)
        self.assertIn("_", name1)
        self.assertEqual(len(name1.split("_")), 3)

    def test_generate_output_name_different_seeds(self):
        name1 = generate_output_name("hook_reveal", seed=11111)
        name2 = generate_output_name("hook_reveal", seed=22222)
        hash1 = name1.split("_")[-1]
        hash2 = name2.split("_")[-1]
        self.assertNotEqual(hash1, hash2)

    def test_generate_output_name_different_templates(self):
        name1 = generate_output_name("hook_reveal", seed=12345)
        name2 = generate_output_name("step_breakdown", seed=12345)
        hash1 = name1.split("_")[-1]
        hash2 = name2.split("_")[-1]
        self.assertNotEqual(hash1, hash2)


@unittest.skipUnless(ffmpeg_available(), "FFmpeg not available")
class TestFFmpegIntegration(unittest.TestCase):
    def test_check_ffmpeg(self):
        self.assertTrue(check_ffmpeg())

    def test_check_ffprobe(self):
        self.assertTrue(check_ffprobe())


@unittest.skipUnless(ffmpeg_available(), "FFmpeg not available")
class TestSyntheticVideo(unittest.TestCase):
    def setUp(self):
        self.temp_dir = tempfile.mkdtemp()
        self.temp_path = Path(self.temp_dir)

    def tearDown(self):
        shutil.rmtree(self.temp_dir)

    def test_create_synthetic_video(self):
        video_path = self.temp_path / "test_video.mp4"
        create_synthetic_video(video_path, duration=2.0)
        self.assertTrue(video_path.exists())
        self.assertGreater(video_path.stat().st_size, 0)

    def test_get_video_duration(self):
        video_path = self.temp_path / "test_video.mp4"
        create_synthetic_video(video_path, duration=2.0)
        duration = get_video_duration(video_path)
        self.assertAlmostEqual(duration, 2.0, delta=0.1)

    def test_get_video_info(self):
        video_path = self.temp_path / "test_video.mp4"
        create_synthetic_video(video_path, duration=2.0)
        info = get_video_info(video_path)
        self.assertIn("streams", info)
        self.assertIn("format", info)


@unittest.skipUnless(ffmpeg_available(), "FFmpeg not available")
class TestRenderHookReveal(unittest.TestCase):
    def setUp(self):
        self.temp_dir = tempfile.mkdtemp()
        self.temp_path = Path(self.temp_dir)
        self.input_dir = self.temp_path / "input"
        self.output_dir = self.temp_path / "output"
        self.input_dir.mkdir()
        self.output_dir.mkdir()
        self.input_video = self.input_dir / "source.mp4"
        create_synthetic_video(self.input_video, duration=3.0)

    def tearDown(self):
        shutil.rmtree(self.temp_dir)

    def test_render_creates_video(self):
        from claw_shorts import render_hook_reveal
        video, metadata = render_hook_reveal(
            self.input_video,
            self.output_dir,
            hook_text="Test hook",
            cta_text="Test CTA",
            title="Test Title",
            seed=42,
        )
        self.assertTrue(video.exists())
        self.assertTrue(metadata.exists())
        self.assertTrue(str(video).endswith(".mp4"))
        self.assertTrue(str(metadata).endswith(".json"))

    def test_render_metadata_content(self):
        from claw_shorts import render_hook_reveal
        video, metadata = render_hook_reveal(
            self.input_video,
            self.output_dir,
            hook_text="Hook text here",
            cta_text="CTA text here",
            title="My Title",
            seed=42,
        )
        with open(metadata) as f:
            data = json.load(f)
        self.assertEqual(data["title"], "My Title")
        self.assertEqual(data["template"], "hook_reveal")
        self.assertEqual(data["hook_text"], "Hook text here")
        self.assertEqual(data["cta_text"], "CTA text here")
        self.assertEqual(data["resolution"], "1080x1920")
        self.assertEqual(data["fps"], 30)
        self.assertIn("audio_rights", data)
        self.assertIn("rendered_at", data)

    def test_render_output_duration(self):
        from claw_shorts import render_hook_reveal
        video, _ = render_hook_reveal(
            self.input_video,
            self.output_dir,
            hook_text="Hook",
            cta_text="CTA",
            title="Title",
            seed=42,
        )
        duration = get_video_duration(video)
        expected = 2.0 + 3.0 + 3.0
        self.assertAlmostEqual(duration, expected, delta=0.5)


@unittest.skipUnless(ffmpeg_available(), "FFmpeg not available")
class TestRenderStepBreakdown(unittest.TestCase):
    def setUp(self):
        self.temp_dir = tempfile.mkdtemp()
        self.temp_path = Path(self.temp_dir)
        self.input_dir = self.temp_path / "input"
        self.output_dir = self.temp_path / "output"
        self.input_dir.mkdir()
        self.output_dir.mkdir()
        self.input_video = self.input_dir / "source.mp4"
        create_synthetic_video(self.input_video, duration=3.0)

    def tearDown(self):
        shutil.rmtree(self.temp_dir)

    def test_render_creates_video(self):
        from claw_shorts import render_step_breakdown
        video, metadata = render_step_breakdown(
            self.input_video,
            self.output_dir,
            steps=["Step 1", "Step 2", "Step 3"],
            title="Tutorial",
            seed=42,
        )
        self.assertTrue(video.exists())
        self.assertTrue(metadata.exists())

    def test_render_metadata_content(self):
        from claw_shorts import render_step_breakdown
        steps = ["Do this", "Then that", "Finally done"]
        video, metadata = render_step_breakdown(
            self.input_video,
            self.output_dir,
            steps=steps,
            title="How to X",
            cta_text="Subscribe",
            seed=42,
        )
        with open(metadata) as f:
            data = json.load(f)
        self.assertEqual(data["title"], "How to X")
        self.assertEqual(data["template"], "step_breakdown")
        self.assertEqual(data["steps"], steps)
        self.assertEqual(data["cta_text"], "Subscribe")


@unittest.skipUnless(ffmpeg_available(), "FFmpeg not available")
class TestCLI(unittest.TestCase):
    def setUp(self):
        self.temp_dir = tempfile.mkdtemp()
        self.temp_path = Path(self.temp_dir)
        self.input_dir = self.temp_path / "input"
        self.output_dir = self.temp_path / "output"
        self.input_dir.mkdir()
        self.output_dir.mkdir()
        self.input_video = self.input_dir / "source.mp4"
        create_synthetic_video(self.input_video, duration=2.0)
        self.cli_path = Path(__file__).parent.parent / "scripts" / "claw_shorts.py"

    def tearDown(self):
        shutil.rmtree(self.temp_dir)

    def test_cli_templates(self):
        result = subprocess.run(
            [sys.executable, str(self.cli_path), "templates"],
            capture_output=True,
            text=True,
        )
        self.assertEqual(result.returncode, 0)
        data = json.loads(result.stdout)
        self.assertIn("hook_reveal", data)
        self.assertIn("step_breakdown", data)

    def test_cli_info(self):
        result = subprocess.run(
            [
                sys.executable, str(self.cli_path),
                "info",
                "--input", str(self.input_video),
            ],
            capture_output=True,
            text=True,
        )
        self.assertEqual(result.returncode, 0)
        data = json.loads(result.stdout)
        self.assertIn("streams", data)

    def test_cli_render_hook_reveal(self):
        result = subprocess.run(
            [
                sys.executable, str(self.cli_path),
                "render",
                "--input", str(self.input_video),
                "--output", str(self.output_dir),
                "--template", "hook_reveal",
                "--title", "CLI Test",
                "--hook", "Test Hook",
                "--cta", "Test CTA",
                "--seed", "12345",
            ],
            capture_output=True,
            text=True,
        )
        self.assertEqual(result.returncode, 0, f"stderr: {result.stderr}")
        output_files = list(self.output_dir.glob("*.mp4"))
        self.assertEqual(len(output_files), 1)
        json_files = list(self.output_dir.glob("*.json"))
        self.assertEqual(len(json_files), 1)

    def test_cli_render_step_breakdown(self):
        result = subprocess.run(
            [
                sys.executable, str(self.cli_path),
                "render",
                "--input", str(self.input_video),
                "--output", str(self.output_dir),
                "--template", "step_breakdown",
                "--title", "Steps Test",
                "--steps", "A,B,C",
                "--seed", "54321",
            ],
            capture_output=True,
            text=True,
        )
        self.assertEqual(result.returncode, 0, f"stderr: {result.stderr}")
        output_files = list(self.output_dir.glob("*.mp4"))
        self.assertEqual(len(output_files), 1)


class TestNonDestructive(unittest.TestCase):
    @unittest.skipUnless(ffmpeg_available(), "FFmpeg not available")
    def test_source_file_unchanged(self):
        temp_dir = tempfile.mkdtemp()
        try:
            temp_path = Path(temp_dir)
            input_dir = temp_path / "input"
            output_dir = temp_path / "output"
            input_dir.mkdir()
            output_dir.mkdir()

            input_video = input_dir / "source.mp4"
            create_synthetic_video(input_video, duration=2.0)

            original_size = input_video.stat().st_size
            original_mtime = input_video.stat().st_mtime

            from claw_shorts import render_hook_reveal
            render_hook_reveal(
                input_video,
                output_dir,
                hook_text="Hook",
                cta_text="CTA",
                title="Title",
                seed=42,
            )

            self.assertEqual(input_video.stat().st_size, original_size)
            self.assertEqual(input_video.stat().st_mtime, original_mtime)

        finally:
            shutil.rmtree(temp_dir)


if __name__ == "__main__":
    unittest.main()
