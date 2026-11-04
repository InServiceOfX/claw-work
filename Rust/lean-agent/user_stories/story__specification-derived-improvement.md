# Better implementations from the specification

As a user, I want recommendations derived from mathematics, not just a description of
my existing code, with scoped correctness evidence and actual performance measurements.

Acceptance: a synthetic business-rule bug has a concrete Rust reproducer and repair;
an algorithmic improvement has Lean equivalence/cost claims under stated Nat semantics
and a separate optimized Rust benchmark. Machine overflow/representation assumptions
and unproved source correspondence remain explicit. Input source is never silently rewritten.
<!-- pdd-story-prompts: ["prompts/workflow_markdown.prompt", "prompts/report_rust.prompt"] -->
