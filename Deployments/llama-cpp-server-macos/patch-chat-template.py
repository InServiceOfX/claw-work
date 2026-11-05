#!/usr/bin/env python3
"""patch-chat-template.py — let a GGUF's chat template accept mid-conversation
system messages.

Qwen3-family templates raise "System message must be at the beginning." when a
system message appears anywhere but first. Some agent harnesses (Claude Code
among them) send exactly that, e.g. a dated environment note after the first
user turn, so every request fails inside the template.

This script reads the template embedded in a profile's GGUF, renders such a
message as an ordinary system turn instead of raising, writes the result to
templates/<profile>.jinja, and prints the profile key that makes launch.sh use
it. The model and its weights are untouched.

Usage (any Python with gguf + pyyaml + jinja2, e.g. through uv):
    uv run --no-project --with gguf --with pyyaml --with jinja2 \\
        python patch-chat-template.py <profile>
"""

from __future__ import annotations

import os
from pathlib import Path
import re
import sys

SCRIPT_DIR = Path(__file__).resolve().parent

RAISE = re.compile(
    r"\{\{-?\s*raise_exception\(\s*['\"]System message must be at the beginning\.?['\"]\s*\)\s*-?\}\}"
)
SYSTEM_TURN = "{{- '<|im_start|>system\\n' + content + '<|im_end|>\\n' }}"


def hf_cache_root() -> Path:
    if os.environ.get("HF_HUB_CACHE"):
        return Path(os.environ["HF_HUB_CACHE"]).expanduser()
    if os.environ.get("HF_HOME"):
        return Path(os.environ["HF_HOME"]).expanduser() / "hub"
    return Path.home() / ".cache" / "huggingface" / "hub"


def model_file(profile: dict, config: dict) -> Path:
    if profile.get("model_path"):
        path = Path(os.path.expandvars(os.path.expanduser(str(profile["model_path"]))))
        if not path.is_absolute():
            path = Path(os.path.expanduser(str(config.get("models_dir", "")))) / path
        return path
    repo, name = profile.get("hf_repo"), profile.get("hf_file")
    if not (repo and name):
        sys.exit("Profile needs either model_path, or hf_repo + hf_file")
    root = hf_cache_root() / ("models--" + repo.replace("/", "--"))
    revision = profile.get("hf_revision")
    if not revision and (root / "refs" / "main").is_file():
        revision = (root / "refs" / "main").read_text().strip()
    snapshots = sorted((root / "snapshots").glob("*"), key=lambda p: p.name != revision)
    for snap in snapshots:
        if (snap / name).exists():
            return snap / name
    sys.exit(f"Not in the HuggingFace cache: {repo}/{name}")


def embedded_template(gguf_path: Path) -> str:
    from gguf import GGUFReader

    field = GGUFReader(str(gguf_path)).fields.get("tokenizer.chat_template")
    if field is None:
        sys.exit(f"No embedded chat template in {gguf_path}")
    return bytes(field.parts[field.data[0]]).decode("utf-8")


def check_renders(template: str) -> None:
    """Render [user, system, user] with jinja2 as a sanity check.

    llama.cpp renders with its own engine, so this is a smoke test, not proof.
    """
    import jinja2

    def raise_exception(msg):
        raise jinja2.TemplateError(msg)

    env = jinja2.Environment(trim_blocks=True, lstrip_blocks=True)
    env.globals["raise_exception"] = raise_exception
    out = env.from_string(template).render(
        messages=[
            {"role": "user", "content": "first"},
            {"role": "system", "content": "MID_SYSTEM_NOTE"},
            {"role": "user", "content": "second"},
        ],
        add_generation_prompt=True,
    )
    if "<|im_start|>system\nMID_SYSTEM_NOTE<|im_end|>" not in out:
        sys.exit("Patched template did not render the mid-conversation system turn")


def main() -> int:
    if len(sys.argv) != 2 or sys.argv[1].startswith("-"):
        print(__doc__.strip())
        return 0 if len(sys.argv) == 1 else 2
    import yaml

    name = sys.argv[1]
    profile_file = SCRIPT_DIR / "profiles" / f"{name}.yml"
    if not profile_file.is_file():
        sys.exit(f"Profile not found: {profile_file}")
    profile = yaml.safe_load(profile_file.read_text()) or {}
    config_file = SCRIPT_DIR / "config.yml"
    config = (yaml.safe_load(config_file.read_text()) or {}) if config_file.is_file() else {}

    gguf_path = model_file(profile, config)
    template = embedded_template(gguf_path)
    patched, count = RAISE.subn(lambda _: SYSTEM_TURN, template)
    if count != 1:
        sys.exit(
            "Expected one 'System message must be at the beginning' check in the "
            f"template, found {count}. This template needs a manual edit."
        )
    check_renders(patched)

    out = SCRIPT_DIR / "templates" / f"{name}.jinja"
    out.parent.mkdir(exist_ok=True)
    out.write_text(patched, encoding="utf-8")
    print(f"Wrote {out.relative_to(SCRIPT_DIR)} (from {gguf_path.name})")
    print("")
    print(f"Add to profiles/{name}.yml, then re-launch:")
    print("")
    print(f"  chat_template_file: templates/{name}.jinja")
    return 0


if __name__ == "__main__":
    sys.exit(main())
