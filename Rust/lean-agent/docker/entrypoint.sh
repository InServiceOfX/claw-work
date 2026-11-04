#!/bin/sh
set -eu
# Only trusted image configuration is read here, never a caller's lakefile.
deps=$(cat /opt/mathlib-path 2>/dev/null || true)
export LEAN_PATH="/tmp/lean-work:$deps"
mkdir -p /tmp/lean-work
case "${1:-version}" in
 version) lean --version; lake --version ;;
 metadata) exec cat /opt/lean-agent-pins.json ;;
 check) shift; exec lean "$@" ;;
 run) shift; exec lean --run "$@" ;;
 shell) shift; exec /bin/sh "$@" ;;
 idle) exec sleep infinity ;;
 server) shift; exec lean --server "$@" ;;
 export)
   case "${2:-}" in Challenge|Solution) module=$2 ;; *) echo "Invalid fixed module name" >&2; exit 2 ;; esac
   cp "/input/$module.lean" "/tmp/lean-work/$module.lean"
   cd /tmp/lean-work
   lean -o "$module.olean" "$module.lean" >&2
   shift 2
   test "$#" -gt 0 || { echo "Expected theorem names are required" >&2; exit 2; }
   # Match v4.35.0 Lake.Check primitiveTargets/builtinTargets as well as the
   # expected theorem roots; omitting primitives is not a sound comparison.
   exec leanexport "$module" -- "$@" $(cat /opt/lean-agent-primitives.txt)
   ;;
 judge)
   cd /tmp
   # Source compilation occurred in other containers. This export-only judge
   # uses the outer Docker boundary instead of unavailable nested user namespaces.
   lake comparator --inadvisably-no-sandbox --config=/opt/primitive-guard.json \
     --challenge-from-export=/opt/primitive-baseline.ndjson \
     --solution-from-export=/input/challenge.ndjson
   exec lake comparator --inadvisably-no-sandbox --config=/input/comparator.json \
     --challenge-from-export=/input/challenge.ndjson \
     --solution-from-export=/input/solution.ndjson
   ;;
 *) echo "Use version, check, run, shell, idle, server, export, or judge" >&2; exit 2 ;;
esac
