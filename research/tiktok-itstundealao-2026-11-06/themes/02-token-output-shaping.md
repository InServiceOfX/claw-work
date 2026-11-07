# Token and output-shaping skills

Research notes from @itstundealao demos of skills that modify AI agent output format or token usage. All figures are from single on-screen demos — they are anecdotes, not benchmarks.

## caveman — JuliusBrussee/caveman

- Video: [7684295430821235971](https://www.tiktok.com/@itstundealao/video/7684295430821235971)
- Repo: https://github.com/JuliusBrussee/caveman
- License: Apache-2.0
- Description: "Why use many token when few token do trick." Skill + proxy for coding agents that compresses output by removing articles, verbose phrasing.
- Provenance: Repo header `JuliusBrussee / caveman` legible on-screen.

### Demo observations (creator's single run)

- Creator tested on two prompts
- Observed ~30% fewer output tokens

### README claims (not independently verified)

The README claims ~65% token reduction. The creator's demo showed ~30% — a single-run data point, not a refutation or confirmation of the claim.

### Mechanism (from README inspection)

Operates as a response filter that rewrites verbose assistant output into terse form. Does not modify the model; post-processes output.

---

## ponytail — DietrichGebert/ponytail

- Video: [7694691465393507607](https://www.tiktok.com/@itstundealao/video/7694691465393507607)
- Repo: https://github.com/DietrichGebert/ponytail
- License: MIT
- Description: "Makes your AI agent think like the laziest senior dev in the room. The best code is the code you never wrote."
- Provenance: Repo name `ponytail` visible; owner inferred from GitHub search.

### Demo observations (creator's single run)

- ~10% cheaper on one simple build in demo
- README claims more aggressive savings

### Mechanism (from README inspection)

System prompt skill that biases the agent toward minimal implementations, avoiding over-engineering. Encourages reusing existing code over writing new code.

---

## i-have-adhd — ayghri/i-have-adhd

- Video: [7693954418621844758](https://www.tiktok.com/@itstundealao/video/7693954418621844758)
- Repo: https://github.com/ayghri/i-have-adhd
- License: MIT
- Description: "A skill to stop your coding agent from burying the answer. ADHD-friendly output."
- Provenance: Repo name `i-have-adhd` and commit author `ayghri` visible on-screen.

### Demo observations

Creator noted the answer-first output style — the conclusion comes before the explanation.

### Mechanism (from README inspection)

Prompt skill that instructs the model to:
- Lead with the answer/conclusion
- Put supporting detail afterward
- Avoid lengthy preamble

---

## Proposed reproducible benchmark (not implemented)

To compare these skills objectively, one could:

1. Select a fixed set of 10–20 coding prompts (e.g., from a public benchmark)
2. Run each prompt with baseline (no skill), caveman, ponytail
3. Measure:
   - Output token count
   - Completion time
   - Subjective code quality (blind review)
4. Report mean ± std across prompts

This was **not done** in the current research — figures cited are single-run anecdotes from the creator's demos.

---

## Summary table

| Skill | Repo | License | Demo observation | README claim |
|-------|------|---------|------------------|--------------|
| caveman | JuliusBrussee/caveman | Apache-2.0 | ~30% fewer output tokens (2 prompts) | ~65% reduction |
| ponytail | DietrichGebert/ponytail | MIT | ~10% cheaper (1 build) | More aggressive |
| i-have-adhd | ayghri/i-have-adhd | MIT | Answer-first style | N/A |

All repos verified live 2026-11-07.
