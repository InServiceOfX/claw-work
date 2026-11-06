# SigmaflowAI (@sigmaflowai2) — /hairline-create single-file editorial HTML

- Video: https://www.tiktok.com/@sigmaflowai2/video/7703001513286700302
- Auto-captions: [`../transcripts/sigmaflowai2-7703001513286700302.md`](../transcripts/sigmaflowai2-7703001513286700302.md)
- Screenshots: [`../screenshots/sigmaflowai2/`](../screenshots/sigmaflowai2/)
- Theme: Hairline skill for cursor-reactive line art figures
- CTA: none (follow for more)

## What is Hairline?

A free Claude skill that draws clean, animated line art that reacts to your cursor — all from one prompt. The output is a single dependency-free HTML file.

- Repo: https://github.com/lucasmarkes/hairline
- Docs: https://hairline.lucasmarkes.com/
- Install: `npx skills add lucasmarkes/hairline`

## How it works

1. Run `/hairline-create <idea>` (e.g., "a sales funnel").
2. Hairline offers 2–3 concept directions.
3. Pick one.
4. It builds the figure, checks it against design rules, and outputs a single HTML file.
5. No dependencies — works anywhere HTML does.

There's also `/hairline-adopt` for converting existing SVGs into the Hairline style.

## The 10 design rules

Hairline teaches Claude ~10 design rules behind its editorial style. Rules shown on screen include:

- **One bright edge / stroke as the only highlight**
- **Rounded corners**
- **No words inside the drawing** (labels outside)

This creates the clean, editorial look of the line art.

## Example output

The video shows:

- A field with flowing figures (intro shot)
- `/hairline-create a sales funnel` → 2 concepts → final `hairline-funnel.html`
- The figure reacts to cursor movement (interactive, not static)

## Use cases

- Editorial/blog illustrations
- Landing page hero figures
- Interactive diagrams for docs
- Architecture/flow visualizations

All single-file, no build step, portable.

## Screenshot index

- `01-hairline-field-figure.jpg` — Intro: flowing line art field figures
- `02-github-hairline.jpg` — GitHub `lucasmarkes/hairline` repo page with design rules
- `03-hairline-create-command.jpg` — `/hairline-create` command in Claude
- `04-hairline-funnel-output.jpg` — Output: `hairline-funnel.html` funnel figure
