# Agent workflow patterns from @itstundealao TikTok (2026-11-07)

**Source:** Public TikTok account [@itstundealao](https://www.tiktok.com/@itstundealao), Nov 2026 audit.
**Captured:** 2026-11-07

**Deduplication note:** The `latent-spaces/brag` finding is already documented in [research/tiktok-mcp-and-skills-2026-11-06](../tiktok-mcp-and-skills-2026-11-06/README.md). Link rather than repeat.

---

## Coverage ledger (audit scope, not "all watched")

| Item | Count |
|---|---|
| Videos shown on public profile page (2026-11-06) | 274 |
| IDs enumerated via yt-dlp metadata | 262 |
| Gap: shown but not listed | 12 (cause unknown) |
| Captions/metadata screened | 262 |
| Transcribed locally (faster-whisper `small`, CPU) | 90 (all 63 from 2026 + 27 older) |
| Frame inspections | 22 |
| Visual video reviews (video-capable model) | 4 |
| Caption-only, not transcribed | 172 older posts |

Older posts were caption-screened and 22 likely practical ones transcribed; the rest are **not proven irrelevant**, only unreviewed at transcript depth.

---

## 1. Goals/context file + weekly planning workflow

**Source:** [7698399869001567510](https://www.tiktok.com/@itstundealao/video/7698399869001567510) (2026-10-19), transcribed.

The creator describes writing weekly goals into a project-level instruction file (he names it similar to "CLAUDE.md" for Claude Code) and scheduling a weekly agent task to turn those goals into a task list. He mentioned Linear integration for task tracking.

**Paraphrase of the pattern:**
1. Maintain a living goals file in the project root that describes what you want to accomplish this week in plain language.
2. Configure a recurring agent run (manual trigger or scheduled, e.g., start of week) that reads the goals file and produces concrete tasks.
3. Optionally sync those tasks into a tracker (Linear, Todoist, etc.) for human triage.

**Application:** Aligns with PROGRESS.md + AGENTS.md patterns. A scheduled Claude Code or similar agent task reading `PROGRESS.md` and emitting a task list is a direct application. No extra paid SaaS required beyond existing Claude/Codex plans (Linear is optional; local Markdown works).

**Caveat:** single demo, no measured productivity claim; treat as a workflow shape, not a benchmark.

---

## 2. Email triage → rough user replies → full drafts for user review (never auto-send)

**Source:** [7620488604678229270](https://www.tiktok.com/@itstundealao/video/7620488604678229270) (2026-03-23), transcribed.

The creator shows a Gmail MCP integration where an agent summarizes the inbox each morning, the user writes terse one-line reply intents (e.g., "decline, too busy this week"), and the agent drafts full replies for human review before sending.

**Paraphrase of the pattern:**
1. Agent fetches inbox and produces a priority summary (sender, subject, urgency heuristic).
2. Human writes short intent notes per message: accept / decline / ask clarifying question / forward to X.
3. Agent expands each intent into a full draft email, presented for approval.
4. Human reviews and sends manually—**agent never auto-sends**.

**Application:** This is a feedforward+feedback loop for communication, not code. Could apply to email/Slack triage without new tooling: existing Gmail MCP in Cursor, no extra SaaS. The "never auto-send" boundary is critical for trust.

**Caveat:** link in the original video was comment-gated; the repo/skill was not independently retrieved. Treat as a pattern shape. If a Gmail MCP skill exists, confirm permissions before enabling.

---

## 3. Plan with expensive model, build with cheaper model

**Sources:**
- [7636039109391174934](https://www.tiktok.com/@itstundealao/video/7636039109391174934) (2026-05-04), transcribed.
- [7629750007746366742](https://www.tiktok.com/@itstundealao/video/7629750007746366742) (2026-04-17), caption + transcript.

The creator demonstrated planning a feature with a strong-reasoning model (e.g., Claude Opus or o1-class), then handing off the plan to a cheaper execution model (e.g., Sonnet, GPT-4o-mini) for implementation. In one demo he reported ~14 min / ~$8 with the expensive model end-to-end vs. ~3 min / <$1 with the cheap model following the pre-written plan.

**Paraphrase of the pattern:**
1. Write a detailed spec/plan using the highest-capability model you have access to—don't skip this step.
2. Hand the plan (spec.md, pseudocode, architecture notes) to a cheaper/faster model for code generation.
3. Run tests and lint feedback on the cheaper model's output; escalate back to the expensive model only for failures the cheap model cannot resolve.

**Caveats:**
- The $8 vs. <$1 numbers are **single-demo, creator-reported**. There is no guarantee this lowers cost or improves quality in general—it depends on the task, model pricing, and whether the plan is actually good.
- Model pricing changes frequently; verify current rates.

**Application:** A spec.md written with Opus or o1, then executed by Sonnet, is a natural application within existing Claude/Codex plans.

---

## 4. Eval per prompt change (system-prompt quality control)

**Source:** [7649420709084646678](https://www.tiktok.com/@itstundealao/video/7649420709084646678) (2026-06-09), transcribed (sponsored content for Braintrust).

The creator advocates running a scored eval suite every time you change a system prompt. The demo used Braintrust (paid SaaS with a free tier); the underlying idea is model-agnostic.

**Paraphrase of the pattern:**
1. Maintain a set of input/expected-output pairs (golden test cases) for any system prompt that controls production behavior.
2. Before deploying a prompt change, run the new prompt against the test cases and compare scores (accuracy, latency, cost, format compliance, etc.).
3. Only ship the change if it improves or holds steady; roll back otherwise.

**Application:** Maps directly to the Agent SDK `defineEval` pattern. No need for Braintrust; local evals or the SDK's built-in harness suffice. This is eval-driven development, consistent with "high-quality tests are the primary oracle" principles.

**Caveat:** video is sponsored; OSS eval tools exist and are already accessible under existing plans.

---

## 5. Content file indexing (creator analytics)

**Source:** [7699512057271422240](https://www.tiktok.com/@itstundealao/video/7699512057271422240) (2026-10-22), transcribed.

The creator describes building a "content file" containing all his posts, their transcripts, engagement metrics, and notes. The demo used paid services (BrightData for scraping, Gladia for transcription).

**Paraphrase of the pattern:**
1. Export or scrape your own content metadata (post URLs, dates, captions, view/like counts).
2. Transcribe audio locally or via API.
3. Store the corpus in a searchable format (CSV, SQLite, vector DB).
4. Query the corpus to find patterns: which hooks performed, what topics resonated, when engagement dropped.

**Application:** Achievable with yt-dlp (free, local, handles TikTok) + faster-whisper (local Whisper, no API cost) + SQLite or flat CSV. No BrightData or Gladia required for personal analytics.

**Caveat:** paid scraping/transcription demos are not necessary for single-creator self-analytics; local tools suffice.

---

## 6. Older startup/growth tips (sourced, included for completeness)

### 6.1 First-customer adoption story

**Source:** [7293662362463194400](https://www.tiktok.com/@itstundealao/video/7293662362463194400) (2023-10-24), transcribed.

The creator describes landing his first customer by lowering adoption risk: free trial, money-back guarantee, hands-on onboarding.

**Paraphrase:** Reduce friction for the first sale by removing risk (trial, guarantee, concierge support). Standard early-stage advice, included for source reference.

### 6.2 Staged 0→10→100→1000 user growth plan

**Source:** [7447562631478644001](https://www.tiktok.com/@itstundealao/video/7447562631478644001) (2024-12-12), transcribed.

The creator outlines a tiered growth plan: 0→10 users via personal outreach, 10→100 via free advice content, 100→1000 via referrals and lightweight paid experiments.

**Paraphrase:** Growth is staged; tactics that work at 10 users fail at 1000 and vice versa. Match effort to current stage.

### 6.3 Using ChatGPT for startup operations

**Source:** [7292848803944746273](https://www.tiktok.com/@itstundealao/video/7292848803944746273) (2023-10-22), transcribed.

The creator lists five ways he uses ChatGPT to run his startup: drafting emails, writing job posts, summarizing meetings, brainstorming features, and editing copy.

**Paraphrase:** Use LLMs for drafting, brainstorming, and editing—tasks where human review is cheap and mistakes are recoverable.

---

## Limitations

1. **Single-demo claims:** cost/time numbers (e.g., $8 vs. <$1, ~14 min vs. ~3 min) are creator-reported from one run, not verified benchmarks.
2. **Coverage gaps:** 172 older posts reviewed at caption level only; 12 unlisted posts not examined. This is not "all watched."
3. **Sponsored content:** eval-per-prompt video was sponsored by Braintrust; OSS alternatives exist.
4. **No code run:** repos mentioned in the audit were not cloned or executed here.

## Related research

- [tiktok-itstundealao-2026-11-06](../tiktok-itstundealao-2026-11-06/README.md) — video tooling catalog from same creator
- [tiktok-mcp-and-skills-2026-11-06](../tiktok-mcp-and-skills-2026-11-06/README.md) — MCP and skills research
