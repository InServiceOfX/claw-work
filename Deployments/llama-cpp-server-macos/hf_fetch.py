#!/usr/bin/env python3
"""hf_fetch.py — download GGUF weights into the standard HuggingFace hub cache.

Same cache, same layout that `mlx_lm`, `transformers`, and `huggingface-cli`
use, so a model fetched here is visible to every other tool on the machine:

    ~/.cache/huggingface/hub/models--<org>--<repo>/
        blobs/<sha256>                       # real bytes
        snapshots/<revision>/<filename>      # symlink into blobs/
        refs/main                            # revision currently checked out

Invoked through ./fetch-model.sh, which picks the interpreter. Run it directly
only if you already have `huggingface_hub` importable.

Subcommands:
    list     <repo_id>                    List repo files with sizes.
    download <repo_id> <file> [file...]   Download into the hub cache.
    path     <repo_id> [file]             Resolve a cached path, no network.
"""

from __future__ import annotations

import argparse
import os
from pathlib import Path
import sys

# ── Cache location ────────────────────────────────────────────────────────────
# Resolution order matches huggingface_hub's own: HF_HUB_CACHE, then
# HF_HOME/hub, then the platform default. Never hard-code a path here — the
# point of this script is that other tools find the same files.


def cache_root() -> Path:
    env = os.environ.get("HF_HUB_CACHE")
    if env:
        return Path(env).expanduser()
    hf_home = os.environ.get("HF_HOME")
    if hf_home:
        return Path(hf_home).expanduser() / "hub"
    return Path.home() / ".cache" / "huggingface" / "hub"


def repo_dir(repo_id: str) -> Path:
    return cache_root() / ("models--" + repo_id.replace("/", "--"))


def human(n: int | None) -> str:
    if not n:
        return "?"
    for unit in ("B", "KB", "MB", "GB"):
        if n < 1000 or unit == "GB":
            return f"{n:.2f} {unit}" if unit != "B" else f"{n} B"
        n /= 1000.0
    return f"{n} B"


# ── Subcommands ───────────────────────────────────────────────────────────────


def cmd_list(args) -> int:
    from huggingface_hub import HfApi

    info = HfApi().model_info(args.repo_id, revision=args.revision, files_metadata=True)
    print(f"repo     : {args.repo_id}")
    print(f"revision : {info.sha}")
    print("")
    for sib in sorted(info.siblings or [], key=lambda s: s.rfilename):
        print(f"  {human(sib.size):>10}  {sib.rfilename}")
    return 0


def cmd_download(args) -> int:
    from huggingface_hub import hf_hub_download

    resolved: list[tuple[str, Path]] = []
    for filename in args.files:
        print(f"[hf_fetch] downloading {args.repo_id}/{filename} ...", file=sys.stderr)
        path = hf_hub_download(
            repo_id=args.repo_id,
            filename=filename,
            revision=args.revision,
            cache_dir=str(cache_root()),
        )
        resolved.append((filename, Path(path)))

    print("")
    print(f"Cached under: {repo_dir(args.repo_id)}")
    print("")
    for filename, path in resolved:
        size = path.stat().st_size if path.exists() else None
        print(f"  {human(size):>10}  {path}")

    # Profiles should reference the repo + file, not the resolved snapshot path:
    # launch.sh re-resolves it, so the config survives a model revision bump.
    print("")
    print("Profile keys (profiles/<name>.yml):")
    print("")
    print(f"  hf_repo: {args.repo_id}")
    print(f"  hf_file: {resolved[0][0]}")
    return 0


def cmd_path(args) -> int:
    root = repo_dir(args.repo_id)
    if not root.is_dir():
        print(f"Not in cache: {args.repo_id} (looked in {root})", file=sys.stderr)
        return 1

    revision = args.revision
    if not revision:
        ref_main = root / "refs" / "main"
        if ref_main.is_file():
            revision = ref_main.read_text(encoding="utf-8").strip()

    snapshots = root / "snapshots"
    candidates = []
    if revision and (snapshots / revision).is_dir():
        candidates.append(snapshots / revision)
    if snapshots.is_dir():
        candidates.extend(
            sorted(
                (p for p in snapshots.iterdir() if p.is_dir() and p not in candidates),
                key=lambda p: p.stat().st_mtime,
                reverse=True,
            )
        )

    for snap in candidates:
        if args.file is None:
            print(snap)
            return 0
        target = snap / args.file
        if target.exists():
            # The snapshot path, not target.resolve() — the resolved form is an
            # opaque blobs/<sha256> name that means nothing to a reader.
            print(target)
            return 0

    what = args.file or "<any snapshot>"
    print(f"Not in cache: {args.repo_id}/{what}", file=sys.stderr)
    return 1


def main() -> int:
    parser = argparse.ArgumentParser(prog="hf_fetch.py", description=__doc__)
    parser.add_argument(
        "--revision",
        default=None,
        help="Branch, tag, or commit SHA. Default: main.",
    )
    sub = parser.add_subparsers(dest="cmd", required=True)

    p_list = sub.add_parser("list", help="List repo files with sizes")
    p_list.add_argument("repo_id")
    p_list.set_defaults(func=cmd_list)

    p_dl = sub.add_parser("download", help="Download files into the hub cache")
    p_dl.add_argument("repo_id")
    p_dl.add_argument("files", nargs="+")
    p_dl.set_defaults(func=cmd_download)

    p_path = sub.add_parser("path", help="Resolve a cached path without network")
    p_path.add_argument("repo_id")
    p_path.add_argument("file", nargs="?", default=None)
    p_path.set_defaults(func=cmd_path)

    args = parser.parse_args()
    return args.func(args)


if __name__ == "__main__":
    sys.exit(main())
