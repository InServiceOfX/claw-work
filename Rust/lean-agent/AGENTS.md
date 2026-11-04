# Lean agent toolkit

Read ../../AGENTS.md and ../../PRIVACY.md. This subproject is PDD-owned by the local
.prompt suite, .pddrc and architecture.json. Story meaning is independent of code.

- Default image includes pinned mathlib and its compatible Lean release, offline.
- Never describe model proofs as automatic verification of arbitrary source.
- Use the upstream exporter/comparator and independent kernel replay. No untrusted
  .olean may enter the judging process. Trust is explicit in reports.
- Keep private source/model/report/manifest/snapshots outside Git checkouts.
- Docker sources and Lean metaprograms are untrusted. Never run input lakefiles on
  the host or give them networking, home/credentials/socket mounts or privileges.
- Do not change an independent challenge statement to make a solution pass.
- Do not replace valid accumulated tests or hand-edit generated story contracts.
- Keep toolchain/dependency locks and licenses; no startup fetch or rebuild.
- No unsupported performance, architecture, source-correspondence or PDD-sync claims.

Tests: cargo fmt --check; cargo clippy --all-targets -- -D warnings; cargo test.
Live container tests are opt-in via LEAN_AGENT_LIVE=1 after the pinned image exists.
Read PROGRESS.md for actual completion evidence, including PDD backend limits.
