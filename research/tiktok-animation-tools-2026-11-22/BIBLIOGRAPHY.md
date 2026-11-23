# Bibliography (all links verified 2026-11-22)

Star counts are live on 2026-11-22, from `gh api repos/OWNER/REPO`. "License" is GitHub's SPDX identifier. Where GitHub reports NOASSERTION or none, the license file itself was read. "Last push" is the UTC date.

## Sources

| Source | URL |
|---|---|
| @nicksaraevdaily — 20 free tools give Claude an animation studio | https://www.tiktok.com/@nicksaraevdaily/video/7708682252066016534 |
| Claude Code Club Notion page — 5 free design repos | https://noocap.notion.site/5-Free-Repos-That-Give-Claude-Real-Design-Skills-Every-Link-3dd508e99dda81ebade8ed0f108c895f |
| Quoted X post with the 20-tool grid, by "eigen" (@eigenqq) | Not found; seen only inside the video |

## Named in the video's voice-over (5)

| Name | Repo | License | Stars | Last push | Notes |
|---|---|---|---|---|---|
| Remotion Skills | https://github.com/remotion-dev/skills | No LICENSE file; the repo is generated from `remotion-dev/remotion` `packages/skills`, so the **Remotion License** applies | 7,815 | 2026-11-18 | 13 skills (`remotion-best-practices`, `-create`, `-markup`, `-render`, `-captions`, `-maps`, `-studio`, …); `npx remotion skills add` or `npx skills add remotion-dev/skills` |
| Remotion | https://github.com/remotion-dev/remotion | Remotion License (custom; GitHub: NOASSERTION) | 75,003 | 2026-11-22 | Free for individuals, for-profits with up to 3 employees, non-profits and evaluation; Company License otherwise |
| Motion Canvas | https://github.com/motion-canvas/motion-canvas | MIT | 20,150 | 2026-07-02 | Latest release v3.17.2 (2024-12-14). Rendering is started from its browser editor; issue [#415 "Render projects headlessly"](https://github.com/motion-canvas/motion-canvas/issues/415) has been open since 2023-02-25 |
| Revideo (Motion Canvas fork) | https://github.com/midrender/revideo | MIT | 4,226 | 2026-07-15 | Adds headless `renderVideo()`; mentioned as the headless route if Motion Canvas style were wanted |
| Manim Community | https://github.com/ManimCommunity/manim | MIT | 42,821 | 2026-11-22 | v0.22.1 (2026-10-27); `pip install manim`; MathTex needs a LaTeX install |
| 3b1b/manim (ManimGL) | https://github.com/3b1b/manim | MIT | 96,826 | 2026-09-09 | Grant Sanderson's original; OpenGL, interactive-first. Manim Community is the documented, pip-stable one |
| Blender | https://github.com/blender/blender (official mirror) | GNU GPL (GitHub: NOASSERTION; `COPYING`: "Apart from the GNU GPL, Blender is not available under other licenses") | 22,340 | 2026-11-22 | Headless: `blender -b file.blend -P script.py -a` |
| Matter.js | https://github.com/liabru/matter-js | MIT | 18,668 | 2026-09-30 | npm 0.20.0 (2024-06-23); fixed-step `Engine.update`, so deterministic only if stepped with a fixed `delta` |

## Readable on the X-post grid (10 more)

