# llama.cpp CUDA deployment — continuation

## Completed

- Moved launch.sh/chat.sh without rewriting either script; source bytes verified.
- Copied and corrected local ignored YAML; public templates contain no personal
  paths. Added missing portable Qwen3.8 Q4/Q5 examples.
- Audited all runtime model/mount/projector paths and excluded missing/LFS inputs.
- Loaded and measured seven real text quantizations plus two projector profiles.
- Selected Qwythos Q5 for Pi: 6/6 bounded checks, faster/lower-memory than Q6.
- Verified the normal Pi greeting, exact prime list, compiled Rust/authored tests,
  six independent cases, reasoning parsing and a separate actual addition tool.
- Added a restart wrapper and one-model Pi setup with scoped thinking/compaction.
- Shell/YAML checks and benchmark-grading regression tests pass.

## Verification and operation

Read BENCHMARKS.md for actual settings, image digest, measurements and limits.
`run-recommended.sh` starts Qwythos Q5 with bounded thinking; --stop releases GPU
memory. The local profile uses 64K context; other copied text profiles use 32K and
projector profiles 16K. Local YAML remains ignored and must never be staged.

Normal Pi configuration was updated only at the user's request. The previous
native Nemotron process was stopped and its inactive Pi entries replaced; source,
examples and model assets remain intact. Other providers/defaults were preserved.
Thinking off is supported but did not pass every coding check; the recommended
model's default is low. A working transport does not guarantee correct answers.

## Migration and publication boundary

Commit the reviewed destination before removing the source working copy. The
Monoclaw cleanup is a separate removal commit on a feature branch, with a pointer
to this deployment; private history is not imported or rewritten. Follow BOTS.md
for remote identity, feature-branch publication and human merge requirements.

Keep raw captures/host receipts outside public Git. The public aggregate report
separates smoke tests from general quality/vision claims. No missing model was
downloaded. Future model/image/context changes require renewed runtime checks.
