# README — Short-Form Video Pipeline (HTML deck + OBS, single live take)

One sentence: build the whole visual side of a vertical STEM explainer as one
self-contained HTML file, record it live in one take with OBS (screen capture
of that file + your own narration), verify the HTML with Playwright before
you ever hit record — not a full video-editor pipeline.

Read `WORKFLOW.md` in this folder for the full writeup: the technique, the
exact OBS settings that worked, every bug actually hit while building this
(and the fix), and an honest time-cost accounting for whether this is
actually faster than a conventional record-then-edit workflow.

Origin: built 2026-07-04 while producing Episode 1 of an "Attention Is All
You Need" explainer series.
Generalizes past that one series — anyone producing a short, vertical,
equation/diagram-heavy explainer video can reuse this pattern.

## Related: virality structure (hook / retention / engagement)

This folder is **how to shoot** (HTML deck + OBS). For **what to put in the
video so it earns distribution** (Noe Murillo Part 1 fundamentals — hook,
retention, “viral tokens”), use the shared skill:

- `skills/short-form-strategy/SKILL.md`
- Framework notes: `skills/short-form-strategy/references/noe-murillo-pt1-framework.md`
- Part 2: `skills/short-form-strategy/references/noe-murillo-pt2-framework.md`
- Source archive: `Data/Public/tiktoks/` (`SCREEN-MATCH.md`, `pt1.srt`, `pt2.srt`)
- Reference accounts to study (not a second framework): `docs/shortform/REFERENCE_CREATORS.md` — first entry `@a13x_duke` (cA13X / Alex, Instagram Alexdukee)

Install it for Claude Code, Grok and Hermes with `skills/install-shortform-skills.sh`.

## Related: automated faceless assembly (Rust pipeline)

This folder is **how to shoot** with your own narration on camera-less decks.
For fully automated faceless videos — TTS voiceover, stock b-roll, burned-in
captions — use the Rust pipeline instead:

- Crate: `Rust/shortform-video/` (see its README for CLI and config)
- Skill: `skills/claw-shorts` (stock B-roll mode, `references/stock-broll.md`)
- Provenance: `docs/shortform/MONEYPRINTERTURBO_ASSESSMENT.md`

Structure (short-form-strategy skill) → then either **shoot** (this folder) or
**assemble** (`shortform-video`).
