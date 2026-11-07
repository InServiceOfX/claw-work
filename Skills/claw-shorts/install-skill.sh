#!/bin/bash
# Install claw-shorts skill to standard harness locations.
# Run from Skills/claw-shorts/ directory.

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SKILL_SRC="$SCRIPT_DIR/skills/claw-shorts"

if [ ! -d "$SKILL_SRC" ]; then
    echo "Error: Skill source not found at $SKILL_SRC" >&2
    exit 1
fi

install_skill() {
    local target="$1"
    local target_dir="$(dirname "$target")"
    
    if [ -d "$target" ]; then
        echo "Skill already exists at $target (skipped)"
        return 0
    fi
    
    mkdir -p "$target_dir"
    cp -r "$SKILL_SRC" "$target"
    echo "Installed to $target"
}

echo "Installing claw-shorts skill..."

# Claude Code
install_skill "$HOME/.claude/skills/claw-shorts"

# Codex, Grok Build, Pi (shared location)
install_skill "$HOME/.agents/skills/claw-shorts"

# Hermes Agent
install_skill "$HOME/.hermes/skills/claw-shorts"

echo ""
echo "Installation complete. Restart clients or reload skills."
echo "Dependencies: FFmpeg (required), Python 3.9+ (required)"
echo "Optional: Pillow, Whisper, Playwright"
