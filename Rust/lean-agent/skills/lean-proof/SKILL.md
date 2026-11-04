---
name: lean-proof
description: Draft, debug and independently check Lean 4 theorems with pinned mathlib in offline Docker. Use for formal mathematics, proof repair, goal feedback or Lean-powered reasoning; not automatic verification of source code.
---

# Lean theorem proving

Tools: `claw-lean` (strict checker), `claw-lean-container` (raw Lean/LSP feedback),
`claw-lean-setup` (explicit builds/install/status). Run `claw-lean doctor` first.
If absent, locate the claw-work checkout and follow Rust/lean-agent/docs/SETUP.md.
Do not fetch packages, run host lakefiles or switch the toolchain to make a proof pass.

1. Clarify the exact theorem, domain, definitions and permitted assumptions. Confirm
   privacy/provider consent before a remote model sees private mathematics or code.
2. Choose a new canonical private directory outside Git trees. See
   [the manifest template](references/manifest.json); paths are relative to the manifest.
3. Author independent expected named declarations in `Challenge.lean`. Only that
   challenge may use `sorry` to state an obligation. Freeze its meaning before solving.
4. Write `Solution.lean` independently with identical definitions and theorem types.
   Prefer narrow imports (e.g. `Mathlib.Data.Nat.Basic` or `Mathlib.Tactic`) and
   existing lemmas/tactics before inventing custom machinery. The full mathlib
   cache is bundled, but umbrella `import Mathlib` can exceed the worker memory budget.
   Never use `sorry`, extra axioms, `native_decide` or weakened statements as a solution.
5. For feedback:
   `claw-lean-container --input-dir PRIVATE_DIR check Solution.lean`.
   Start with a five-attempt, 120-second-per-attempt budget; inspect errors and make
   narrow proof edits. Stop and report the precise open goal on exhaustion.
6. For editor/client feedback:
   `claw-lean-container --input-dir PRIVATE_DIR --timeout-seconds 3600 server`.
   Use stdin/stdout framed JSON-RPC, no TTY. Document URIs live under /input.
   Empty diagnostics, raw elaboration and tactic success are NOT a certificate.
7. Final gate:
   `claw-lean check --manifest PRIVATE_DIR/manifest.json --output NEW_REPORT_DIR`.
   Only independent statement/primitive/axiom/kernel checks establish model acceptance.
   Exit codes: 0 checked, 1 rejected, 2 unknown/error. Preserve failures honestly.
8. Report exact theorem names, assumptions, toolchain/image identity, proof status and
   limits. Checking the mathematical model does not prove source correspondence,
   performance, physical validity or completeness of the chosen specification.

The source-build image is core-only and a different Lean revision; it cannot replace
the stable mathlib image in strict checking. No provider/model is selected by tools.
Do not publish private models/reports or change harness permission policy.
