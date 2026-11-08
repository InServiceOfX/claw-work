# Personal Files — progress

## Completed

- Approved Ubuntu/Finder-inspired user story and review card; explicit Rust
  technology correction accepted without changing approved behavior.
- Native Rust GTK file browser, persistent Favorites/Recent/Frequent, visible
  path entry, hidden files, Name/Modified/Size sorting, path clipboard actions.
- Star/context-menu/drag favorites, navigation/shortcuts, folder monitoring,
  background worker with stale-result rejection, skins and local image picker.
- Cargo package/lockfile, PDD prompts/mapping, independent story/acceptance tests,
  build, launch and per-user installation scripts.
- 15 deterministic tests and real GTK desktop story (15 scenarios) passed.
- Temporary Python reference retired after passing Rust acceptance checks.
- Release installed in the Ubuntu application menu and ~/.local/bin; launched
  successfully at the requested Physics directory. Formatting, Clippy, launcher
  validation and prompt-graph checks passed.

## Remaining scope

- Interactive user feedback and iterative improvements.
- PDD automated architecture/apply was interrupted, not completed. Generated
  machine contract, semantic alignment and automated sync remain unverified;
  see docs/VERIFICATION.md. Do not claim automated PDD success.
- Physical drag interoperability with other file managers is not automated.
- macOS is explicitly deferred to another computer/project.

## Last worked on

2026-10-15 — First usable Rust iteration on `feat/personal-files`. See README
for launch/install and docs/VERIFICATION.md for evidence and limits.

## Tabs update — 2026-10-16

Branch: feat/personal-files-tabs. Approved tab story implemented in Rust.
Visible scrollable tab strip, plus/Ctrl+T, folder/sidebar context opening,
independent selection/sorting/history, close/cycle controls and restart
restoration. Missing folders and old preferences handled without losing
favorites. One worker and one active folder monitor remain in use.

Validation: 18 Rust acceptance tests plus the real GTK desktop story with
original browsing cases and four additional tab scenario groups. See
docs/VERIFICATION.md for final results. Installer replaces the executable
atomically so an old running version can close normally; restart to upgrade.

PDD tabs intake captured; incremental architecture failed due to unavailable
model provider. Prompts/story/code maintained directly; no automatic PDD
synchronization or generated contract success claimed.

## Sidebar activation fix — 2026-10-16

Connected navigation to ListBox row-activated rather than the row action.
Explicit single-click activation; repeating a click refreshes the current
folder without increasing visits. Corrected GTK regression fails before fix
and passes after; Downloads fixture verifies content and active-tab isolation.
Updated prompts/story and installed the release. Restart existing windows.
