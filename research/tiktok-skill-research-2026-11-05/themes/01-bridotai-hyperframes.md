# Brian Scully (@bridotai) — HyperFrames talking-head edit

- Video: https://www.tiktok.com/@bridotai/video/7701679351661743391
- Auto-captions: [`../transcripts/bridotai-7701679351661743391.txt`](../transcripts/bridotai-7701679351661743391.txt)
- Screenshots: [`../screenshots/bridotai/`](../screenshots/bridotai/)
- Theme: talking-head AI edit (HyperFrames)
- CTA: none that gates a download. He ends by saying you do not have to comment a keyword; he just wants to know if it helped.

## Walkthrough

Brian says influencers oversell AI video editing. He shows a ChatGPT-like plugin search for "HyperFrames", result text "HyperFrames by HeyGen - Write HTML, render video", and clicks "+ Install plugin". He then advises recording a talking-head piece as one continuous file: multiple takes of each line, then move on without stopping the recording. He drops `IMG_1228.MOV` into chat, tags `@Hyperframes`, and pastes an editing prompt. After custom visual tweaks, he says to ask the AI to save the process as a skill so later videos reuse the same style.

On-screen chat title (mock or local UI): "What should we build in bridotai?" Model label shown: "GPT-4.5 Sol Medium".

## Editing prompt (on screen, full text from the watch + screenshot)

The on-screen card at ~0:40–1:00 is titled "YOUR EDITING PROMPT / HYPERFRAMES / INSTRUCTIONS". The screenshot at `03-editing-prompt.jpg` clips the last line; the watch transcribed the complete paragraph:

> Edit this video for social media. I recorded several takes. Only use the final take for each line. Edit them together tightly so there are no pauses or breaths at the beginning of each clip. After you make those edits, review the cuts to make sure they are tight and don't cut off the end of any words. Add captions using a simple sans-serif font and add a big bold title over the first clip.

## Skill / workflow to reuse

1. Install HyperFrames (HeyGen plugin, or `npx skills add heygen-com/hyperframes` — see `PUBLIC-EXTRAS.md`). Needs Node 22+ and FFmpeg per public docs.
2. Record one file. For each spoken line, do several takes and leave the last good take as the keeper.
3. Upload the file to Claude or ChatGPT and invoke HyperFrames (`@Hyperframes`).
4. Paste the prompt above (or a branded variant).
5. Iterate visually in chat ("make the title smaller", etc.).
6. Ask the model to save the finished process as a skill so the next short inherits the style.

Claude does not natively cut or grade MP4s. HyperFrames (HTML + render) or FFmpeg is the renderer.

## Screenshot index

- `01-hyperframes-install.jpg` (~0:14) plugin search
- `02-upload-at-hyperframes.jpg` (~0:37) `IMG_1228.MOV` + `@Hyperframes`
- `03-editing-prompt.jpg` (~0:58) prompt card (last sentence slightly cropped in this still)
