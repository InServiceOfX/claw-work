# Five harnesses, one tool contract

See [SETUP.md](SETUP.md) for explicit no-overwrite installation, native skill
locations, reload commands and bounded workflow examples.

The integration is a Rust CLI plus two Agent Skills and a constrained Docker/LSP
launcher. All five shell-capable harnesses use the same independent checker:
Claude Code, Codex, Grok Build, Hermes Agent and Pi. No provider SDK, MCP daemon
or always-running model agent is required. No command chooses an AI provider.

- lean-proof: formulate an independent theorem, draft/debug with bounded raw
  feedback, then submit to the strict statement/primitive/axiom/kernel gate.
- lean-review: bounded private source selection, independent mathematical
  requirements, explicit manual source mapping, counterexample/reproducer and
  specification-derived improvement.
- claw-lean-container: offline feedback/program/LSP with fixed Docker restrictions,
  immutable local image selection, deadline and own-container cleanup.
- claw-lean: deterministic inventory, snapshot, doctor and strict JSON checking.

Use framed stdin/stdout for LSP, not a terminal/TTY. See tests/lsp_live.rs for
initialize/open/change/goal coverage. The launcher does not execute caller lakefiles
on the host or permit caller-provided Docker privileges.

Installed bundles resolve their own references. Standard profile discovery can be
checked with local client inspectors/loaders; installation does not prove every
vendor's live model will follow the skill correctly. See LOCAL_SETUP_EVIDENCE.md
for the actual tests performed, not inferred certification.

Offline Docker does not constrain external harness model traffic. Obtain approval
and configure that harness's provider/privacy policy before exposing private input.
No global configuration edits, blanket permission grants or model login are needed.
