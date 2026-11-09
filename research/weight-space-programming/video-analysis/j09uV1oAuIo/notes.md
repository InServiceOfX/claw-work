# The most important paper in AI right now

- Creator: Matthew Berman
- Canonical video: https://www.youtube.com/watch?v=j09uV1oAuIo
- Publication date (YouTube date field): 2026-11-04
- Duration: 26:47
- Research date: 2026-11-07

## What this covers

An explanation of **Sparks of Weight-Space Programming**, a research blog by
Rikiya Takehi, Jayden Teoh, Alex Zhang and Omar Khattab (2026-11-02). The host
calls it "paper"; the primary source identifies its citation as a blog post
(visible on-screen at ~1:54). No arXiv identifier exists for this work.

## Confirmed visual corrections

| Timestamp | Host audio (ASR) | On-screen / Primary source | Note |
|---|---|---|---|
| ~1:54 | "paper" | On-screen shows "post" | Source is a blog post, not a peer-reviewed paper |
| ~19:47 | "Gemma 2 27B" (ASR) | On-screen: Gemma 3 270M | Size differs by factor of 100 (270M vs 27B); architecture differs |
| ~24:25 | Glossed over | Negative Gemma result table | Host does not emphasise the control failure |

## Timestamped claim map

### Experiments with raw weights only (no tools, no forward passes)

- **00:00–03:58**: Defines WSP as an LLM reading/writing another network's
  parameters. Broad claims about billions/trillions of weights are rhetoric,
  not demonstrated scale.
- **04:04–05:15; 06:24–08:46**: Four game-policy identification (Exp 1). Networks
  have 17,253 weights. Input channels were labelled by role in the prompt.
- **08:47–10:24**: Three small diffusion models (Exp 2); 23,824 weights. Shape
  identity inferred but filled shapes drawn as outlines. This imperfection
  matters—the demonstration is not full behavioural reconstruction.
- **05:16–06:23**: Kylon sponsorship (not research evidence).

### Experiments with Python tools (no forward passes)

- **10:25–12:36**: Character-level language models (Exp 3); 7.16M weights. Python
  analysis allowed, up to 100 runs. Model execution forbidden. Intermediate
  blocks only (input/output layers removed).

### Experiments with tools and forward-pass budget

- **12:37–15:13**: Car/ramp policy repair (Exp 4); 171,664 weights. Receives scene
  description and one trajectory (not weights alone). Reported 0%→100% success.
- **15:14–16:39**: MinAtar policies from scratch (Exp 5); ~20K weights. Strong
  against selected PPO baselines; Seaquest worse than 500M-step PPO, Asterix
  roughly matches, Breakout much stronger (517 vs 25).
- **16:40–19:47**: Go policy construction (Exp 6); 1.48M weights. Uses Python
  and 0–50 forward-pass batches. A "forward pass" is up to 128 games or 256
  positions—a batch/rollout budget, not single inference.

### Sycophancy identification (negative result)

- **19:48–21:46**: Exp 7. **Failed using weights alone.** Succeeded only after
  allowing attention-pattern observations (up to 16 texts, both models run,
  attention weights returned). Preserve this negative result.

### Qwen3.5-0.8B loop repair (headline result)

- **21:47–24:45**: Exp 8 starts ~21:42. MATH-500 7.9%→48.1%, GSM8K 13.8%→67.1%
  after 50 forward-pass budget units. IFEval 21.0%→62.3%. Edit touches 131,072
  parameters (0.016%). Token-limit hits fall from 93.0% to 33.1%—**not zero**.
  Loops reduced, not eliminated.

### Gemma control (negative result)

- **24:10–24:46**: Gemma 3 270M does not materially improve (6.8%→≤7.7%). Host at
  ~19:47 says "Gemma 2 27B" (ASR); on-screen shows Gemma 3 270M. Primary source
  confirms 270M—smaller than Qwen3.5-0.8B (270M vs 800M), not larger.
  **Repairing a pathology ≠ adding missing mathematical ability.**

### Speculation

- **24:47–26:47**: Interpretability, recursive self-improvement, all-software-as-
  weights ideas. These are forecasts, not demonstrated outcomes.

## Access-per-experiment table (primary-source verified)

| Exp | Task | Params | Raw weights | Python | Forward passes | Tools for exec |
|---|---|---|---|---|---|---|
| 1 | Game ID | 17,253 | Yes | No | No | No |
| 2 | Diffusion shape | 23,824 | Yes | No | No | No |
| 3 | Language/heads | 7.16M | Middle blocks only | Yes (≤100 runs) | No | No |
| 4 | Car repair | 171,664 | Yes | One reply | One trajectory | No |
| 5 | MinAtar | ~20K | Yes | No | No | No |
| 6 | Go | 1.48M | Yes | Yes | 0–50 batches | No |
| 7 | Sycophancy | 270M | Yes (failed) | Yes | Attention only | No |
| 8 | Math loops | 0.8B | Via kernel | Yes (≤300 runs) | 10/25/50 batches | No |

Source: S1 access descriptions and S2 exp8_math/README.md. Do not adopt
generalised "Experiments 1–5 no tools" summaries without verifying each
experiment's actual access conditions.

## Evidence and uncertainty

- Reviewed full ASR caption transcript and sampled stills; no continuous video
  playback or independent human listening.
- Full visual review completed 2026-11-07 (separate pass); corrections above
  confirmed against primary text and on-screen evidence.
- ASR errors: "Astro/Asher" for Astra, "Quen/Quinn" for Qwen, "GSMAK" for GSM8K,
  "Sequest" for Seaquest, "Asterisk" for Asterix.
- Benchmark values checked against primary blog and 23:50 frame; not independently
  reproduced.

## Assessment for reusable workflow

Useful as experiment-design briefing, not a ready-made model-upgrade skill.

A safe workflow would:
1. Snapshot/hash a disposable checkpoint.
2. Establish a concrete measured failure with decoding-only baselines.
3. Expose bounded architecture/tensor summaries to an analyst.
4. Propose a minimal reproducible edit.
5. Score held-out performance, repetition, instruction following, latency and
   regressions against unchanged and sham-edit controls.
6. Keep explicit rollback.

The 0.8B model is NOT interchangeable with another checkpoint. Layer indices,
directions, tokenizer, architecture, quantization and failure mode must match.

**Do not adopt video-model suggestions** such as "print high-magnitude rows to
detect induction heads" or "outline drawing proves the model understands shapes".
These are not justified by controlled experiment; use exhaustive tests instead.

This ingest performed no local-model experiment and tested none of the proposed
changes. Video host claims are secondary; verify all numbers against primary
sources before implementation.
