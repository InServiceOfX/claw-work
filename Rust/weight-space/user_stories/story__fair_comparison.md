# Fair local comparison

As a researcher, I want the original, edited and decoding-control models to
answer the same frozen problems under matching budgets, so an apparent gain
cannot be explained by different prompts, token caps or hidden failed runs.

Source: the approved experiment scope and EXPERIMENT_PLAN.

Acceptance: record every attempt including errors, strict final-answer accuracy,
termination, repetition and actual tokens. Never grade a correct number inside
unfinished thinking as a correct answer. Pair reports by problem identity and
reject mismatched cases/settings. Retain negative results and state uncertainty.
Run on the selected local GPU without changing ordinary model settings.

<!-- pdd-story-prompts: prompts/evaluation_rust.prompt, prompts/cli_rust.prompt -->
