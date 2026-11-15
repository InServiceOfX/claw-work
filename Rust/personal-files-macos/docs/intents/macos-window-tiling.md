# Request: standard macOS window-tiling hotkeys

Received: 2026-10-30. Kind: add.
Scope: Personal Files macOS native window integration; Ubuntu unchanged.
Delivery in this change: human-owned user story and linked intent/prompt source
only. Implementation, regeneration and behavioral verification are pending.

## Original request

> Add another user story related to Personal Files application. As a user I want to be able to use the hotkeys for tiling, described here: https://support.apple.com/en-ng/guide/mac-help/mchl9674d0b0/mac for my Personal Files application.

## Interpretation

Personal Files should participate in the native window-tiling actions and
hotkeys Apple documents for the running macOS version. Preserve normal browsing
shortcuts/state, including when the path editor or file list has keyboard focus.
No custom window manager, third-party utility, global shortcut remapping or
emulation of unavailable OS features is requested.

## Traceability and limits

- Read-only PDD plan: add-another-user-story-related-to-personal-files-6f1d9dc7.
  The planner did not identify a target; repository inspection resolves it to
  the native AppKit window integration in prompts/native_rust.prompt.
- User story: ../../user_stories/story__macos_window_tiling.md.
- Product Intent: ../PRODUCT_INTENT.md, “Native window tiling”.
- Apple source reviewed: https://support.apple.com/en-ng/guide/mac-help/mchl9674d0b0/mac.
- Human-owned story authored locally from the request and Apple source.
  No provider-backed apply/story generation, generated contract, semantic
  validation, code regeneration or runtime test is claimed.
