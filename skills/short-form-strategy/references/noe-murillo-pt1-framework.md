# Noe Murillo — Viral Fundamentals (Part 1) Framework

Source video (local + transcript), re-parsed 2026-09-23 from the download in `Data/Public/tiktoks/`:

- `How to be a Successful Content Creator pt 1.mp4`
- `pt1.srt` / `pt1.txt`
- On-screen text matched to timestamps: `Data/Public/tiktoks/SCREEN-MATCH.md`
- Earlier URL: https://www.tiktok.com/@thenoemurillo/video/7670717441508429070

This is a **principle extract** from Part 1 only. Success = short-form **reach**: stop scroll → hold watch → earn interactions so the algorithm redistributes.

## The three levers (always in this order)

```
HOOK (0–3s) → RETENTION (whole runtime) → ENGAGEMENT (viral tokens)
```

Rough view ceilings he implies:

- Hook + solid editing/story → often enough for ~100k-class outcomes
- Same + deliberate engagement design → path to multi-million outcomes

(Treat numbers as his framing, not a guarantee.)

---

## 1. Hook — first three seconds

**Job:** Make the viewer stop scrolling and start watching.

**Why it matters:** Watch-through of the open signals “people stick,” so distribution expands.

### Three hook types (stack when possible)

| Type | What it is | How to do it well |
|------|------------|-------------------|
| **Visual** | Motion/action on first frame | Door open/close, walk to car, set boxes on counter, close laptop — any small action that implies “something is about to happen” |
| **Verbal** | Spoken first line (talk / “yapping” videos) | Straight to the point. **Never** “Hey guys, welcome back… today we’re going to talk about…” |
| **Text** | On-screen text | Reinforce the same message as visual/verbal — make the promise readable muted |

### Visual hooks when you only have static images (e.g. claw-dj)

Noe’s visual rule is **movement or action** — not a pretty still. If the product draw is **audio** (claw-dj mixes, track previews, “listen to this transition”) but the art is **static generated images** (album-art style cards, waveform stills, UI mockups, Imagine stills), a frozen first frame **fails the visual hook**.

**Rule:** audio can be the *reason to stay*; it cannot replace the *stop-scroll visual*. Pair the sound with motion.

#### Preferred pipeline (Grok / Imagine tools)

Video starts from an image — there is no text-to-video. Load the Imagine skill when generating. Default path:

1. **`image_gen`** (or code-rendered still) → strong first frame at **9:16** if vertical short-form
2. Optional **`image_edit`** → variants / consistency across beats
3. **`image_to_video`** → animate that still (source becomes frame 1); prefer short **6s** shots, simple single motion or camera move
4. Optional **`reference_to_video`** only if multi-image reference is truly needed (usually compose with `image_edit` first, then `image_to_video`)
5. **FFmpeg concat** (stream copy) to stitch shots under the mix / VO

If video tools are unavailable in the session, say so and fall back to: Ken Burns / pan-zoom in editor, quick hard cuts across a still sequence, or live UI screen-cap of claw-dj with mouse/fader motion.

#### Motion vocabulary for audio-first / static-art shorts

Design the open so the eye sees change in 0–3s even while the mix plays:

| Motion | Use when |
|--------|----------|
| Slow push-in / pull-out on cover art | Instant “something’s starting” without busy animation |
| Parallax / light sweep / vinyl spin | Music, DJ, collection vibes |
| Waveform or progress bar animating | Makes the audio *visible* |
| Deck / fader / needle drop action | claw-dj product demo — hand or UI control moving |
| Hard cut every ~0.5–1.5s across stills | Cheap motion if only static assets exist |
| Text slam + scale punch | When verbal/text carry the claim |

**claw-dj example open:** still of a crate/volume → `image_to_video` needle-drop or fader move (0–2s) + first beat of the mix + verbal cold open + on-screen text. Do **not** open on a silent static logo card.

#### Visual checklist add-ons (static/audio projects)

- [ ] First 3s is **not** a static hold of a generated image
- [ ] Motion plan named (camera move / UI action / cut rhythm)
- [ ] Tool path chosen: `image_gen` → `image_to_video` (or explicit editor fallback)
- [ ] Audio bed starts in or immediately after the hook without replacing visual motion

### Hook checklist (agent must fill before locking a script)

- [ ] First frame has **visible motion** or a clear visual change
- [ ] First spoken line is a **claim, tension, or payoff tease** — not a channel greeting
- [ ] On-screen text restates the hook in ≤ ~8 words if muted viewing is common
- [ ] After 3 seconds, the viewer has a reason to care about second 4

### Verbal hook templates (from Noe’s examples)

Fill the `{braces}`. Keep lines short. Deliver **without** a channel greeting.

#### Anti-template (never use)

```
Hey you guys, welcome back to my channel.
Today what we're going to talk about is {topic}.
```

#### T1 — Outcome brag → proof path  
*(Noe open: “So I make over $100,000 a month… So I want to show you guys exactly how I got here.”)*

```
So I {impressive_outcome}.
So I want to show you exactly how I got here.
```

**Slots:** `{impressive_outcome}` = income, views, niche result, product metric  
**claw-dj:** `So I just mixed a {N}-minute set that people actually finish.` / `So I want to show you exactly how I built it.`

