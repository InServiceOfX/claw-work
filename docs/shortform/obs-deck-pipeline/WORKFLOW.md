# WORKFLOW.md — HTML-deck + OBS pipeline for short-form STEM video

For any AI agent (Claude Code, Codex, Hermes, or otherwise) asked to help
produce a short vertical (9:16) explainer video — equations, code, diagrams,
anything screenshot-able — read this before reinventing any of it. Everything
below was actually hit and fixed in one real production session, not
theorized in advance.

## The core technique

Build the entire visual side as **one self-contained HTML file**:

- One `<div class="slide">` per beat. JS toggles a single `.active` class;
  Prev/Next buttons and Left/Right arrow keys move between them. No slide
  library, no build step — it's ~150 lines of vanilla JS/CSS.
- A `data-narration="..."` attribute on each slide holding that beat's
  teleprompter line. A script reads `slides[idx].dataset.narration` into a
  visible-but-not-recorded panel — the presenter reads this live while
  recording; see "the recorded region vs. the reading region" below for how
  it stays out of the actual footage.
- Equations are **hand-typeset in CSS**, not rendered via KaTeX/MathJax.
  Fractions are a flex column with a `border-bottom` divider; square roots
  are a radical glyph next to a `border-top`'d radicand; vectors are two
  stacked rows between oversized bracket glyphs. This avoids a CDN
  dependency entirely (Claude Artifacts' CSP blocks font/script CDNs
  outright, and a self-hosted KaTeX is heavy) and gives pixel-identical
  output across every screenshot/recording tool, instead of depending on
  whatever math-rendering an editor's markdown preview happens to use.
- Any image (e.g. a screenshot of the actual paper) gets embedded as a
  `data:image/png;base64,...` URI directly in the HTML — fully
  self-contained, works from a bare `file://` URL, no server needed.

Why HTML+OBS instead of a conventional video editor for the visual side:
equation slides are fundamentally *typeset documents*, and CSS is a more
direct tool for that than compositing images in a timeline editor. The only
place a real editor (OpenCut, etc.) enters this pipeline is at the very end,
trimming dead air off a single recorded take.

## The recorded region vs. the reading region

The page has a bordered "frame" div (the thing that actually gets cropped
into the recording) and, outside it in normal document flow, a narration
panel, Prev/Next buttons, and a live readout of the exact OBS crop numbers.
The presenter sees all of it; OBS is configured (via a Crop/Pad filter, see
below) to capture only the frame. This means:

- You get a live, visible teleprompter without it ever touching the footage.
- No second window, no cross-window state sync, no BroadcastChannel hacks —
  it's one page, one browser tab, one set of click/arrow-key controls.

## OBS setup (the part that actually cost the most debugging time)

1. **Source**: add a **macOS Screen Capture** source (modern OBS on macOS
   merged what used to be separate "Display Capture"/"Window Capture" source
   types into this one, using ScreenCaptureKit). In its properties, set
   Capture Type to **Window**, targeting the specific browser window showing
   the deck (Chrome, in the run this doc is based on — Safari works too, any
   Chromium/WebKit browser does).
