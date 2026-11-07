# Joshua Blatch (@joshuablatch) — Pulse motion-opportunities ledger

- Video: https://www.tiktok.com/@joshuablatch/video/7695819667444714765 (81s)
- Auto-captions: [`../transcripts/joshuablatch-7695819667444714765.md`](../transcripts/joshuablatch-7695819667444714765.md)
- Screenshots: [`../screenshots/joshuablatch/`](../screenshots/joshuablatch/)
- Theme: approve-before-build motion-graphics library
- CTA: none (comments CTA only; core workflow shown in-video)
- Public prompts: https://www.signalai.agency/pulse.html (A–E + ledger.csv)

## Core workflow (from watch + on-screen UI)

Claude Code edits a talking-head / founder video. In parallel, a strict "ideas only" mod (Josh calls it his **Pulse**) watches the same work and pitches **missing motion-graphic components** before spending tokens on builds.

On-screen sections (dark Notion-like / custom dashboard):

1. **01 The library** — catalog of reusable motion modules, e.g. `M01 Signal`, `M02 Transitions`, `M03 Score`, `M05 Your Attunement`, `M06 Systems`, `M10 How it works`, `M11 Then & Now`, `M12 Series Openers`, `M13 Pop-outs`, `M14 Loops`, `M18 Scan Trail`, `M19 Spread Map`, `M20 Load Shift` (Attuned / Systems School brand).
2. **02 The rule** — path shown: `10. Motion/OPPORTUNITIES.md` (also spoken as Motion opportunities ledger). Yellow callout: **"Ideas only: Josh picks from options before anything is built."** Explicit token-saving gate.
3. **03 Where the idea came from** — transcript excerpt + catalog gap check. Example spoken line about five scans (`97 → 81 → 75 → 66 → 60`). Closest existing modules fail: `M11 Then & Now` only morphs two scans; `M09 One Number` is an aggregate, not one marker over five scans → gap logged with timestamp + exact line + why.
4. **05 Approve or pass** — "Pulse" cards with **Approve / Pass / Add a note**. Example pitch **"Relay Line"**: hairline token passing through Record → Edit → Motion → Approve stations (reusable BTS / how-we-work explainer).
5. **06 Three directions** — after approve, Claude returns **2–3 still frames** (not animations yet). Example for Scan Trail: **A "Dotted Descent"** vs **B "Ledger"** — caption: *"Scan Trail, A B C, as stills before anything is animated"*.
6. **07 Built / 08 The loop** — picked still becomes a library module (`M18 Scan Trail`, `M19 Spread Map`, `M20 Load Shift` cited as built from an Achilles video). Next edit runs can reuse them.

Spoken close: same loop works for **sales decks, email templates, ad creative** — anything you keep rebuilding. Claude watches work already being done, pitches what's missing, human says yes/no.

## Faceless & automated short-form analogue (zero paid SaaS)

Josh's on-camera Reels are talking-head + branded motion overlays. For **faceless** Remotion / Playwright / FFmpeg pipelines the **Pulse pattern is the valuable part**, not the face:

1. **Keep a versioned motion catalog** (`catalog.md` or CSV): one line per Remotion composition / canvas kit (`name`, `what it shows`, `path`, `preview still`).
2. **Weekly (or per-batch) gap pass — ideas only.** Prompt Claude/Codex/Grok: read this week's scripts + transcripts + the catalog + `ledger.csv`. Pitch ≤3 missing components. **Do not render.** Status `pitched` only.
3. **Human Approve / Pass / Later** in the ledger (CSV, Markdown table, or local HTML buttons). Never spend a full Remotion/Playwright render before this gate (token + time saver).
4. **Still directions before animation.** For each approved idea, render 2–3 **static** HTML/Remotion stills (different metaphors, not three colorways). Pick A/B/C.
5. **Build once into the library** with parameters (copy, numbers, accent color, duration). Add README + preview. Update catalog so the next gap pass knows it exists.
6. **Faceless render path:** approved components → Remotion / Playwright+canvas (`cth9191/animate` or local canvas workflow) + local TTS (Kokoro/Piper/F5-TTS) + FFmpeg. No ElevenLabs, no CapCut Desktop required.
7. **Generalize beyond motion:** same ledger for thumbnail layouts, hook card variants, end-screen CTAs, email blocks.

Signal AI's public prompts A (gap pass), B (still directions), C (build once), D (`ledger.csv`), E (optional approve page) are enough to stand up this loop under Claude/Codex/Grok with local files only.

## Screenshot index

- `01-library-intro.jpg` (~0:00) — library + "Motion graphics built by Claude"
- `02-opportunities-rule.jpg` (~0:10) — rule + `OPPORTUNITIES.md`
- `03-approve-pass-pulse.jpg` (~0:40) — Approve/Pass Pulse card "Relay Line"
- `04-built-modules.jpg` (~0:56) — built M18/M19/M20
