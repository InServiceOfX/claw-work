# Portable agent use

The implementation is a Rust CLI plus a standard SKILL.md and shell/LSP interfaces,
not a provider SDK or mandatory MCP daemon. No toolkit command contacts an AI provider.

- **Claude Code, Codex, pi, Hermes, Grok and other shell-capable agents:** explicitly
  read `skills/lean-review/SKILL.md` and follow it using the same CLI.
- Native skill discovery and installation paths differ. No loader-specific registration,
  global symlink or user configuration has been installed by this project.
- Editor/LSP clients can attach to `docker run -i … IMAGE server`. Use framed JSON-RPC,
  not a terminal/TTY. See `tests/lsp_live.rs` for a tested initialize/open/change/goal session.
- There is no newly implemented MCP adapter and no claim that every named harness's
  native skill discovery was tested. CLI, Docker run/exec and actual LSP behavior were
  tested independently of an agent vendor.
- If a harness would upload private input, obtain permission and configure its own
  privacy/provider policy first. Offline containers do not constrain external AI traffic.
