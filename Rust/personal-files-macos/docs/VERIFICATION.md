## 2026-11-18 — native file clipboard

- Full `./verify.sh`: fmt, strict all-target Clippy, 44 deterministic tests,
  all four opt-in GUI groups sequentially, release build, plist lint and strict
  signature verification. No operation success tests remain disabled. GUI tests
  remain opt-in in the default Cargo run but are explicitly run by verify.sh.
- File-operation tests exercise files, folders, dangling/nested symlinks,
  extended attributes, resource forks, ACLs, permissions/timestamps, no-read
  rename, alias descendants, special-item copy failure, late target collision,
  root replacement by a link, blocked source restoration and parent renaming
  during forced EXDEV. The EXDEV test is injected, not a live external volume.
- Native clipboard fixture creates synthetic windows/files/preferences and
  restores the clipboard. It exercises native Command-C/V/Option-V key
  equivalents, actual asynchronous copy/move, conflict errors, normal path-field
  text editing, Unicode/escaping, invalid mixed batches, context versus app menu
  window routing, serialized work and rejecting Quit until cancellation outcome.
- Existing native windows, saved tabs, browsing/SMB injection and strict actual
  tiling geometry continue to pass. New fixture dialogs are captured only in
  the explicitly selected synthetic fixture; normal application dialogs remain
  native and long results are scrollable.
- Independent static review and fixes were performed separately from execution.
  Cross-volume destructive fallback was removed to avoid late-writer data loss;
  those moves are refused, not represented as supported. The reviewed stale
  recovery-path bug was fixed with unconditional descriptor-relative restoration
  and empty-directory cleanup plus a regression test.
- This is AppKit representation and native key-equivalent evidence, not physical
  Finder-to-app/app-to-Finder keyboard automation or a transactional filesystem
  snapshot. Real external-volume metadata behavior and live SMB remain manual.
  APFS rejected the attempted invalid-byte filename fixture with EILSEQ; the
  executable fixture therefore covers Unicode/escaping, not actual invalid-byte
  file creation. Filesystem URL conversion no longer uses lossy UTF-8.
- The freshly built release also passed the synthetic native clipboard fixture.
  After graceful shutdown, the installed per-user app was upgraded by staged,
  signature-verified, exclusive publication. The previous bundle is preserved
  in a dated `.noindex` backup under Applications. Installed executable byte-
  matched the tested release and its signature passed. NSWorkspace verified
  registration and the running bundle resolve to the installed app. Preferences
  were not reset or inspected and Finder was not replaced.
- No provider-backed PDD apply/sync, generated contract or semantic-validation
  result is claimed; reviewed intent, prompts, independent story and tests are
  maintained with the implementation in this harness.

## 2026-11-14 — claw-work relocation

- Full `./verify.sh` passed from `Rust/personal-files-macos/` in claw-work:
  formatting, strict Clippy, 31 deterministic tests, three explicitly invoked
  GUI tests (native windows, saved tabs, desktop plus asynchronous tiling),
  native release packaging, plist lint and strict bundle signature.
- After clearing this package's native release artifact, build.sh visibly
  recompiled personal-files-macos from the claw-work project. The freshly
  compiled release passed its synthetic New Window fixture independently.
- Original-to-destination hash comparison covered all 52 transferred files
  before documentation changes. Final comparison covered 49 unchanged files;
  the remaining three contained only routing/public-portability documentation
  edits. No runtime source, dependency, test or script changed in relocation.
- Public-index gate passed over the entire staged tree after adding reviewed
  hash/size entries for eight pre-existing Ubuntu icon PNGs. Those files were
  verified identical to their committed baseline, syntactically valid, under
  128 KiB, and without text, EXIF or personal metadata. No Docker/context/image
  or provider changes were made by this migration.
- Installed app's executable byte-matched this fresh release; signature passed.
  Old app preserved in a timestamped .noindex backup. NSWorkspace verified the
  installed application was both registered and running after name-based launch;
  Spotlight's Applications query returned only that installed bundle.