| Name | Repo | License | Stars | Last push | Notes |
|---|---|---|---|---|---|
| Anime.js | https://github.com/juliangarnier/anime | MIT | 74,990 | 2026-11-02 | v4 animations take `autoplay: false` and `.seek()`; HyperFrames has an anime.js adapter (`window.__hfAnime`) |
| Motion (motion.dev) | https://github.com/motiondivision/motion | MIT | 35,464 | 2026-11-22 | Hybrid WAAPI/JS engine; controls are pausable and `time` is settable, but HyperFrames has no Motion adapter (it has a WAAPI one) |
| React Spring | https://github.com/pmndrs/react-spring | MIT | 29,326 | 2026-11-23 | Spring physics integrated per animation frame; no seek-to-time API |
| mo.js | https://github.com/mojs/mojs | MIT | 18,944 | 2026-07-30 | Last release v1.7.1 (2023-10-06); `setProgress()` exists but no HyperFrames adapter |
| Flubber | https://github.com/veltman/flubber | MIT | 7,011 | 2022-11-08 | npm 0.4.2 (2022); dormant but complete. `interpolate(a, b)(t)` is a pure function of t |
| Lottie Web | https://github.com/airbnb/lottie-web | MIT | 32,547 | 2025-09-01 | Not archived; no push for 14 months. Active successor: dotLottie Web (below) |
| Rive Runtime | https://github.com/rive-app/rive-runtime | MIT | 1,268 | 2026-11-23 | C++ runtime; web runtime is https://github.com/rive-app/rive-wasm (MIT, 1,033). Authoring needs the Rive editor (hosted service) |
| KUTE.js | https://github.com/thednp/kute.js | MIT | 2,657 | 2026-03-26 | v2.2.6 (2026-03-26); SVG/CSS tween engine |
| AnimXYZ | https://github.com/ingram-projects/animxyz | MIT | 2,508 | 2026-11-20 | CSS-variable keyframes for enter/leave transitions in Vue/React; `@animxyz/vue@0.6.9` (2026-07-27) |
| Inochi2D | https://github.com/Inochi2D/inochi2d | BSD-2-Clause | 1,885 | 2026-11-20 | Realtime 2D puppet (VTuber) SDK in D; rigging app https://github.com/Inochi2D/inochi-creator (BSD-2-Clause, last push 2025-06-16) |

## Related tools worth knowing (not in the sources)

| Name | Repo | License | Stars | Last push | Why |
|---|---|---|---|---|---|
| dotLottie Web | https://github.com/LottieFiles/dotlottie-web | MIT | 955 | 2026-11-21 | Rust + WASM (ThorVG) Lottie player; what HyperFrames' Lottie adapter uses |
| velato | https://github.com/linebender/velato | Apache-2.0 OR MIT | 164 | 2026-09-16 | Rust Lottie parser/renderer on Vello: a Rust-native path for Lottie overlays |
| Rapier | https://github.com/dimforge/rapier | Apache-2.0 | 6,111 | 2026-11-01 | Rust 2D/3D physics (`rapier2d`), with cross-platform determinism as an opt-in feature; the heavyweight alternative to writing our own |
| HyperFrames | https://github.com/heygen-com/hyperframes | Apache-2.0 | — | — | Already adopted (11-14). Its runtime adapters: GSAP, CSS keyframes, WAAPI, Lottie/dotLottie, anime.js v4, Three.js, d3, TypeGPU, map libraries |
| GSAP | https://github.com/greensock/GSAP | GreenSock Standard "No Charge" License (no LICENSE SPDX) | 31,095 | 2026-11-04 | Already the HyperFrames default |

## Notion page repos (5, all re-checked)

| Name | Repo | License | Stars | Last push |
|---|---|---|---|---|
| Taste Skill | https://github.com/Leonxlnx/taste-skill | MIT | 120,212 | 2026-11-22 |
| UI/UX Pro Max | https://github.com/nextlevelbuilder/ui-ux-pro-max-skill | MIT | 152,922 | 2026-11-22 |
| Awesome Claude Design | https://github.com/VoltAgent/awesome-claude-design | MIT | 4,666 | 2026-06-20 |
| awesome-design-md (where the DESIGN.md files are) | https://github.com/VoltAgent/awesome-design-md | MIT | 128,668 | 2026-10-28 |
| Design Motion Principles | https://github.com/kylezantos/design-motion-principles | MIT | 1,636 | 2026-05-30 |
| Design.md Chrome | https://github.com/bergside/design-md-chrome | MIT | 3,207 | 2026-05-25 |
