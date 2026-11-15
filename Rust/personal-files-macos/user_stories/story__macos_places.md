# My familiar places on my Mac
As a macOS user, I want the Ubuntu Personal Files browsing features in a native
Rust Mac app, so my familiar folders and path workflows are immediately available.

Source: Ernest's macOS request and approval, independently of generated code.
1. Open a deep or hidden folder from the always-visible path bar; absolute,
   home-relative and current-folder-relative inputs work. A file path selects
   the file in its parent. Preserve symlink aliases and spaces/Unicode.
2. Pin/unpin by star, context menu or local directory drag. One sidebar click
   opens a place. Favorites, Recent/Frequent and last folder survive restart.
   Unavailable favorites stay removable. Automatic history never alters pins.
3. Show Name, Modified, Size; reversible sorting with folders first, dotfiles
   always visible, multiple selection, no recursive folder totals.
4. Copy actual paths or separately shell-quoted paths to the Mac clipboard.
   Native path-field text copying keeps working. Path text never executes.
5. Open documents with their registered Mac app; back/forward/up, refresh and
   Command shortcuts work. Folder changes refresh without increasing visits.
   Rapid navigation never displays an older scan.
6. Silver, Midnight, Aurora, Automatic and a local image background persist.
   Automatic uses extensions only; invalid images retain the working image.
7. Missing/unreadable paths retain the displayed location and favorites.
   Show errors. Corrupt state is backed up before recovery. No browsing/copy/
   favorite/drop operation moves, renames or deletes documents; no telemetry.
8. Build and launch a .app without sudo or replacing Finder. Retain Ubuntu.

Verification must distinguish model tests, native widget/callback tests and
physical drag/default-document-opening checks. Temporary fixtures only.
<!-- pdd-story-prompts: prompts/core_rust.prompt, prompts/model_rust.prompt, prompts/native_rust.prompt, prompts/build_bash.prompt -->