- Tests used synthetic preferences/documents and injected SMB replies. No real
  credentials, user documents or private state were copied to the public repo.
  Live SMB authentication and physical keyboard checks remain separate.

## 2026-11-14 — installed application and Spotlight

- Permanent-cache release rebuild passed plist/signature checks and the native
  New Window fixture using synthetic preferences/documents.
- Installed executable byte-matched the tested cache executable; strict
  installed-bundle signature verification passed. Replaced only the authorized
  ~/Applications/Personal Files.app, preserving the previous bundle in a
  timestamped .noindex backup. Real preference contents were not inspected.
- Gracefully terminated the preview, registered the installed bundle and
  launched with `open -a "Personal Files"`. NSWorkspace verified both registered
  and running paths are ~/Applications/Personal Files.app.
- mdls returned Personal Files and io.monoclaw.PersonalFiles.macOS; mdfind scoped
  to ~/Applications returned only the installed app after excluding the old
  backup. This proves metadata/name resolution; no physical Spotlight key/input
  automation is claimed.
- Temporary preview/Cargo/Swift directories and the initial PDD error dump were
  moved to Trash. Full source/runtime checks below remain applicable: no runtime
  edits followed their passing run. Live SMB authentication remains manual.

## 2026-11-14 — New Window (supersedes earlier foreground limitation for this run)

- Full verify.sh PASSED after the window-context change: fmt, warnings-denied
  all-target Clippy, 31 deterministic tests, all three opt-in GUI tests run
  sequentially, native release build, plist lint and strict signature checks.
- native_new_windows_story passed: Command–N from the real path editor creates
  separate visible NSWindows/controls/delegates. Independent folder/tab/selection/
  draft state, focused menu routing, isolated injected connection results,
  shared favorites/appearance and unchanged source fixtures asserted. Closing
  original/secondary/last windows and reopening were exercised. Fixture polling
  waits for startup scans before applying folder-specific actions.
- native_saved_session_story passed with restored tabs and independent sorting.
- native_desktop_story passed including native browsing/SMB reply injection and
  its strict asynchronous native tiling subprocess. Actual Center/Fill/Left/
  Right/Top/Bottom/Restore geometry, list/editor responder identities and native
  shortcuts passed. Physical keys and physical multi-window arrangements remain
  distinct manual acceptance, not inferred from these tests.
- Separate release bundle at /tmp/personal-files-smb-cache/Personal Files.app.
  Existing installed app/preferences unchanged. GUI tests use synthetic temporary
  files/preferences only; no real SMB authentication or Keychain read.
- Read-only PDD plan recorded in docs/intents/macos-new-windows.md. Harness-
  maintained story/prompts/code/tests reviewed together; no generated contract,
  provider-backed sync or semantic-validation result is claimed.

## 2026-11-14 — editable paths and SMB

- Baseline: 28 deterministic tests passed; two GUI tests explicitly ignored.
- Final: formatting and strict all-target Clippy passed. 31 deterministic tests
  passed (7 library, 2 native URL-parser, 15 acceptance, 1 packaging, 6 recovered
  behavior); two desktop tests remain opt-in.
- Added tests cover IPv4/IPv6, optional username/port, server-only URLs, percent
  escapes mixed with Unicode, credential/query/fragment/control/traversal
  rejection, cancellation, real asynchronous scanning of synthetic mount folders,
  unchanged documents/history and stale success/error rejection after navigation
  and tab changes.
- Native saved-session fixture passed outside the filesystem sandbox. Native
  browsing fixture passed, including injected mount reply delivery, mounted-path
  display and rejecting a password URL without saving it. Final release-binary fixture passed too, including reconnecting to the
  already displayed folder restoring its POSIX path.
- Full verify.sh stopped at the existing asynchronous tiling assertion:
  `native tiling blocked: cannot acquire foreground key window`. Earlier native
  browsing printed its background-session foreground-routing limitation. No
  relaxed assertions or skipped required checks are represented as success.
