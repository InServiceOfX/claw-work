# Progress

## Accepted meaning
Default mathlib-enabled offline image; Rust-first portable tooling; repository-wide
privacy gates; explicit separation of model proofs from source verification.
Lean 4.35.0/mathlib commit 56a005855ea17e0b0958f73ec801ae140b0a1679 is the default.
The supplied 4.37.0-pre source is an explicitly separate core-only build profile.

## PDD evidence
Approved intent b87acccd was recorded by the installed PDD intent apply.
The bounded architecture backend timed out during step 2/19 after six minutes.
It did NOT produce a complete prompt graph, generated story contracts, synchronization
or verification result. Operational partial state/logs remain untracked.
This session maintains the approved prompt graph/stories and implements from those
specifications directly. Deterministic PDD lint and actual tests are separate evidence.
No claim of successful automated PDD sync or semantic story validation is made.

## Implemented and locally verified
- Default offline Lean/mathlib image and separately built supplied core-only source profile.
- Non-root/run/exec/program/LSP acceptance; actual context and saved-image layer canaries.
- Rust recursive bounded inventory/private snapshots, strict export-only primitive/statement/axiom/kernel checking and typed reports.
- Separate root privacy prompt/story, indexed-blob gate, opt-in local hook and no-upload CI configuration.
- Portable review skill/workflows, concrete synthetic bug/repair, and recurrence-derived optimization with scoped model proofs and independent timing.
- Formatting/clippy, 11 ordinary Rust tests, 9 live tests, six named example obligations, shell smoke/canary tests and strict PDD structural lint passed locally.

See VERIFICATION.md for exact image identities, measurements, evidence and limits.
No automated PDD sync/semantic validation, native harness-loader certification,
source-code refinement proof, remote CI run, push or image publication is claimed.
Human publication review remains required. Existing unrelated PDD adoption work is preserved.

## Local five-harness deployment follow-up

The new accepted deployment intent is `ed75a262`. Planning and durable intent
capture completed in this subproject; the incremental architecture backend was
interrupted at a 30-second bound before generated prompts/sync/story validation.
The invoking harness maintained the affected prompts/story/architecture/tests.

Added native ARM64 stable-profile pins alongside AMD64, explicit image/build and
no-overwrite installation tooling, a bounded raw Lean/LSP launcher, self-contained
`lean-proof` and `lean-review` skills, and five-harness setup documentation.
The supplied public source revision is built separately as a native ARM64 core-only
image, not substituted into the stable mathlib checker. All global installation
here is covered by the user's explicit local deployment request; no existing
harness/provider/permission configuration is modified.

See docs/LOCAL_SETUP_EVIDENCE.md for current actual tests, image identities,
discovery observations and memory limits. The earlier VERIFICATION.md is historical
Linux x86-64 evidence, not a claim that this follow-up reran those tests remotely.
