#!/usr/bin/env python3
"""Explicit local builds and no-overwrite harness installation. No provider calls."""
import argparse
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import platform
import shutil
import subprocess
import sys
import tempfile

# Resolve symlinks so the installed launcher finds the checkout, not ~/.local/bin.
ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "scripts"))
_spec = importlib.util.spec_from_file_location("lean_feedback", ROOT / "scripts/lean-container.py")
feedback = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(feedback)

SKILLS = ("lean-review", "lean-proof")


def skill_roots(home):
    # Codex, Pi and Grok Build share the Agent Skills directory. No config edits.
    return (home / ".agents/skills", home / ".claude/skills", home / ".hermes/skills")


def payloads(home):
    links = {
        home / ".local/bin/claw-lean": ROOT / "target/release/claw-lean",
        home / ".local/bin/claw-lean-container": ROOT / "scripts/lean-container.py",
        home / ".local/bin/claw-lean-setup": ROOT / "scripts/setup.py",
    }
    bundles = {}
    for base in skill_roots(home):
        for skill in SKILLS:
            source = ROOT / "skills" / skill
            bundles[base / skill] = {
                p.relative_to(source): p.read_bytes()
                for p in source.rglob("*") if p.is_file()
            }
    return links, bundles


def reject_symlink_parents(path):
    for parent in path.parents:
        if parent.is_symlink():
            raise ValueError("Installation parent is symlinked: " + str(parent))


def install(home, apply=False):
    home = home.expanduser().absolute()
    if home.resolve() != home or not home.is_dir():
        raise ValueError("Home must be an existing canonical directory")
    links, bundles = payloads(home)
    pending_links, pending_bundles = {}, {}
    states = []
    # Complete preflight before any mkdir/write. Differing or unknown content
    # is never considered owned merely because a directory has our name.
    for dest, source in links.items():
        reject_symlink_parents(dest)
        if not source.is_file() or not os.access(source, os.X_OK):
            raise ValueError("Build/chmod tool before installation: " + str(source))
        if dest.is_symlink() and os.readlink(dest) == str(source):
            status = "unchanged"
        elif os.path.lexists(dest):
            raise ValueError("Conflicting tool (not overwritten): " + str(dest))
        else:
            pending_links[dest] = source
            status = "install"
        states.append({"path": str(dest), "status": status})
    for dest, files in bundles.items():
        reject_symlink_parents(dest)
        if os.path.lexists(dest):
            if dest.is_symlink() or not dest.is_dir():
                raise ValueError("Conflicting skill (not overwritten): " + str(dest))
            actual = {p.relative_to(dest): p for p in dest.rglob("*") if not p.is_dir()}
            if (set(actual) != set(files) or any(p.is_symlink() for p in dest.rglob("*"))
                    or any(actual[p].read_bytes() != data for p, data in files.items())):
                raise ValueError("Modified/foreign skill (not overwritten): " + str(dest))
            status = "unchanged"
        else:
            pending_bundles[dest] = files
            status = "install"
        states.append({"path": str(dest), "status": status})
    created = []
    if apply:
        try:
            for dest, source in pending_links.items():
                dest.parent.mkdir(parents=True, exist_ok=True)
                dest.symlink_to(source)  # exclusive; never replaces a racing writer
                created.append(dest)
            for dest, files in pending_bundles.items():
                dest.parent.mkdir(parents=True, exist_ok=True)
                dest.mkdir()  # exclusive; track only directories this call created
                created.append(dest)
                for relative, data in files.items():
                    path = dest / relative
                    path.parent.mkdir(parents=True, exist_ok=True)
                    with path.open("xb") as stream:
                        stream.write(data)
        except BaseException:
            for dest in reversed(created):
                if dest.is_symlink():
                    dest.unlink()
                else:
                    shutil.rmtree(dest)
            raise
    return {"applied": apply, "entries": states,
            "harnesses": {"claude-code": ".claude/skills", "codex": ".agents/skills",
                          "grok-build": ".agents/skills", "hermes": ".hermes/skills",
                          "pi": ".agents/skills"},
            "note": "Default homes only; no provider/config edits. Restart/reload harnesses."}


