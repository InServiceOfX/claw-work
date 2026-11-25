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

The installer appends the checkout's local path to each installed `SKILL.md`, so
an agent started elsewhere can find the tools. Re-run it if the checkout moves.
Skip `grok` when `~/.grok/config.toml` lists `~/.agents/skills` in `skills.paths`.

The installed copy holds only the skill text. The tools it drives
(`Rust/claw-shorts`, `Rust/shortform-video`, `Rust/claw-edit`, the canvas
renderer and the Python templates) stay in the claw-work checkout.

## Check an install

Verified on 2026-11-14 with these exact commands:

| Harness | Check | Notes |
|---------|-------|-------|
| Claude Code | skills listing in a new session shows `claw-shorts`, `short-form-strategy` | |
| Codex | start a new session; `$claw-shorts` | Codex 0.167 reads both `~/.codex/skills` and `~/.agents/skills`; install only to `~/.agents/skills` to avoid duplicates |
| Pi | start a new session or `/reload`; `/skill:claw-shorts` | Pi's package manager always loads `~/.agents/skills` as trusted user skills; no entry in `~/.pi/agent/settings.json` is needed |
| Grok Build | `grok inspect` lists both as `user` skills | |
| Hermes | `hermes skills list` shows both under `media`, enabled | Hermes caches skill prompts in `~/.hermes/.skills_prompt_snapshot.json`; the cache is keyed by file signatures and rebuilds on the next session by itself |

## On another machine (instructions for any agent)

1. In a claw-work checkout that contains `skills/install-shortform-skills.sh`,
   run `skills/install-shortform-skills.sh --dry-run` and read what it would
   remove or copy.
2. Run it without `--dry-run`, optionally naming harnesses
   (`claude agents grok hermes`).
3. Run the checks above. Old names (`short-form-viral-content`,
   `short-form-video-assembly`, `canvas-explainer`, `transcript-video-edit`)
   must be gone; if a harness keeps its own copy elsewhere, remove that copy
   by hand and say so.
4. Build the tools the skills drive only when they are used:
   `Rust/claw-shorts` and `Rust/shortform-video` (`cargo build --release`).
