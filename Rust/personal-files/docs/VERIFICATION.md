# Verification — first Ubuntu Rust iteration

The maintained app uses Rust 2021 with gtk-rs GTK 3. On this Ubuntu machine:
Rust 1.97.1, GTK runtime 3.24; Cargo.lock pins dependencies. Build artifacts are
outside the repository. The installed application requires no Python process.

## Behavioral evidence

- `cargo test --locked`: **15 acceptance tests passed**. They check path
  resolution, symlink aliases, Unicode/spaces, hidden and broken-link entries,
  missing/permission failures, all sort orders with folders first, actual POSIX
  shell round-trip quoting without command execution, restart persistence,
  0600 state, independent Recent/Frequent/Favorites, bounded history, corrupt
  and invalid state recovery, disconnected favorites, automatic themes and
  atomic-save failure preserving previous bytes.
- Explicit opt-in `cargo test --locked desktop_story -- --ignored --nocapture`:
  **1 real GTK test passed**, containing **15 interaction scenarios**: startup
  and hidden files; absolute/relative address-entry signals; star/sidebar
  activation; real desktop clipboard; context-menu copying; column sorting;
  row activation/file-path selection; failed navigation; back/forward/refresh;
  drag URI payload and favorite handler; skins and image validation; actual
  folder monitoring; stale scan rejection; fresh-window restart; clearing
  visits while retaining favorites. Fixtures and preferences are temporary.

The desktop test is deliberately ignored in ordinary cargo test and was run
separately on the real Ubuntu display. URI/drop handler coverage does not prove
every external file manager's physical mouse-driven drag behavior. Default
document opening is delegated to GIO and inspected; tests do not open real user
documents or assert all installed external applications work.

A window-only fixture screenshot was inspected locally; screenshots and test
logs are not committed. The installed application is intended for interactive
feedback and further iteration.

## Build and installation checks

`cargo fmt --check`, `cargo clippy --locked --all-targets -- -D warnings`,
Bash syntax checks, eight prompt/output mappings and three story links passed.
The installed Desktop Entry passed desktop-file-validate. The release binary is
a stripped native ELF linked only to installed system libraries (no /tmp or
Python dependency). Its hash matches the built release. The installed app was
opened at the requested Physics folder; the live process and saved successful
visit were confirmed. Build files are in the user cache, outside the repo.

## PDD evidence and limits

The exact original request was planned read-only, reviewed by the human and
approved. `intent apply` captured its immutable event/Product Intent and ran
architecture step 1, then was explicitly stopped during step 2 of 19 to bound
the provider workflow. Its local status was not marked as successfully applied.
Provider usage/cost at interruption was not reported and is not invented here.

The subsequent Rust correction was planned in the Rust project scope. The
agent authored and reviewed the final versioned prompts, architecture mapping,
human story and Rust port directly. PDD story inventory discovered the story and all three linked prompts. Its
Python-oriented regression inventory reports `missing`; the Rust behavioral
suite above was run explicitly and is not claimed as a PDD-emitted regression.

No machine-generated story contract, provider-backed semantic story detection,
or automated PDD sync was completed. Those layers are not counted as passing.
The temporary Python reference passed its 26 checks before the port; Rust's
own evidence above, rather than Python's results, establishes Rust parity.

## Tabs update — 2026-10-16

Approved review and immutable PDD intake are retained in docs/intents/. The
tabs apply attempt recorded intent but failed incremental architecture with
`No models available after initial filtering`; the receipt is preserved in the
local user cache. No automatic apply/sync or generated contract pass is claimed.
The agent maintained the tab prompts, story and Rust implementation directly.

Final validation: formatting, Clippy with warnings denied, Bash syntax and
Desktop Entry validation passed. All **18 deterministic Rust tests passed**,
including three new session/legacy-preference tests. The explicit GTK desktop
story passed its original 15 groups plus four tab groups: keyboard/independent
state and visits; context/plus/close controls and last-tab protection; rapid
switching and ordered/active session restoration; missing restored folders.
Pending invalid input is checked not to replace a saved successful folder.
These fixture tests do not modify actual user preferences.

A live X11 screenshot attempt hit a Cairo assertion. The optional test
screenshot renderer was changed to an owned offscreen raster surface; the
combined GTK story then passed with that renderer as well. This is test
harness behavior, not a screenshot feature of the installed app.