- Separate build.sh completed a native release bundle in
  /tmp/personal-files-smb-cache/Personal Files.app, with plist and strict signature
  checks. Installed ~/Applications app and real preferences were not changed.
- No real server connection or Keychain read was performed. Authentication,
  remember-password reuse, actual share selection and reconnect to Finder mounts
  need a hands-on check. NetFS FFI and option names were checked against the
  installed macOS SDK NetFS.h. Synthetic delivery does not prove live SMB.
- PDD read-only planning ran; no provider-backed apply/sync, generated contract
  or semantic-validation result is claimed. Story and both affected prompts
  were maintained with source/tests in this harness and reviewed together.

# Verification — Personal Files macOS

## Latest: recovered improvements, 2026-11-02

- Branch `bots/preserve-macos-local-improvements` from origin/master `2037afe`.
  Storage-layer unreadable-preferences protection, ordered tab-session restore,
  independent tab sorting and staged/verified cached builds are selectively
  ported. Current native UI/tiling and exclusive installation are retained.
- Regression baseline: five of the six recovered-behavior tests failed against
  untouched application source; eight of nine packaging scenarios rejected the
  original build script. Existing tests are retained.
- Formatting, warnings-denied Clippy, 28 deterministic Rust tests (including one
  wrapper running nine synthetic Python packaging cases) and strict PDD
  prompt/story structural lint passed. Tests cover legacy preferences/session
  compatibility, relative/missing saved paths, active tab/order, independent
  sorting, no restore/switch visit inflation, closed tabs, failed navigation,
  unreadable-original protection and successful reload.
- Packaging checks inject build/copy/plist/sign/verification/publication failure,
  verify previous bundle bytes remain/restored, assert replacement without
  directory merging, retain recovery after a concurrent app appears, and cover
  first build/cache overrides and spaces in paths. All mocks/installs/cache paths
  are synthetic; the exclusive rename uses the real non-GUI application helper.
- `native_saved_session_story` passed: real AppKit tab strip, Command-T/W and
  cycle key equivalents, add/close controls, two restored NSTabView items with
  active index/folder and independent header sorting, unchanged visit counts.
  Fixture preferences/documents are temporary; no real clipboard is accessed.
- Full `verify.sh` DID NOT PASS: unchanged `native_desktop_story` fails at
  `native_desktop.rs::exercise_tiling`, asserting Fn-Control modifiers for the
  OS-supplied Left menu item (observed Control only). An untouched temporary
  archive of origin/master `2037afe` fails at the identical assertion. No tiling
  assertions were weakened; full native/tiling acceptance remains UNVERIFIED.
  The saved-tab fixture is separate and does not replace the required full test.
- Separate native-host release packaging, plist lint and strict signature checks
  passed in `/private/tmp/monoclaw-recovered-app-20261102/`. Production installed
  apps/preferences and the normal user cache were not changed.
- PDD intent `preserve-recovered-macos-improvements-50bf5576` was recorded.
  Its automated incremental architecture failed because the configured local
  model was unavailable; no fallback, automated sync, generated contracts or
  semantic alignment is claimed. This harness maintained the scoped artifacts.

Reproduction (Cargo targets/cache remain outside source):

```bash
cargo fmt --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked
cargo test --locked --test desktop native_saved_session_story -- --ignored --nocapture
./verify.sh # retains the full native tiling gate; currently fails as above
```

## Earlier: canonical-location consolidation, 2026-10-31

- Moved reviewed source into Monoclaw/Rust/personal-files-macos without changing
  application/test/prompt bytes. Source manifest matched before Markdown-only
  active-location/evidence updates.
- Fresh formatting, warnings-denied Clippy and 21 non-GUI tests passed.
- Full verify.sh DID NOT PASS: native desktop acceptance reached the asynchronous
  tiling verifier, which could not acquire a foreground key window. One bounded
  sequential retry failed at the same assertion; stopped without weakening tests.
  The synchronous fixture also explicitly reported foreground routing unverified.
