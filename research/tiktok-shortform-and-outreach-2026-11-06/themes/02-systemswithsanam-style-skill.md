# Sanam (@systemswithsanam) — Inbox folder + style interview skill

- Video: https://www.tiktok.com/@systemswithsanam/video/7703550492319780110 (76s)
- Auto-captions: [`../transcripts/systemswithsanam-7703550492319780110.md`](../transcripts/systemswithsanam-7703550492319780110.md)
- Screenshots: [`../screenshots/systemswithsanam/`](../screenshots/systemswithsanam/)
- Theme: single-file inbox → style interview → reusable editing skill
- CTA: none (no paywall explicitly called out on screen)
- Related prior ingest: `claw-work` `research/tiktok-skill-research-2026-11-05/themes/02-abloommedia-capcut.md`

## Core workflow (from watch + on-screen steps)

Talking-head tutorial. Overlay steps (no paywall):

1. **Desktop folder named `CapCut`** — drop **one** raw video to edit.
2. **Claude Code desktop app** — prompt: *"Go to my Desktop, grab the local files and edit them for me in CapCut directly."* If prior CapCut Desktop projects exist, Claude can inspect them for style suggestions.
3. **Style interview prompt:** *"Interview me on my editing style: cuts, emojis, text on screen, fonts and colors."* (exact overlay text ~0:38)
4. **Iterate** on that one video (suggestions → edits → more suggestions).
5. **Save it as a skill** once "good enough," so the next drop-in video reuses the workflow.
6. **Ops reality check (spoken):** first video ≈ **3 hours** and ~**6–16 iterations**; now **1–2 iterations**. Skill updated **weekly** with new editing styles to test.
7. Steady state: new raw → CapCut folder → skill already knows what to do.

Spoken framing: not about variety / reinventing the wheel — about **time** (most valuable thing you cannot purchase). Film once with stumbles/repeats; let the agent cut.

## Faceless & automated short-form analogue (zero paid SaaS)

Sanam's demo is **talking-head + CapCut Desktop computer-use**. That GUI path is **low priority** for faceless automation. Steal the **pattern**, not the app:

1. **Inbox folder convention** — `inbox/raw/` (or `CapCut/` analogue) where a watcher / skill picks up the next asset. One file in → one job.
2. **Style interview → `style.md` / skill** — one-time (then weekly) Q&A capturing: cut rhythm, caption style, emoji policy, fonts, colors, hook/CTA rules, last-take preference. Persist as a local Claude/Codex/Grok skill under `~/.claude/skills/` or project skills folder.
3. **Messy-take filming rule → last-take cutter** — already in `@bridotai` / `@adamstewartmarketing` / `@jamout.ai` HyperFrames notes: keep best/final take per line, cut dead air. For faceless, this becomes **script TTS takes** or multi-draft VO selection.
4. **Iteration budget** — expect a cold-start tax (her 3h / 6–16 iters), then 1–2 iters once the skill exists. Encode that in the skill: first run is exploratory; later runs are constrained.
5. **Weekly skill patch** — treat style drift as a scheduled skill update, not a full rewrite of the renderer.
6. **Zero CapCut path:** Whisper/faster-whisper transcript → edit plan → Remotion or FFmpeg assembly → local TTS. Optional public CapCut-CLI skill only if someone explicitly wants CapCut project files; default stack stays OSS.

## Reusable skill template (`style.md`)

```markdown
# Editing style guide

## Cut rhythm
- Cut dead air (pauses > 0.8s)
- Keep last take per line (best performance)
- Max clip length: 8s before visual change

## Caption style
- Font: [font name]
- Size: [size]
- Position: bottom third
- Emojis: [yes/no, frequency]

## Colors
- Primary: [hex]
- Accent: [hex]
- Text: [hex]

## Pacing
- Hook: ≤ 3s
- First value beat: by 0:07
- CTA: final 5s
```

## Screenshot index

- `01-no-paywall.jpg` (~0:14) — "no paywall" callout
- `02-desktop-capcut-folder.jpg` (~0:22) — step 3 paste prompt — Desktop/CapCut folder
- `03-style-interview-prompt.jpg` (~0:38) — style interview prompt
- `04-save-as-skill.jpg` (~0:54) — step 5 "Save it as a skill"
