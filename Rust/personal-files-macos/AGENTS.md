# Personal Files — macOS
Native Rust + AppKit application. The Ubuntu app lives alongside this project at Rust/personal-files and is independent.
Read docs/PRODUCT_INTENT.md, prompts, user_stories and PROGRESS.md first.
Git scope follows the repository `BOTS.md` and `AGENTS.md`. No commits or pushes without an explicit instruction.
Use ./build.sh, ./launch.sh [PATH], ./install.sh (per-user, no sudo).
Cargo output stays in ~/Library/Caches/personal-files-macos/target.
Run cargo fmt --check, cargo clippy --locked --all-targets -- -D warnings,
cargo test --locked and ./verify.sh (opts into native desktop fixture tests).
AppKit runs on the main thread. Objective-C calls stay in native.rs and its
native_desktop.rs/native_tiling_test.rs acceptance helpers. bundle.rs has only
the narrowly scoped Darwin exclusive-rename FFI; file_ops.rs owns descriptor-relative
Darwin copy/rename and metadata FFI, without Objective-C. Native geometry tests must
return to the outer event loop between actions; never count a menu entry as
proof of resizing. Keep focus/frame/state assertions and accumulated tests.
Never use user documents/preferences as fixtures, never upload paths/content.
Browsing, copying paths/files, favoriting, and drops MUST NOT move/rename/delete
source documents. Only explicit file move-by-paste may relocate them; cross-volume
moves are refused safely rather than implemented with copy/delete.
Preserve symlink aliases. Never silently replace Finder or a prior installed app.
PDD prompts are durable intent. Automated apply/sync requires an authorized
provider; do not claim it passed when source is maintained in this harness.
Update PROGRESS.md and docs/VERIFICATION.md after material changes.
