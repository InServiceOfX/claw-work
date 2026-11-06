# Cecilia Kim (@ceciliakimdesign) — App UI polish with Paper + Mobbin MCPs

- Video: https://www.tiktok.com/@ceciliakimdesign/video/7691841009700834590
- Auto-captions: [`../transcripts/ceciliakimdesign-7691841009700834590.md`](../transcripts/ceciliakimdesign-7691841009700834590.md)
- Screenshots: [`../screenshots/ceciliakimdesign/`](../screenshots/ceciliakimdesign/)
- Theme: MCP-driven UI design polish (Paper + Mobbin)
- CTA: none (organic build diary, part 9/14)

## Walkthrough

Day 9 of a 14-day app build ("Nessie" — a finance/planning demo). Cecilia switches from Codex to **Claude Code + Opus 5.5** with two MCPs:

1. **Paper MCP** (`https://paper.design/docs/mcp`) — provides Paper Shaders (ready-made effects like water, grain, gradients).
2. **Mobbin MCP** (`https://mobbin.com/mcp`) — provides app screenshot lookups for UI inspiration.

She starts with a Pinterest vibe board to lock in the aesthetic before any design work. Claude generates 5 design directions using Paper Shaders; she combines her two favorites (background from one, components from another). She adds film grain texture for mood, then animates the background with a **Metal shader** (Apple's graphics code) for moving water effect.

When it still looks "tacky," she looks up apps like Partiful on Mobbin. Their pattern: dark background, color only at the top, darker components. Perfect for Nessie since "the Loch Ness monster lives in dark water."

## Typography

- **Inter** — for all readable text (clean, legible sans-serif).
- **EB Garamond Italic** — for titles (coziness, personality).
- Both are free on Google Fonts.

## UX simplification

She removes drag handles and arrows from lists and adds one "reorder" button that opens its own sheet. More intuitive, less clunky.

## Technical details

The moving water effect:

- Metal shader runs in SwiftUI with `colorEffect`.
- A `TimelineView` redraws it ~30 times per second.
- Only the lit water at the top moves (sinking into deep dark blue).
- Stops completely with Reduce Motion or Low Power Mode on.
- Cards and buttons use Apple's **Liquid Glass** on iOS 26.

## Install commands

Paper MCP:

```sh
claude plugin marketplace add paper-design/agent-plugins && claude plugin install paper-desktop@paper
```

Mobbin MCP:

```sh
claude mcp add mobbin --scope user --transport http https://api.mobbin.com/mcp
```

## Skill / workflow to reuse

1. Start with a vibe board (Pinterest aesthetic pictures) before any design.
2. Connect Paper MCP and Mobbin MCP to Claude Code.
3. Ask Claude for design directions using Paper Shaders.
4. Combine favorites, add film grain texture.
5. For motion, use a Metal shader with `TimelineView` (respect Reduce Motion / Low Power).
6. Look up real apps on Mobbin when the design feels generic.
7. Pick 2 fonts: 1 readable (Inter), 1 with personality (EB Garamond Italic).
8. Simplify UX — consolidate interactions into single-action sheets.

## Screenshot index

- `01-nessie-home-moving-bg.jpg` — Nessie home screen with moving water background
- `02-five-design-directions.jpg` — Claude generating 5 Paper Shader design directions
- `03-mobbin-partiful-ui.jpg` — Mobbin inspiration (Partiful dark UI pattern)
- `04-reorder-sheet-ux.jpg` — Simplified reorder sheet UX
