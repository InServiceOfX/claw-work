#!/usr/bin/env bash
# install-skill.sh — copy the llama-cpp-macos skill into each AI harness's
# user skill directory, so any Claude Code, Codex, Grok Build, pi or Hermes
# session can follow the setup and connection workflow.
#
# Usage:
#   ./install-skill.sh            # install for every harness found
#   ./install-skill.sh --dry-run  # show what would happen
#
#   ~/.claude/skills   Claude Code           (/llama-cpp-macos)
#   ~/.agents/skills   Codex, Grok Build, pi ($llama-cpp-macos, /skill:llama-cpp-macos)
#   ~/.hermes/skills   Hermes Agent
#
# Copies are self-contained. An identical existing copy is left alone; a
# different one is an error, so local edits are never overwritten — move it
# aside yourself to upgrade. Restart running clients to pick the skill up.

set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
NAME=llama-cpp-macos
SRC="$SCRIPT_DIR/skills/$NAME"
DRY_RUN=0

case "${1:-}" in
    "") ;;
    --dry-run|-n) DRY_RUN=1 ;;
    *) sed -n '2,17p' "${BASH_SOURCE[0]}" | sed 's/^# \{0,1\}//'; exit 2 ;;
esac

[[ -f "$SRC/SKILL.md" ]] || { echo "Error: $SRC/SKILL.md not found" >&2; exit 1; }

roots=()
[[ -d "$HOME/.claude" ]] && roots+=("$HOME/.claude/skills")
if [[ -d "$HOME/.agents" || -d "$HOME/.codex" || -d "$HOME/.grok" || -d "$HOME/.pi" ]]; then
    roots+=("$HOME/.agents/skills")
fi
[[ -d "$HOME/.hermes" ]] && roots+=("$HOME/.hermes/skills")

if [[ ${#roots[@]} -eq 0 ]]; then
    echo "No harness home found (~/.claude, ~/.agents, ~/.codex, ~/.grok, ~/.pi, ~/.hermes)."
    exit 1
fi

status=0
for root in "${roots[@]}"; do
    dest="$root/$NAME"
    if [[ -e "$dest" ]]; then
        if diff -rq "$SRC" "$dest" >/dev/null 2>&1; then
            echo "up to date  $dest"
        else
            echo "DIFFERENT   $dest — not overwritten; move it aside to upgrade" >&2
            status=1
        fi
        continue
    fi
    if [[ "$DRY_RUN" == 1 ]]; then
        echo "would copy  $dest"
    else
        mkdir -p "$root"
        cp -R "$SRC" "$dest"
        echo "installed   $dest"
    fi
done
exit "$status"
