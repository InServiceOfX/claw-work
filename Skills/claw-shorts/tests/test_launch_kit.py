#!/usr/bin/env python3
"""
Tests for claw-shorts Launch Kit.

Tests the launch mode, repo metadata fetching, and launch plan generation.
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

from launch_kit import (
    RepoMetadata,
    ClipScript,
    LaunchPlan,
    fetch_github_repo_metadata,
    generate_launch_plan,
    save_launch_plan,
    load_launch_plan,
    generate_posting_kit,
    _extract_tagline_from_readme,
    _generate_hashtags,
    _extract_steps_from_readme,
    LAUNCH_CLIP_TEMPLATES,
)


class TestRepoMetadata(unittest.TestCase):
    def test_dataclass_creation(self):
        meta = RepoMetadata(
            name="test-repo",
            description="A test repository",
            readme_text="# Test\n\nThis is a test.",
            language="Python",
            topics=["cli", "testing"],
            stars=100,
            url="https://github.com/test/test-repo",
        )
        self.assertEqual(meta.name, "test-repo")
        self.assertEqual(meta.description, "A test repository")
        self.assertEqual(meta.language, "Python")
        self.assertEqual(meta.stars, 100)
        self.assertIn("cli", meta.topics)

    def test_default_values(self):
        meta = RepoMetadata(
            name="minimal",
            description="",
            readme_text="",
            language="",
        )
        self.assertEqual(meta.topics, [])
        self.assertEqual(meta.stars, 0)
        self.assertEqual(meta.screenshots, [])


class TestClipScript(unittest.TestCase):
    def test_clip_creation(self):
        clip = ClipScript(
            clip_number=1,
            template="hook_reveal",
            hook_text="Stop scrolling",
            body_beats=["Show problem", "Show solution"],
            cta_text="Follow for more",
        )
        self.assertEqual(clip.clip_number, 1)
        self.assertEqual(clip.template, "hook_reveal")
        self.assertEqual(clip.duration_target, 15.0)


class TestLaunchPlan(unittest.TestCase):
    def test_plan_creation(self):
        clips = [
            ClipScript(
                clip_number=i,
                template="hook_reveal",
                hook_text=f"Hook {i}",
                body_beats=["Beat 1", "Beat 2"],
                cta_text="CTA",
            )
            for i in range(1, 6)
        ]
        
        plan = LaunchPlan(
            project_name="test-project",
            tagline="A test project",
            clips=clips,
            posting_order=[1, 3, 2, 4, 5],
            titles=["Title 1", "Title 2", "Title 3", "Title 4", "Title 5"],
            captions=["Caption 1", "Caption 2", "Caption 3", "Caption 4", "Caption 5"],
            hashtags=["#shorts", "#coding"],
            disclosure_onscreen="",
            disclosure_spoken="",
        )
        
        self.assertEqual(plan.project_name, "test-project")
        self.assertEqual(len(plan.clips), 5)
        self.assertEqual(plan.posting_order[0], 1)
        self.assertIn("#shorts", plan.hashtags)
        self.assertNotEqual(plan.created_at, "")


class TestLocalRepoFetch(unittest.TestCase):
    def setUp(self):
        self.fixtures_dir = Path(__file__).parent.parent / "fixtures" / "sample-repo"
    
    def test_fetch_local_repo(self):
        if not self.fixtures_dir.exists():
            self.skipTest("Fixtures directory not found")
        
        meta = fetch_github_repo_metadata(str(self.fixtures_dir))
        
        self.assertEqual(meta.name, "fastcli")
        self.assertIn("blazing-fast", meta.description.lower())
        self.assertEqual(meta.language, "Rust")
        self.assertIn("# fastcli", meta.readme_text)


class TestTaglineExtraction(unittest.TestCase):
    def test_extract_tagline_simple(self):
        readme = "# My Project\n\nThis is a great tool for developers."
        tagline = _extract_tagline_from_readme(readme)
        self.assertIn("great tool", tagline)
    
    def test_extract_tagline_with_links(self):
        readme = "# Project\n\n[Docs](link) and [API](api) are available."
        tagline = _extract_tagline_from_readme(readme)
        self.assertNotIn("[", tagline)
        self.assertIn("Docs", tagline)
    
    def test_extract_tagline_skips_headers(self):
        readme = "# Header\n## Another Header\n\nActual description here."
        tagline = _extract_tagline_from_readme(readme)
        self.assertIn("description", tagline.lower())


class TestHashtagGeneration(unittest.TestCase):
    def test_generate_basic_hashtags(self):
        meta = RepoMetadata(
            name="test",
            description="test",
            readme_text="",
            language="Python",
        )
        hashtags = _generate_hashtags(meta)
        
        self.assertIn("#shorts", hashtags)
        self.assertIn("#coding", hashtags)
        self.assertIn("#python", hashtags)
    
    def test_generate_topic_hashtags(self):
        meta = RepoMetadata(
            name="test",
            description="test",
            readme_text="",
            language="",
            topics=["cli", "mcp-server"],
        )
        hashtags = _generate_hashtags(meta)
        
        self.assertIn("#cli", hashtags)
    
    def test_generate_readme_hashtags(self):
        meta = RepoMetadata(
            name="test",
            description="test",
            readme_text="This is an AI-powered LLM agent.",
            language="",
        )
        hashtags = _generate_hashtags(meta)
        
        self.assertIn("#ai", hashtags)


class TestStepExtraction(unittest.TestCase):
    def test_extract_numbered_steps(self):
        readme = """
