# Open another Personal Files window on macOS

As a Personal Files user on macOS, I want to open a new window, so I can
browse different folders side by side in separate Personal Files windows.

Source: Ernest's 2026-11-14 follow-up:
> I like it already. Thank you. One more: as a user, I want to be able to open a "new Window" and have another "Window" open where I can have another "window" of Personal Files open.

## Acceptance
1. File > New Window / Command–N opens another visible native macOS window,
   starting with one tab at the current folder. The original window, its tabs,
   selection, history and unfinished path text remain intact. This also works
   while the path editor has focus.
2. Each window has independent folders, tabs, sort, selection, navigation
   history, path drafts, directory scans and SMB connection results. Navigating
   or connecting in one window MUST NOT change another window's location.
3. Menu/keyboard actions apply to the focused window. Existing Command–T,
   Command–L, Command–K, copy and browsing actions remain usable in every window.
4. Favorites, Recent/Frequent history and appearance are shared across open
   windows. Changes in one window must not be lost when another window saves.
5. Closing a window (including Command–W on its last tab) closes only that
   window and stops its scan polling. Other windows keep working, including
   after the original window closes. Command–Q still quits the app.
6. Closing the last window leaves New Window available through the app menu.
   Reopening restores the last saved tab session. Restoring all windows on the
   next app launch is outside this story; retain the existing single-session
   persistence behavior.
7. All windows support the existing native resizing/tiling behavior. AppKit
   remains on the main thread. Tests MUST NOT use real documents/preferences,
   make real SMB connections, or create/rename/move/delete user documents.

Verification uses visible independent NSWindows, native menu key dispatch,
synthetic directories/preferences and injected SMB replies. Synthesized keys
are not physical-keyboard or cross-app window-arrangement evidence.
<!-- pdd-story-prompts: prompts/native_rust.prompt, prompts/verify_bash.prompt -->
