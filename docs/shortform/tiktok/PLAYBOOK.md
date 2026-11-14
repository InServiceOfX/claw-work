# Playbook: closing, cold outreach, entity maps, and short-form content

A synthesis of seven TikToks ingested 2026-10-30 plus YouTube Shorts from 2026-10-31 (see [README.md](README.md)),
aimed at one goal: **market an app and build faceless short-form channels that
generate income online.** Verbatim lines are quoted from the per-video notes,
which have full context, timestamps, and screenshots.

Sources: closing ([dani-closes](danidocumented-7698978039249521933/README.md)),
follow-ups ([dani-followups](danidocumented-7698982436352445709/README.md)),
psychology ([dani-20-hacks](danidocumented-7699623985130081549/README.md)),
entity map ([jamout.ai](jamout.ai-7700225527377857822/README.md)),
framing ([ronan0437](ronan0437-7688325930997075214/README.md)),
following ([themekhicarter](themekhicarter-7696704990320004366/README.md)),
editing ([ginyboi](ginyboi-7678511193039981854/README.md)),
income streams and story-wrapped ads ([wanderlustalley YouTube Short](../youtube/wanderlustalley-cY6GojPFOYc/README.md), added 2026-10-31).

---

## 1. Closing phrases (calls, demos, DMs)

Put these in your own words. The source warns that "regurgitated closes"
make you sound like every other salesperson.

| Situation | Line |
|---|---|
| Vague stall | "What are you afraid of happening?" If they deflect, try a playful absurd guess ("So you're afraid of me taking your money and going to Vegas?"). They laugh, and then they tell you the real reason. |
| "You're more expensive" | "Well, if we were the same price, which one would you pick?" ... "Why?" ... "So now you understand why we aren't the same price." |
| Indecision | "Do you know where the word decide comes from? In Latin, to cut off. So we cannot make a decision without cutting off something. So which future are you cutting off?" |
| Cost of inaction | "You do it, you get 10% better, that's worst case. You do it, you get 100% better, your whole life changes. Or the riskiest option of all, you walk out that door and you're in the exact same spot you're in now, six months from now." |
| Repeated past failures | "You're not gonna fail forever, right? So, when does that stop?" |
| Final push | "If your competitor was watching this call right now, would they want you to say yes, or would they want you to say exactly where you are?" |
| "It's a lot of money" | Agree with them, then go quiet. |
| Asking for the sale | Don't ask *if* they want to buy. Ask *how* they're going to invest. |

