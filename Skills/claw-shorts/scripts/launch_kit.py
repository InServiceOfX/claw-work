#!/usr/bin/env python3
"""
claw-shorts Launch Kit: Generate launch videos for dev tools and indie apps.

Point a coding agent at a GitHub repo URL or product URL, optionally with a
screen recording, and render 5 captioned vertical Shorts plus a posting kit.

Uses only open-source, local tools: FFmpeg, Python, and optionally Piper TTS.
No paid SaaS dependency, no AI-generated faces, no auto-posting.
"""

import json
import os
import re
import subprocess
import tempfile
import urllib.request
import urllib.error
from dataclasses import dataclass, field, asdict
from datetime import datetime
from pathlib import Path
from typing import Optional, List, Dict, Any
from urllib.parse import urlparse


@dataclass
class RepoMetadata:
    """Metadata extracted from a GitHub repository."""
    name: str
    description: str
    readme_text: str
    language: str
    topics: List[str] = field(default_factory=list)
    stars: int = 0
    url: str = ""
    homepage: str = ""
    screenshots: List[str] = field(default_factory=list)


@dataclass
class ClipScript:
    """Script for a single Short clip."""
    clip_number: int
    template: str
    hook_text: str
    body_beats: List[str]
    cta_text: str
    duration_target: float = 15.0
    visual_notes: str = ""
    audio_notes: str = ""


@dataclass
class LaunchPlan:
    """Complete plan for 5 launch Shorts."""
    project_name: str
    tagline: str
    clips: List[ClipScript]
    posting_order: List[int]
    titles: List[str]
    captions: List[str]
    hashtags: List[str]
    disclosure_onscreen: str
    disclosure_spoken: str
    created_at: str = ""
    
    def __post_init__(self):
        if not self.created_at:
            self.created_at = datetime.now().isoformat()


LAUNCH_CLIP_TEMPLATES = [
    {
        "number": 1,
        "name": "hook_problem",
        "template": "hook_reveal",
        "purpose": "Grab attention with a pain point the viewer recognizes",
        "hook_pattern": "Tired of [problem]? / Still doing [tedious task]?",
        "body_pattern": "Show the problem briefly (messy code, slow process)",
        "cta_pattern": "Watch the solution →",
    },
    {
        "number": 2,
        "name": "demo_wow",
        "template": "demo_zoom",
        "purpose": "Show the product working in one impressive shot",
        "hook_pattern": "Watch [tool] do this in [N] seconds",
        "body_pattern": "Screen recording of the key feature",
        "cta_pattern": "Link in bio to try it",
    },
    {
        "number": 3,
        "name": "how_it_works",
        "template": "step_breakdown",
        "purpose": "3-step explainer that makes it look easy",
        "hook_pattern": "Here's how [tool] works:",
        "body_pattern": "Step 1: X / Step 2: Y / Step 3: Z",
        "cta_pattern": "Try it yourself",
    },
    {
        "number": 4,
        "name": "before_after",
        "template": "before_after",
        "purpose": "Visual contrast showing improvement",
        "hook_pattern": "Before vs After [tool]",
        "body_pattern": "Split screen or sequential comparison",
        "cta_pattern": "Get the [tool]",
    },
    {
        "number": 5,
        "name": "social_proof",
        "template": "hook_reveal",
        "purpose": "Stars, downloads, or testimonial (if available)",
        "hook_pattern": "[N] developers are using this",
        "body_pattern": "GitHub stars counter, testimonial, or use case list",
        "cta_pattern": "Join them",
    },
]


def fetch_github_repo_metadata(repo_url: str) -> RepoMetadata:
    """
    Fetch metadata from a GitHub repository URL.
    
    Works offline if the repo is cloned locally; otherwise makes API calls.
    Accepts local paths (with or without .git directory), or GitHub URLs.
    """
    parsed = urlparse(repo_url)
    
    if parsed.scheme == "" or parsed.netloc == "":
        path = Path(repo_url)
        if path.exists() and path.is_dir():
            return _fetch_local_repo(path)
    
    if "github.com" in parsed.netloc:
        path_parts = parsed.path.strip("/").split("/")
        if len(path_parts) >= 2:
            owner, repo = path_parts[0], path_parts[1]
            repo = repo.replace(".git", "")
            return _fetch_github_api(owner, repo)
    
    raise ValueError(f"Could not parse repository URL: {repo_url}")


