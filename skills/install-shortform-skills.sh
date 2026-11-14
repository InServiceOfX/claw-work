#!/usr/bin/env bash
# Install the short-form skills (short-form-strategy, claw-shorts) into local
# agent harnesses, replacing earlier copies of them and of the skills they
# replaced. Touches only those skill names.
#
# Usage: skills/install-shortform-skills.sh [--dry-run] [claude] [agents] [grok] [hermes]
#   No harness names = all four.
# Locations (override with env):
#   CLAUDE_SKILLS_DIR  (default ~/.claude/skills)
#   AGENTS_SKILLS_DIR  (default ~/.agents/skills; Codex, Pi)
#   GROK_SKILLS_DIR    (default ~/.grok/skills)
#   HERMES_SKILLS_DIR  (default ~/.hermes/skills/media)
set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
sources=(
  "short-form-strategy=$here/short-form-strategy"
  "claw-shorts=$here/claw-shorts/skills/claw-shorts"
)
# Skills merged into the two above on 2026-11-14.
retired=(short-form-viral-content short-form-video-assembly canvas-explainer transcript-video-edit)

dry_run=0
targets=()
for arg in "$@"; do
  case "$arg" in
    --dry-run) dry_run=1 ;;
    claude|agents|grok|hermes) targets+=("$arg") ;;
    *) echo "unknown argument: $arg" >&2; exit 2 ;;
  esac
done
[ ${#targets[@]} -eq 0 ] && targets=(claude agents grok hermes)

dir_for() {
  case "$1" in
    claude) echo "${CLAUDE_SKILLS_DIR:-$HOME/.claude/skills}" ;;
    agents) echo "${AGENTS_SKILLS_DIR:-$HOME/.agents/skills}" ;;
    grok)   echo "${GROK_SKILLS_DIR:-$HOME/.grok/skills}" ;;
    hermes) echo "${HERMES_SKILLS_DIR:-$HOME/.hermes/skills/media}" ;;
  esac
}

run() {
  if [ "$dry_run" -eq 1 ]; then echo "would: $*"; else "$@"; fi
}

for entry in "${sources[@]}"; do
  src="${entry#*=}"
  [ -f "$src/SKILL.md" ] || { echo "missing $src/SKILL.md" >&2; exit 1; }
done

for target in "${targets[@]}"; do
  dest_root="$(dir_for "$target")"
  echo "== $target: $dest_root"
  run mkdir -p "$dest_root"
  for name in "${retired[@]}"; do
    if [ -f "$dest_root/$name/SKILL.md" ]; then
      run rm -rf "${dest_root:?}/$name"
      echo "   removed retired $name"
    fi
  done
  for entry in "${sources[@]}"; do
    name="${entry%%=*}"
    src="${entry#*=}"
    dest="$dest_root/$name"
    [ -e "$dest" ] && run rm -rf "${dest:?}"
    run cp -R "$src" "$dest"
    if [ "$dry_run" -eq 0 ]; then
      printf '\n> Installed copy. Canonical source: claw-work `%s` (edit there, then re-run `skills/install-shortform-skills.sh`).\n' \
        "skills/${src#"$here"/}" >>"$dest/SKILL.md"
      find "$dest" -name '__pycache__' -type d -prune -exec rm -rf {} +
    fi
    echo "   installed $name"
  done
done
echo "Restart clients or reload skills."
