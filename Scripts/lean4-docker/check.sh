#!/usr/bin/env bash
# lean-check: build a Lake project, audit its proofs, optionally regenerate and
# compare emitted code.  Usage (inside the container, cwd = Lake project root):
#   lean-check [--emit "<cmd>"] [--golden <file> --generated <file>] [--theorems "A.b C.d"]
set -euo pipefail
EMIT=""; GOLDEN=""; GENERATED=""; THMS=""
while [ $# -gt 0 ]; do
  case "$1" in
    --emit) EMIT="$2"; shift 2;;
    --golden) GOLDEN="$2"; shift 2;;
    --generated) GENERATED="$2"; shift 2;;
    --theorems) THMS="$2"; shift 2;;
    *) echo "unknown arg $1" >&2; exit 2;;
  esac
done

echo "== toolchain"; cat lean-toolchain 2>/dev/null || true; lean --version
echo "== lake build"; lake build

echo "== escape hatches in sources (each hit must be reviewed by a human)"
if grep -rnE '\b(sorry|admit|native_decide|implemented_by|extern|unsafe|axiom)\b|decide \+native' \
     --include='*.lean' . --exclude-dir=.lake; then
  echo "WARNING: escape hatches found (see above)"
else
  echo "none"
fi

if [ -n "$THMS" ]; then
  echo "== #print axioms (expect only propext, Classical.choice, Quot.sound)"
  mods=$(find . -path ./.lake -prune -o -name '*.lean' -print | grep -vE 'lakefile|/templates/' \
         | sed 's|^\./||; s|\.lean$||; s|/|.|g' | sed 's/^/import /')
  { echo "$mods"; for t in $THMS; do echo "#print axioms $t"; done; } > /tmp/Axioms.lean
  out=$(lake env lean /tmp/Axioms.lean 2>&1) || { echo "$out"; echo "FAIL: axioms file did not elaborate"; exit 1; }
  echo "$out"
  if echo "$out" | grep -E "depends on axioms" | grep -vE "\[(propext|Classical.choice|Quot.sound)(, (propext|Classical.choice|Quot.sound))*\]" | grep -q .; then
    echo "FAIL: non-standard axiom"; exit 1
  fi
  if echo "$out" | grep -q "sorryAx"; then echo "FAIL: sorry"; exit 1; fi
fi

if [ -n "$EMIT" ]; then echo "== emit: $EMIT"; bash -c "$EMIT"; fi
if [ -n "$GOLDEN" ]; then
  echo "== compare $GENERATED vs $GOLDEN"
  cmp "$GENERATED" "$GOLDEN" && echo "MATCH" && sha256sum "$GOLDEN"
fi
echo "== lean-check OK"
