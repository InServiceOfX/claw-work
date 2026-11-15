# Browse several folders in tabs

As a Personal Files user on macOS, I want Command–T to open another tab,
so I can browse several folders in one window like a typical Mac application.

Source: Ernest's 2026-10-30 request:
> Also, as a user of Personal Files I want to be able to open up tabs in this Mac OS version of Personal Files just like any other typical Mac OS application, with Command T

Status: implemented in v0.2.0. Model and native tab/key-equivalent acceptance
passed with temporary fixtures; physical-keyboard checks remain manual.

## Acceptance
1. Command–T and File > New Tab open/select a tab at the currently displayed
   folder. A visible native AppKit tab strip lets me select tabs by clicking.
2. Each tab retains its own directory, back/forward history, selection and
   unfinished path-field text. Navigating one tab does not navigate another.
3. Command–W / Close Tab closes the selected tab and selects a remaining tab;
   closing the last tab closes the window. A visible close-tab control is available.
   Control–Tab / Control–Shift–Tab selects the next/previous tab.
4. Favorites, visits and appearance remain shared; switching/opening a tab
   alone does not count as a directory visit or undo another tab's preferences.
5. Command–T works with the path editor or file list focused. Ordinary text
   editing, copy and browsing shortcuts keep their meanings.
6. Opening, switching or closing tabs never creates, renames, moves or deletes
   documents. Late scan results from an inactive/closed tab must not replace
   the active view. Scanning remains on one worker for the browser window.
7. Tiling resizes the entire window and all tabs remain accessible.
8. As approved on 2026-11-02, quitting/reopening without a PATH restores tab
   order, folders, active tab and independent sorting. An explicit PATH opens one
   tab. Missing folders fall back to Home independently; valid other tabs and
   favorites survive. Selection, unfinished path text and navigation history are
   runtime state. Restoration and switching do not add visits.

Verification: temporary fixtures for per-tab state and stale replies; actual
AppKit tab selection, menu key equivalents and keyboard dispatch. Distinguish
callback/shortcut synthesis from a human pressing physical keys.
<!-- pdd-story-prompts: prompts/native_rust.prompt, prompts/model_rust.prompt -->
