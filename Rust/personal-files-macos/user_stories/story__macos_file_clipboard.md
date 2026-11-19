# Finder-style file copy, paste and move

As a Personal Files user, I want to copy and paste files and folders using familiar
macOS conventions, so I can organize them and exchange selections with Finder.

1. Select one or more files or folders, press Command–C, navigate to another
   folder, then press Command–V: copies appear there and originals remain.
2. Copy items, navigate to a destination, and press Command–Option–V: the items
   move there within the same volume. Moving between volumes currently reports
   an unsupported error and retains the source; copying between volumes is allowed.
   A normal paste must never remove or move the originals.
3. Native Edit and context menus expose the corresponding actions. File URLs
   work in both directions with Finder. The path editor retains normal text
   editing; explicit full-path and shell-quoted path copying remain available.
4. Existing destination items are never silently overwritten. Errors and partial
   operations are visible. Copying or moving into the source or its descendant,
   including via a symlink alias, must not damage data or recurse indefinitely.
5. Symlinks remain links. A failed cross-volume move must not delete the source.
   Test with synthetic data only; retain all existing browsing and native tests.
6. Once verified, install the rebuilt native app while keeping the prior app
   recoverable; do not replace Finder or reset user preferences.

<!-- pdd-story-prompts: prompts/native_rust.prompt, prompts/file_ops_rust.prompt -->
