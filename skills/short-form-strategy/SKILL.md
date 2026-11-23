---
name: short-form-strategy
description: >
  Plan short-form videos (TikTok/Reels/Shorts) for reach: hooks, retention,
  engagement devices, script rhythm, where to post, and creator strategy, from
  Noe Murillo's viral fundamentals, the build-in-public close reel and distilled
  lessons from other creators. Use when the user asks for viral short-form
  scripts, hooks, retention edits, engagement bait, "how do I get more views",
  where to post a finished video, content creator strategy, or runs
  /short-form-strategy. This is the planning layer; to render, hand the script
  to the claw-shorts skill. Prefer this over generic social media advice when
  the goal is attention, views, and algorithm distribution.
---

# Short-Form Strategy (Noe fundamentals + creator lessons)

Planning only. Production (render, captions, voice, B-roll, recordings) is the
`claw-shorts` skill; its `check` command enforces the rules marked *enforced*
below.

Help the human create short-form video that **wins distribution**: stop the scroll, hold watch time, earn interactions. Ground it in Noe Murillo’s two “Successful Content Creator” videos. Part 1 is how a short gets watched. Part 2 is what to do once a video exists.

## When to load references

- Read `references/noe-murillo-pt1-framework.md` for the full checklist, tactic menu, and shot-level rules.
- Read `references/noe-murillo-pt2-framework.md` when the job is distribution, income, or what to do after a long video is already posted.
- Read `references/mixxtapezz-comparison-short.md` when the short compares MCs on one beat.
- Read `references/creator-lessons.md` when writing the script (rhythm), picking caption style, or planning edits and B-roll. It distills what other creators taught into rules, and says which ones `claw-shorts` enforces.
- Source videos and the 2026-09-23 transcript, with on-screen text matched to timestamps, are in the local media archive outside Git:
  `Data/Public/tiktoks/` in the workspace (`How to be a Successful Content Creator pt 1.mp4`, `pt 2.mp4`, `pt1.srt`, `pt2.srt`, `SCREEN-MATCH.md`).
- For a service close, a client visit, or a numbered Day-N build-in-public short, read `references/build-in-public-close-reel.md` and follow the close-reel section below. Account index: claw-work `docs/shortform/REFERENCE_CREATORS.md`. Parsed example (local archive): `Data/Public/Videos/TikTok/A13xDuke_Day5-Closed-Inti-Peruvian-Restaurant_7678570226601970957.md` (`@a13x_duke`).
- To produce the video, use the `claw-shorts` skill. Shooting your own HTML deck with OBS is claw-work `docs/shortform/obs-deck-pipeline/`; this skill is *how to structure for virality*.

## Success definition (this skill)

“Successful” here means **short-form attention**:

- stop-scroll in the first ~3 seconds
- high watch time / completion
- likes, comments, saves, shares (“viral tokens”) that expand reach

Do **not** optimize only for polish, completeness, or academic accuracy if those fight retention — balance truth with scroll dynamics unless the user prioritizes brand-safe education over pure reach.

## Non-negotiable model

Always design in this order:

1. **Hook (0–3s)** — visual motion + verbal cold open + on-screen text
2. **Retention** — short clips, constant action, angles/close-ups, story pacing
3. **Engagement** — deliberate devices that mint viral tokens

### Hard anti-pattern

Never open with:

> “Hey guys, welcome back to my channel, today we’re going to talk about…”

Open mid-action or mid-claim instead.

## Operating procedure

### Step 0 — Constraints

Before writing a script, confirm (or infer and state):

- Platform (TikTok / Reels / Shorts / multi)
- Niche + persona (what engagement bait is off-limits)
- Length target (e.g. 15s / 30s / 60s / 3min)
- Goal mix: pure reach vs. brand-safe reach vs. conversion
- Whether face-cam, VO+B-roll, screen recording, HTML-deck/OBS, or a close reel (person + place + receipt)