- Native foreground/hotkey/tiling acceptance is therefore CURRENTLY UNVERIFIED
  in this execution session, notwithstanding the dated earlier success below.
- Separately ran build.sh: release bundle build, plist lint and signature
  verification passed. Installed application/preferences were not replaced.
- Relocation review requested this explicit current blocker entry; corrected.
  Independent review was static and did not independently execute the tests.

## Earlier: v0.2.0 tiling fix and tabs, 2026-10-30

- Baseline: 19 deterministic tests passed. Added native reproduction failed:
  NSApplication.windowsMenu was nil. Fixed native registration, nil-action
  submenu wrappers and unnecessarily restrictive window minimum/layout.
- Current: cargo fmt --check, Clippy --locked --all-targets -D warnings,
  21 deterministic tests, explicit native desktop integration, release build,
  plutil and codesign verification passed via ./verify.sh.
- Two sequential synthetic fixture processes are exercised by the desktop test:
  native menus/tabs/clipboard/browser behavior; asynchronous actual AppKit
  window-manager actions. Both stay on the initial thread with temporary state.
- Native tab checks synthesize Command-T/W and Control-Tab key equivalents,
  select NSTabView tabs, test add/close buttons and independent folder/draft/
  selection while preserving shared visits.
- Native tiling checks assert OS-provided Window-menu actions/modifiers, perform
  Fn-Control-F/C/R key-equivalent synthesis, and invoke the OS half-screen menu
  actions. Verify complete expected frames (40pt margin tolerance), original
  size/position restoration, Center size preservation from a normal window,
  actual path-field editor versus table first responder, desktop—not full-screen—
  geometry, unchanged preferences/selection/path draft, and compact 480x280 layout.
- Native window-manager work requires returning to NSApplication's outer event
  loop. Earlier synchronous callback-only geometry attempts could not flush
  deferred OS work; the asynchronous verifier waits beyond animation before
  checking frames. This is a verifier architecture change, not emulated tiling.
- Physical Fn-Control-arrow key presses and multi-window arrangements were not
  exercised. Do not claim physical input coverage from synthesized key events.
- Read-only independent review covered model.rs and bounded native menu/tab/
  layout ranges plus acceptance helpers. No production blocker found; two test
  evidence gaps (partial frame predicates and unchecked focus) were corrected.
  Reviewers did not independently execute commands.
- Native helper first-iteration foreground limitation below is historical;
  the current asynchronous verifier acquires/checks the key window, explicitly
  asserts both actual first responders, and fails if it cannot acquire focus.
- Initial implementation used the Monoclaw-personal-files worktree after a
  concurrent session changed the shared checkout. On 2026-10-31 the user requested
  consolidation and local commits: canonical source is now Monoclaw/Rust/
  personal-files-macos on bots/consolidate-macos-apps. All moved source hashes
  matched before location-documentation updates. Ubuntu source unchanged.

- Installed v0.2.0 at ~/Applications/Personal Files.app with the previous bundle
  preserved under ~/Applications/Personal Files Backups/. Confirmed bundle version
  0.2.0 and signature, then ran both native fixture modes against the installed
  release executable itself; both passed. No real preferences were used as fixtures.

No provider-backed PDD regeneration, semantic alignment or generated contract
was invoked/claimed.

## Historical evidence (first iteration)

### Earlier story-only addition, 2026-10-30

Added user_stories/story__macos_window_tiling.md from Ernest's request and the
linked Apple guide, with durable request/Product Intent/native prompt links.
Checked the source hotkey matrix and local metadata/reference paths.
No application source or binary changed, and no runtime tests, native tiling
acceptance, PDD generation or semantic validation were run for this story.
The implementation/foreground limitations documented below remain open.

## Executed on this Mac, 2026-10-18

- `cargo fmt --check`: passed.
- `cargo clippy --locked --all-targets -- -D warnings`: passed.
- `cargo test --locked`: **19 passed** (4 state/installer tests + 15 preserved
  filesystem/preferences/appearance acceptance tests); native test intentionally
  ignored in this ordinary run.
