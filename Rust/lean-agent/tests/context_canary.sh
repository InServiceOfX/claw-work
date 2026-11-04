#!/usr/bin/env bash
set -euo pipefail
root=$(cd "$(dirname "$0")/.." && pwd)
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
mkdir "$work/context" "$work/context/docker"
for file in .dockerignore Dockerfile Dockerfile.source docker/entrypoint.sh docker/pins.json docker/pins-arm64.json docker/primitives.txt; do
 cp "$root/$file" "$work/context/$file"
done
printf 'synthetic-exclusion-canary\n' > "$work/context/.env"
printf 'synthetic-exclusion-canary\n' > "$work/context/docker/private-canary.env"
docker build --progress=plain -f - --output="type=local,dest=$work/export" "$work/context" >"$work/build.log" 2>&1 <<'DOCKER'
FROM scratch
COPY . /context
DOCKER
if test -e "$work/export/context/.env" || test -e "$work/export/context/docker/private-canary.env"; then
 echo "ERROR: private canary entered actual Docker context" >&2; exit 1
fi
test "$(find "$work/export/context" -type f | wc -l)" -eq 7
echo "Actual Docker context canary check passed."
