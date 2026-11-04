#!/usr/bin/env bash
set -euo pipefail
image="${LEAN_AGENT_IMAGE:-claw-work/lean-agent:lean4.35.0-mathlib}"
work=$(mktemp -d)
name="claw-lean-smoke-$$"
trap 'docker rm -f "$name" >/dev/null 2>&1 || true; rm -rf "$work"' EXIT
chmod 755 "$work"
module=Mathlib.Data.Nat.Basic
# Importing the entire umbrella is a separate high-memory stress test.
if test "${LEAN_AGENT_FULL_IMPORT:-0}" = 1; then module=Mathlib; fi
printf 'import %s\nexample (x y : Nat) : x + y = y + x := by omega\n' "$module" > "$work/Good.lean"
printf 'import %s\nexample : (1 : Nat) = 2 := by rfl\n' "$module" > "$work/Bad.lean"
cat > "$work/Program.lean" <<'LEAN'
def main : IO Unit := IO.println "lean-program-ok"
LEAN
base=(--pull=never --network=none --read-only --cap-drop=ALL
 --security-opt=no-new-privileges --user 12345:12345 --memory=4g --memory-swap=4g --cpus=2
 --pids-limit=256 --tmpfs /tmp:rw,noexec,nosuid,nodev,size=512m
 --mount "type=bind,src=$work,dst=/input,readonly")
docker run --rm "${base[@]}" "$image" version
docker run --rm "${base[@]}" "$image" check /input/Good.lean
if docker run --rm "${base[@]}" "$image" check /input/Bad.lean >"$work/bad.log" 2>&1; then
 echo "ERROR: false property accepted" >&2; exit 1
fi
docker run --rm "${base[@]}" "$image" run /input/Program.lean | grep -qx lean-program-ok
docker run -d --name "$name" "${base[@]}" "$image" idle >/dev/null
test "$(docker exec "$name" id -u)" = 12345
docker exec "$name" lean /input/Good.lean
docker run --rm "${base[@]}" "$image" shell </dev/null
test "$(docker run --rm "${base[@]}" "$image" shell -c 'printf shell-forwarding-ok')" = shell-forwarding-ok
if docker run --rm "${base[@]}" "$image" shell -c 'exit 42'; then
 echo "ERROR: shell command status was ignored" >&2; exit 1
fi
echo "Container smoke checks passed."
