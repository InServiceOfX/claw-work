# Weight-Space Programming: Research Notes

Research survey on weight-space programming (WSP), November 2026.

## Summary

**What WSP is.** A general-purpose LLM, used as shipped (no task-specific training),
reads and/or writes the parameters of another network directly, guided by a
natural-language specification, **without an optimizer, gradients or a fitting loop**.

**Central source.** Takehi, Teoh, Zhang and Khattab (MIT CSAIL), "Sparks of
Weight-Space Programming", blog post dated 2026-11-02. **Not peer-reviewed, not on
arXiv.** The editor model was GPT-6 Astra.

**Headline result.** In Experiment 8, edits to 2 tensors (131,072 weights, 0.016%)
of Qwen3.5-0.8B took MATH-500 from 7.9% to 48.1% and reduced the share of answers
hitting the 32,768-token limit from 93.0% to 33.1% (not eliminated). Hugging Face
re-run: 51.0%. Independent audit (Achintya K J): 51% on 200 problems.

**The limitation.** That result is a **loop fix**, not new mathematical ability.
On Gemma 3 270M, which rarely loops, the same procedure gained nothing (6.8% to
7.0–7.7%). The method is a contrastive direction plus rank-1 projection on output
matrices—a known interpretability/abliteration technique.

**Raw-weight access.** Experiments where GPT-6 Astra received weights as text
without Python tools or forward passes were limited to networks of 17K–172K
parameters. Reading or improving billion-parameter models required Python tools
and a budgeted forward-pass quota.

**Practical verdict.** No general reproducible "edit weights by reasoning" method
exists yet. What can be reproduced locally:

1. **Toy WSP**: hand-written weights for tiny networks on known algorithms.
2. **Diagnostic narrow edit**: measure a failure mode, derive a contrastive
   direction, apply rank-1 orthogonalisation to a few output matrices, compare
   against decoding baselines on a copied checkpoint.

## Contents

- `sources.md` — Primary and arXiv sources with validation status
- `reading-guide.md` — Ranked arXiv reading picks with prerequisites and caveats
- `feasibility.md` — Local experiment design for Apple Silicon (generic hardware class)
- `SKILL.md` — Draft reusable workflow (experimental, not performance-proven)
- `video-analysis/` — Video commentary analysis (captions + sampled stills only)

## Key corrections from preliminary draft

1. Evidence grades replaced: "B = peer-reviewed or widely replicated" was overstated
   for single-run preprints. Grades now specify exact replication status.
2. "Most rigorous so far" removed for arXiv:2611.05341 (Nusrat & Nusrat)—no
   systematic comparison supports that claim.
3. Token-limit hits: 93.0% → 33.1%, not eliminated. Loops reduced, not removed.
4. Forward-pass budget clarified: a "pass" is a batch/rollout budget (up to 512
   problems or 1,024 texts), not a single inference.
5. Gemma 3 270M is smaller than Qwen3.5-0.8B (270M vs 800M), not larger.
6. ThinkEdit (arXiv:2503.22048) addresses overly *short* reasoning in DeepSeek-R1
   distills, the opposite failure mode to loops. Gains are a few points, not tens.

## Provenance

- No models downloaded, loaded or modified.
- No experiments executed.
- No benchmark independently reproduced.
- Video analysis based on automatic YouTube captions and sampled stills, not
  continuous playback or independent human transcription.
- Full primary-source blog and arXiv abstracts read; full text read for selected
  papers (arXiv:2406.11717, arXiv:2503.22048, arXiv:2611.05341, arXiv:2611.08070).
