# 02 — Notion page: "5 Free Repos That Give Claude Real Design Skills (Every Link)"

- **Source:** https://noocap.notion.site/5-Free-Repos-That-Give-Claude-Real-Design-Skills-Every-Link-3dd508e99dda81ebade8ed0f108c895f
- **Publisher:** Claude Code Club (the page's URL property is claudecodeclub.ai)
- **Read:** 2026-11-22, headless Chrome. A plain fetch returns only the word "Notion" because the page is rendered by JavaScript. Some repeated loads got a Cloudflare "verify you are human" page; we did not try to get past it, and a later single load returned the full page.
- **Screenshots:** none. The text below is the record.

## What it is

This is the link page for the same five repos @duncanrogoff listed in the video already covered in
[`../../tiktok-motion-sfx-design-2026-11-14/themes/04-duncanrogoff.md`](../../tiktok-motion-sfx-design-2026-11-14/themes/04-duncanrogoff.md)
(Claude Code Club is his community). Most of it repeats that research. It adds install commands, workflow tips and one correction.

### The five items as the page lists them

| # | Page title | Repo given | Install line on the page |
|---|---|---|---|
| 1 | Taste Skill — kill the AI-generated look | github.com/Leonxlnx/taste-skill | `npx skills add Leonxlnx/taste-skill -g` |
| 2 | UI/UX Pro Max — give Claude actual taste | github.com/nextlevelbuilder/ui-ux-pro-max-skill | `/plugin marketplace add nextlevelbuilder/ui-ux-pro-max-skill` then `/plugin install ui-ux-pro-max@ui-ux-pro-max-skill` |
| 3 | Awesome Claude Design — 68 ready-made design systems | github.com/VoltAgent/awesome-claude-design (MIT) | `git clone https://github.com/VoltAgent/awesome-claude-design.git` |
| 4 | Motion presets — make your builds actually move | none: "the strongest verified home for this is the GSAP motion built into Taste Skill" | — |
| 5 | DESIGN.md Chrome extension — export any site's design system | github.com/bergside/design-md-chrome (Chrome Web Store: "DESIGN.md Style Extractor — TypeUI") | load unpacked from `chrome://extensions` with Developer mode on |

### Item 4 disagrees with our earlier research

The page says: "'Design Motion Principles' maps to the GSAP motion inside Taste Skill (and UI/UX Pro Max's motion presets) rather than a standalone repo I could confirm by that name."

The video does show a standalone repo by that name. Its path, `kylezantos/design-motion-principles`, is on screen (see the 11-14 screenshot `04-kylezantos-design-motion-principles.jpg`). It is MIT-licensed, has 1,636 stars and was last pushed on 2026-05-30, checked on 2026-11-22. **Our 11-14 match stands. The Notion page is wrong on this point.**

### Tips on the page worth keeping (paraphrased)

- **The five work as a pipeline.** Export a DESIGN.md from a site you like, or start from one of the 68. UI/UX Pro Max and Taste Skill then hold the build to it: "reference in, system enforced, motion applied".
- **Give a real reference.** "Build against this DESIGN.md" beats "make it nice".
- **Don't over-animate.** Put motion on one or two deliberate moments per section, keep transitions quick and let most of the page sit still. This matches the "AI-slop" motion list already folded into `skills/claw-shorts/hyperframes/frame.md`.
- **Respect the brands you export.** Use the extracted systems to learn patterns, and don't copy a brand's identity.
- The page also concedes that the repos are free but "still run on your Claude usage".

## What's new versus what's already done

| Item | Status in our stack |
|---|---|
| Taste Skill | Already vendored in the private workspace's frontend-design-skills docs and installed as the `design-taste-frontend` skill (plus its variants). Nothing to do. |
| UI/UX Pro Max | Researched on 11-14 as "Maybe"; still web-UI oriented. The page's `/plugin` install line is new, but we don't need it for video. |
| Awesome Claude Design / DESIGN.md | Format adopted on 11-14 as `skills/claw-shorts/hyperframes/frame.md`; the files themselves are in `VoltAgent/awesome-design-md` (see the 11-14 evaluation). |
| Motion presets / Design Motion Principles | Rules already folded into `frame.md`. The page's identification is wrong (see above). |
| DESIGN.md Chrome | Still "Maybe": handy for pulling a reference look into DESIGN.md, but it needs a person in a real browser, so it isn't headless. |

**Verdict: nothing new to adopt.** The page is a duplicate of the 11-14 batch plus one wrong identification.