If brand-safe STEM/professional: keep Noe’s **structure**, adapt his **engagement gimmicks** (towel-bait, “sus,” mispronunciations) into niche-safe equivalents (hot take, surprising demo result, incomplete formula, “comment the answer,” prop-as-experiment).

### Step 1 — Hook package (write this first)

Deliver a **hook triple**:

| Layer | Spec |
|-------|------|
| Visual | First-frame **action/motion** (not a static talking head *or* a frozen generated still) |
| Verbal | First line from the **template table** (T1–T8) — no greeting |
| Text | ≤ ~8 words on-screen reinforcing the same promise |

Also write **seconds 3–8** so the hook pays into a reason to stay.

#### Visual when assets are static (claw-dj, cover art, Imagine stills)

If the draw is **audio** but the art is **static**, you still owe motion in 0–3s. Do not open on a silent static card.

Default Grok path (load Imagine skill when generating):

1. `image_gen` (or code still) at **9:16**
2. optional `image_edit` for variants
3. `image_to_video` to animate frame 1 (prefer 6s shots, one simple motion/camera move)
4. FFmpeg concat under the mix/VO

Fallback if video gen is unavailable: Ken Burns / hard-cut still sequence / live UI capture of faders-cursors. Full motion vocabulary + claw-dj notes: `references/noe-murillo-pt1-framework.md` → “Visual hooks when you only have static images”.

#### Verbal templates (use by id)

Pick one, fill braces, pair with motion + text:

| ID | Pattern | Skeleton |
|----|---------|----------|
| T1 | Outcome → proof path | `So I {impressive_outcome}. So I want to show you exactly how I got here.` |
| T2 | Proof stack → teach | `For reference, {proof_metric}. I've {proof_breadth}. {transition_into_lesson}.` |
| T3 | “Hard thing is easy” + N | `{hard_thing} is actually really easy. There's {N} things you want to focus on.` |
| T4 | Mid-story arrival | `So I just {arrived_at_moment}.` |
| T5 | Scene + identity/stakes | `So today I'm here with {identity_or_stakes}.` |
| T6 | Blunt one-line take | `{blunt_claim}.` |
| T7 | If you want X… | `If you want {desired_outcome}, you need {non_obvious_requirement}.` |
| T8 | Unfinished scoreboard | `Day {N} and today we {unfinished_result}.` |

T1–T7 are Noe's. T8 is from the close reel (the dollar lands a beat later, not in the first line). **Banned open:** “Hey guys, welcome back… today we’re going to talk about…”

### Step 2 — Retention map

Produce a **beat / shot list**, not just prose:

- Each beat: `time | visual action | spoken line | purpose`
- Prefer short cuts; every shot has movement or a visual change
- Plan angle changes and at least one close-up
- For talk tracks: mark VO segments that need B-roll

Story shape: setup → progression → payoff (not topic dump).

