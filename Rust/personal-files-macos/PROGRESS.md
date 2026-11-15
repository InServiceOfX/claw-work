# 2026-11-14 — migrated to claw-work and rebuilt

- Canonical source is now claw-work `Rust/personal-files-macos/`, on local
  branch `feat/personal-files-macos-migration`. Ubuntu remains independent at
  `Rust/personal-files/`; its source and icon bytes were not changed.
- Copied all 52 tracked files from Monoclaw `dd6a868` with matching hashes before
  documentation edits; 49 remained unchanged through final comparison. Updated
  README/agent routing and removed one personal absolute path in historical
  intent before public-repo import. No private Git history/runtime state copied.
- Full verify.sh passed FROM THIS CHECKOUT: fmt, strict all-target Clippy, 31
  deterministic tests, native New Window/saved-tab/desktop/SMB fixtures, strict
  actual tiling geometry in both focus modes, release build and signature/plist.
- Explicitly cleared only this package's cached native release and freshly
  recompiled from claw-work. Its release New Window fixture passed. Installed
  that release at ~/Applications/Personal Files.app after graceful shutdown,
  exclusive staging/publication and byte comparison; preserved a prior-app
  .noindex backup. Application registration, name launch and Spotlight lookup
  verified against the installed path. Preferences were not migrated or reset.
- Full public-index privacy gate passed. It initially found missing review
  entries for eight existing Ubuntu folder/star PNGs. Reviewed their format,
  visual design, dimensions/CRC and lack of embedded text/EXIF/personal metadata;
  added exact hash/size entries to reviewed-binaries.json without changing them.
- Removed the original tracked Monoclaw directory after preservation/tests.
  Its history and a migration-routing receipt remain in that repo. This is a
  local source migration; no push/merge, imported private history or additional
  provider-backed PDD regeneration/semantic-validation claim.
- Earlier progress entries below describe historical locations/builds. Current
  README and docs/MIGRATION.md govern where to work now.

# 2026-11-14 — installed rebuild, Spotlight and local Git delivery

- Rebuilt the current SMB/New Window source into the permanent per-user cache
  at ~/Library/Caches/personal-files-macos and ran the release New Window
  fixture successfully. No runtime code changed after the full verify.sh pass.
- Gracefully quit the temporary preview. Staged/validated and exclusively
  published the rebuilt bundle at ~/Applications/Personal Files.app. Byte-
  compared its executable with the tested release and verified its signature.
  Previous installed app preserved under a timestamped .noindex backup in
  ~/Library/Application Support/Personal Files/Backups/.
- Unregistered temporary/cache launch entries, registered/imported the installed
  bundle, then launched by application name. NSWorkspace confirmed the installed
  path is both registered and running. Spotlight metadata has the correct
  display name/bundle ID and its Applications query returns only the installed
  app. Older Applications backup preserved with a .noindex folder suffix.
- Moved this work's /tmp preview, Cargo target, Swift module cache and initial
  PDD diagnostic dump to Trash; removed empty diagnostic directories. Durable
  build cache, installed app, prior-app backups and real preferences retained.
- User requested a local commit of the relevant macOS stories/prompts/code/
  tests/docs on feat/macos-smb-path-navigation. No push or merge requested.

# 2026-11-14 — New Window

- Added File > New Window / Command–N, including path-editor focus. Opens a
  distinct NSWindow with one tab at the current folder. Preserves the source
  window's tabs, history, selection and draft; each window owns controls/model,
  scan timer/worker and SMB reply receiver. Nested callbacks restore their
  window context; menus route to the focused live window.
- Share favorites/visits/appearance before window context transitions and
  redraw shared changes without resetting path drafts. Closing a window stops
  its timer and drops its model/receiver, preserving other windows. Closing the
  last window leaves New Window usable. Existing last-saved tab session remains;
  this does not add persistence for all windows. Native Objective-C roots keep
  existing app-lifetime ownership; closed-window scan workers are dropped.
- Saved story__macos_new_windows.md and request record, Product Intent, native
  and verify prompts. Existing SMB changes remain intact on the same uncommitted
  feature branch; no commit/push, installation or real preference changes.
- Full verify.sh PASSED: formatting, strict Clippy, 31 deterministic tests,
  sequential native New Window, saved-tab and desktop/SMB fixtures, real native
  tiling geometry in list/editor focus, release build, plist and strict signing
  checks. Earlier foreground-acquisition failures below are historical; this
  run acquired focus and passed all required checks.
- Rebuilt /tmp/personal-files-smb-cache/Personal Files.app. New Window fixtures
  cover real visible windows, independent tabs/folders/selection/drafts,
  connection result isolation, focused commands, shared preferences, closing
  original/secondary/last windows and reopening. No real documents/servers or
  credentials used. PDD plan was read-only; no provider-backed apply/sync,
  generated contract or semantic-validation claim.

# 2026-11-14 — editable full paths and SMB connections

- Implemented on `feat/macos-smb-path-navigation`; no commit or push.
- Added independent stories for full-path editing and native SMB connections,
  request record, Product Intent and native/model prompt requirements.
- Retained Command–L/Return local navigation; added SMB URLs in the same bar
  plus Go > Connect to Server… / Command–K. macOS NetFS/NetAuth handles optional
  username, login/share chooser and Keychain credentials. The center table
  browses the returned mount's optional subfolder, showing its actual POSIX path.