def _fetch_local_repo(path: Path) -> RepoMetadata:
    """Extract metadata from a local Git repository."""
    readme_text = ""
    for readme_name in ["README.md", "README.rst", "README.txt", "README"]:
        readme_path = path / readme_name
        if readme_path.exists():
            readme_text = readme_path.read_text(errors="replace")[:10000]
            break
    
    name = path.name
    
    package_json = path / "package.json"
    cargo_toml = path / "Cargo.toml"
    pyproject = path / "pyproject.toml"
    setup_py = path / "setup.py"
    
    description = ""
    language = ""
    
    if package_json.exists():
        try:
            data = json.loads(package_json.read_text())
            description = data.get("description", "")
            name = data.get("name", name)
        except json.JSONDecodeError:
            pass
    
    if cargo_toml.exists():
        language = "Rust"
        content = cargo_toml.read_text()
        desc_match = re.search(r'^description\s*=\s*"([^"]+)"', content, re.MULTILINE)
        if desc_match:
            description = desc_match.group(1)
        name_match = re.search(r'^name\s*=\s*"([^"]+)"', content, re.MULTILINE)
        if name_match:
            name = name_match.group(1)
    elif pyproject.exists():
        language = "Python"
        content = pyproject.read_text()
        desc_match = re.search(r'^description\s*=\s*"([^"]+)"', content, re.MULTILINE)
        if desc_match:
            description = desc_match.group(1)
    
    if not language:
        language = _detect_language(path)
    
    screenshots = _find_screenshots(path)
    
    return RepoMetadata(
        name=name,
        description=description,
        readme_text=readme_text,
        language=language,
        topics=[],
        stars=0,
        url=str(path.resolve()),
        homepage="",
        screenshots=screenshots,
    )


def _fetch_github_api(owner: str, repo: str) -> RepoMetadata:
    """Fetch repository metadata from GitHub API."""
    api_url = f"https://api.github.com/repos/{owner}/{repo}"
    readme_url = f"https://api.github.com/repos/{owner}/{repo}/readme"
    
    headers = {
        "Accept": "application/vnd.github.v3+json",
        "User-Agent": "claw-shorts/1.0",
    }
    
    req = urllib.request.Request(api_url, headers=headers)
    try:
        with urllib.request.urlopen(req, timeout=10) as response:
            repo_data = json.loads(response.read().decode())
    except urllib.error.URLError as e:
        raise RuntimeError(f"Failed to fetch repo data: {e}")
    
    readme_text = ""
    try:
        headers["Accept"] = "application/vnd.github.v3.raw"
        req = urllib.request.Request(readme_url, headers=headers)
        with urllib.request.urlopen(req, timeout=10) as response:
            readme_text = response.read().decode(errors="replace")[:10000]
    except urllib.error.URLError:
        pass
    
    screenshots = _extract_images_from_readme(readme_text)
    
    return RepoMetadata(
        name=repo_data.get("name", repo),
        description=repo_data.get("description", "") or "",
        readme_text=readme_text,
        language=repo_data.get("language", "") or "",
        topics=repo_data.get("topics", []),
        stars=repo_data.get("stargazers_count", 0),
        url=repo_data.get("html_url", f"https://github.com/{owner}/{repo}"),
        homepage=repo_data.get("homepage", "") or "",
        screenshots=screenshots,
    )


