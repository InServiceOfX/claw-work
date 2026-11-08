# PDD source and evidence

The original accepted intent and initial apply ID are preserved in `docs/intents/`.
The first session's PDD apply recorded intent, then stopped during architecture
generation. Its operational state says `applying`, not completed. It produced no
reviewed prompt or story outputs.

Continuation uses PDD 0.0.310.dev0. Read-only intent planning and command-help
discovery succeeded. The installed CLI falls back to bundled LiteLLM pricing
when offline; that warning does not mean planning failed. Planning correctly
identified the unfinished checkout as brownfield. The app-layer baseline ran
before adapter integration: **33 tests passed**, covering chunk preservation,
WAV assembly, reference preparation, output refusal, retries and cancellation.

The continuation preserves the already requested meaning. The three `.prompt`
files and independent user story are authored by the agent, not claimed as
PDD-generated or separately human-approved. `architecture.json` maps their
specific outputs. Numerical model modules remain research code characterized
against recorded upstream tensors, not targets for blind regeneration.

No new provider calls, remote generation, paid workflows or whole-project sync
were invoked. PDD apply/sync, generated story contracts, semantic story detection
and PDD certification are **not complete**. Local source edits and deterministic
tests must not be described as successful PDD regeneration.

A later session (2026-11-07) verified the already-built native CLI on this Mac:
a multi-chunk narration, speech recognition of that WAV, unload memory, the
Rust suite, the Python protocol/lifecycle/packaging tests, and a local install.
That verification did not regenerate prompts or code. See `docs/HANDOFF.md`.
