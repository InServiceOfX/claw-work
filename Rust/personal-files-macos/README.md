# Personal Files — macOS v0.2.0

Native **Rust + AppKit** file browser, porting the browsing-focused Ubuntu app
that now lives in the public claw-work repo at `Rust/personal-files`. No GTK, web server, Electron, Python application runtime,
telemetry, or Finder replacement.

## Launch

Installed for this user at `~/Applications/Personal Files.app`:

```bash
open "$HOME/Applications/Personal Files.app"
```

From this directory, build and run at an optional folder/file path:

```bash
./launch.sh
./launch.sh "/path/to/a/folder"
```

The launcher rebuilds first so source changes cannot silently launch a stale
binary. Build output and a launchable app are under
`~/Library/Caches/personal-files-macos/`, never committed to the repository.
`./install.sh` installs into `~/Applications` without sudo. It **refuses an
existing destination**, stages a complete bundle and publishes it atomically
without overwriting a concurrent install. Move an old install aside deliberately
before reinstalling. A failed staging operation retains its temporary directory
for inspection; it does not alter an installed app.

## Tabs and native window tiling

- **⌘T / File > New Tab** opens a native AppKit tab at the current folder.
  Click tabs to switch. Each keeps its own folder, history, selection and path
  draft and sorting; Favorites, appearance and visits remain shared.
- **⌘W** closes the current tab; the last tab closes the window.
  **Control–Tab / Control–Shift–Tab** selects next/previous tabs. Add/close
  buttons are also available. Tabs share one scan worker and reject stale scans.
- **Window** is registered with AppKit so macOS supplies **Fill**, **Center**,
  **Move & Resize**, Restore and their native shortcuts: **Fn–Control–F/C/R**
  and **Fn–Control–arrow keys**, where supported by the OS. The layout adapts
  down to 480×280 content size; desktop Fill does not enter full screen.
- These are in-window NSTabView folder tabs, not detachable NSWindow tab groups.
  Reopening without a startup PATH restores tab order, folders, active tab and
  each tab's sorting. A startup PATH opens one tab. Missing folders fall back to
  Home independently. Selection, drafts and back/forward history are not saved.

## Browse

- Always-visible path bar: absolute paths, `~/…`, and relative paths; Return
  navigates. A file path opens its parent and selects the file. Symlink aliases,
  spaces and Unicode are retained. Hidden files are always shown.
- Sidebar: Home, available Desktop/Downloads, Computer, Favorites, Recent (7)
  and Frequent (5), with full-path tooltips. One click navigates.
- Star / **⌘D** pins or unpins the current folder. Folder context menus and
  dragging local directories into the sidebar add favorites. Sidebar context
  menu removes favorites, including disconnected ones. Drops never move files.
- **Name / Modified / Size** headers sort; click again to reverse. Folders
  always stay first. Folder sizes are “—”, not recursive totals.
- Multiple selection; **Copy Path / ⌘C** copies selected full paths, or the
  current folder when selection is empty. **⇧⌘C** shell-quotes each path.
  Copy inside the path editor stays native text copying.
- Double-click / Return / **⌘O** opens the selection; documents use NSWorkspace
  and the registered Mac application, never shell execution.
- **⌘[ / ⌘]** or Option–Left/Right: back/forward. **⌘↑**: parent.
  **⌘L**: focus/select path. **⌘R / F5**: refresh.
- Appearance menu / ◐: Silver, Midnight, Aurora, Automatic; choose/remove a
  local background image. Invalid images do not replace a working choice.
  Automatic uses extensions only, never document contents.
- Clear Visit History preserves manual favorites. Automatic history is bounded
  to 200 distinct directories. Successful navigation counts as a visit;
  refresh/metadata polling does not. A single scan worker rejects stale results.

This remains a browsing app: no file create/rename/move/delete or thumbnails,
recursive folder sizes or automatic default-file-manager replacement.

## Local state and macOS permissions

Preferences live at:

```text
~/Library/Application Support/Personal Files/preferences.json
```

