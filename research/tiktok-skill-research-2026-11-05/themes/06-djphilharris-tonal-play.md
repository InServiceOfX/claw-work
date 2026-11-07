# Phil Harris (@djphilharris) — tonal play, for Mixxx / claw-dj

- Video: https://www.tiktok.com/@djphilharris/video/7694719261549317379
- Auto-captions: [`../transcripts/djphilharris-7694719261549317379.txt`](../transcripts/djphilharris-7694719261549317379.txt)
- Screenshots: moved to [claw-dj's Phil Harris archive](https://github.com/InServiceOfX/claw-dj/tree/master/docs/research/screenshots/djphilharris); no local screenshot copies remain here.
- Theme: DJ tonal play (not an AI video-edit skill)
- CTA: "Hope this helps, follow for daily DJ tricks and tips." No comment-keyword gate.

This is the Mixxx-facing note. Product implementation, if any, belongs in InServiceOfX/claw-dj — not as runtime code in this public notes PR.

## What the video actually shows

It does **not** show Mixxx, Serato, or Rekordbox performing a live mix of two songs. Phil explains how viral "play a melody from one vocal sample" transitions (tonal play / tone play) are prepared.

He says there are only two practical ways to pitch notes up and down for this on CDJs, and neither is really "inside" the CDJ:

1. **Pre-production + hot cues.** Load the isolated sample into a DAW (on-screen: **Ableton Live**). Duplicate it, space the copies, pitch each copy to the next scale tone, export one audio file, load that file on a Pioneer CDJ (CDJ-3000-class deck shown next to a DJM-V10), and set a hot cue at the start of each pitched note. Hitting the hot cues plays the melody.
2. **Hardware sampler Scale Mode.** Load the unpitched sample into a standalone sampler (Pioneer **DJS-1000**). Enable **Scale Mode**, which maps the sample across the 16 performance pads at scale pitches. Play the pads like a keyboard.

No commercial track titles, EQ, or filter recipes appear. Demo audio is a pitched vocal syllable ("hoo"). Background neon: "EDM COACHING .COM".

## Mixxx analog (derived, not shown on screen)

Mixxx has no Pioneer-style Scale Mode on its samplers. Two workarounds:

### A. Pre-produced hot-cue track (matches method 1)

1. In Audacity (or any editor/DAW), pitch-shift copies of the sample into a melody and export one WAV/MP3.
2. Load it on a Mixxx deck.
3. Place hot cues at the start of each note.
4. Map MIDI pads to those hot cues.

### B. Multi-sampler (live, no DAW export)

1. Load the same short sample into several Mixxx sampler decks.
2. Set each sampler's key/pitch independently (for example +0, +2, +4 semitones). Disable keylock if you want playback speed to follow pitch; enable it if you want high-quality key shift at constant length.
3. Map pads to those samplers and finger-drum the melody.

claw-dj could later treat this as a user story: list a vocal one-shot, generate pitched sampler copies or a hot-cue map, and expose pads to the MIDI engine. That work should land in the claw-dj repo, not here.

## Screenshot index

- `01-hook.jpg` (~0:10) viral-transition hook
- `02-ableton-pitch-sequence.jpg` (~0:33) Ableton arrangement
- `03-cdj-hotcues.jpg` (~0:40) CDJ waveform / hot cues
- `04-djs1000-scale-mode.jpg` (~0:55) DJS-1000 SCALE control