- `cargo test --locked --test desktop -- --ignored --nocapture`: **1 passed**
  with explicitly limited foreground coverage (below).
- `./verify.sh`: passed including locked native release build,
  `plutil -lint` and `codesign --verify` for the cached bundle.
- `CARGO_BUILD_TARGET=invalid-target-must-not-be-used ./build.sh`: passed,
  showing explicit host selection overrides a bad ambient default target.
- `bash -n build.sh launch.sh install.sh verify.sh`: passed.
- `./install.sh`: installed `~/Applications/Personal Files.app`.
  Publication uses a tested Darwin exclusive atomic rename, not merging ditto
  directly into the final destination.

Tests use temporary synthetic documents/preferences. Clipboard representations
are snapshotted in memory and restored; a concurrent user copy is not overwritten.
No real saved places or user document contents are used as fixtures.

## Native desktop evidence

The fixture executes on AppKit's main thread inside NSApplication.run. It checks
real table row counts, hidden files, star action, path-entry target/action,
back navigation, native selection, plain and shell-quoted NSPasteboard contents,
native text-editor Copy, header sort callbacks, invalid path preservation,
file-path selection, valid/invalid NSImage handling, all themes, local URL drop
handler filtering, rejected remote/non-file/text input, metadata polling without
extra visits or selection loss, latest-request wins, clear-history/favorites,
source documents unchanged, and a fresh native window restoring saved state and
navigating by sidebar action.

This is widget/callback and local clipboard evidence, **not pointer-driven
cross-application drag/drop or proof of every menu shortcut**. Off-screen
synthetic view capture was inspected; it omits some window/compositor-backed
chrome and is not a full desktop visual acceptance.

## Known foreground blocker

This harness session reports nil AppKit keyWindow/mainWindow despite a visible
fixture and active text editor. Consequently NSApplication responder-chain
`sendAction:copy:to:nil` fails; native editor Copy addressed directly succeeds.
The fixture prints `UNVERIFIED foreground menu routing`, tests editor Copy
directly and preserves the strict foreground assertion behind
`PERSONAL_FILES_REQUIRE_FOREGROUND=1`. The strict bundled launch through
LaunchServices also failed this assertion in this session.

Do **not** describe that stricter test as passed or silently skipped. A person
must foreground the app and verify ⌘L, path-field ⌘C, table ⌘C, ⌘D,
⌘[/⌘], ⌘↑ and ⌘R. This may be a session activation restriction or an app
activation defect; the cause is not proven. Full interactive parity is pending.

## Independent review

A read-only reviewer found three substantive issues, all fixed:
1. An externally replaced folder could refresh to its parent's entries while
   retaining the old displayed path. Added a failing reproduction, then reject
   this refresh transition; regression now passes.
2. Ambient Cargo target configuration could package a stale host executable.
   Explicit native target and matching artifact path now used and exercised.
3. Direct installer ditto was non-atomic/racy. Stage first, then publish with
   renamex_np RENAME_EXCL; collision tests prove both bundles remain intact.

Reviewer re-inspected these fixes and reported no blockers in model.rs,
bundle.rs, build.sh and install.sh. Two broader/narrow-native review attempts
timed out; independent review of native.rs remains incomplete.

## Remaining manual checks

- Foreground activation/menu/keyboard behavior described above.
- Physical Finder ↔ Personal Files directory drag/drop.
- Opening synthetic documents through registered default apps.
- Resize/visual appearance on a normal foreground desktop; no universal binary,
  older macOS hardware, notarization or distribution certification tested.

## PDD evidence

Read-only intent plan `look-at-the-changes-here-81504e6e` and user approval
preserved. Durable prompt/story source maintained locally. Automated intent
apply/sync would delegate to another model/provider and was not invoked without
separate authorization. No generated story contract, semantic alignment or
automatic generation success is claimed.
