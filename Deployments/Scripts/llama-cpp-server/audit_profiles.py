#!/usr/bin/env python3
"""Audit local YAML mount mappings and GGUF headers without loading weights."""
import argparse
import json
import pathlib
import struct
import yaml


def resolve_model(model, volumes):
    model = pathlib.PurePosixPath(model)
    for mount in sorted(volumes, key=lambda v: len(v["container"]), reverse=True):
        inside = pathlib.PurePosixPath(mount["container"])
        if model.is_relative_to(inside):
            return pathlib.Path(mount["host"]) / model.relative_to(inside)
    return None


def file_status(path):
    if path is None:
        return "unmounted"
    if not path.is_file():
        return "missing"
    with path.open("rb") as file:
        head = file.read(128)
    if head.startswith(b"version https://git-lfs.github.com/spec/v1"):
        return "lfs_pointer"
    if len(head) < 24 or head[:4] != b"GGUF":
        return "not_gguf"
    magic, version, tensors, metadata = struct.unpack("<4sIQQ", head[:24])
    if version not in (2, 3) or not tensors or not metadata or path.stat().st_size < 4096:
        return "incomplete_header"
    return "ready"


def audit(root):
    root = pathlib.Path(root)
    config = yaml.safe_load((root / "config.yml").read_text())
    volumes = config.get("docker", {}).get("volumes", [])
    results = []
    for file in sorted((root / "profiles").glob("*.yml")):
        profile = yaml.safe_load(file.read_text())
        actual = resolve_model(profile.get("model_path", ""), volumes)
        projector = resolve_model(profile["mmproj_path"], volumes) if profile.get("mmproj_path") else None
        documented = profile.get("host_model_path")
        row = {"profile": file.stem, "model_status": file_status(actual),
               "host_path_matches_mount": actual is not None and documented is not None and pathlib.Path(documented).resolve() == actual.resolve(),
               "projector_status": file_status(projector) if profile.get("mmproj_path") else "not_requested",
               "context": profile.get("ctx_size")}
        results.append(row)
    return results

if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", default=str(pathlib.Path(__file__).resolve().parent))
    args = parser.parse_args()
    print(json.dumps(audit(args.root), indent=2))
