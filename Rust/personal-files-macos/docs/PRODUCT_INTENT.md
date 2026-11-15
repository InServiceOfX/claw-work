# Product Intent — Personal Files for macOS
Approved 2026-10-17: “Sounds good Rust, native macOS application, and feature parity with Ubuntu version please”.

Build a separate Rust application using AppKit, not GTK, a browser, or a Python
runtime. Ubuntu remains unchanged. Preserve its accepted browsing-focused
behavior: explicit Favorites, automatic Recent (7) and Frequent (5), bounded
200-directory history, last-location restore, hidden files always visible,
absolute/home/relative path entry, file-path parent navigation and selection,
symlink-alias preservation, three reversible sorts with folders first,
multi-selection, plain and POSIX shell-quoted path clipboard, default-app document
opening, back/forward/up, refresh, directory monitoring, stale-scan rejection,
Silver/Midnight/Aurora/Automatic appearance and validated local backgrounds.
Star, context menu and directory drag/drop can pin favorites. Missing favorites
remain removable. Clearing automatic history preserves explicit favorites.
Use standard macOS Command shortcuts, native clipboard/default application APIs,
and a launchable per-user .app bundle. Do not replace Finder.

Browsing, copying, favoriting and drops MUST NOT alter source files or execute
path text. No network service, telemetry, content upload or document-content
inspection for themes. Scanning uses a single worker and UI stays on main thread.
Save local state atomically with mode 0600 under
~/Library/Application Support/Personal Files/preferences.json; recover invalid
state with a preserved backup and visible warning. Show filesystem/save errors.
Tests use synthetic temporary files/preferences, never real personal state.

Non-goals remain file creation/rename/move/delete, thumbnails, recursive
folder sizes. Package builds in the user's cache, not tracked source.
See user_stories/story__macos_places.md for independent acceptance.

## Native window tiling — requested 2026-10-30

Personal Files should support Apple's standard macOS window-tiling hotkeys
for the running OS version, both with path-editor and file-list focus. This
includes Fill, Center, half-screen tiles, return to previous size and supported
multi-window arrangements. Preserve browsing state and existing application
shortcuts; use native window behavior rather than global key remapping or a
third-party tiling tool. Do not confuse desktop Fill with full screen, invent
undocumented hotkeys, or require emulation on older macOS versions.

Source: https://support.apple.com/en-ng/guide/mac-help/mchl9674d0b0/mac.
Independent acceptance: user_stories/story__macos_window_tiling.md.
Implementation requested after Ernest reported being unable to tile. Reproduce
native window/menu eligibility before fixing it. Physical desktop verification
is distinct from a registered menu or callback test.

## Tabs — requested 2026-10-30

Command–T / File > New Tab opens another tab at the current directory in a
native AppKit tab strip. Each tab keeps directory/history/selection/path draft;
favorites/history counts/appearance remain shared. Command–W closes the current
tab (last tab closes the window); Control–Tab/Control–Shift–Tab switches tabs.
Keep one directory worker and reject old-tab results. The original iteration
did not require session restore; the approved recovery port below now adds it.
See user_stories/story__macos_tabs.md.

## Recovered improvements — approved 2026-11-02

Retain unreadable existing preferences unchanged until a successful reload;
every save must refuse to overwrite defaults after a read failure. Preserve
the existing corrupt-state backup/recovery behavior.

Reopen ordered tab folders, active index and independent sort choices, compatible
with legacy settings and the older tabs/active_tab schema. Explicit startup PATH
opens one tab. Missing folders independently fall back to Home. Restore/switch
actions do not add visits; selection, path drafts and runtime history are not
session data. Failed navigation retains the saved successful folder.

Stage, validate, sign and verify cached builds before replacing a working app.
Restore the previous cached app on publication failure; if a concurrent app
prevents safe restoration, retain and report the recovery location. Continue to
refuse existing installed destinations. Keep newer native UI/tiling behavior.
Acceptance: user_stories/story__recovered_improvements.md.

<!-- pdd-intent-entry:preserve-recovered-macos-improvements-50bf5576:start -->
## Preserve recovered macOS improvements

- Intent event: [`docs/intents/intent__preserve-recovered-macos-improvements-50bf5576.md`](intents/intent__preserve-recovered-macos-improvements-50bf5576.md)
- Change kind: `add`
- Supersedes: none
- Scope: `existing_pdd_subproject_change`
- Technology: `not stated`

> do that please
>
> Approved proposal: selectively port the recovered Personal Files macOS improvements onto the newer implementation: protect unreadable existing preferences from being overwritten, restore ordered tab sessions and the active tab after restart, preserve independent sorting for each tab, and stage/sign/verify the cached app before replacement. Preserve the newer native AppKit tabs, window tiling, one scan worker, stale-result protection, and exclusive no-overwrite installation. Bring the corresponding regression tests. Affected files are src/core.rs, src/model.rs, src/native.rs, and build.sh.
<!-- pdd-intent-entry:preserve-recovered-macos-improvements-50bf5576:end -->

## Editable full paths and SMB — requested 2026-11-14

Retain the visible editable full-path bar, Command–L and Return navigation.
Add SMB URLs by hostname/IP and optional username, plus Go > Connect to Server
(Command–K). Use macOS NetAuth for connection/share selection and Keychain
credential reuse/saving; never implement a password store. Browse the resulting
mount and optional subfolder in the central listing, showing its actual POSIX
path. Background connections must preserve responsiveness and reject replies
after another navigation or tab transition. Errors/cancellation retain the last
successful view and saved location. Reject password-bearing URLs without
persisting or echoing them. Ubuntu is out of scope.
See user_stories/story__macos_smb_navigation.md and
 docs/intents/macos-smb-navigation.md.

## Multiple native windows — requested 2026-11-14

File > New Window / Command–N opens another separate native macOS window with
one tab at the current folder. Preserve the existing window's tabs, selection,
history and path draft. Each window owns its browser/tabs, one scan worker,
one outstanding SMB connection and display state; callbacks/menu commands must
route to the correct window and restore context after nested native callbacks.
Share favorites, visits and appearance without lost updates. Closing any window
stops its polling and drops its browser/connection receiver, leaving others
usable. Closing the last window leaves New Window available; Command–Q quits.
Retain single-session save/restore (the last saved window's tabs); persisting all
windows is outside this request. See user_stories/story__macos_new_windows.md.
