# Product Intent — Personal Files

Status: approved first iteration, implemented in **Rust + GTK 3 for Ubuntu**.
Meaning can evolve through ordinary feedback; maintain requirements, prompts,
story and acceptance checks together.

## User outcome

The user wants familiar directories available immediately instead of repeatedly
clicking through a deep or hidden directory hierarchy. For example,
`~/Documents/Physics` should
become directly available after visiting it, and also be manually pinnable.

## Approved behavior

An always-visible path field accepts absolute, home-relative and current-folder
relative paths. Hidden files are visible by default. A Finder-inspired list
has Name, Modified and Size sorting and a sidebar of explicit Favorites plus
automatic Recent/Frequent places. All survive restarting. Pin by star,
right-click, or folder drag. Copy plain full paths or separately shell-quoted
paths to the desktop clipboard. Use default Ubuntu applications for documents.
Provide normal back/forward/parent navigation, keyboard shortcuts, local folder
monitoring, readable errors and selectable skins/backgrounds.

Recent/Frequent is the automatic “queue”; it does not silently modify manual
Favorites. Save the latest 200 distinct visited directories; show seven Recent
and five Frequent, ranked by recency and by count/recency respectively. Refresh
does not count as a visit. Preserve unavailable favorites for removable mounts.
Store local preferences privately and atomically. Do not alter source files as
a side effect of browsing, copying paths or favoriting.

Skins: Silver, Midnight, Aurora, Automatic. Automatic uses a majority of file
extensions as an explainable first version, with Silver for mixed/empty folders.
It does not inspect document contents or infer private topics. A user-selected
local background image is supported and persists. Images stay outside Git.

## Boundaries and choices

- Ubuntu native desktop only. The human explicitly deferred macOS to another
  computer/project.
- Rust is authoritative. The human corrected the initially proposed Python
  technology after approving the behavior, and requested removal/sunsetting of
  Python after Rust parity. Rust passes the acceptance checks; Python is retired.
- File creation, rename, move, delete, thumbnails and recursive folder
  sizes are outside this first browsing-focused iteration.
- No telemetry, network content processing, or automatic OS-default replacement.
- Existing unrelated repository files must be preserved; feature branch only.

## Request provenance

The original request captured by `pdd intent apply` is retained in redacted form under
`docs/intents/intent__ubuntu-finder-places-0eb63386.md`.
Personal absolute paths from that request are not republished. The record
keeps the historical Python scope/technology before the correction.
`docs/intents/rust-correction.md` preserves the later explicit Rust request and
approval to iterate. It supersedes technology and scope, not the accepted user
behavior. The original event is not rewritten to pretend Rust was chosen first.

The automated PDD architecture run was interrupted during step 2/19. This agent
then maintained the prompt graph, story, code and behavioral evidence directly.
No successful automated intent-apply/sync or generated contract is claimed.

<!-- pdd-intent-entry:can-we-also-be-able-to-open-tabs-in-the-personal-cdc1a062:start -->
## Can we also be able to "open tabs" in the Personal Files application?

- Intent event: [`docs/intents/intent__can-we-also-be-able-to-open-tabs-in-the-personal-cdc1a062.md`](intents/intent__can-we-also-be-able-to-open-tabs-in-the-personal-cdc1a062.md)
- Change kind: `add`
- Supersedes: none
- Scope: `existing_pdd_subproject_change`
- Technology: `not stated`

> Can we also be able to "open tabs" in the Personal Files application?
<!-- pdd-intent-entry:can-we-also-be-able-to-open-tabs-in-the-personal-cdc1a062:end -->

## Approved tabs behavior — 2026-10-16

Ctrl+T and the plus button duplicate the current folder in a new foreground
tab. Folder and saved-place context menus offer Open in New Tab. Each tab keeps
its folder, selection, sorting and back/forward history while the app runs.
Favorites and visit history remain shared. Ctrl+Tab/Ctrl+Shift+Tab cycle with
wrap; ×/Ctrl+W close a tab while retaining at least one. Background scan results
must not overwrite another tab, and switching must not count as a visit.

Restart restores tab order, folder paths, per-tab sorting and active tab.
Selections and back/forward stacks are runtime state, not persisted. Missing
saved folders fall back to Home in their own tab. Old preferences migrate by
opening the previous last folder with favorites and visit history preserved.
An explicit CLI PATH starts that requested folder rather than restoring the
previous session. See user_stories/story__tabs.md and docs/intents/tabs-approval.md.

The tabs `intent apply` captured its event but failed incremental architecture
because no model provider was configured. Prompts, story and Rust changes were
maintained directly; automated apply/sync/semantic detection is not claimed.
