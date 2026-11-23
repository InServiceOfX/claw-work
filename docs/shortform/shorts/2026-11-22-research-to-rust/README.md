# Build log 01: "I watch TikToks so my agents can build tools" (2026-11-22)

A faceless vertical Short for YouTube Shorts, TikTok and Instagram Reels about
the loop behind claw-work. A creator's Short goes in, coding agents parse and
judge it, and the useful tools come out as our own Rust code. All of the
footage is real output from that loop: the Galton board and shape morph from
`Rust/claw-motion`, and the sin x ≈ x scene from the Manim math mode. The
research batch it describes is
[`research/tiktok-animation-tools-2026-11-22/`](../../../../research/tiktok-animation-tools-2026-11-22/README.md).

This folder is also the template for the weekly or biweekly claw-work build
log: copy it, rewrite `short.json` and the narration texts, and swap the clips.

## Make it

From this folder:

```bash
./make_clips.sh                       # 1. footage: Galton board, morph, Manim (about 20 s)
./voice.sh path/to/reference.wav      # 2. narration with voxcpm-tts, one WAV per scene
./render.sh                           # 3. merge: footage + voice + clip sound + cut ticks
```

`render.sh` writes `out-<timestamp>/short.mp4` with `contact.png` (one frame
per scene), `captions.srt`, `timeline.json`, `verification.json` (loudness) and
`posting_kit.md`. Scene lengths come from the narration WAVs, so a new take
re-times the video with no other edits.

### The voice, step by step

`voice.sh` runs this for each scene. Run it by hand to redo a single take:

```bash
voxcpm-tts say narration/scene_03.txt -o narration/scene_03.wav \
  --voice path/to/reference.wav --seed 42 --force
```

- The reference must be one speaker you are authorized to use. A CC0 LibriVox
  reader works, as in the voxcpm-tts Short.
- A different `--seed` (or `VOXCPM_SEED=7 ./voice.sh ref.wav`) gives a
  different read of the same text. Redo only the scenes that sound off.
- `./voice.sh --say` voices everything with macOS `say`. That's a timing
  preview only: it's how the checked render below was made, not a voice to
  post.
- The spoken text is in `narration/scene_NN.txt`. The same text is in each
  scene's `narration` in `short.json`, which drives captions and checks; if
  you edit one, edit the other.

### What gets mixed

| Layer | Where it comes from | Level |
| --- | --- | --- |
| Narration | `narration/scene_NN.wav` | reference level |
| Galton board knocks | the clip's own track, one knock per collision (scenes 1 and 8) | `video_volume_db: -12` |
| Cut ticks | synthesized by `claw-shorts` on each of the 8 scene changes | `sfx.cut_db: -24` |
| Whole mix | normalized together | -14 LUFS, -1 dBTP; render fails if off |

If the knocks fight the voice, lower `video_volume_db` (for example to -16).
For no ticks, set `"sfx": {"cuts": false}`.

## The plan (short-form-strategy)

**Logline.** One TikTok about twenty animation tools became a Rust physics
engine, shape morphs and a math mode, through a loop of watch, parse, judge
and build.

**Hook triple**

| Layer | Content |
| --- | --- |
| Visual | Balls already pouring through the Galton board at frame 0 (the clip starts 3 s in), with a knock on every hit |
| Verbal (T1, outcome first) | "I watched one TikTok, and my coding agents turned it into this." |
| Text | "One TikTok became this." |

**Script.** Full sentences throughout (house style). The short lines are
deliberate and rare: "Three of fifteen passed." lands the verdict, and
"Watch. Parse. Judge. Build." is the closing summary.

