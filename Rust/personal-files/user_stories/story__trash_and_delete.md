<!-- pdd-story-prompts: prompts/operations_rust.prompt, prompts/ui_rust.prompt -->
<!-- pdd-story-dev-units: prompts/operations_rust.prompt, prompts/ui_rust.prompt -->

# Remove files and folders from the context menu

As a Personal Files user, I want to right-click selected files or folders and
move them to Trash or delete them so I can manage them as I do in Ubuntu Files
or Finder.

Source: the user's 2026-11-08 request. This story is independent of current code.
Status: approved by the user on 2026-11-08 ("approve please").

## Acceptance

1. Right-clicking a file or folder MUST offer Move to Trash and a clearly
   separate Delete Permanently action. A selected group MUST be supported.
2. Move to Trash MUST use the desktop trash service. An unavailable or failed
   trash operation MUST report the error and MUST NOT silently become a
   permanent deletion.
3. Delete Permanently MUST show a confirmation naming the target(s) before
   deleting files or nonempty directories. Cancel MUST preserve every target.
4. Removing a symbolic link MUST affect the link itself, never its target.
   Browsing, copying paths, pinning and merely opening the menu MUST NOT
   remove anything.
5. A completed operation MUST refresh the directory listing. Failures and
   partial failures MUST remain visible instead of reporting complete success.
6. Verification MUST use disposable fixtures, including nonempty folders,
   links, cancellation and failures, without touching the user's documents.