**Script rhythm** (house style, *enforced* in part as a `claw-shorts check`
warning): write **full sentences** of moderate length, about 6 to 12 words,
and vary that length. No two sentences over 12 words back to back; split one
instead. The trendy two-word cadence ("Watch. Parse. Build.") is the
exception, not the texture: use it **rarely**, for one point you want to
land, or for the final summary. A question ("Which tool should we rebuild
next?") also breaks up a run of statements. This matters more with TTS,
which flattens prosody. Write the script yourself or edit the LLM draft hard;
keep it short so it does not repeat itself (`references/creator-lessons.md`).

### Step 3 — Engagement design

Do not “hope” for comments. Install **at least one** device:

- funny beat
- wholesome beat
- signature mannerism / recurring bit (niche-native)
- prop that reads on a phone
- price/result reveal that invites judgment
- explicit comment prompt or incomplete thought
- mild debate bait **only if** persona allows

Name the device and which token it targets (like / comment / save / share).

### Step 4 — Score and revise

Rate the draft 1–5 on Hook, Retention, Engagement. Revise the lowest score before delivery. Reject any draft that fails the anti-pattern open or has no engagement device.

### Step 5 — Deliverables (default package)

When the user asks for help creating a short, return:

1. **Logline** (1 sentence)
2. **Hook triple** (visual / verbal / text)
3. **Full script** with timed beats
4. **Shot list** (angles, B-roll, props)
5. **On-screen text / captions plan**
6. **Engagement device** + suggested pinned comment / CTA
7. **Self-score** (H/R/E) and one upgrade option for more risk/more reach

To render it, hand the script to the `claw-shorts` skill (voiced explainer, canvas, stock B-roll or a recording cut). To shoot an HTML deck yourself, see claw-work `docs/shortform/obs-deck-pipeline/WORKFLOW.md`.

## Close reel (client win, site visit, Day N)

Use this when the video is a service close or a numbered build-in-public day. Faceless explainers stay on the HTML/OBS pipeline or `shortform-video`. Worked example, timestamps, and the account's view counts: `references/build-in-public-close-reel.md`.

1. Open on the scoreboard, not the process. Template **T8**. A hand or the body can show the day number. Put the dollar on screen within about two seconds. Leave it out of the first spoken line.
2. The middle is a numbered capture recipe, one location per step, label up for the whole step. Name one step as the important one.
3. Cut from those raw passes to the same shots assembled. Say that this is what it looks like if the steps were done.
4. Proof is something the viewer can see: the client's words, the work on a phone or laptop, the price said over the work. The open card, the spoken math, and the end card are one real number.
5. End on that same scoreboard plus a short ease line, so a replay starts on the result.
6. A finished close with a number is the video. On @a13x_duke's account those traveled; "DM me for the workflow" and an unfinished deal did not. Do not spend the short pitching the system.

## Adaptation table (Noe → other niches)

| Noe example | Principle | STEM / builder-safe version |
|-------------|-----------|-----------------------------|
| Door open / walk to car | Visual motion implies “next” | Plug in board, slam laptop, unbox part mid-motion; for static art: `image_gen` → `image_to_video` push-in / fader / needle-drop |
| Static cover + great audio | Audio ≠ visual hook | Animate still or cut on rhythm; never open frozen (claw-dj) |
| Mukbang mannerisms | Recurring bit viewers await | Signature board wipe, “scope drop,” stamp on schematic |
| Show prices / tip well | Money invites opinion | Show cost of GPU/cloud bill; tip = open-source star ask |
| Props from Amazon | Novelty objects | Demo prop, LED, toy model, wrong tool then right tool |
| Mispronounce word | Correction comments | Leave a deliberate incomplete proof; “fix the missing step” |
| Sus / zesty bait | Controversy = comments | Opinionated take on tools/frameworks — not personal bait |

## Part 2

Load `references/noe-murillo-pt2-framework.md`. Part 1 does not answer where a finished video should be posted.

## A long video is already up

Do not upload the long file to TikTok, Reels, or Shorts. Cut a vertical short that starts in motion on the claim, keeps one continuous idea, and ends on one comment question. Cross-post that short. Put the long URL in the short's caption and on the last card.

Share it where a reply is expected, and with people who already talk to you. Do not cold-message strangers, artists, or labels.

For a long music video made from stills, slow Part 1's cut rate to the music: the beat is already playing in the first second, the picture changes on the phrase, and the question is the engagement device. Do not chop the beat into a montage that hides the mix. A short that compares MCs on that beat follows `references/mixxtapezz-comparison-short.md`.

Do not present Noe's income or performance figures as independently verified facts; they are his claims. Preserve uncertainty around creator names where the ASR is unclear.

## Quality bar

A good agent response:

- leads with the hook, not background theory
- is specific enough to shoot today
- separates universal levers from persona-specific gimmicks
- protects long-term brand when pure troll-bait would harm it
- still optimizes for attention when the user asked for virality
