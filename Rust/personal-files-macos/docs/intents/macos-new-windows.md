# macOS New Window — 2026-11-14

Action: add. Status: accepted for implementation by the original request.
Scope: Personal Files macOS only; preserve the accepted SMB work.

Ernest asked for another separate Personal Files window. Implement File > New
Window / Command–N, one tab at the current folder, with independent browsing,
tabs and connection state. Share favorites/history/appearance. Closing any
window leaves others open; closing the last leaves New Window available.
Preserve existing tab-session persistence without inventing all-window restore.

Story: ../../user_stories/story__macos_new_windows.md.
Affected prompts: native_rust.prompt and verify_bash.prompt.

Read-only PDD plan i-like-it-already-thank-you-one-more-as-a-user-i-d27a2c1a
ran against this subproject. Its question about product area is resolved by
repository inspection: native window lifecycle/routing, not a new product.
No additional provider was authorized. This harness maintains scoped prompt,
story, code and acceptance tests; no automated apply/sync, generated contract
or semantic validation is claimed.
