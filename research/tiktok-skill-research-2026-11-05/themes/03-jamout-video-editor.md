# Jam | AI & Marketing (@jamout.ai) — audio-prompt "video editor" skill

- Video: https://www.tiktok.com/@jamout.ai/video/7695100487216975118
- Auto-captions: [`../transcripts/jamout.ai-7695100487216975118.txt`](../transcripts/jamout.ai-7695100487216975118.txt)
- Screenshots: [`../screenshots/jamout/`](../screenshots/jamout/)
- Theme: skill packaging / audio prompt injection + HyperFrames
- CTA: "Want more help? Comment JAM" plus "check out the link in my bio." Free bio shelf (not gated behind the comment): https://jamout.ai/resources and https://jamout.ai/claude-skills ; free Club Claude https://www.skool.com/clubclaude/about . Paid Club Jam is separate (`PUBLIC-EXTRAS.md`).
- Spoken URL: she says HyperFrames is on GitHub "at heygen.com/hyperframes" (HeyGen docs + `npx skills add` pattern).

## Walkthrough

0:00–0:27 speaks to the human: Claude and ChatGPT can edit videos; open the app, hold the mic up to the phone so the AI hears the rest. 0:27–3:41 addresses the AI: become a video editor and save the following as a permanent skill. 3:41–4:02 tells the human to stop the mic, press send, and follow for more tips.

## Dictated skill (as taught)

- Name: `video editor`
- Save under `.claude/skills`
- Dependencies: check for FFmpeg, Whisper, and Node.js; explain installs; ask before any password.
- Framework: install HyperFrames from HeyGen (`npx skills add`). If skills land in `.agents/skills`, symlink that folder to `.claude/skills`.

Nine rules:

1. Before editing, ask three questions: Where is it going? What should viewers do (CTA)? Captions/graphics or a clean cut?
2. Never alter the original. Copy to a local work folder (not iCloud or Dropbox). Save the render next to the original under a new name.
3. Transcribe with Whisper. Keep the best take of each line. Cut pauses longer than half a second.
4. If one recording contains several distinct videos, split them into separate files.
5. Match saved brand colors, fonts, and voice. Pull from a "B-roll kit". Ask once for branding if missing, then remember it.
6. Export 9:16 for Reels/TikTok, wide for YouTube, highest quality.
7. Level AI voice and the human voice on separate tracks.
8. Try to fix dark or grainy footage; say honestly if a reshoot is needed.
9. Report the saved path and remind the user to watch it. The user is the director.

Then: render a 7-second test clip in the user's caption style, recap capabilities in three sentences, and suggest one first-edit idea.

## Skill / workflow to reuse (marketing shorts)

This is the most complete "save it as a skill" recipe of the talking-head group. Pair it with official HyperFrames docs rather than a paid community. For claw-dj shorts: 9:16, last-take cutting, caption style, never overwrite originals.

## Screenshot index

- `01-step1-setup.jpg` (~0:52) FFmpeg / Whisper / Node
- `02-step2-hyperframes.jpg` (~1:12)
- `03-nine-rules.jpg` (~1:45)
- `04-step4-test.jpg` (~3:28)
