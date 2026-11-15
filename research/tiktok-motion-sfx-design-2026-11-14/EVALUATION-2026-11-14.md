# Hands-on evaluation: does the 2026-11-14 research improve our Shorts?

The research pack in this folder ranked tools from four TikToks by reading their
pages. This file records what happened when we installed and ran them on
2026-11-14 (Mac mini M4 Pro, 64 GB, macOS, Node 24.19, FFmpeg 8.0.1, Docker 29.2),
and what was adopted into `skills/claw-shorts/`.

The test piece: a 27 s vertical "3 free tools" Short (HyperFrames, GSAP skills,
uisfx) with local synthetic narration, word-timed captions and eight SFX, built
as `skills/claw-shorts/hyperframes/`. It was rendered and measured, not posted.

## Verdicts after use

| Tool | Research verdict | After running it | Adopted |
| --- | --- | --- | --- |
| HyperFrames 0.9.50 | Useful | **Adopt.** Local install, local transcription, a browser gate that found real layout faults our tools missed, deterministic renders | New claw-shorts mode (`hyperframes/`) |
| GSAP skills | Useful | **Keep as reference.** Generic web GSAP (timelines, ScrollTrigger, React). The HyperFrames skill already carries the video-specific GSAP contract (paused timeline, `fromTo` on seek, no `repeat: -1`) | Not installed; the example uses its timeline practices |
| uisfx 0.4.0 | Maybe | **Adopt for interface-like moments.** 78 cues × 12 styles, CC0 1.0 sounds, MIT code; npm package byte-identical to the repo | Pinned in `hyperframes/package.json`; `scripts/sfx.sh` |
| Real SFX (Clipwing) | Useful | **Later, with a recorded CC0 library.** Its setup check passed with Homebrew's standard FFmpeg 8.0.1, which lacks the `drawtext` and `rubberband` filters its README asks for; we did not reach the steps that use them. Its HyperFrames importer read our project correctly (4 cuts at the exact planned times) but also counts every caption entrance; mixing needs an indexed sound library | No; still no LICENSE file in the repo |
| Design Motion Principles | Useful | **Reference.** Web-UI motion audit; its "AI-slop" list (pulsing loops, blur on everything, stagger everywhere) is folded into `frame.md` Don'ts | Rules only |
| awesome-claude-design / DESIGN.md | Useful | **Adopt the format.** Correction below: the files are in a second repo | `hyperframes/frame.md` |
| huashu-motion 0.4.0 | Maybe | **Maybe, unchanged.** Now has an English README. Its network calls (grep of `cli/`, `runtime/`) are documented downloads: instrument samples (CC0 / CC-BY-3.0), the Kokoro voice, Whisper models. Not run | No |
| Remotion | Maybe | Not run (company license above 3 employees) | No |
| Taste Skill, UI UX Pro Max, Design.md Chrome, motion.dev, anime.js, shadcn charts | Maybe | Web front-end tools; nothing to adopt for video yet | No |

## HyperFrames: local install or Docker?

**Local install, project-pinned** (`npm install hyperframes@0.9.50` in the video
project, never global). It took 10 s and 108 MB and used the installed Chrome and
Homebrew FFmpeg. `--docker` builds a local image (`hyperframes-renderer:0.9.50-arm64`,
3.4 GB) from a Dockerfile inside the CLI package; no prebuilt image is pulled.

| | Local | `--docker` |
| --- | --- | --- |
| First render of the 27 s example | 50 s (cold) | 170 s (includes the image build) |
| Warm render | 13.5 s | 20.6 s |
| Same bytes on a re-run | yes | yes |
| Local vs Docker | SSIM 0.993, PSNR 34.5 dB: the same picture, different bytes | |

Docker earns its place on CI or the Linux machine, where pinning Chromium, fonts
and FFmpeg matters more than speed. On Apple Silicon the image is linux/arm64,
which HyperFrames documents as not byte-identical to amd64 renders anyway.

## Did it improve the content? What changed