| # | Time (say preview) | Picture | Narration |
| --- | --- | --- | --- |
| 1 | 0.0-7.6 | Galton board pouring, caption band | I watched one TikTok, and my coding agents turned it into this. Every bounce you hear is computed in Rust. |
| 2 | 7.6-13.7 | Title: "1. Watch / A 36 s TikTok: / 20 free animation tools" | It starts when I save a Short that names some tools. This one promised twenty free tools for animation. |
| 3 | 13.7-23.0 | Terminal: transcribe, 13 lines, 44 frames, 7 kept, 15 of 20 identified | Then an agent parses it, like Claude Code, Codex, Grok or pi. It pulls the transcript and key frames, and checks every license. |
| 4 | 23.0-32.2 | Verdict: ADOPT Manim, REWRITE Matter.js and Flubber, SKIP 12 others | Next comes the judgment, one tool at a time. Can an agent render it frame by frame, with no screen? Three of fifteen passed. |
| 5 | 32.2-37.5 | Shape morph, "2. Rewritten in Rust" | Two of them we rewrote as our own Rust code. This shape morph is one of them. |
| 6 | 37.5-44.0 | Manim: sin x ≈ x, graph, Taylor term | The third one, Manim, we kept exactly as it is. Its math typography is too good to rebuild. |
| 7 | 44.0-51.1 | Terminal: 26 tests, 18 s in 5 s, same bytes, -14.0 LUFS | Every new tool ships with tests and measured results. The same scene renders to the same bytes every time. |
| 8 | 51.1-58.4 | Galton board: the finished bell curve | That is the whole loop, and we run it every week or two. Watch. Parse. Judge. Build. |
| 9 | 58.4-63.6 | End card: claw-work, open source, GitHub URL, comment prompt | Everything is open source on GitHub. Which tool should my agents rebuild next? |

Every number on screen is from the 2026-11-22 run: transcript segments,
candidate frames, tools identified, tests, render time and loudness.

**Engagement.** A judgment invite: viewers pick the next tool (earns comments).
Pinned comment: "Name a tool or library you want rebuilt in Rust. I'll run it
through the loop next." It also seeds the next episode.

**Self-score.** Hook 4/5 (motion and sound from frame 0, a concrete outcome).
Retention 3/5: 64 s with the `say` preview is over the 45 s sweet spot. Engagement 4/5.
To go shorter, merge scene 2 into one sentence ("It starts with a Short that
promised twenty free animation tools.") and drop scene 7. That brings it to
about 50 s at the cost of the proof numbers.

**Riskier option.** Open on the finished bell curve and ask "Why does this
always make a bell curve?", then reveal that an agent built the simulator from
a TikTok. That gets more curiosity, but the loop becomes the B-story.

## The loop, in general

This is what the video describes and how each step runs here:

1. **Watch.** You save a TikTok or YouTube Short whose creator names tools,
   repos or techniques. Download it (720p) into the local media archive.
2. **Parse.** A harness (Claude Code, Codex, Grok Build, or pi on a Codex
   subscription) runs `Scripts/shortform-ingest`. It does a timestamped
   transcript, candidate frames from scene changes plus a dense early grid,
   and readable screenshots of every list. Then it identifies each named tool
   and checks its repo, license, stars and last push. Linked pages (Notion,
   Linktree) are captured as text.
3. **Judge.** Each tool gets the same questions. Can an agent drive it
   headless, frame by frame? Does it add something our modes can't do? Is
   the license usable? The verdict is adopt as is, rewrite in Rust, skip, or
   closed with what would reopen it. A creator stays on the watch list only if
   their videos change a rule or a tool.
4. **Build.** Adopted tools are pinned and wrapped as a claw-shorts mode.
   Rewrites become Rust crates with tests and measured results, credited to
   the original. Then the skills (`short-form-strategy`, `claw-shorts`) learn
   the new mode, so every harness can use it.
5. **Post.** A build-log Short like this one, made by those same tools.

## Checked (2026-11-22, `say` preview)

- `claw-shorts check`: no errors. One warning: 170 words (about 68 s) is over 45 s.
- Render: 1080×1920, 63.6 s, -14.0 LUFS, -1.37 dBTP.
- Every scene viewed on the contact sheet: captions inside the safe area,
  clip caption band above the board, end card readable.
- **Not done:** the voxcpm-tts narration (your step 2), listening to the final
  mix, and posting. The GitHub URL on the end card shows the
  repository; push and merge this branch before posting so the tools are
  there.
