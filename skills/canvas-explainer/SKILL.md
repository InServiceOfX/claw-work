---
name: canvas-explainer
description: Create faceless explainers, launch clips, and motion graphics using deterministic JavaScript canvas frames and a local browser/FFmpeg render loop. Use for coded animation with storyboard previews, reference-led styling, and optional narration. Use a transcript edit workflow for cutting recordings.
---

# Canvas explainer

Use the Chase AI video's central idea: write the image as a function of time,
inspect sampled frames, then render it. There is no need to train a video model.
JavaScript is the drawing layer; Rust can orchestrate media work and FFmpeg
encodes it. Respect an existing renderer instead of rebuilding it for each video.

## Workflow

1. Establish one audience takeaway, duration, aspect ratio, style references and
   audio choice from the request. Use an explicit **silent** draft when no audio
   is supplied; do not silently subscribe to TTS or synthesize a speaker's voice.
   For narration-led work, measure the audio first and align scene changes to real
   phrase/word timestamps. Keep inputs local unless upload was authorized.
2. Write timed beats, then render representative keyframes before a full pass.
   Show an opening, the main transformation, payoff and each major text state.
   Use public or authorized references as composition inspiration, with original
   content; do not fetch paid templates or promise an exact creator recreation.
3. Implement `window.renderFrame(t)` so any timestamp can be drawn independently.
   Clear the canvas each call. Use seeded randomness or fixed geometry; derive
   motion from `t`, not wall-clock time, requestAnimationFrame accumulation,
   network state or unseeded Math.random. Await fonts/assets before capture.
4. Read [references/rendering.md](references/rendering.md) for the bundled local
   starter and renderer. These are a small runnable baseline, not the `animate`
   creator's plugin. Use existing HyperFrames/Playwright tooling when available.
5. Inspect a contact sheet and frames around scene joins for clipping, contrast,
   safe zones, label accuracy and text hold time. Sparse frames cannot establish
   smooth motion, so check a short motion preview too. Fix observed defects;
   normally stop after two correction passes and report unresolved issues rather
   than looping indefinitely. A user-requested revision can start another pass.
6. Encode, check dimensions/fps/duration and fully decode the MP4. For audio,
   preserve the approved soundtrack and check synchronization separately. Deliver
   the video, scene source and actual evidence; distinguish silent draft, visual
   inspection and audio audition. Publishing is a separate action.

## Good defaults

Choose readable type, a limited palette and one visual idea per beat. Match
composition to the user's reference and brand; avoid preset effects for their
own sake. 30 fps and a short low-resolution preview are economical starting
points; 60 fps and 1080p should serve an actual output requirement. Set a render
budget before a long job. Code-based rendering avoids per-clip video-generator
fees, but model usage, TTS, compute and iteration still cost time or money.

## Evidence

Source: https://www.tiktok.com/@chase_ai_/video/7693602525081111821,
archived as auto-captions plus selected frames in claw-work's 2026-11-05 research.
The archive names `cb9191/animate`; its upstream could not be retrieved during
this implementation, so its current interface is unverified. Do not install it
or claim it tested merely because the video shows commands.
