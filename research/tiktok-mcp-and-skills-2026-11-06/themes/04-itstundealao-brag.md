# Tunde Alao (@itstundealao) — /brag HyperFrames launch-video generator

- Video: https://www.tiktok.com/@itstundealao/video/7686156516189080854
- Auto-captions: [`../transcripts/itstundealao-7686156516189080854.md`](../transcripts/itstundealao-7686156516189080854.md)
- Screenshots: [`../screenshots/itstundealao/`](../screenshots/itstundealao/)
- Theme: "Fact or Hype?" test of `/brag` skill for launch video generation
- CTA: none (skill review video)

## What is /brag?

A Claude skill from `latent-spaces/brag` that turns any side project into a ~20 second launch video from a single command.

- Repo: https://github.com/latent-spaces/brag (MIT)
- Docs: https://latent-spaces.github.io/brag/
- App landing: https://letsbrag.app

## Install

As a plugin:

```sh
/plugin marketplace add latent-spaces/brag
/plugin install brag@brag
```

Or via npx:

```sh
npx skills add https://github.com/latent-spaces/brag --skill brag
```

Dependencies: HyperFrames + FFmpeg + Node 22+

## Tunde's test

Tunde runs `/brag` on his storage-cleaning app ("freemyphone" — duplicate photo cleaner) that he launched the week before.

### Timing and cost

- Wall time: ~10 minutes
- Output tokens: ~61,400
- Input tokens: ~676
- Cost: **~$6.17**

The usage overlay shows skills consuming limits: `/hyperframes-cli`, `/brag`, `/hyperframes`.

### Verdict

**Fact** — the output is a "fire ~20s launch video." His reaction: "What the heck? How did it even get that? No, that's actually crazy."

## How it works

1. `/brag` analyzes the project (repo, app landing, features).
2. It generates a script/storyboard for a short launch video.
3. It calls HyperFrames to render the video with motion graphics.
4. Output is a vertical video suitable for TikTok/Reels.

The prompt shown in the video: "make a vertical version for TikTok."

## Related research

- HyperFrames workflows covered in `research/tiktok-skill-research-2026-11-05/`
- Ray Fu launch-video content systems covered in `research/ray-fu-content-and-marketing-2026-11-05/` (PR #4)

## Screenshot index

- `01-github-brag-repo.jpg` — GitHub `latent-spaces/brag` repo page
- `02-brag-command.jpg` — `/brag` command being run in Claude
- `03-cost-token-breakdown.jpg` — Usage overlay: ~$6.17, ~61k output tokens
- `04-output-video-mock.jpg` — Output video mockup/preview