def _detect_language(path: Path) -> str:
    """Detect the primary language of a local repository."""
    extensions = {
        ".py": "Python",
        ".rs": "Rust",
        ".ts": "TypeScript",
        ".tsx": "TypeScript",
        ".js": "JavaScript",
        ".jsx": "JavaScript",
        ".go": "Go",
        ".java": "Java",
        ".rb": "Ruby",
        ".swift": "Swift",
        ".kt": "Kotlin",
        ".c": "C",
        ".cpp": "C++",
        ".cs": "C#",
    }
    
    counts: Dict[str, int] = {}
    for ext, lang in extensions.items():
        count = len(list(path.rglob(f"*{ext}")))
        if count > 0:
            counts[lang] = counts.get(lang, 0) + count
    
    if counts:
        return max(counts, key=counts.get)
    return ""


def _find_screenshots(path: Path) -> List[str]:
    """Find screenshot images in a local repository."""
    screenshots = []
    image_extensions = [".png", ".jpg", ".jpeg", ".gif", ".webp"]
    
    search_dirs = [
        path / "docs",
        path / "screenshots",
        path / "images",
        path / "assets",
        path / ".github",
        path,
    ]
    
    for search_dir in search_dirs:
        if not search_dir.exists():
            continue
        for ext in image_extensions:
            for img in search_dir.glob(f"*{ext}"):
                screenshots.append(str(img))
                if len(screenshots) >= 5:
                    return screenshots
    
    return screenshots


def _extract_images_from_readme(readme: str) -> List[str]:
    """Extract image URLs from README markdown."""
    img_pattern = r'!\[.*?\]\((https?://[^\)]+)\)'
    html_pattern = r'<img[^>]+src="(https?://[^"]+)"'
    
    urls = re.findall(img_pattern, readme)
    urls.extend(re.findall(html_pattern, readme))
    
    return urls[:5]


def generate_launch_plan(
    metadata: RepoMetadata,
    recording_path: Optional[Path] = None,
    sponsored: bool = False,
    affiliate: bool = False,
) -> LaunchPlan:
    """
    Generate a 5-clip launch plan from repository metadata.
    
    The plan includes hooks, beats, captions, hashtags, and disclosure text.
    """
    name = metadata.name
    desc = metadata.description or _extract_tagline_from_readme(metadata.readme_text)
    lang = metadata.language
    
    clips = []
    for template_info in LAUNCH_CLIP_TEMPLATES:
        clip = _generate_clip_script(template_info, metadata, recording_path)
        clips.append(clip)
    
    hashtags = _generate_hashtags(metadata)
    
    disclosure_onscreen = ""
    disclosure_spoken = ""
    if sponsored:
        disclosure_onscreen = "#ad #sponsored"
        disclosure_spoken = "This video is sponsored."
    elif affiliate:
        disclosure_onscreen = "#affiliate"
        disclosure_spoken = "This video contains affiliate links."
    
    titles = [
        f"{name}: {desc[:50]}..." if len(desc) > 50 else f"{name}: {desc}",
        f"Watch {name} in action",
        f"How {name} works in 3 steps",
        f"Before vs After {name}",
        f"Developers love {name}",
    ]
    
    captions = [
        f"Tired of the old way? {name} fixes this. {' '.join(hashtags[:5])}",
        f"This is {name} doing its thing. Link in bio. {' '.join(hashtags[:5])}",
        f"3 steps to use {name}. Easy as that. {' '.join(hashtags[:5])}",
        f"The difference {name} makes. {' '.join(hashtags[:5])}",
        f"Join the {name} community. {' '.join(hashtags[:5])}",
    ]
    
    if disclosure_onscreen:
        captions = [f"{c} {disclosure_onscreen}" for c in captions]
    
    posting_order = [1, 3, 2, 4, 5]
    
    return LaunchPlan(
        project_name=name,
        tagline=desc,
        clips=clips,
        posting_order=posting_order,
        titles=titles,
        captions=captions,
        hashtags=hashtags,
        disclosure_onscreen=disclosure_onscreen,
        disclosure_spoken=disclosure_spoken,
    )