def build_image(args):
    feedback.local_docker()
    target = args.platform or ("linux/arm64" if platform.machine() in ("arm64", "aarch64")
                               else "linux/amd64")
    if not args.source:
        subprocess.run(["docker", "build", "--platform=" + target, "--progress=plain",
                        "-t", feedback.DEFAULT_IMAGE, str(ROOT)], check=True)
        return
    source = Path(args.source).expanduser().resolve(strict=True)
    # This is an explicitly authorized PUBLIC upstream checkout, not review input.
    revision = feedback.capture(["git", "-C", str(source), "rev-parse",
                                 "--verify", "--end-of-options", args.revision + "^{commit}"])
    if feedback.capture(["git", "-C", str(source), "status", "--porcelain"]):
        raise ValueError("Source checkout must be clean; archive only a reviewed public revision")
    if not (source / "CMakePresets.json").is_file() or not (source / "stage0/src/CMakeLists.txt").is_file():
        raise ValueError("Expected a public Lean 4 source checkout")
    with tempfile.TemporaryDirectory(prefix="claw-lean-public-source-") as temp:
        temp = Path(temp).resolve()
        archive = temp / "public-source.tar"
        with archive.open("wb") as stream:
            subprocess.run(["git", "-C", str(source), "archive", revision],
                           stdout=stream, check=True)
        digest = hashlib.sha256()
        with archive.open("rb") as stream:
            for chunk in iter(lambda: stream.read(1024 * 1024), b""):
                digest.update(chunk)
        sha = digest.hexdigest()
        # Pass the tar as named context, not the checkout. No .git/untracked files.
        context = temp / "source"
        context.mkdir()
        subprocess.run(["tar", "-xf", str(archive), "-C", str(context)], check=True)
        tag = "claw-work/lean-agent:source-" + revision[:10]
        print(json.dumps({"revision": revision, "archive_sha256": sha,
                          "image": tag, "platform": target}), flush=True)
        subprocess.run(["docker", "build", "--platform=" + target, "--progress=plain",
                        "-f", str(ROOT / "Dockerfile.source"), "--build-context",
                        "source=" + str(context), "--build-arg", "JOBS=" + str(args.jobs),
                        "-t", tag, str(ROOT)], check=True)


def smoke():
    binary = ROOT / "target/release/claw-lean"
    subprocess.run([str(binary), "doctor"], check=True)
    with tempfile.TemporaryDirectory(prefix="claw-lean-smoke-") as temp:
        # macOS /var and /tmp are symlinks; checker deliberately rejects aliases.
        base = Path(temp).resolve()
        for example in ("identity", "summation", "withdrawal"):
            subprocess.run([str(binary), "check", "--manifest",
                            str(ROOT / "examples" / example / "manifest.json"),
                            "--output", str(base / example)], check=True)


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    sub = parser.add_subparsers(dest="command", required=True)
    for name in ("install", "status"):
        p = sub.add_parser(name)
        p.add_argument("--home", type=Path, default=Path.home())
        if name == "install":
            p.add_argument("--apply", action="store_true", help="Explicitly authorize local writes")
    p = sub.add_parser("build-image")
    p.add_argument("--source", help="Explicitly select a clean PUBLIC Lean source checkout")
    p.add_argument("--revision", default="HEAD")
    p.add_argument("--platform", choices=["linux/arm64", "linux/amd64"])
    p.add_argument("--jobs", type=int, choices=range(1, 33), default=4)
    sub.add_parser("smoke")
    args = parser.parse_args(argv)
    try:
        if args.command in ("install", "status"):
            print(json.dumps(install(args.home, getattr(args, "apply", False)), indent=2))
        elif args.command == "build-image":
            build_image(args)
        else:
            smoke()
        return 0
    except (ValueError, OSError, subprocess.SubprocessError) as error:
        print("Lean setup failed: " + str(error), file=sys.stderr)
        return 2


if __name__ == "__main__":
    sys.exit(main())
