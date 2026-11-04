#!/usr/bin/env python3
"""Bounded, offline Lean feedback/LSP. Not an independent proof certificate."""
import argparse
import json
import os
from pathlib import Path
import signal
import subprocess
import sys
import uuid

DEFAULT_IMAGE = "claw-work/lean-agent:lean4.35.0-mathlib"


def capture(args):
    return subprocess.check_output(args, text=True, timeout=15).strip()


def local_docker():
    # Match the Rust checker's local-only policy. Explicit host overrides and
    # the selected context must both be local, never an SSH/TCP upload target.
    host = os.environ.get("DOCKER_HOST", "unix://")
    endpoint = capture(["docker", "context", "inspect", "--format",
                        "{{.Endpoints.docker.Host}}"])
    if not host.startswith("unix://") or not endpoint.startswith("unix://"):
        raise ValueError("Only a local Unix-socket Docker daemon is allowed")


def resolve_image(image):
    if not image or image.startswith("-") or any(c.isspace() for c in image):
        raise ValueError("Invalid image reference")
    local_docker()
    data = json.loads(capture(["docker", "image", "inspect", image]))[0]
    identity = data["Id"]
    if (not identity.startswith("sha256:") or len(identity) != 71
            or any(c not in "0123456789abcdef" for c in identity[7:])):
        raise ValueError("Invalid local image identity")
    return identity, data["Os"] + "/" + data["Architecture"]


def docker_args(args, identity, platform, name):
    if not 1 <= args.timeout_seconds <= 3600:
        raise ValueError("Deadline must be 1..3600 seconds")
    if not 128 <= args.memory_mib <= 65536:
        raise ValueError("Memory must be 128..65536 MiB")
    if not 1 <= args.cpus <= 64:
        raise ValueError("CPUs must be 1..64")
    command = ["docker", "run", "--rm", "--pull=never", "--platform=" + platform,
               "--name", name, "--network=none", "--read-only", "--cap-drop=ALL",
               "--security-opt=no-new-privileges", "--pids-limit=256",
               "--log-driver=none", "--user", f"{os.getuid() or 65532}:{os.getgid() or 65532}",
               f"--memory={args.memory_mib}m", f"--memory-swap={args.memory_mib}m",
               f"--cpus={args.cpus}",
               "--tmpfs=/tmp:rw,noexec,nosuid,nodev,size=768m"]
    if args.command in ("check", "run", "server"):
        if not args.input_dir:
            raise ValueError("Select --input-dir explicitly; do not mount a whole home/checkout")
        path = Path(args.input_dir).expanduser().absolute()
        if any(c in str(path) for c in ",\n\r"):
            raise ValueError("Unsafe Docker mount delimiter")
        if path.resolve() != path or not path.is_dir():
            raise ValueError("Input must be a canonical directory, not a symlink")
        if path == Path.home().resolve() or path == Path("/"):
            raise ValueError("Refusing a home/root mount; select a narrow proof workspace")
        if args.command != "server":
            if (not args.file or Path(args.file).name != args.file
                    or not args.file.endswith(".lean") or args.file.startswith("-")):
                raise ValueError("Choose one .lean filename directly within --input-dir")
            selected = path / args.file
            if selected.is_symlink() or not selected.is_file():
                raise ValueError("Expected a regular Lean source file")
        command += ["--mount", f"type=bind,src={path},dst=/input,readonly"]
    if args.command == "server":
        command.append("-i")
    command += ["--entrypoint=/usr/bin/timeout", identity, "--signal=TERM",
                "--kill-after=2", str(args.timeout_seconds),
                "/usr/local/bin/lean-container", args.command]
    if args.command in ("check", "run"):
        command.append("/input/" + args.file)
    return command


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--image", default=DEFAULT_IMAGE)
    parser.add_argument("--input-dir")
    parser.add_argument("--timeout-seconds", type=int, default=120)
    parser.add_argument("--memory-mib", type=int, default=4096)
    parser.add_argument("--cpus", type=int, default=2)
    parser.add_argument("command", choices=["version", "metadata", "check", "run", "server"])
    parser.add_argument("file", nargs="?")
    args = parser.parse_args(argv)
    name = "claw-lean-feedback-" + uuid.uuid4().hex
    child = None
    handlers = {}

    def interrupted(signum, _frame):
        raise KeyboardInterrupt

    try:
        identity, platform = resolve_image(args.image)
        command = docker_args(args, identity, platform, name)
        for sig in (signal.SIGINT, signal.SIGTERM):
            handlers[sig] = signal.signal(sig, interrupted)
        # Inherit streams unchanged: no TTY, no stdout logging, LSP framing intact.
        child = subprocess.Popen(command)
        return child.wait(timeout=args.timeout_seconds + 15)
    except subprocess.TimeoutExpired:
        print("Lean feedback deadline exceeded", file=sys.stderr)
        return 124
    except KeyboardInterrupt:
        return 130
    except (ValueError, OSError, subprocess.SubprocessError) as error:
        print(f"Lean feedback unavailable: {error}", file=sys.stderr)
        return 2
    finally:
        for sig, handler in handlers.items():
            signal.signal(sig, signal.SIG_IGN)
        if child is not None:
            try:
                subprocess.run(["docker", "rm", "-f", name], stdout=subprocess.DEVNULL,
                               stderr=subprocess.DEVNULL, timeout=10, check=False)
            except (OSError, subprocess.TimeoutExpired):
                print("Container cleanup failed; inspect " + name, file=sys.stderr)
            if child.poll() is None:
                child.kill()
            child.wait()
        for sig, handler in handlers.items():
            signal.signal(sig, handler)


if __name__ == "__main__":
    sys.exit(main())