2. **Crop/Pad filter** on that source, using the four numbers the page
   itself computes and displays live (`getBoundingClientRect()` on the frame
   element vs. the window's viewport, in both logical and Retina-2x-scaled
   units — OBS may report either depending on version, so the page shows
   both and you try one against OBS's own live preview). **These numbers are
   specific to the exact window size at the moment they were read — re-read
   them from the page each session, never hardcode a previous run's
   values.**
3. **Canvas resolution — separate from the above, and easy to miss**:
   `Settings → Video → Base (Canvas) Resolution` **and**
   `Output (Scaled) Resolution` both need to be set to your target output
   (e.g. `1080x1920`). Cropping/scaling a *source* never touches the
   recording *canvas* — if you skip this, the .mov comes out full
   widescreen with your correctly-cropped content stuck in one corner and
   black filling the rest. This bit us after a full take had already been
   recorded; there's no fixing it in post short of re-cropping the whole
   file, so verify canvas resolution *before* the real take, not after.
4. **Source Transform** (right-click source → Edit Transform): Bounding Box
   Type → "Stretch to bounds" (or scale-to-inner — same result when source
   and canvas aspect ratios already match), Size → your canvas resolution
   (`1080 x 1920`), Position → `0, 0`.

**Actual working values from one real run** (illustrative — always re-derive
the crop numbers per session, everything else is stable across sessions):

```
Settings → Video:        BaseCX=1080  BaseCY=1920  Output=1080x1920  FPS=60
Source:                   macOS Screen Capture, type=Window, app=Chrome
Crop/Pad filter:          left=659  top=334  right=659  bottom=661
Source Transform:         bounds_type=stretch, bounds={1080,1920}, pos={0,0}, scale={1,1}
```

## Bugs actually hit, and the fix (read this before you re-derive it)

- **Negative crop values → OBS shows diagonal hash marks instead of your
  content.** Cause: the browser window was shorter than the full page
  (title + frame + narration + buttons + hint stacked vertically), so the
  frame's bottom edge was scrolled below the visible viewport when the crop
  numbers were computed. `viewportHeight − frame.bottom` went negative, and
  OBS's Crop/Pad filter treats a **negative value as pad**, not crop — it
  silently adds empty space instead of erroring. The fix isn't "resize the
  window correctly," it's making the page size itself so this can't happen:
  measure `document.documentElement.scrollHeight` vs. `window.innerHeight`
  on load/resize, and if the page overflows, shrink the frame (preserving
  its aspect ratio, so `Δwidth × 16/9 = Δheight` exactly) until it doesn't.
- **A single-shot size correction can undershoot.** The natural instinct is
  to compute the needed shrink once from the measured overflow. In practice
  this converged wrong by exactly the height of a placeholder string: a
  crop-number readout element started with short "calculating…" text and
  only got its real (taller, multi-line) content *after* the fit routine
  ran — so the fit routine sized against a page that was about to grow
  taller right after it finished. Fix: populate any content that changes
  size dynamically **before** measuring, not after. More generally: iterate
  the fit (a few `requestAnimationFrame` passes checking convergence) rather
  than trusting a single closed-form correction, since anything on the page
  whose size depends on its own content (not just CSS) can throw off a
  one-shot calculation.
- **Bluetooth headset mic + OBS Monitoring = distracting echo, not a
  bug but worth knowing.** Turning on Audio Monitoring for a mic source
  doesn't affect what's recorded — monitoring only controls whether *you*
  hear that source live. If your mic and your listening device are the same
  Bluetooth headset, monitoring introduces the headset's normal ~100–300ms
  round-trip latency, which is genuinely hard to talk over. Leave Monitoring
  off and rely on the Mixer's visual level meter (zero latency) to confirm
  the mic is live instead of listening by ear.
- **Bluetooth headset mics force a lower-quality call-audio profile the
  instant the mic activates** — both directions (what you hear and what
  gets recorded) drop to mono/narrower-band the moment the mic is in use.
  This is a Bluetooth protocol limitation (HFP/HSP vs. A2DP), not an OBS
  setting — nothing to configure around it, just don't be surprised the
  recorded voice sounds thinner than the same headphones sound for music.

## Verify with Playwright before trusting any layout claim

Install once per machine as its own small project (not tied to any one
video's repo — this is a general tool):

```bash
mkdir -p tools/playwright-runner && cd tools/playwright-runner
bun add playwright
bunx playwright install chromium   # ~260MB, one-time
```

Before relying on the deck's layout, load it headlessly and check:

- No `pageerror`/console errors.
- All expected slides present (`document.querySelectorAll('.slide').length`).
- Screenshot the **frame element specifically**
  (`page.locator('.frame').screenshot(...)`), not just a full-page
  screenshot — full-page screenshots in headless Chromium can show
  paint-timing artifacts (a slide that looks blank in one screenshot but
  renders fine in an element-scoped screenshot a moment later) that look
  like real bugs but aren't. Cross-check with an element screenshot before
  concluding something is actually broken.
- Test at more than one viewport size, **including a short/tiled-window
  size** (e.g. 800×900), not just one comfortable default — the negative-
  crop bug above only shows up when the window is shorter than the page.

## Where the files live

Code, tex, and the human-readable script/teleprompter doc live in the
actual project's repo (for example a `Documents/<Series>.md` next to the
code the series explains). Anything that's a **produced artifact** for one
specific video — the slide-deck HTML itself, source screenshots, the
exported `.mov`/`.mp4` — lives under
`Data/Public/Generated/<SeriesName>/EpisodeN/` on the user's machine, never
committed into the code repo. The code repo's job is to stay just code;
video production output isn't that.

## OBS project state — do you need to save anything?

No explicit "save project" step exists or is needed — OBS continuously
persists its **Scene Collection** and **Profile** to disk as you make
changes:

```
~/Library/Application Support/obs-studio/basic/scenes/<name>.json      (+ a .bak of the prior version)
~/Library/Application Support/obs-studio/basic/profiles/<name>/basic.ini
```

Practical recommendations, not requirements:

- Rename the default "Untitled" Scene Collection and Profile to something
  identifiable (e.g. the series name) via OBS's Scene Collection / Profile
  menus, so the whole source+filter+transform setup is trivially reusable
  for the next episode instead of rebuilt from scratch.
- `File → Export Scene Collection` for a portable JSON backup, if you want
  one outside OBS's own app-support directory.
- An agent (this one included) can read those JSON/ini files directly via
  Bash to inspect or document the exact working configuration — there's no
  GUI-automation access to OBS itself, but the on-disk config is plain,
  readable JSON/INI.

## Is this actually faster? (honest accounting)

The prompt for building this was: an intern's 30-second bio video took
another intern 1–2 hours of scripting and 3–4 hours of editing, not counting
lights/camera setup. This pipeline does not make a *single* video faster the
first time through — this session's Episode 1 took a full extended session,
dominated by genuinely debugging the crop/canvas/audio issues above. What it
*does* do is make that debugging cost a one-time, front-loaded, and now
fully documented/code-fixed cost:

- The deck's auto-fit logic and live crop-readout mean the negative-crop bug
  class cannot recur, on any future episode, on any window size.
- The OBS Scene Collection/Profile persists and is reusable — episode 2
  reopens the same scene, same source, same filter chain; only the crop
  numbers need re-reading if the window size changed.
- What's genuinely *not* compressible by tooling: the script-wording pass
  itself (a human deciding exactly what to say, beat by beat) and the live
  performance of the recording. Those are creative/human work, not
  automation targets.

Rough expectation for episode 2+, with this groundwork already in place:
script iteration 30–60 min (human time), OBS setup ~5 min (reused scene),
the take itself 5–15 min including a retry or two, trim/export 10–15 min —
well under the fixed cost of a from-scratch pipeline, because the fixed cost
was paid once, here.
