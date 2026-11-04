# A proof result I can trust within its stated assumptions

As a user, I want the expected mathematical statements independently checked, so an
agent cannot obtain a passing report by admitting a theorem or changing the question.

Acceptance: reject sorry/admit or imported sorryAx, unexpected axioms/native oracle,
changed theorem statements/definitions, missing names, failed independent replay,
timeouts/output limits and fake source-side success text. Only pinned trusted imports;
no caller-produced compiled object enters the trusted judge. The independent challenge
is the specification boundary and may contain statement placeholders, never acceptance proof.
<!-- pdd-story-prompts: ["prompts/proof_rust.prompt", "prompts/report_rust.prompt"] -->
