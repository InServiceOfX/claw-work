# Review local code through explicit mathematical models

As a user, I want an agent to review a local clone/file/directory, formulate intended
properties and mathematical models, check them and report what is established or unresolved.

Acceptance: bounded read-only recursive inventory, skips and source hashes; private output;
source maps and language-semantics assumptions; independent intended requirements;
separate model-proof, source-correspondence and specification-review status.
A source bug finding needs a concrete reproducer, not merely an unsuccessful tactic.
<!-- pdd-story-prompts: ["prompts/input_rust.prompt", "prompts/proof_rust.prompt", "prompts/report_rust.prompt"] -->