1. **Safe area.** `hyperframes check` uses TikTok, Reels and Shorts vertical-ad
   guides. Scaled to 1080×1920, the area clear on all three is y 288-1248,
   x 120-888 above y 840 and x 120-780 below it. Our rule (90 px sides, text
   above 1330 px) put the hook and captions under TikTok's action buttons. The
   claw-shorts skill now states the tighter rule. `Rust/claw-shorts` and
   `Rust/shortform-video` still lay out with the old margins: follow-up.
2. **Word timings.** Parakeet (sha256-pinned, about 640 MB in
   `~/.cache/hyperframes`) transcribed 24.7 s of narration in 2.5 s. It spelled
   names wrong ("Waterframes", "78 Qs", "G sap"), which is why
   `scripts/captions.py` keeps the script's words and takes only the timing. That
   fixes the "captions show Whisper's spelling" limit noted in
   `Rust/claw-shorts/README.md`.
3. **A review loop with numbers.** `check` also flagged a line held 0.37 s
   (under its readable floor), a word covered by a card, and missing clip
   durations. `snapshot --review` wrote a beat sheet and a five-frame strip
   around every cut, which is what we looked at before rendering.
4. **SFX on the frame.** `hyperframes audio` reports each cue's peak, so a cue
   can start one frame after its picture. In the render, all eight cues
   were within 4 ms of plan (narration-subtracted onset analysis), except one
   whose loudest point is 2 frames after its reported peak.
5. **Loudness still needs us.** HyperFrames only matches clips to each other.
   `scripts/finish.sh` takes the render from -20.0 to -14.2 LUFS at -1.37 dBTP;
   a -1 dBTP ceiling alone gave -0.92 after AAC encoding, so it aims at -1.5.

## Lessons for the next script

- **Spell for the voice, caption for the eye.** "G sock" (my first spelling of
  GSAP) and spelled-out "U I S F X" both read badly; the second also stretched
  one line to 6 s. Keep `narration.txt` and `script.txt` separate.
- **Read the transcript for missing words.** The voice ran "Three: free" into
  one word. The caption tool spreads such a word over its neighbour, but a
  dropped word can also mean the voice skipped it: listen to that spot.
- **TTS leaves about 0.15 s between most words**, so caption grouping by pause
  needs a higher threshold (0.3 s) than speech by a person.

## Cautions found while running it

- HyperFrames telemetry is **on by default** (`HYPERFRAMES_NO_TELEMETRY=1`), and
  its `/hyperframes` skill tells the agent to run a usage beacon
  (`npx hyperframes events`).
- `transcribe` without `--no-runtime-install` may `brew install whisper-cpp` or
  build whisper.cpp from source.
- `hyperframes add` copied a snippet to the system clipboard.
- Fonts beyond the bundled weights come from Google Fonts and GSAP from jsDelivr
  at render time unless vendored.
- uisfx and Real SFX disagree on what counts as "real": uisfx sounds are
  procedurally generated (its `LICENSE-AUDIO` says so). HyperFrames' sound guide
  rules out synthesized dings; use the UI cues for UI-like moments only.
- Real SFX suggested 18 cues for the 27 s piece (target 5 per 10 s); the
  HyperFrames guide argues for few, exact cues. We shipped 8.

## Corrections to this research pack

- **Awesome Claude Design:** the README links to getdesign.md, but the DESIGN.md
  files themselves are in
  [VoltAgent/awesome-design-md](https://github.com/VoltAgent/awesome-design-md)
  (MIT, 84 files at commit `fcc7028`, 2026-10-28), which is also the source of the
  `getdesign` npm CLI (`npx getdesign@latest add <name>`). The getdesign.md
  pages show each file free, with no sign-in; the paid items are starter kits
  and video templates.
- **huashu-motion** now ships `README.en.md`.
- **Real SFX**: `setup.py check` passes with Homebrew's standard `ffmpeg`
  8.0.1 even though that build lacks `drawtext` and `rubberband`; expect the
  README's `ffmpeg-full` requirement to bite at the steps that draw labels or
  pitch-shift (not reached here).

## Not done

- Nothing was posted or uploaded. HyperFrames skills were not installed
  globally (that needs authorization); the example uses the project-local CLI.
- The audio was measured, not listened to.
- Real SFX's mix step, huashu-motion and Remotion were not run.
