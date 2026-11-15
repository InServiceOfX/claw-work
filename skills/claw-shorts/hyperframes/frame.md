---
version: alpha
name: claw-shorts — Frame (video layer)
description: >
  Video design spec for claw-work faceless Shorts rendered with HyperFrames.
  Same palette as Rust/claw-shorts/examples/style.json, so HyperFrames, canvas
  and FFmpeg renders look like one channel. Dark terminal ground, one green
  accent that carries meaning, condensed black display type, mono chrome.
unit: the frame — 1080×1920 vertical primary
principle: atoms are fixed · composition is free · numbers come from the narration

colors:
  ground: "#0b0f14"
  panel: "#18202b"
  ink: "#ffffff"
  muted: "#9aa4b2"
  accent: "#4ade80"
  warn: "#fbbf24"
  fail: "#f87171"

typography:
  hero:     { fontFamily: "Archivo Black", px: 150, lineHeight: 0.95, upper: true }
  title:    { fontFamily: "Archivo Black", px: 104, lineHeight: 1.0 }
  numeral:  { fontFamily: "Archivo Black", px: 300, lineHeight: 0.85, color: "{colors.accent}" }
  caption:  { fontFamily: "Archivo Black", px: 68, lineHeight: 1.12 }
  chip:     { fontFamily: "Space Mono", px: 40, weight: 700 }
  label:    { fontFamily: "Space Mono", px: 34, weight: 400, color: "{colors.muted}" }
  code:     { fontFamily: "Space Mono", px: 36, weight: 400 }

spacing:
  side: "90px"          # content never closer to the left/right edge
  top-safe: "168px"     # platform status/search bar
  band-bottom: "1330px" # every word above this line
  captions-top: "1150px"
  content-bottom: "1100px" # scene content above the caption band

components:
  chip:
    backgroundColor: "{colors.panel}"
    border: "3px solid {colors.accent}"
    rounded: "999px"
    typography: "{typography.chip}"
    description: "A fact with a source: license, count, where it runs. One chip lands per spoken fact."
  card:
    backgroundColor: "{colors.panel}"
    rounded: "28px"
    shadow: "none"
    description: "Code, terminal or diagram surface. Flat; no drop shadow, no glass."
  numeral:
    typography: "{typography.numeral}"
    description: "List index (01, 02, 03) — the dominant element of a list scene."
---

# claw-shorts — Frame

## Overview

A developer's terminal at phone scale. The ground is near-black
(`{colors.ground}`), content sits on flat `{colors.panel}` cards, and the one
accent, terminal green `{colors.accent}`, marks the word or number the
narration is saying right now. Archivo Black declares; Space Mono labels.

## Rules

- **Frame 0 is the hook.** The first frame already shows the claim in hero type;
  never a black frame or a logo.
- **One idea per scene**, one dominant element at 3–6× its nearest neighbour.
- **Accent means "this is the point".** One accent word per caption group, one
  accent element per scene. Never decorative.
- **Text lands on the word that says it**, from the transcript's timings, and
  holds dead still once landed.
- **Safe band.** Nothing above 168 px; every word above 1330 px; scene content
  above 1100 px so captions own 1150–1330 px; 90 px side margins.
- **Facts carry sources.** A chip states a license, a count or where it runs,
  and the same fact is in the posting caption.
- **Disclosure.** The end card names the synthetic voice.

## Don't

- No breathing, pulsing or floating loops on titles or chips.
- No blur-in on every element; vary entrances per scene.
- No full-screen linear gradients (they band under H.264).
- No synthesized dings as SFX on non-interface moments.