#### T2 — Social proof stack, then teach  
*(“My videos consistently average over a million views… I've gone viral on every niche…”)*

```
For reference, {proof_metric}.
I've {proof_breadth}.
{transition_into_lesson}.
```

**Slots:** `{proof_metric}`, `{proof_breadth}`, `{transition_into_lesson}`  
**claw-dj:** `For reference, this transition is the one that keeps people in the set.` / `I've run it on house, hip-hop, and open-format crates.`

#### T3 — Pattern interrupt: “hard thing is easy” + number  
*(“Going viral is actually really easy. There's three things you want to focus on.”)*

```
{hard_thing} is actually really easy.
There's {N} things you want to focus on.
```

**Slots:** `{hard_thing}`, `{N}` (usually 3)  
**claw-dj:** `Building a DJ set people don't skip is actually really easy. There's three things you want to focus on.`

#### T4 — Mid-story arrival (cold open into the scene)  
*(“So I just got the keys to my new apartment.”)*

```
So I just {arrived_at_moment}.
```

**Slots:** `{arrived_at_moment}` = keys, unbox, first export, first live mix, new volume mounted  
**claw-dj:** `So I just dropped a new volume into claw-dj.` / `So I just hit export on a live mix.`

#### T5 — Scene intro with identity/stakes  
*(“So today I'm here with the solo date king.” — start in the bit, not the channel)*

```
So today I'm here with {identity_or_stakes}.
```

**Slots:** `{identity_or_stakes}` = persona, co-host, the problem, the tool  
**claw-dj:** `So today I'm here with a crate that shouldn't work on paper.`

#### T6 — Blunt one-line take (commentary / “yapping”)  
*(Style of the straight-to-point talking hooks he praises — claim first, no throat-clearing)*

```
{blunt_claim}.
```

Optional second line for tension:

```
{blunt_claim}.
{I'm / even I} {qualification}.
```

**Slots:** keep it one breath; controversy optional and brand-gated  
**claw-dj:** `If your set has no story, the algorithm is doing you a favor by burying it.`

#### T7 — Direct stakes / “if you want X”  
*(His engagement pitch energy: “if you want millions of views…”)*

```
If you want {desired_outcome}, you need {non_obvious_requirement}.
```

**claw-dj:** `If you want people to save this mix, you need a transition they can feel in the first eight bars.`

#### How to pick a verbal template

| If the video is… | Prefer |
|------------------|--------|
| Tutorial / mastermind / framework | **T1** or **T3** |
| Proof-heavy creator story | **T2** |
| Vlog / day-in-life / product moment | **T4** or **T5** |
| Hot take / commentary | **T6** |
| CTA toward a result | **T7** |

Always pair the chosen verbal template with a **visual motion plan** (live action or static→`image_to_video`) and a **text hook** that restates the same promise muted.

---

## 2. Retention — watch time + completion

**Job:** Maximize how long people watch and how many finish.

**Drivers he names:** editing + storytelling.

### Editing rules

1. **Keep clips short** — cut dead air and static holds.
2. **Every clip has movement or action** — still frames kill retention.
3. **Angle variety** — wide / medium / close; change shot when energy dips.
4. **Close-ups** for emphasis, texture, and “you are there.”

### Commentary / talking-head rules

- Wording is tight (no throat-clearing).
- Pacing is deliberate (energy matches platform speed).
- Prefer **voiceover + B-roll storytelling** over static face-cam monologue when possible.

### Retention checklist

- [ ] Average shot length is short for the niche (default bias: cut more)
- [ ] No multi-second static with nothing happening
- [ ] Story has a through-line (setup → progress → payoff), not a topic dump
- [ ] Mid-video “reset” hooks every ~5–15s if length allows (new visual or verbal promise)

---

## 3. Engagement — “viral tokens”

**Job:** Earn likes, comments, saves, shares. He treats each as a **viral token** that buys more distribution.

**Threshold framing:**

- Strong hook + edit + story ≈ easy high-five-figure / low-six-figure views (his “easy 100k”)
- Millions require **designed engagement**, not just polish

### Creative prompts he uses every video

Ask explicitly while scripting/shooting:

1. What’s something **funny**?
2. What’s something **wholesome**?
3. What else will make people **do something** (comment, stitch, share, argue)?

### Engagement tactic menu (from the video)

| Tactic | Mechanism | Notes for agent use |
|--------|-----------|---------------------|
| Signature mannerisms | Repeatable bits viewers wait for / comment on | Invent niche-native “bits,” not copy his fry-dip unless food content |
| Show prices / tip well | Reaction + judgment comments | Money on screen = opinions |
| Props | Visual novelty + shareability | Shop simple props that read on phone |
| Mispronounce a word | Correction comments | Use sparingly; can feel gimmicky in expert/STEM niches |
| Mild “sus / zesty” bait | Troll energy → comment volume | High risk to brand; only if persona allows |
| Comment-bait scenarios | Controversial or ambiguous social signal | Prefer curiosity/debate over hate-bait for long-term brands |

### Engagement checklist

- [ ] At least **one deliberate engagement device** is designed in (not “hope they like it”)
- [ ] There is a clear reason to **comment** (question, take, incomplete opinion, joke setup)
- [ ] There is a reason to **save/share** (list, framework, recipe, template, status signal)
- [ ] Device fits the creator’s long-term brand (don’t trade one viral for permanent wrong niche)

---

## End-to-end production loop (agent workflow)

When helping create a short:

1. **Define niche + persona constraints** (what engagement tactics are off-limits).
2. **Write the 3-second hook** first (visual + verbal + text).
3. **Outline retention beats** as a shot list (each beat = action + purpose).
4. **Inject engagement devices** before finalizing script.
5. **Self-score** the draft on Hook / Retention / Engagement (1–5 each); revise lowest score.
6. **Export checklist** for shoot/edit (angles, B-roll, on-screen text, CTA).

## What this Part 1 does *not* cover

- Part 2 (income, cross-posting, range, personal brand): `noe-murillo-pt2-framework.md`
- Platform-specific posting cadence, sounds, hashtags, SEO
- Production tooling: the `claw-shorts` skill; for an HTML deck + OBS STEM pipeline, see claw-work `docs/shortform/obs-deck-pipeline/`
