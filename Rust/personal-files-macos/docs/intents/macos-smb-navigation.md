# macOS editable paths and SMB — 2026-11-14

Action: add. Status: accepted for implementation by the original request.
Scope: Personal Files macOS only.

Ernest wants to alternate central GUI directory browsing with manual editing of
the full path near the top, pressing Enter to navigate, including Finder-style
SMB connections by IP with an optional username. He likes macOS saving the
login credentials once and reusing them afterward, and explicitly asked for
stories followed by implementation.

Implementation interpretation: reuse native NetAuth/Keychain authentication;
accept server-only URLs (share picker) and share/subfolder URLs, and show the
resulting mounted POSIX path. Add Go > Connect to Server / Command–K while
retaining Command–L and existing local-path behavior. Do not store passwords.

Acceptance: ../../user_stories/story__macos_smb_navigation.md.
Prompts: native_rust.prompt and model_rust.prompt. No Ubuntu changes.

Read-only PDD planning ran successfully (after sandbox process-inspection
restriction). Its lexical candidate selected install_bash.prompt, which is not
the responsible unit; code inspection identified native/model instead.
No authorized generation provider was selected. This harness maintained the
scoped story, prompt, code and tests; automated apply/sync, generated contracts
and semantic alignment are not claimed.
