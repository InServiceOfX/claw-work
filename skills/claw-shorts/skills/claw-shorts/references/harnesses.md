# Harness integration for the short-form skills

Two skills cover short-form work: `short-form-strategy` (planning) and
`claw-shorts` (production). Install both together.

## Install

From a claw-work checkout:

```sh
skills/install-shortform-skills.sh                  # all harnesses below
skills/install-shortform-skills.sh claude           # only Claude Code
skills/install-shortform-skills.sh claude hermes    # a subset
```

The script replaces an existing copy of these two skills, and removes the
copies of the skills they replaced (`short-form-viral-content`,
`short-form-video-assembly`, `canvas-explainer`, `transcript-video-edit`) from
the same harness directories, so old names stop triggering. It touches nothing
else. Restart clients or reload skills afterwards (Pi: `/reload`).

| Harness | Skill root | Discovery |
|---------|------------|-----------|
| Claude Code | `~/.claude/skills/<name>/` | `/claw-shorts`, `/short-form-strategy`, or natural language |
| Codex, Pi | `~/.agents/skills/<name>/` | `$claw-shorts` / `/skill:claw-shorts`, or natural language |
| Grok Build | `~/.grok/skills/<name>/` | `/claw-shorts` |
| Hermes Agent | `~/.hermes/skills/media/<name>/` | `skills_list`, then `skill_view claw-shorts` |

Override a location with `CLAUDE_SKILLS_DIR`, `AGENTS_SKILLS_DIR`,
`GROK_SKILLS_DIR` or `HERMES_SKILLS_DIR`.

The installed copy holds only the skill text. The tools it drives
(`Rust/claw-shorts`, `Rust/shortform-video`, `Rust/claw-edit`, the canvas
renderer and the Python templates) stay in the claw-work checkout.
