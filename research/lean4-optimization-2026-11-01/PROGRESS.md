# PROGRESS: docs/research/lean4-optimization

## 2026-11-01: research only

- Identified the submission from Arjay McCandless's TikTok: `lean-derive-hargup`
  (Harsh Gupta, @hargup), PR #153 in `arjaythedev/twelve-dollar-server-challenge`.
  Spec, invariant, and read plans are in Lean 4.35.0 with kernel-checked equivalence proofs,
  and a Lean generator emits the C. Officially 280,000 users / 836 MB.
- Assessment: worth a narrow skill + Docker-backed CLI ("prove fast plan =
  reference under invariants, then differential-test and benchmark"). Not built yet.
- Docker: no official leanprover image. The community `leanprovercommunity/lean4`
  is stale (2024-08-20), so the plan is our own pinned Dockerfile (sketch in README).
- Branch `bots/lean4-optimization-research` (yeung-assistant), PR into master.

## Next

- Ernest decides whether to build. If yes: `Scripts/lean4/Dockerfile` + wrapper,
  reproduce `lake build` of hargup's `spec/` in the container, then draft
  `shared/openclaw/skills/lean4-verified-optimization/SKILL.md`.
