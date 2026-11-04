#!/usr/bin/env bash
set -euo pipefail
root=$(cd "$(dirname "$0")/.." && pwd)
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
mkdir "$work/context" "$work/context/docker"
for file in .dockerignore Dockerfile Dockerfile.source docker/entrypoint.sh docker/pins.json docker/primitives.txt; do
 cp "$root/$file" "$work/context/$file"
done
marker=claw-synthetic-image-exclusion-marker-v1
printf '%s\n' "$marker" > "$work/context/.env"
printf '%s\n' "$marker" > "$work/context/docker/private-canary.env"
cd "$root"
cargo build --release --locked --bin claw-public-check
target/release/claw-public-check context "$work/context"
docker build --progress=plain --platform=linux/amd64 -t claw-work/lean-agent:privacy-canary-test "$work/context" >"$work/build.log" 2>&1
# Scan every saved layer/config blob, including gzip/zstd layers, not only the
# merged final filesystem. Never pass a real secret as a command-line canary.
timeout --kill-after=10 360 docker image save claw-work/lean-agent:privacy-canary-test |
 timeout --kill-after=10 360 target/release/claw-public-check artifact-canary "$marker"
echo "Image archive/config/layer canary check passed."
