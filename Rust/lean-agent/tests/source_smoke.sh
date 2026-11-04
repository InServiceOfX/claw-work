#!/usr/bin/env bash
set -euo pipefail
image=${1:-claw-work/lean-agent:source-8c6cec8}
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
chmod 755 "$work"
printf 'import Std\nexample (n : Nat) : n + 0 = n := by omega\n' > "$work/Good.lean"
printf 'import Std\nexample : (1 : Nat) = 2 := by rfl\n' > "$work/Bad.lean"
printf 'def main : IO Unit := IO.println "source-profile-ok"\n' > "$work/Program.lean"
args=(--rm --pull=never --network=none --read-only --user 12345:12345 --cap-drop=ALL
 --security-opt=no-new-privileges --memory=2g --memory-swap=2g --cpus=2 --pids-limit=256
 --tmpfs /tmp:rw,noexec,nosuid,nodev,size=256m --mount "type=bind,src=$work,dst=/input,readonly")
docker run "${args[@]}" "$image" lean --version
docker run "${args[@]}" "$image" lake --version
docker run "${args[@]}" "$image" lean /input/Good.lean
if docker run "${args[@]}" "$image" lean /input/Bad.lean > "$work/bad.txt" 2>&1; then
 echo 'Source profile incorrectly accepted a false theorem' >&2; exit 1
fi
docker run "${args[@]}" "$image" lean --run /input/Program.lean | grep -qx source-profile-ok
echo 'Core-only source build smoke checks passed; no stable-mathlib compatibility claim.'
