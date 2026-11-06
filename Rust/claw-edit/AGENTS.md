# claw-edit

Read the repository AGENTS.md and PRIVACY.md. Use a feature branch.

This is a new conventional Rust utility, outside the explicitly prompt-owned Lean
and public-boundary dev units. No existing architecture maps its outputs. Do not
claim PDD generation or synchronization; do not change those unrelated graphs.

The agent chooses an editorial cut list; this executable validates and renders
it. Keep editorial judgment and transcription outside the executable. Preserve
originals, reject existing output directories, use subprocess argument arrays,
and make no network/model calls. Keep media and reports outside Git.

Check with cargo fmt --check, cargo test --offline --locked, cargo clippy
--offline --locked --all-targets -- -D warnings. Run the ignored FFmpeg test
explicitly for media changes. Use synthetic footage only.
