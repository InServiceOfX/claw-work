# TikTok AI/MCP/skill workflow research (2026-11-06)

Parsed five public TikToks covering AI-assisted UI design, context engineering, design skill stacks, launch video generation, and single-file interactive figures. Stats below are from yt-dlp at download time (2026-11-06).

## Videos

| Creator | Handle | URL | Length | Views / likes / saves (download) | Theme | Notes file |
| --- | --- | --- | --- | --- | --- | --- |
| Cecilia Kim | [@ceciliakimdesign](https://www.tiktok.com/@ceciliakimdesign) | [7691841009700834590](https://www.tiktok.com/@ceciliakimdesign/video/7691841009700834590) | 71s | 6,224 / 423 / 293 | App UI polish: Claude Code + Paper MCP + Mobbin MCP | [themes/01-ceciliakimdesign-paper-mobbin.md](themes/01-ceciliakimdesign-paper-mobbin.md) |
| Beau Johnson | [@buildingnpublic](https://www.tiktok.com/@buildingnpublic) | [7703729143749397774](https://www.tiktok.com/@buildingnpublic/video/7703729143749397774) | 59s | 356 / 12 / 8 | ICM: folder-based agent context (97% fewer tokens) | [themes/02-buildingnpublic-icm.md](themes/02-buildingnpublic-icm.md) |
| Nate Herk | [@nateherkai](https://www.tiktok.com/@nateherkai) | [7684323553537232142](https://www.tiktok.com/@nateherkai/video/7684323553537232142) | 28s | 1,200,000 / 93,500 / 93,065 | UI taste & design skill stack | [themes/03-nateherkai-design-skills.md](themes/03-nateherkai-design-skills.md) |
| Tunde Alao | [@itstundealao](https://www.tiktok.com/@itstundealao) | [7686156516189080854](https://www.tiktok.com/@itstundealao/video/7686156516189080854) | 44s | 1,200,000 / 83,900 / 73,455 | `/brag` HyperFrames launch-video generator | [themes/04-itstundealao-brag.md](themes/04-itstundealao-brag.md) |
| SigmaflowAI | [@sigmaflowai2](https://www.tiktok.com/@sigmaflowai2) | [7703001513286700302](https://www.tiktok.com/@sigmaflowai2/video/7703001513286700302) | 34s | 45,900 / 1,958 / 1,909 | `/hairline-create` single-file editorial HTML | [themes/05-sigmaflowai2-hairline.md](themes/05-sigmaflowai2-hairline.md) |

Auto-captions (cleaned spoken text) live in `transcripts/`. Chosen stills (720px wide, readable UI, EXIF-stripped) live in `screenshots/<creator>/`.

## Theme map

1. **MCP-driven UI polish.** Cecilia shows Claude Code + Opus 5.5 with Paper MCP (shaders, design directions) and Mobbin MCP (app screenshots for inspiration). Metal shader for moving water background, Inter + EB Garamond typography, Liquid Glass cards on iOS 26.
2. **Interpretable Context Methodology (ICM).** Beau covers Jake Van Clief's folder-based agent architecture: one `_map.md` file at the root, numbered stage folders, and `CONTEXT.md` / `_config/` for decisions. The team's benchmark claims 97% fewer tokens vs re-reading chat history.
3. **Design skill stacks.** Nate demos four skills to avoid generic AI UI: Emil Kowalski design skills (typography/layout), Impeccable (design vocabulary), Taste Skill (reference great sites), plus Figma MCP + Playwright for design→build→screenshot loops.
4. **Launch video generation.** Tunde runs `/brag` from `latent-spaces/brag` on his duplicate-photo cleaner app. ~10 min wall time, ~$6 / ~61k output tokens, outputs a ~20s vertical launch video. Depends on HyperFrames + FFmpeg.
5. **Single-file interactive figures.** SigmaflowAI demos Hairline (`lucasmarkes/hairline`): `/hairline-create a sales funnel` → 2–3 concept options → one dependency-free HTML with cursor-reactive line art. 10 design rules (stroke highlights, rounded corners, no labels inside drawings).

## Related prior research

- **Ray Fu content/marketing systems**: already covered in open PR [#4](https://github.com/InServiceOfX/claw-work/pull/4) (`research/ray-fu-content-and-marketing-2026-11-05/`). Cross-reference that folder for HyperFrames, 5-agent short-form content team, and zero-paid-SaaS playbooks.
- **TikTok skill research (2026-11-05)**: `research/tiktok-skill-research-2026-11-05/` covers HyperFrames (Brian Scully, Adam Stewart, Jam), CapCut edit workflows (Sarah/abloommedia), and JS canvas video (Chase AI).

## Bibliography

See [BIBLIOGRAPHY.md](BIBLIOGRAPHY.md).