def _extract_tagline_from_readme(readme: str) -> str:
    """Extract a one-line description from the README."""
    lines = readme.strip().split("\n")
    for line in lines:
        line = line.strip()
        if line.startswith("#"):
            continue
        if not line:
            continue
        if line.startswith("!"):
            continue
        desc = re.sub(r'\[([^\]]+)\]\([^\)]+\)', r'\1', line)
        desc = re.sub(r'[*_`]', '', desc)
        desc = desc.strip()
        if len(desc) > 5:
            return desc[:200]
    return ""


def _generate_clip_script(
    template_info: dict,
    metadata: RepoMetadata,
    recording_path: Optional[Path],
) -> ClipScript:
    """Generate a single clip script from template and metadata."""
    name = metadata.name
    desc = metadata.description or "this tool"
    
    number = template_info["number"]
    template = template_info["template"]
    
    if number == 1:
        problem = _infer_problem(metadata)
        hook = f"Still {problem}?" if problem else f"Stop what you're doing"
        body = ["Show the pain point", f"Introduce {name}", "Hint at the solution"]
        cta = "Watch the solution →"
    elif number == 2:
        hook = f"Watch {name} do this"
        body = ["Screen recording shows the key feature", "Fast, impressive demo"]
        cta = "Link in bio"
        if recording_path:
            body.insert(0, f"Use recording: {recording_path}")
    elif number == 3:
        steps = _extract_steps_from_readme(metadata.readme_text)
        hook = f"How to use {name}:"
        body = steps if steps else ["Install it", "Configure it", "Run it"]
        cta = "Try it yourself"
    elif number == 4:
        hook = f"Before vs After {name}"
        body = ["Before: the old way", "After: the {name} way", "Side by side comparison"]
        cta = f"Get {name}"
    else:
        stars = metadata.stars
        hook = f"{stars}+ developers use this" if stars > 0 else f"Developers love {name}"
        body = ["Show GitHub stars or usage", "Quick testimonial or feature list"]
        cta = "Join them"
    
    visual_notes = ""
    if recording_path:
        visual_notes = f"Use provided recording: {recording_path}"
    elif metadata.screenshots:
        visual_notes = f"Screenshots available: {', '.join(metadata.screenshots[:2])}"
    
    return ClipScript(
        clip_number=number,
        template=template,
        hook_text=hook,
        body_beats=body,
        cta_text=cta,
        visual_notes=visual_notes,
    )


def _infer_problem(metadata: RepoMetadata) -> str:
    """Infer the problem the tool solves from README."""
    readme = metadata.readme_text.lower()
    
    problem_patterns = [
        r"tired of (\w+)",
        r"no more (\w+)",
        r"stop (\w+)",
        r"avoid (\w+)",
        r"instead of (\w+)",
    ]
    
    for pattern in problem_patterns:
        match = re.search(pattern, readme)
        if match:
            word = match.group(1)
            if word.endswith("ing"):
                return word
            return word + "ing"
    
    return ""


def _extract_steps_from_readme(readme: str) -> List[str]:
    """Extract installation/usage steps from README."""
    steps = []
    
    lines = readme.split("\n")
    in_steps = False
    
    for line in lines:
        line = line.strip()
        
        if re.match(r"^#+\s*(install|usage|getting started|quick start)", line, re.I):
            in_steps = True
            continue
        
        if in_steps:
            if re.match(r"^#+", line):
                break
            
            step_match = re.match(r"^\d+[.\)]\s*(.+)", line)
            if step_match:
                step = step_match.group(1)
                step = re.sub(r'`[^`]+`', '', step)
                step = step.strip()
                if len(step) > 5:
                    steps.append(step[:50])
                    if len(steps) >= 3:
                        return steps
    
    return steps[:3] if steps else []


