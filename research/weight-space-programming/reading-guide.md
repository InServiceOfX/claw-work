# Reading Guide: Ranked arXiv Papers for WSP

The central WSP proposal is **not an arXiv paper**. Start with the MIT CSAIL blog
post (S1) and the released Exp 8 logs (S2). The papers below are either WSP
follow-ups or the specific prior methods that the WSP experiments reproduce.

IDs reference `sources.md`.

## Tier 0: The proposal itself (not arXiv)

**0a. S1 — Sparks of Weight-Space Programming** (Takehi et al., 2026-11-02)
- URL: https://rikiyat.github.io/blog/weight-space-programming/
- Read for: the access table per experiment (raw text vs Python vs forward passes),
  the Exp 8 harness rules, the Gemma negative result.
- Caveat: single runs; "What comes next" section is forecast, not demonstrated.

**0b. S2 — Exp 8 logs README** plus **S4 — independent audit**
- URLs: https://github.com/RikiyaT/lm_understand-logs/tree/main/exp8_math ,
  https://achintyakj.tech/blog/weight-space-audit
- Read for: edit formula, protected tensors, sha256 manifest, fp32→bf16 build,
  decoding baseline comparison.
- Enables: reproducible experiment design.

## Tier 1: Essential for applying WSP-style edits

### 1. Arditi et al. — Refusal Direction
- **arXiv**: [2406.11717v3](https://arxiv.org/abs/2406.11717) (2024-06-17)
- **Title**: Refusal in Language Models Is Mediated by a Single Direction
- **Prerequisite**: Residual-stream basics (Elhage et al. 2021 Transformer Circuits)
- **Relevance**: The exact operator family used in Exp 8: difference-in-means
  direction plus W ← W − r̂r̂ᵀW on residual-writing matrices.
- **Caveat**: This is a jailbreak paper. Use the operator; never use the
  refusal-removal application.
- **Enables**: Track B edit operator and capability-regression checks.

### 2. Sun, Yan, Weng — ThinkEdit
- **arXiv**: [2503.22048v4](https://arxiv.org/abs/2503.22048) (2025-03-27)
- **Title**: ThinkEdit: Interpretable Weight Editing to Mitigate Overly Short
  Thinking in Reasoning Models
- **Prerequisite**: Attention-head attribution
- **Relevance**: Most direct prior art for Exp 8 type of edit (reasoning-length
  direction removed from output projections).
- **Caveat**: Addresses overly *short* reasoning (the opposite failure to loops);
  gains are a few points, not tens.
- **Enables**: Known-good baseline procedure for comparison.

### 3. Venhoff et al. — Reasoning Steering Vectors
- **arXiv**: [2506.18167v4](https://arxiv.org/abs/2506.18167) (2025-06-22)
- **Title**: Understanding Reasoning in Thinking Language Models via Steering Vectors
- **Prerequisite**: Linear representation hypothesis
- **Relevance**: Shows thinking-model behaviours (backtracking, uncertainty) are
  linear directions—conceptual basis for diagnosing loops as a direction.
- **Enables**: Diagnosis step (contrastive texts → direction → layer attribution).

### 4. Panickssery et al. — Contrastive Activation Addition
- **arXiv**: [2312.06681v4](https://arxiv.org/abs/2312.06681) (2023-12-09)
- **Title**: Steering Llama 2 via Contrastive Activation Addition
- **Prerequisite**: None beyond basic transformer knowledge
- **Relevance**: Cleanest description of building contrastive pairs and averaging
  activation differences.
- **Enables**: Activation-steering control arm before weight modification.

### 5. Nusrat & Nusrat — Clinical Network Audit
- **arXiv**: [2611.05341v1](https://arxiv.org/abs/2611.05341) (2026-11-03)
- **Title**: Weight-space programming for auditing clinical decision networks
- **Prerequisite**: None
- **Relevance**: WSP-native follow-up with blind audits, planted flaws, ~420K test
  cases, and comparison against simple statistics.
- **Caveat**: Tiny networks (433–4,801 params); single model (Claude Opus 5.5);
  cannot certify correctness.
- **Enables**: Track A design pattern (planted flaw → blind audit → exhaustive test).

### 6. Ahmed — Skeptical Weight-Reading Control
- **arXiv**: [2611.08070v1](https://arxiv.org/abs/2611.08070) (2026-11-05)
- **Title**: Can an LLM Actually Reason and Program about Neural Network's Weights…
- **Prerequisite**: None
- **Relevance**: Essential skeptical control. Label permutation shows reading relies
  on dataset priors; semantic claims track CIFAR-10 label order, not learned features.
- **Enables**: Permutation/relabel control for any "read the weights" claim.

## Tier 2: Foundations for writing weights from a spec

### 7. Weiss, Goldberg, Yahav — RASP
- **arXiv**: [2106.06981v2](https://arxiv.org/abs/2106.06981) (2021-06-13)
- **Title**: Thinking Like Transformers
- **Relevance**: Program primitives that attention and MLPs can implement.

### 8. Lindner et al. — Tracr
- **arXiv**: [2301.05062v5](https://arxiv.org/abs/2301.05062) (2023-01-12)
- **Title**: Tracr: Compiled Transformers as a Laboratory for Interpretability
- **Relevance**: Compiler from RASP to real weights—the deterministic baseline for
  comparing LLM-written weights.

### 9. Lee, Oozeer, Quirke — Accuracy Hides Failures
- **arXiv**: [2610.22891v1](https://arxiv.org/abs/2610.22891) (2026-10-16)
- **Title**: What Accuracy Hides In Learned Transformer Programs
- **Relevance**: Shows why 200 sampled examples are insufficient—use exhaustive
  evaluation over bounded domains.

## Tier 3: Boundaries (what WSP is not)

### 10. Ilharco et al. — Task Arithmetic
- **arXiv**: [2212.04089v3](https://arxiv.org/abs/2212.04089) (2022-12-08)
- **Title**: Editing Models with Task Arithmetic
- **Relevance**: Fixed arithmetic on fine-tune deltas. Requires same pre-trained
  initialisation. Not WSP, but the right tool for compatible fine-tunes.

### 11. Ainsworth et al. — Git Re-Basin
- **arXiv**: [2209.04836v6](https://arxiv.org/abs/2209.04836) (2022-09-11)
- **Title**: Git Re-Basin: Merging Models modulo Permutation Symmetries
- **Relevance**: Permutation symmetry. Explains why comparing weights across
  independently trained models is ill-posed without alignment.

### 12. Meng et al. — ROME
- **arXiv**: [2202.05262v5](https://arxiv.org/abs/2202.05262) (2022-02-10)
- **Title**: Locating and Editing Factual Associations in GPT
- **Relevance**: Wikipedia uses ROME to draw the boundary with WSP. Fixed-algorithm
  rank-1 edit for single facts.

## Not core reading

- WeightLoop (P20): 5 pages, offline replay only.
- P21 (diffusion restoration): self-declared conjectural.
- Merging variants (TIES, DARE, soups): fixed algorithms, not reasoning.
- Weight-generator/neural-functional papers: trained models that process weights
  (contrast cases, not WSP).