- One background mount worker, generation-protected completion and existing one
  directory scan worker. Cancellation/failure retains the prior view; tab/local
  navigation invalidates replies. Reject password URLs without echo/persistence.
- 31 deterministic tests and strict Clippy/format passed; native saved-tab and
  browsing fixtures passed, including injected SMB replies and credential
  rejection. Full verify.sh still fails the pre-existing foreground tiling check.
  Separate release bundle, plist and signature verification passed. Final release-binary
  browsing fixture also passed, with foreground-routing limitations reported.
- Built review bundle: /tmp/personal-files-smb-cache/Personal Files.app.
  Existing installed app/preferences and Ubuntu were untouched. Real SMB
  authentication, saved password reuse and mounted-share reconnect need a manual
  check. No real servers/credentials used in tests. No automated PDD sync,
  generated contract or semantic alignment is claimed.

# Personal Files macOS — progress

## 2026-11-02 — recovered improvements ported onto v0.2.0

- Local branch `bots/preserve-macos-local-improvements`, based on `origin/master`
  at `2037afe`. Only this subproject changed; Ubuntu and unrelated local work
  remain unchanged. Local commit authorized on 2026-11-03; no push or
  installed-app/preference changes.
- Storage-layer saves refuse to replace unreadable existing preferences.
  Successful reload restores normal saving; corrupt-state backup remains intact.
- Restore tab order/folders/active index and independent sort choices, compatible
  with older saved sessions and legacy settings. Explicit PATH starts one tab;
  missing folders independently fall back to Home. Restore/switch do not add
  visits; one worker, pending navigation and stale-result rejection remain.
- Cached app builds stage/validate/sign/strictly verify before publication;
  failures restore the previous cache app, or retain/report it when concurrent
  publication prevents restoration. Installed-app exclusive publication remains.
- Added regression coverage, independent acceptance story and matching prompts.
  See docs/VERIFICATION.md for current evidence and the pre-existing tiling-test
  failure, reproduced on untouched origin/master. Native saved tabs passed.
- PDD recorded the approved intent, but incremental architecture failed because
  the configured local model at 127.0.0.1:8080 was unavailable. No provider
  fallback, automated sync or semantic-validation success claimed; this harness
  maintained prompts/source/tests. Strict structural prompt/story lint passed.

## Completed — v0.2.0
- Native Window menu registered with AppKit; real OS Fill/Center/Move & Resize
  and Fn-Control shortcuts are supplied by macOS, not a custom window manager.
- Reproduced missing-menu failure before fix. Cleared bogus submenu wrapper
  actions, which could interfere with validation/key dispatch. Minimum content
  size reduced to 480x280; responsive layout and scrollable columns.
- Native in-window NSTabView tabs: Command-T, File > New Tab, add/close buttons,
  Command-W (last tab closes window), Control-Tab / Control-Shift-Tab switching.
- One shared Browser/preferences/scan worker with per-tab folder, history,
  selection, status, pending-navigation and path-draft snapshots. Switching does
  not add visits, lose shared preferences, or accept stale old-tab replies.
- 21 deterministic tests plus two sequential native fixture processes passed:
  native tabs/key equivalents; asynchronous real OS tiling geometry with file
  list and actual path field editor focused. Checks complete frames including
  restored position, non-fullscreen state and unchanged browser data.
- Independent reviewers found no production blocker in model/menu/tab code
  within their reviewed scopes; tightened geometry and focus assertions per
  review. See docs/VERIFICATION.md.
- Ubuntu unchanged. Local feature-branch commits authorized for consolidation;
  no push requested.

## Delivery / canonical source
- Active source is repos/Monoclaw/Rust/personal-files-macos/ on feature branch
  bots/consolidate-macos-apps. On 2026-10-31 the user requested consolidation,
  local commits and scratch cleanup. The reviewed temporary-worktree source
  replaced the older untracked copy; older duplicate preserved in Trash.
  All source hashes matched before documentation-only location updates.
- Fresh canonical-location formatting, Clippy and 21 non-GUI tests passed.
  Full verify.sh did NOT pass: native tiling could not acquire a foreground key
  window in two bounded attempts. Current native routing remains unverified;
  tests unchanged. Separate release build, plist and signature checks passed.
- v0.2.0 installed at ~/Applications/Personal Files.app. Previous bundle preserved
  under ~/Applications/Personal Files Backups/pre-0.2.0.cRgdeI/Personal Files.app.
  Installed binary itself passed both native fixture modes and signature/version
  checks. Existing preferences were not used by fixtures or reset by the update.

## Remaining acceptance
- Physical key presses, native multi-window arrangements, cross-app physical
  drag/drop and registered document-app opening remain hands-on checks.
- Tabs are in-window native AppKit controls, not independently detachable
  NSWindow tab groups. Session persistence is now covered by the recovery port.
- Automated PDD apply/sync/contracts/semantic validation not invoked or claimed.
- Older hardware/macOS, universal binaries and notarization untested.

## Last worked on
2026-11-03 — Reviewed the recovered preference/session/sorting/packaging port
for the requested local feature-branch commit. All 27 scoped files matched the
previously tested recovery snapshot before this status update. Existing
validation evidence remains applicable; no push, installation or real
preference changes.