Favorites, history, last folder, tab folders/order/active index, per-tab sorting,
theme and image are saved atomically
with mode 0600. Corrupt state is preserved as a timestamped backup before
recovery. Unreadable existing preferences block saves until access is fixed and
the app restarts, preserving the original file. Errors appear in the status line.
macOS may ask permission when you
choose protected Desktop, Documents, Downloads or external folders. Full Disk
Access is not automatically requested or granted.

## Build and verify

Requires macOS, Rust/Cargo and Apple Command Line Tools; full Xcode is not
required. The bundle is built for the native host architecture, even when Cargo
has a different default target. `CARGO_TARGET_DIR` is honored.
`PERSONAL_FILES_CACHE` selects a separate cache for isolated verification.
Cached bundles are staged, plist-validated, signed and strictly verified before
replacing the previous build. Publication failures restore the previous build
or report its retained recovery path if a concurrent app prevents restoration.
The build is
ad-hoc signed for local use, **not notarized or Developer ID signed**.

```bash
./build.sh
./verify.sh
```

`verify.sh` runs formatting, Clippy with warnings denied, deterministic tests
(including nine synthetic packaging scenarios; Python 3 is a test dependency),
sequential saved-tab and full native desktop fixtures, release packaging, plist validation
and signature verification. See `docs/VERIFICATION.md` for scope and limits.

**Verified for v0.2.0:** native tab selection and Command-T/W key-equivalent
synthesis; actual OS Center/Fill/half-screen/restore frames with asserted table
and path-field-editor focus; Fn–Control–F/C/R key-equivalent synthesis and
preserved browsing state. The window-manager verifier returns to the outer
event loop and waits for native animation; mere menu registration is not counted
as geometry evidence.

**Still manual:** physical keyboard presses (especially Fn–Control–arrows),
multi-window arrangements, cross-app physical drag/drop and default-document
opening. Synthesized menu events are not a claim of physical input coverage.

## Canonical source

Active implementation is `Rust/personal-files-macos/` in the **claw-work**
repository. Ubuntu remains separate at `Rust/personal-files/`.

The project moved from Monoclaw on 2026-11-14, including the editable-path/SMB
and independent-window features. Only tracked, reviewed source/product/test
artifacts were transferred; no private repository history or runtime state.
Build, launch and install from this directory using the scripts above. The
bundle identifier, application name, preference location and user cache remain
compatible with existing installations. See `docs/MIGRATION.md` for provenance
and verification.

## PDD source

`docs/PRODUCT_INTENT.md`, `docs/intents/macos-port.md`, `prompts/`,
`architecture.json` and `user_stories/` preserve approved intent and independent
acceptance. Source was maintained in the current coding harness. A separate
provider-backed `pdd intent apply` / sync, generated contracts and semantic
alignment were **not run or claimed**. Ubuntu source is unchanged.

## SMB servers and editable paths

Press **⌘L**, edit the full path near the top, then **Return**. Local absolute,
`~` and relative paths continue to work. Enter an SMB URL such as
`smb://tester@192.0.2.10/share/folder`, or use **Go > Connect to Server… (⌘K)**.
A server-only URL opens macOS's share picker. macOS handles login and saved
Keychain credentials; use its remember-password option when offered. Enter
passwords only in the macOS authentication dialog.

After connection the central listing shows the mounted folder and the path bar
shows its actual `/Volumes/…` path, which you can edit and browse normally.
The app keeps one outstanding connection; finish/cancel its macOS dialog before
retrying. Navigating elsewhere ignores a late connection result but leaves any
successful mount available to macOS. Live authentication/reconnect acceptance
remains manual; automated fixtures never access a real server or Keychain.

## Separate windows

Choose **File > New Window (⌘N)** to open another native Personal Files window
at the current folder. Each window has its own tabs, editable path, history,
selection and SMB connection. Favorites, Recent/Frequent and appearance stay
shared. **⌘W** closes a tab (or its window on the last tab); other windows stay
open. After closing every window, **⌘N** opens one again. **⌘Q** quits the app.
The existing saved-tab session is retained; restoring every open window after
quitting is not included.