**Call mechanics:** tell them they might not be a fit; go silent after an
objection and after the price ("sat in silence for 21 seconds and closed the
deal"); repeat their words back; collect small yeses before the price; anchor
on high market prices; let them name a price first; ask "what's stopped you
from solving this before?"; sell the cost of doing nothing; be genuinely
willing to walk away. Use the honest versions. Skip fake scarcity and pressure.

## 2. Cold emails and DMs

**Never write:** "I'm just following up", "just checking in", or "did you get
a chance to look at the proposal?" They hand the prospect an easy reason to ignore you.

**A 4-touch sequence for an app, built from the videos:**

1. **Open by giving value first (reciprocity, hack #8) with a qualifying frame (hacks #1/#14):**
   "Hi {name}, I noticed {specific observation}. Here's one quick fix: {tip}.
   We built {app} for {who}; it might not be a fit if {disqualifier}, but if
   {pain} is costing you {time/money}, happy to show you in 10 minutes."
   Mirror their words: reuse the exact phrasing from their site or post (hack #3).
2. **Assumptive follow-up with something new:** "Hey {name}, getting back to
   you after a busy week. I have two new ideas I wanted to run by you before you
   decide. Does {day} {time} {timezone} work?"
3. **Promised follow-up (set it up in touch 2):** "I'll send the 60-second
   walkthrough tomorrow." Next day: "As promised, here it is: {link}. Any
   thoughts, and are you ready to try it?"
4. **Break-up / flip:** "Hey {name}, I just saw your name and was wondering,
   did you decide to give up on {their stated goal}?" The source's logic: most
   people won't say yes to that, so they get back to you.

## 3. Entity map (getting recommended by LLMs)

Goal: when someone asks ChatGPT, Claude, or Perplexity for "a tool that does X",
they name our app.

- Publish an **entity map**: a human-readable page listing the entities (the
  organization, the founder (Person), the app (SoftwareApplication), its
  features or services, events, named methods or terms), each with a short
  description, **relations** (`OFFERS`, `TARGETS`, `CREATED_BY`), and
  **evidence** (testimonials, press, where we were recommended). The same
  content is published as **JSON-LD schema markup** on the site.
- Only map what the site actually says. Strengthen the site first with
  paragraphs that state what is different, who is on the team, and the full
  range of what we offer.
- Link the short-form channels as `sameAs` so the faceless brand and the app
  resolve to one entity.
- Measure: a monthly prompt check ("best tool for X?") across LLMs, logging
  citations.

## 4. Short-form content (faceless-first)

**Strategy**
- **Viral is not the same as followers.** Two of one creator's videos with
  similar views gained 573 vs. 6.3K followers. The difference was value.
  Track follows per 1K views.
- A faceless channel can't rely on looks or on-camera personality, so it has to
  win on **specific knowledge** plus a recognizable format and voice.
- **Give a reason to come back:** numbered series and save-bait lists
  ("6 closes so good they feel unethical", "20 hacks, save these").
- **Consistency wins:** be unavoidable on the timeline. Batch and schedule.

**Production pipeline (maps onto automation)**
1. Script (LLM) -> 2. Voiceover/A-roll (TTS) -> 3. Auto-cut pauses
(transcript-based) -> 4. B-roll every 3-5s, or a branded **"B-roll space"**
frame that shows app screens and icons -> 5. Captions (short and bouncy for
young audiences, clear full lines for older ones) -> 6. Light music and SFX,
kept "raw" so effects don't steal from the message.

**Framing and safe zones**
- Text hook at the **top** of the frame, not mid-chest.
- Subject or eye line about **one third down**, with headroom.
- Captions around the **vertical middle**, clear of the bottom UI and the
  right-edge buttons.

**Formats worth reusing (all work faceless)**
- A numbered list that builds up on screen with voiceover (dani videos).
- "Never say X, say Y instead" on a whiteboard (follow-ups video).
- "Never here, always here" before/after with red and green markers (framing video).
- Chapter cards ("Chapter 1 ... Chapter 5") for step-by-step tutorials (editing video).
- A proof card up front (an analytics or results screenshot) to earn the next 30 seconds.
- A ~1-second full-screen numbered list as the very first frame (the "12 streams of
  income" card). It is too dense to read in one pass, so viewers rewatch or screenshot it (wanderlustalley).
- "Part N of adding another one": a build-in-public series that ends on a
  cliffhanger ("all we have to do now is launch") (wanderlustalley).

## 5. Passive-income streams and native ads (YouTube Short)

From a sponsored Short ([wanderlustalley](../youtube/wanderlustalley-cY6GojPFOYc/README.md))
whose creator lists **12 streams of income**: YouTube Adsense, Group Trip Host,
Brand Partnerships, Marketing Producer, Affiliate Links, Buy Me a Coffee, Airbnb,
Meta Adsense, UGC content, Merch, Bikini line, Google Adsense. Stream 13 is a
Printify print-on-demand shop.

- **Streams that work faceless:** platform ad revenue (YouTube, Meta, Google),
  affiliate links, Buy Me a Coffee or tips, merch or print-on-demand. Brand deals
  and UGC come after the audience. Many "passive" streams on such lists are really active work.
- **Story first, product as the payoff:** about 40s of personal story and
  process B-roll, then the sponsor or app in the last 8s. This is a template
  for demoing our apps without it feeling like an ad, and for briefing creators
  if we pay for native placements.
- **A build-in-public series for the apps:** "I have N apps, this is part 2 of
  launching #N+1."

## 6. Next actions

- [ ] Template the 4-touch cold sequence for the first app we market.
- [ ] Draft the app's entity map page + JSON-LD (a small generator script could live in `Python/`).
- [ ] Add caption and hook safe-zone defaults and a "B-roll space" layout to the short-form generator (see `../MONEYPRINTERTURBO_ASSESSMENT.md`). Done since: safe zones and an 8-word hook rule in `Rust/claw-shorts`; Ken Burns stills in `Rust/shortform-video`. The B-roll space layout is still open.
- [ ] Pilot a faceless "I have N apps / part N of launching the next one" series with a list-card hook.
- [ ] Start a numbered faceless series (for example "closing lines that work") using these notes as the script bank.