def _generate_hashtags(metadata: RepoMetadata) -> List[str]:
    """Generate relevant hashtags from metadata."""
    hashtags = ["#shorts", "#coding", "#devtools"]
    
    if metadata.language:
        lang_tag = f"#{metadata.language.lower().replace(' ', '').replace('+', 'plus')}"
        hashtags.append(lang_tag)
    
    for topic in metadata.topics[:5]:
        tag = f"#{topic.lower().replace('-', '').replace(' ', '')}"
        hashtags.append(tag)
    
    category_tags = {
        "cli": "#cli",
        "api": "#api",
        "server": "#backend",
        "mcp": "#mcp",
        "agent": "#aiagent",
        "llm": "#llm",
        "ai": "#ai",
        "ml": "#machinelearning",
        "web": "#webdev",
        "react": "#react",
        "vue": "#vuejs",
        "next": "#nextjs",
    }
    
    readme_lower = metadata.readme_text.lower()
    for keyword, tag in category_tags.items():
        if keyword in readme_lower and tag not in hashtags:
            hashtags.append(tag)
            if len(hashtags) >= 10:
                break
    
    return hashtags[:10]


def save_launch_plan(plan: LaunchPlan, output_path: Path) -> None:
    """Save a launch plan to JSON file."""
    data = {
        "project_name": plan.project_name,
        "tagline": plan.tagline,
        "clips": [asdict(clip) for clip in plan.clips],
        "posting_order": plan.posting_order,
        "titles": plan.titles,
        "captions": plan.captions,
        "hashtags": plan.hashtags,
        "disclosure_onscreen": plan.disclosure_onscreen,
        "disclosure_spoken": plan.disclosure_spoken,
        "created_at": plan.created_at,
    }
    
    output_path.parent.mkdir(parents=True, exist_ok=True)
    with open(output_path, "w") as f:
        json.dump(data, f, indent=2)


def load_launch_plan(path: Path) -> LaunchPlan:
    """Load a launch plan from JSON file."""
    with open(path) as f:
        data = json.load(f)
    
    clips = [
        ClipScript(
            clip_number=c["clip_number"],
            template=c["template"],
            hook_text=c["hook_text"],
            body_beats=c["body_beats"],
            cta_text=c["cta_text"],
            duration_target=c.get("duration_target", 15.0),
            visual_notes=c.get("visual_notes", ""),
            audio_notes=c.get("audio_notes", ""),
        )
        for c in data["clips"]
    ]
    
    return LaunchPlan(
        project_name=data["project_name"],
        tagline=data["tagline"],
        clips=clips,
        posting_order=data["posting_order"],
        titles=data["titles"],
        captions=data["captions"],
        hashtags=data["hashtags"],
        disclosure_onscreen=data.get("disclosure_onscreen", ""),
        disclosure_spoken=data.get("disclosure_spoken", ""),
        created_at=data.get("created_at", ""),
    )


def generate_posting_kit(plan: LaunchPlan, output_dir: Path) -> Path:
    """Generate a posting kit with all metadata for the 5 clips."""
    kit_path = output_dir / "posting_kit.json"
    
    kit = {
        "project": plan.project_name,
        "tagline": plan.tagline,
        "generated_at": datetime.now().isoformat(),
        "clips": [],
    }
    
    for i, clip_num in enumerate(plan.posting_order):
        clip_idx = clip_num - 1
        clip = plan.clips[clip_idx]
        
        kit["clips"].append({
            "posting_order": i + 1,
            "clip_number": clip_num,
            "title": plan.titles[clip_idx],
            "caption": plan.captions[clip_idx],
            "hashtags": plan.hashtags,
            "template": clip.template,
            "hook": clip.hook_text,
            "cta": clip.cta_text,
        })
    
    if plan.disclosure_onscreen or plan.disclosure_spoken:
        kit["disclosure"] = {
            "onscreen": plan.disclosure_onscreen,
            "spoken": plan.disclosure_spoken,
        }
    
    kit["posting_tips"] = [
        "Post clip 1 (problem/hook) first to test engagement",
        "Post clip 3 (how it works) second for educational value",
        "Post clip 2 (demo) third once you have some followers",
        "Space clips 2-3 days apart for algorithm favor",
        "Best times: weekday mornings (8-10am) or evenings (7-9pm)",
    ]
    
    output_dir.mkdir(parents=True, exist_ok=True)
    with open(kit_path, "w") as f:
        json.dump(kit, f, indent=2)
    
    return kit_path