The release is installed through an atomic executable replacement so running
older copies can finish normally. Existing windows need to close and reopen
to load tabs. Tab folder paths, order, sort and active index persist; in-memory
selection/navigation stacks are intentionally not restored across restarts.

## Sidebar mouse activation regression — 2026-10-16

The original sidebar test invoked the row action directly. A replacement using
ListBox row-activated failed before the fix (previous folder remained visible)
and passed afterward. The additional temporary Downloads fixture verifies
current tab and contents, path entry, preserved background-tab location and
repeated activation refreshing new files without inflating visits. This tests
GTK's mouse-activation signal dispatch, not an external pointer driver.
All 18 deterministic checks and the combined GTK story pass. See the regression
intake record for cause and the corrected test boundary.

## Local Ubuntu rebuild — 2026-11-08

Built and installed from this claw-work subproject at revision 95556d2. The
installed stripped native ELF and release build have identical SHA-256 digests;
all dynamic dependencies resolve. Desktop Entry validation passes. GIO reports
Personal Files as visible and returns its Desktop Entry for a Personal Files
search. Launching through that entry opened the installed executable and
initialized a disposable preferences file. Only that fixture process was closed;
existing user windows and preferences were preserved. This proves application
registration/search and launch, not a physical Super-key interaction.

All 18 existing deterministic acceptance tests pass. The explicitly enabled
GTK desktop story passes its existing browsing/sidebar/tab scenario groups.
The three new story files are proposed acceptance intent, not proof of new
Trash/delete or command-panel functionality. Those features have not yet been
implemented. The read-only PDD plan succeeded; meaning approval is pending and
no new provider-backed apply, sync or generated contract was run.

## Approved Trash/Delete and compact commands — 2026-11-08

User approval: "approve please", following the three-story review. All three
new human stories are approved; their prompt associations and ten prompt/output
mappings/dependencies were checked. Public intent copies redact machine paths;
the original exact request and approval digest remain in private local state.

### Executable behavior

- 27 deterministic acceptance tests pass: the original 18 plus nine new action
  tests. New coverage includes deleting nonempty folders, preserving symlink
  targets, broken links, partial failures, root/relative rejection, injected
  trash failure without permanent-delete fallback, actual GIO trash entries
  and .trashinfo metadata in an isolated XDG directory, exact Unicode/quoted
  cwd, shell pipelines, stdout/stderr/exit status, concurrent bounded capture,
  closed stdin, missing cwd, process-group cancellation, and cancellation when
  the owning test process drops its task then exits immediately.
- The explicit real GTK desktop_story passes all earlier browsing/sidebar/tab
  scenarios plus context-menu selection, Cancel/Accept, nonempty-folder and
  symlink deletion, listing refresh, persistent partial-error dialog, actual
  Move to Trash menu activation, initially hidden compact panel, native GTK
  accelerator activation with command-entry focus, toggle/Enter/Run/Stop,
  exact submitted cwd across tab changes, responsive navigation, invalid UTF-8,
  output truncation, preference non-persistence and window-close cancellation.
  The GTK story launches a child with isolated XDG data/config directories
  before GLib initializes, so real user Trash/preferences are untouched.
- The panel's rendered fixture screenshot was inspected. Synthetic GTK signals
  and accelerator activation are verified; physical mouse/keyboard interaction
  is not claimed. The command runner is deliberately noninteractive.

### Build and installation

Formatting, strict Clippy across all targets, Bash syntax and git diff checks
pass. Rebuilt and installed the current Ubuntu release on the feature branch.
The installed executable matches its release build (SHA-256
`c4927dcf09aa61111f00645f7b0530f743db9ff669338f35b9ec3fd15c4191aa`).
Dynamic dependencies resolve; --help succeeds; the Desktop Entry validates,
is visible, and matches the Personal Files application search. Launching the
entry initializes a native window with disposable preferences. Only the owned
fixture instance was closed. Existing windows/preferences were preserved;
close and reopen an older window to load the update. An actual Super-key
interaction was not performed.

### PDD limits

The exact approved apply recorded the durable event, then failed on the local
model request without a provider fallback. Prompts, independent stories, code
and tests were maintained directly. No automated sync, generated machine story
contract or semantic story detection is claimed. Deterministic contract lint
finds zero structural errors; the two new module prompts and all stories have
zero warnings. The older core/UI prompts retain 26 wording warnings (12/14),
and the CLI returns 1 for that warning-bearing inventory. It is not recorded
as a clean lint pass. The feature is committed on
feat/personal-files-trash-commands and is not merged to master.
