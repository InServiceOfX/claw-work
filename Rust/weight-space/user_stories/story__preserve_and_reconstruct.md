# Preserve the source and reconstruct an auditable edit

As a local experimenter, I want to apply a published model edit to a fresh copy
and prove exactly what changed, so I can reproduce it and return to the base.

Source: the approved conversational request and PRODUCT_INTENT.

Acceptance: the original is byte-identical after success or failure. A wrong
base hash, duplicate index, unsupported tensor, source alias or existing output
is rejected. A valid edit produces the exact published edited tensor hashes.
An independent audit rejects a changed header, protected tensor, extra changed
entry, omitted replacement or truncated file. Reports distinguish touched from
actually changed entries. No model data is stored in Git.

<!-- pdd-story-prompts: prompts/gguf_rust.prompt, prompts/edit_rust.prompt -->
