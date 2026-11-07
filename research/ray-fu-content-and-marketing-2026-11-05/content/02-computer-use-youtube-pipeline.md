# Computer-use YouTube script & production pipeline

- **URL:** https://www.tiktok.com/@ray_fu/video/7663284714752494879
- **Views / duration:** ~205.4K / 104s (Jul 16, 2026)
- **What the video shows:** Niche RPM again (finance explainer $15–50 CPM; AI/tech $10–25; gaming ~$2). Claude “Fable” agent with computer-use scrapes YouTube for faceless niche videos ≥500K views this month, transcribes winners, then prompt: “Here’s a video that got 2M views… write a better 8-minute script with updated info, better hook, better story.” Higgsfield MCP Explainer skill turns script into VO + captions + animation; splits long-form into Shorts/TikTok/IG. Hermes/Claude agent posts 5–6×/week.

## Paid SaaS in video → replacements
| Video | Replacement |
|---|---|
| Higgsfield MCP / Explainer | Remotion templates + Playwright/ffmpeg motion pipeline (see motion-graphics video); or `darkzOGx/youtube-automation-agent` production agent with local TTS |
| Claude Fable computer-use YouTube scrape | `yt-dlp` search + YouTube Data API + Agent-Reach YouTube channel; transcript via yt-dlp subs / faster-whisper |
| Hermes auto-poster | Scheduled Claude/Codex job + YouTube Data API upload with **manual approve** |

## Prompt framework (preserve)
```
Here is a transcript of a video that got {N} views on {topic}.
Write a better {duration}-minute script that covers the same subject with:
- updated information (cite current year facts)
- a stronger pattern-interrupt hook in the first 15 seconds
- a clearer story arc and soft CTA
Output spoken English with pause markers and visual direction tags.
```

## Adaptation notes
Prefer **inspired-by** structure (hook/pacing/sectioning) over verbatim rewrite of another creator’s script. Keep a human editorial pass. High-RPM niche math is the reusable insight; Higgsfield is not required.
