<!-- pdd-story-prompts: prompts/core_rust.prompt, prompts/ui_rust.prompt -->
<!-- pdd-story-dev-units: prompts/core_rust.prompt, prompts/ui_rust.prompt -->

# Multiple folders in tabs

As a Personal Files user, I want multiple folders open in tabs so I can switch
between locations without losing my place.

Source request: `Can we also be able to "open tabs" in the Personal Files application?`

Approved review: Ctrl+T duplicates the current folder in a new tab; a plus
button and folder/saved-place context actions also open tabs; each tab retains
its folder, selection, sorting and back/forward history; close with ×/Ctrl+W
while retaining one tab; cycle with Ctrl+Tab/Ctrl+Shift+Tab; restore open tabs
after restart. The user explicitly approved this review on 2026-10-16.

## Acceptance

1. Ctrl+T and the plus button open the displayed folder in a new foreground
   tab, even when the path entry has focus. Folder and saved-place context menus
   offer Open in New Tab. New tabs have independent navigation history.
2. A visible, scrollable tab strip has folder-name labels, full-path tooltips
   and close buttons. Switching retains each tab's folder, selected files,
   sorting and back/forward history. Path entry and clipboard actions operate
   on the active tab. Favorites and Recent/Frequent remain shared.
3. Ctrl+Tab selects the next tab and wraps; Ctrl+Shift+Tab selects the previous
   and wraps. Switching or refreshing MUST NOT add visits. An asynchronous scan
   from a different tab MUST NOT overwrite the displayed folder.
4. Closing an active tab selects a neighbor; closing an inactive tab preserves
   the active view. Closing the last tab is disabled and Ctrl+W keeps it open.
   Closing tabs does not delete folders, favorites or visit history.
5. Restart restores tab order, folder paths, per-tab sorting and active tab.
   Runtime selections/back-forward history remain independent while the app
   runs; those are not promised across restarts. A missing startup folder falls
   back to Home in that tab without navigating another tab.
6. Old preferences without tabs still load, preserving favorites/history and
   reopening the previous last folder. Invalid saved tab paths/indexes are
   sanitized without discarding otherwise valid favorites.

Evidence: the Rust acceptance suite checks session serialization and legacy
state; the explicit GTK desktop_story checks tab keyboard dispatch, context
actions, independent state/history, close behavior, stale-scan isolation and
fresh-window restart. Existing browsing regressions remain intact.
