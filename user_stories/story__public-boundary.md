# Public without private data

As the repository owner, I want all agent-authored work to remain publicly shareable
without exposing my private data, source, credentials or local environment.

## Acceptance

- Only synthetic/public fixtures and portable paths enter the public tree.
- The complete staged/indexed tree is checked; staging a safe working copy over an
  unsafe indexed blob must not bypass the gate.
- Private artifact paths, recognizable secrets and personal paths are rejected
  without printing secret contents.
- Docker uses an independently tested allowlisted context. Private canaries must
  not enter the image, history, labels or publication logs.
- Private review reports and proof workspaces remain outside Git trees.
- CI runs without provider credentials and does not upload review artifacts.
- Publication still requires human privacy review; a scanner is not a universal guarantee.

<!-- pdd-story-prompts: ["prompts/public_boundary_rust.prompt"] -->
