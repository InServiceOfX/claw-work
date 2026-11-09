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

## Repository rebuild and story intake — 2026-11-08

Rebuilt the current claw-work source for Ubuntu and installed the release using
this subproject's scripts. Installed/build hashes match. The Desktop Entry is
valid, visible in Applications and returned by desktop application search;
launching that entry initialized the installed browser with temporary preferences.
An actual Super-key interaction was not performed. Existing windows were preserved.
All 18 deterministic tests and the explicit GTK desktop story passed.

Three requested independent stories were drafted under user_stories/: Ubuntu
installation/discovery, Trash/permanent deletion, and a toggleable compact
command runner. New behavior remains proposed pending the PDD meaning review.
The read-only exact-request intent plan succeeded; no new intent apply,
automated synchronization or generated story contract is claimed.

## Approved Trash/Delete and compact commands — 2026-11-08

The user approved the three-story review. Implemented explicit context-menu
Trash and confirmed permanent deletion with symlink-safe behavior, worker
execution, listing refresh and persistent partial-failure dialogs. Empty-space
menus do not offer to delete the current folder. Added initially hidden bottom
Commands panel, Commands/Ctrl+grave toggle, Enter/Run, selectable output and exit
status, Stop and window-close cancellation. Commands use the submitted active
folder, retain bounded output and do not freeze navigation or persist history.

Added two prompt-owned modules for file operations and shell commands; updated
UI/library/appearance prompts, architecture dependencies, Product Intent and
three independent stories. The exact approved PDD apply recorded its intent
but failed at the local model request. Artifacts were maintained directly;
automatic sync, generated contracts and semantic detection are not claimed.
See docs/VERIFICATION.md for final verification and installation evidence.

Final verification: 27 deterministic tests plus the explicit isolated GTK story
pass; fmt, strict Clippy, Bash syntax, prompt graph and Desktop Entry pass.
Installed release c4927dcf matches the build; desktop search and actual isolated
launcher startup pass. Three new human stories are approved. Contract lint has
zero errors but 26 older core/UI wording warnings; no clean lint or automatic
PDD sync claim. Committed on feat/personal-files-trash-commands. Not merged to master.
Restart an already open window to load this release.
