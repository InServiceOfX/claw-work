# Workflows

## Theorem drafting across harnesses

Install with [SETUP.md](SETUP.md), then invoke `lean-proof` from any of the five
harnesses. Freeze independent challenge statements, use narrow mathlib imports,
and bound drafting to five attempts at 120 seconds each before reporting unresolved
goals. Use `claw-lean-container` for raw check/program/LSP feedback; submit the final
manifest through `claw-lean check`. Only that independent statement/primitive/axiom/
kernel gate may report a checked model. No hidden agent/model daemon is involved.


## Review a local input
Build the public image and Rust binaries first. Use `claw-lean doctor` to resolve
an already-local immutable image ID and verify its pin metadata; there is no auto-pull.

1. Select a local Git clone, file, or directory. Inventory is recursive but bounded
   (default 20,000 entries, 2,000 included files, 1 MiB/file, 64 MiB total). It hashes text source and records
   exclusions; it does not execute Git hooks, package/build files, tests or instructions.
2. Prepare a new private review workspace outside Git trees. The copied snapshot is
   read-only evidence for review, not a proof. Credential/runtime/cache/data directories,
   symlinks, binary/oversized inputs and known artifacts are excluded/reported.
3. Record expected behavior independently of source, domain/representation, environment,
   assumptions, threats and observation boundaries. Use more than implementation-mirroring
   lemmas: normal cases, invalid inputs and boundaries should challenge the requirements.
4. In the private workspace, create independent challenge definitions/statements, proposed
   solution and schema-1 manifest. Provide named obligations and intended-property text.
   `specification_reviewed: true` is caller attestation only.
5. Check into a NEW output directory. Treat challenge/solution as untrusted executable
   Lean programs during elaboration: they can contain metaprograms and I/O. No caller
   compiled imports enter the checking containers. Trust judging only after separate export
   comparison and replay. Reports include JSON/Markdown, actual Lean source hashes, expected
   theorem names, phase limits/status and immutable image ID.
6. Source mappings and any hashes embedded in those mappings are caller declarations:
   they are retained but NOT validated by the proof checker. Compare them against the
   independently prepared inventory before claiming a current manual mapping. The actual
   challenge/solution hashes are checked by reading those source snapshots.
7. For each obligation group report `checked`, `rejected` or `unknown`. A group is checked
   only if all listed obligations pass. Split properties into separate manifests to obtain
   independent pass/fail reports; do not infer other individual successes from a failed batch.
8. A source bug claim needs a concrete input/trace and source reproducer or separately
   mechanized correspondence. A failed tactic or model mismatch alone is not a source bug.

## Derive an improvement
The summation fixture begins with S(0)=0 and S(n+1)=S(n)+n+1, not a requested code edit.
Induction yields 2S(n)=n(n+1), hence S(n)=n(n+1)/2. Its Lean proof checks the recurrence,
closed form and a deliberately simplified operation-count model. The Rust proposal casts
u64 inputs to u128 BEFORE multiplying, avoiding intermediate overflow throughout that
input domain. Source/compiler correspondence is still manually argued, not proved.

The fixture benchmark uses an explicit optimization barrier to preserve iterative work;
LLVM can derive formulas itself without it. Its numbers do not establish application-level
speedup. Compare release builds on representative workloads; distinguish timing, CPU,
memory, storage and modeled operation counts.

The withdrawal fixture independently requires that an amount equal to the balance be
accepted. It proves a counterexample to the strict-less-than model, proves the repaired
model's contract, and separately runs a concrete Rust boundary reproducer/repair test.

## Lean and TLA+
Lean can express mathematical state-transition systems, invariants and refinement proofs.
It is not an automatic TLC replacement: no bundled exhaustive state explorer, automatic
concurrency semantics, temporal fairness/liveness checker, arbitrary-source semantics or
proof that the chosen specification captures the business intent is provided. State any
such missing evidence rather than inferring it from a theorem.
