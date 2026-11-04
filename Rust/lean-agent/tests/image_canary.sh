#!/usr/bin/env bash
set -euo pipefail
root=$(cd "$(dirname "$0")/.." && pwd)
# The checker rejects OS symlink aliases (notably /var on macOS).
work=$(python3 -c 'import tempfile,pathlib; print(pathlib.Path(tempfile.mkdtemp()).resolve())')
trap 'rm -rf "$work"' EXIT
mkdir "$work/context" "$work/context/docker"
for file in .dockerignore Dockerfile Dockerfile.source docker/entrypoint.sh docker/pins.json docker/pins-arm64.json docker/primitives.txt; do
 cp "$root/$file" "$work/context/$file"
done
marker=claw-synthetic-image-exclusion-marker-v1
printf '%s\n' "$marker" > "$work/context/.env"
printf '%s\n' "$marker" > "$work/context/docker/private-canary.env"
cd "$root"
cargo build --release --locked --bin claw-public-check
target/release/claw-public-check context "$work/context"
case "$(docker info --format '{{.Architecture}}')" in
 aarch64|arm64) platform=linux/arm64 ;;
 x86_64|amd64) platform=linux/amd64 ;;
 *) echo 'Unsupported canary platform' >&2; exit 2 ;;
esac
docker build --progress=plain --platform="$platform" -t claw-work/lean-agent:privacy-canary-test "$work/context" >"$work/build.log" 2>&1
# Scan every saved layer/config blob, including gzip/zstd layers, not only the
# merged final filesystem. Never pass a real secret as a command-line canary.
# Standard-library deadline supervision also works on macOS without GNU timeout.
python3 - "$marker" <<'PY'
import subprocess, sys, time
export = subprocess.Popen(['docker', 'image', 'save', 'claw-work/lean-agent:privacy-canary-test'], stdout=subprocess.PIPE)
scan = None
try:
    scan = subprocess.Popen(['target/release/claw-public-check', 'artifact-canary', sys.argv[1]], stdin=export.stdout)
    export.stdout.close()
    deadline = time.monotonic() + 360
    if scan.wait(timeout=max(1, deadline-time.monotonic())) != 0:
        raise RuntimeError('Image canary scan failed closed')
    if export.wait(timeout=max(1, deadline-time.monotonic())) != 0:
        raise RuntimeError('Image export failed closed')
finally:
    for child in (scan, export):
        if child is not None and child.poll() is None:
            child.kill()
            child.wait()
PY
echo "Image archive/config/layer canary check passed."
