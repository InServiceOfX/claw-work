# Sources and Evidence Ledger

Verified 2026-11-07. ArXiv metadata checked against arxiv.org pages.

## Evidence grades

- **A**: Primary artifact with released data/logs, independently recomputed or
  re-run by at least one external party.
- **B**: Primary peer-reviewed method paper or widely replicated technique with
  multiple independent implementations.
- **C**: Primary preprint, single runs, not yet independently replicated. Default
  for new arXiv submissions.
- **D**: Commentary, opinion, or secondary summary.
- **X**: Conjecture, forecast, or explicitly stated future work.

## Core WSP sources (non-arXiv)

| ID | Title | Authors | Date | URL | Type | Grade |
|---|---|---|---|---|---|---|
| S1 | Sparks of Weight-Space Programming | Takehi, Teoh, Zhang, Khattab (MIT CSAIL) | 2026-11-02 | [Blog](https://rikiyat.github.io/blog/weight-space-programming/) | Blog post | C (results); X ("What comes next") |
| S2 | Exp 8 logs (lm_understand-logs) | R. Takehi | 2026-11 | [GitHub](https://github.com/RikiyaT/lm_understand-logs) | Logs, diffs, rebuild code | A |
| S3 | Qwen3.5-0.8B-weight-space-programming | R. Takehi; re-run by A. Khaliq | 2026-11-02 | [HF](https://huggingface.co/RikiyaT/Qwen3.5-0.8B-weight-space-programming) | Edited checkpoint, Apache-2.0 | A |
| S4 | Does "Sparks of Weight-Space Programming" hold up? | Achintya K J | 2026-11-04 | [Blog](https://achintyakj.tech/blog/weight-space-audit) | Independent audit | A/C (single-sample eval on 200 problems) |
| S5 | The "Astra can read and write weights…" post is misleading | M. Honnibal | 2026-11-03 | [Blog](https://honnibal.dev/blog/astra-read-write-misleading) | Critique | D |
| S6 | Weight-Space Programming from scratch on simple tasks | C. Singh | 2026-11-02 | [Report](https://csinva.io/weight-space-programming-autoresearch/analyze/toy_examples/report.html) | Independent toy experiments (MIT) | C |
| S7 | LLMs are not superhuman at reading and writing weights | S. Goedecke | 2026-11-02 | [Blog](https://www.seangoedecke.com/llms-are-not-superhuman-at-reading-and-writing-neural-network-weights/) | Opinion | D |
| S8 | Qwen3-0.6B model card | Qwen team | live | [HF](https://huggingface.co/Qwen/Qwen3-0.6B) | Vendor doc (Apache-2.0) | B |
| W | Wikipedia: Weight-space programming | — | accessed 2026-11-07 | [WP](https://en.wikipedia.org/wiki/Weight-space_programming) | Discovery only | — |

## arXiv papers

### WSP follow-ups (November 2026)

| ID | arXiv | Title | Authors | Posted | Grade | Note |
|---|---|---|---|---|---|---|
| P1 | [2611.08070v1](https://arxiv.org/abs/2611.08070) | Can an LLM Actually Reason and Program about Neural Network's Weights… | N. S. Ahmed | 2026-11-05 | C | Skeptical; label permutation breaks semantic interpretation |
| P2 | [2611.05341v1](https://arxiv.org/abs/2611.05341) | Weight-space programming for auditing clinical decision networks | H. Nusrat, O. Nusrat | 2026-11-03 | C | Tiny nets (433–4,801 params); single model; cannot certify correctness |
| P20 | [2611.07319v1](https://arxiv.org/abs/2611.07319) | WeightLoop | M. Li et al. | 2026-11-05 | C | 5 pages, offline replay |
| P21 | [2611.04268v1](https://arxiv.org/abs/2611.04268) | Weight-Space Programming for Improving Image-Restoration Diffusion Models | Z. Zheng, Doubao | 2026-11-03 | X | Self-declared conjectural |

### Prior art: weight orthogonalisation and steering

| ID | arXiv | Title | Authors | Posted | Grade |
|---|---|---|---|---|---|
| P8 | [2406.11717v3](https://arxiv.org/abs/2406.11717) | Refusal in Language Models Is Mediated by a Single Direction | Arditi et al. | 2024-06-17 | B |
| P9 | [2503.22048v4](https://arxiv.org/abs/2503.22048) | ThinkEdit: Interpretable Weight Editing to Mitigate Overly Short Thinking | Sun, Yan, Weng | 2025-03-27 | B |
| P5 | [2308.10248v5](https://arxiv.org/abs/2308.10248) | Steering Language Models With Activation Engineering (ActAdd) | Turner et al. | 2023-08-20 | B |
| P6 | [2312.06681v4](https://arxiv.org/abs/2312.06681) | Steering Llama 2 via Contrastive Activation Addition | Panickssery et al. | 2023-12-09 | B |
| P22 | [2506.18167v4](https://arxiv.org/abs/2506.18167) | Understanding Reasoning in Thinking Language Models via Steering Vectors | Venhoff et al. | 2025-06-22 | B |

### Foundations: compiled transformers and circuits

| ID | arXiv | Title | Authors | Posted | Grade |
|---|---|---|---|---|---|
| P3 | [2106.06981v2](https://arxiv.org/abs/2106.06981) | Thinking Like Transformers (RASP) | Weiss, Goldberg, Yahav | 2021-06-13 | B |
| P4 | [2301.05062v5](https://arxiv.org/abs/2301.05062) | Tracr: Compiled Transformers as a Laboratory for Interpretability | Lindner et al. | 2023-01-12 | B |
| P23 | [2012.14913v2](https://arxiv.org/abs/2012.14913) | Transformer Feed-Forward Layers Are Key-Value Memories | Geva et al. | 2020-12-29 | B |
| P28 | [2610.22891v1](https://arxiv.org/abs/2610.22891) | What Accuracy Hides In Learned Transformer Programs | Lee, Oozeer, Quirke | 2026-10-16 | C |

### Boundaries: task arithmetic and merging

| ID | arXiv | Title | Authors | Posted | Grade |
|---|---|---|---|---|---|
| P10 | [2212.04089v3](https://arxiv.org/abs/2212.04089) | Editing Models with Task Arithmetic | Ilharco et al. | 2022-12-08 | B |
| P14 | [2209.04836v6](https://arxiv.org/abs/2209.04836) | Git Re-Basin: Merging Models modulo Permutation Symmetries | Ainsworth et al. | 2022-09-11 | B |
| P15 | [2202.05262v5](https://arxiv.org/abs/2202.05262) | Locating and Editing Factual Associations in GPT (ROME) | Meng et al. | 2022-02-10 | B |

## Key claim verification

| Claim | Value | Source | Independent check |
|---|---|---|---|
| Exp 8 MATH-500 base → edit (50 passes) | 7.9% → 48.1% | S1, S2 | S3: 51.0%; S4: 51% (200 subset) |
| Exp 8 token-limit hits | 93.0% → 33.1% | S1, S2 | S3: 31.6% |
| Exp 8 parameters changed | 131,072 (0.016%) | S2 | Verified from manifest |
| Decoding baseline (rep_penalty 1.1) | 26% MATH-500 subset | S4 only | Not independently replicated |
| Gemma 3 270M no gain | 6.8% → 7.0–7.7% | S1 | S4: strict 2.6–3.8% |
| Raw-weight experiments (no tools) | 17K–172K params | S1, S5 | Confirmed from S1 access table |

## Video analysis

See `video-analysis/` for caption-based notes on Matthew Berman's coverage. Video
host claims are secondary; numbers above come from primary sources.