# Installation

1. Install dependencies
2. Configure the app
3. Run the server
"""
        steps = _extract_steps_from_readme(readme)
        self.assertEqual(len(steps), 3)
        self.assertIn("Install", steps[0])
    
    def test_extract_from_quick_start(self):
        readme = """
# Quick Start

1. Clone the repo
2. Run npm install
3. Start with npm run dev
"""
        steps = _extract_steps_from_readme(readme)
        self.assertEqual(len(steps), 3)


class TestLaunchPlanGeneration(unittest.TestCase):
    def test_generate_plan_from_metadata(self):
        meta = RepoMetadata(
            name="myproject",
            description="A tool for developers",
            readme_text="# MyProject\n\nMakes coding easier.",
            language="TypeScript",
            stars=50,
        )
        
        plan = generate_launch_plan(meta)
        
        self.assertEqual(plan.project_name, "myproject")
        self.assertEqual(len(plan.clips), 5)
        self.assertEqual(len(plan.titles), 5)
        self.assertEqual(len(plan.captions), 5)
        self.assertIn("#shorts", plan.hashtags)
    
    def test_generate_plan_with_disclosure(self):
        meta = RepoMetadata(
            name="sponsored-project",
            description="A sponsored tool",
            readme_text="",
            language="",
        )
        
        plan = generate_launch_plan(meta, sponsored=True)
        
        self.assertIn("#ad", plan.disclosure_onscreen)
        self.assertIn("sponsored", plan.disclosure_spoken.lower())
    
    def test_generate_plan_affiliate(self):
        meta = RepoMetadata(
            name="affiliate-project",
            description="Has affiliate links",
            readme_text="",
            language="",
        )
        
        plan = generate_launch_plan(meta, affiliate=True)
        
        self.assertIn("#affiliate", plan.disclosure_onscreen)


class TestLaunchPlanIO(unittest.TestCase):
    def setUp(self):
        self.temp_dir = tempfile.mkdtemp()
        self.temp_path = Path(self.temp_dir)
    
    def tearDown(self):
        shutil.rmtree(self.temp_dir)
    
    def test_save_and_load_plan(self):
        clips = [
            ClipScript(
                clip_number=i,
                template="hook_reveal",
                hook_text=f"Hook {i}",
                body_beats=["Beat"],
                cta_text="CTA",
            )
            for i in range(1, 6)
        ]
        
        original = LaunchPlan(
            project_name="test",
            tagline="Test tagline",
            clips=clips,
            posting_order=[1, 2, 3, 4, 5],
            titles=["T1", "T2", "T3", "T4", "T5"],
            captions=["C1", "C2", "C3", "C4", "C5"],
            hashtags=["#test"],
            disclosure_onscreen="",
            disclosure_spoken="",
        )
        
        plan_path = self.temp_path / "plan.json"
        save_launch_plan(original, plan_path)
        
        self.assertTrue(plan_path.exists())
        
        loaded = load_launch_plan(plan_path)
        
        self.assertEqual(loaded.project_name, original.project_name)
        self.assertEqual(loaded.tagline, original.tagline)
        self.assertEqual(len(loaded.clips), 5)
        self.assertEqual(loaded.posting_order, original.posting_order)


class TestPostingKit(unittest.TestCase):
    def setUp(self):
        self.temp_dir = tempfile.mkdtemp()
        self.temp_path = Path(self.temp_dir)
    
    def tearDown(self):
        shutil.rmtree(self.temp_dir)
    
    def test_generate_posting_kit(self):
        clips = [
            ClipScript(
                clip_number=i,
                template="hook_reveal",
                hook_text=f"Hook {i}",
                body_beats=["Beat"],
                cta_text=f"CTA {i}",
            )
            for i in range(1, 6)
        ]
        
        plan = LaunchPlan(
            project_name="testkit",
            tagline="Test posting kit",
            clips=clips,
            posting_order=[1, 3, 2, 4, 5],
            titles=["T1", "T2", "T3", "T4", "T5"],
            captions=["C1", "C2", "C3", "C4", "C5"],
            hashtags=["#shorts", "#test"],
            disclosure_onscreen="",
            disclosure_spoken="",
        )
        
        kit_path = generate_posting_kit(plan, self.temp_path)
        
        self.assertTrue(kit_path.exists())
        
        with open(kit_path) as f:
            kit = json.load(f)
        
        self.assertEqual(kit["project"], "testkit")
        self.assertEqual(len(kit["clips"]), 5)
        self.assertEqual(kit["clips"][0]["posting_order"], 1)
        self.assertIn("posting_tips", kit)


class TestLaunchTemplates(unittest.TestCase):
    def test_template_count(self):
        self.assertEqual(len(LAUNCH_CLIP_TEMPLATES), 5)
    
    def test_template_structure(self):
        for template in LAUNCH_CLIP_TEMPLATES:
            self.assertIn("number", template)
            self.assertIn("name", template)
            self.assertIn("template", template)
            self.assertIn("purpose", template)
            self.assertIn("hook_pattern", template)


if __name__ == "__main__":
    unittest.main()
