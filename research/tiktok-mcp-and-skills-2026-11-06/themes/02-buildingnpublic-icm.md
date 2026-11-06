# Beau Johnson (@buildingnpublic) — ICM: folder-based agent context

- Video: https://www.tiktok.com/@buildingnpublic/video/7703729143749397774
- Auto-captions: [`../transcripts/buildingnpublic-7703729143749397774.md`](../transcripts/buildingnpublic-7703729143749397774.md)
- Screenshots: [`../screenshots/buildingnpublic/`](../screenshots/buildingnpublic/)
- Theme: Interpretable Context Methodology (ICM) — folder structure as agentic architecture
- CTA: open source / free (GitHub card shown on screen)

## What is ICM?

**Interpretable Context Methodology** by Jake Van Clief & David McDermott (Eduba / University of Edinburgh). It replaces multi-agent orchestration frameworks with **filesystem structure**: numbered stage folders, plain markdown prompts/context, and local scripts. One agent reads the right files at the right step.

Open source, MIT license. Main repos:

- Skill/architect repo: https://github.com/RinDig/icm-architect (~2.1k★)
- Methodology repo: https://github.com/RinDig/Interpretable-Context-Methodology (~1.4k★)
- arXiv paper: https://arxiv.org/abs/2603.16021

## Memory claim

In *The Cost of Remembering* (Van Clief, McDermott, Kumar — https://github.com/RinDig/cost-of-remembering), answering from an ICM-style folder memory used **97% fewer tokens** than re-reading full chat history on the LongMemEval benchmark. The video notes "that number is one benchmark."

## Three key concepts (from the video)

1. **Map by the door** — One root map file (`THE MAP · ONE FILE AT THE ROOT`) tells a fresh session where everything lives. Folders: `01 RESEARCH`, `02 SCRIPT`, `03 PRODUCTION`, `_CONFIG`.

2. **Decisions in writing** — `_CONFIG` is set once: `voice.md`, `conventions.md`, `_config/` (VOICE / RULES / CHOICES). "The recipe stays; the ingredients change" (Layer 3 reference vs Layer 4 working artifacts — Table 2 from the paper).

3. **One room at a time** — Numbered folders (`01`, `02`, `03`, …) are pipeline steps. Each loads only needed context and leaves an editable artifact (e.g. `02/output/script_draft.md`). Aimed at sequential, human-reviewed workflows.

## Folder structure example

```
project/
├── _map.md           # WHERE EVERYTHING LIVES
├── _CONFIG/
│   ├── voice.md      # writing style
│   ├── conventions.md # formatting rules
│   └── choices.md    # project decisions
├── 01/               # RESEARCH stage
│   ├── input/
│   └── output/
├── 02/               # SCRIPT stage
│   ├── input/
│   └── output/
│       └── script_draft.md
└── 03/               # PRODUCTION stage
    ├── input/
    └── output/
```

## Why use ICM?

- Stop re-explaining your business to AI every morning.
- Small routing files (`_map.md`, `CONTEXT.md`) instead of monolithic system prompts.
- Human-reviewable artifacts at each stage.
- Works with Claude Code, Codex, or any agent that can read files.

## Screenshot index

- `01-icm-folder-intro.jpg` (~0:08) — Manila "ICM" folder intro with Jake Van Clief credit
- `02-97-percent-fewer-tokens.jpg` (~0:14) — Token comparison bar chart: 97% fewer tokens vs chat history
- `03-arxiv-icm-paper.jpg` (~0:18) — arXiv paper title and abstract
- `04-github-icm-architect.jpg` (~0:55) — GitHub card for `RinDig/icm-architect`
