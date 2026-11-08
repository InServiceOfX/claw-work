# Personal places, one click away

As an Ubuntu user, I want a Finder-inspired file browser that remembers places
I visit, lets me pin favorites, and makes navigating and copying paths immediate,
so I can reach my usual directories without repeatedly clicking through folders.

Source: the original request, the reviewed intent plan, approval to build
and iterate, and subsequent explicit preference for Rust. This story is derived
from that conversation, independently of the implementation.

## Acceptance

1. Visit a deeply nested folder once, close the app, and reopen it: the folder
   appears in Recent and the last folder reopens. Repeated visits also rank it
   in Frequent. Explicit favorites remain separate and persistent.
2. Pin a folder with the star, context menu, or folder drag into the sidebar.
   Open it with one sidebar click in the current tab; double-click also displays
   it immediately. Clicking the current folder again refreshes its contents
   without adding a visit. Other tabs keep their locations. Remove the favorite
   without changing the source folder.
3. Paste an absolute path, home-relative path or path relative to the displayed
   folder into the always-visible path bar and press Enter. No Go menu is needed.
   A file path selects that file in its parent directory.
4. Hidden files and folders appear from the first launch. Click Name, Modified
   or Size to sort; click again to reverse. Folders stay first. No recursive
   folder-size calculation delays browsing.
5. Copy the full path of a file, folder, multiple selection or current folder
   into the desktop clipboard. A separate shell-quoted form safely preserves
   spaces, apostrophes and shell metacharacters when pasted into a terminal.
6. Back, forward, parent-folder navigation and documented keyboard shortcuts
   work. Open a document using Ubuntu's default application.
7. Switch Silver, Midnight, Aurora or Automatic skin, or choose a local
   background image. The choice survives restart. Automatic uses file types;
   empty/mixed folders have a usable fallback. Invalid images retain the prior
   working selection.
8. Missing/unreadable paths report an error and retain the current view and
   favorites. Disconnected favorites remain available for later use/removal.
   Corrupt preferences are preserved during recovery. Source files are never
   renamed, moved or deleted by browsing, path copying, or favoriting.
9. Folder changes refresh the view without counting as extra visits. Clearing
   visit history preserves manual favorites. Rapid navigation must not display
   stale results from an earlier folder scan.
10. The maintained implementation and installed application are Rust on Ubuntu.
    Remove the temporary Python project once Rust passes the acceptance checks.

## Evidence

`tests/acceptance.rs` exercises filesystem, sorting, persistence, quoting and
negative cases. `ui::tests::desktop_story` runs on a real GTK display and checks
widget signals, clipboard contents, sidebar navigation, restart, sorting, drag
URI handlers, appearance, monitor updates and stale scan results. Drag handler
coverage does not claim physical pointer-driven drag interoperability.

The PDD-generated machine contract and LLM semantic alignment check are not
available from the interrupted architecture run. They are not claimed as passed.

<!-- pdd-story-prompts: prompts/core_rust.prompt, prompts/appearance_rust.prompt, prompts/ui_rust.prompt -->
<!-- pdd-story-dev-units: prompts/core_rust.prompt, prompts/appearance_rust.prompt, prompts/ui_rust.prompt -->
