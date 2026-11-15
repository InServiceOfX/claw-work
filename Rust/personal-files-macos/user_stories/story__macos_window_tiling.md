# Tile Personal Files with standard macOS hotkeys

As a macOS user, I want to use Apple's standard window-tiling hotkeys with
Personal Files, so I can arrange it beside my other applications without
reaching for the mouse or learning application-specific shortcuts.

## Source and scope

Requested 2026-10-30 by Ernest:

> Add another user story related to Personal Files application. As a user I want to be able to use the hotkeys for tiling, described here: https://support.apple.com/en-ng/guide/mac-help/mchl9674d0b0/mac for my Personal Files application.

Independent reference: Apple, [Mac window tiling icons & keyboard
shortcuts](https://support.apple.com/en-ng/guide/mac-help/mchl9674d0b0/mac),
reviewed 2026-10-30. The page offers version-specific guidance for macOS
Sequoia 15, Tahoe 26 and Golden Gate 27; the retrieved default was macOS 27.
Use the shortcuts/actions available on the macOS version under test. This
story does not require a tiling emulator on older macOS versions.

Status: **native integration implemented in v0.2.0; single-window automated
acceptance passed; physical-keyboard/multi-window acceptance pending**.
This story is derived from the user's request and Apple's documentation, not
from the current implementation. Evidence is in docs/VERIFICATION.md.

## Acceptance

### 1. Move and resize the active Personal Files window

Given Personal Files is the active, foreground application with a normal
(non-full-screen) window on a macOS version supporting the documented action,
the following hotkeys perform the same native window action as Apple's menus:

| Action | Apple hotkey | Expected outcome |
| --- | --- | --- |
| Fill | Fn–Control–F | Fill the desktop's available area, without entering full screen. |
| Center | Fn–Control–C | Center the active window on the desktop. |
| Left | Fn–Control–Left Arrow | Tile the active window in the left half of the desktop. |
| Right | Fn–Control–Right Arrow | Tile the active window in the right half of the desktop. |
| Top | Fn–Control–Up Arrow | Tile the active window in the top half of the desktop. |
| Bottom | Fn–Control–Down Arrow | Tile the active window in the bottom half of the desktop. |
| Return to Previous Size | Fn–Control–R | Restore the pre-tiling size according to native macOS behavior. |

Fn may be labeled with the Globe symbol on the keyboard. Honor native display,
Dock/menu-bar, tiling-margin and window-size constraints rather than requiring
hard-coded pixel coordinates.

### 2. Participate in native multi-window arrangements

Given suitable companion windows are available, Personal Files participates in
the following native arrangements when supported by the macOS version under
test. macOS chooses the other windows according to its normal behavior; this
story does not introduce app-specific companion-window selection.

| Action | Apple hotkey | Active Personal Files window / companion placement |
| --- | --- | --- |
| Left & Right | Fn–Control–Shift–Left Arrow | Personal Files left; second window right. |
| Right & Left | Fn–Control–Shift–Right Arrow | Personal Files right; second window left. |
| Top & Bottom | Fn–Control–Shift–Up Arrow | Personal Files top; second window bottom. |
| Bottom & Top | Fn–Control–Shift–Down Arrow | Personal Files bottom; second window top. |
| Left & Quarters | Fn–Control–Option–Shift–Left Arrow | Personal Files left; companions top-right and bottom-right. |
| Right & Quarters | Fn–Control–Option–Shift–Right Arrow | Personal Files right; companions top-left and bottom-left. |
| Top & Quarters | Fn–Control–Option–Shift–Up Arrow | Personal Files top; companions bottom-left and bottom-right. |
| Bottom & Quarters | Fn–Control–Option–Shift–Down Arrow | Personal Files bottom; companions top-left and top-right. |

### 3. Preserve native keyboard handling and browsing state

- Tiling hotkeys work with focus in the path editor as well as in the file
  list. Personal Files must not swallow them as text editing, file navigation,
  refresh, path copy or selection actions.
- Resizing/tiling must not change the current directory, path-editor text,
  selected entries, manual favorites or visit counts, and must not open or
  modify source files.
- Existing browsing shortcuts remain unchanged, including Command–Up for
  parent navigation and Command–R for refresh; these are distinct from the
  Fn–Control combinations.
- The window's path field, navigation, sidebar and file list remain usable
  after tiling and restoring. Window sizing must not unnecessarily prevent
  macOS from offering the requested layouts.
- Use the native system behavior without a third-party tiling tool, global
  key remapping, new Accessibility permissions or changes to system shortcuts.

### 4. Keep desktop tiling distinct from unsupported shortcuts

Apple lists no default keyboard shortcut for individual corner tiles,
Quarters, or the full-screen/ Split View actions on the referenced page.
Do not invent hotkeys for those actions or count entering full screen as
passing the desktop Fill/half-screen tests. This story does not add custom
hotkeys or require unsupported OS features.

## Verification plan

- Launch the actual app on an interactive Mac desktop, bring its window to the
  foreground and confirm it owns the active/key window.
- Record macOS version, keyboard/Fn configuration, display layout and relevant
  tiling settings. Check the matching Apple guide version for available actions.
- Exercise each supported single-window action with file-list focus, then with
  path-editor focus; inspect the resulting frame/visible desktop area and
  restore the previous size. Repeat native multi-window actions with suitable
  synthetic companion windows.
- Compare browse state before/after and explicitly assert that tiling did not
  navigate, refresh-as-a-command, increment visits, change selection/favorites,
  alter editor text or mutate fixture documents. Recheck normal browsing keys.
- Separate automated AppKit callback/frame tests from actual native hotkey
  dispatch. A background test with no active/key window is **blocked**, not
  evidence that tiling works. Existing foreground-activation uncertainty remains
  unresolved until observed on a real desktop.

The later implementation verifies native menu entries/modifiers, actual OS
Fill/Center/half-screen/restore geometry, Fn–Control–F/C/R key-equivalent
synthesis and both actual responder focus modes. Physical-keyboard dispatch,
multi-window arrangements, semantic alignment and generated contracts remain
unverified; do not equate a synthesized menu event with a physical key press.

<!-- pdd-story-prompts: prompts/native_rust.prompt -->
<!-- pdd-story-dev-units: prompts/native_rust.prompt -->
