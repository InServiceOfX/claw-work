# Canvas baseline

`assets/scene.html` is an original silent, seven-second portrait animation about
aligning two musical phrases. `scripts/render.mjs` captures its canvas with
Playwright and runs FFmpeg. It blocks browser network requests and uses only
inline/local scene code. It is not a sandbox for hostile JavaScript.

The script needs Node, FFmpeg, and `playwright` resolvable from the script. Inspect
existing installations first. If missing and installing project dependencies is
authorized, copy this skill into a disposable project and install a reviewed
Playwright version there (the bundled package.json pins the version used for
local verification); install its Chromium browser only if needed. An existing
Google Chrome can be selected with `CANVAS_BROWSER_CHANNEL=chrome`. It uses a
fresh headless profile, not the user's browser session. Do not
replace a user's global browser or skill configuration.

From the skill directory, with dependencies available:

```bash
node scripts/render.mjs assets/scene.html /tmp/canvas-first-draft 7 30
ffprobe -v error -show_streams -show_format /tmp/canvas-first-draft/video.mp4
ffmpeg -v error -i /tmp/canvas-first-draft/video.mp4 -f null -
ffmpeg -n -i /tmp/canvas-first-draft/video.mp4 -vf 'fps=1,scale=180:-1,tile=7x1' -frames:v 1 /tmp/canvas-first-draft/contact.jpg
```

Arguments are HTML, **new** output directory, duration in seconds and integer fps.
Defaults: seven seconds, 30 fps. There must be exactly one `canvas#scene` with
fixed even dimensions and an async or sync `window.renderFrame(t)` function.
Capture happens at `t = frame_index / fps`; frame count is `ceil(duration * fps)`.
Keep captions/graphics inside phone-safe margins. The script outputs frames plus
silent `video.mp4`; it does not generate a script, narration or word timing.

For a storyboard, call renderFrame at the planned timestamps and capture those
canvas states before the full run. For an approved soundtrack, measure it first
and render enough frames to cover it. A separate explicit FFmpeg mux can add AAC
audio; don't hide length mismatch with `-shortest` unless the intended truncation
is understood. Keep the silent render as an editable master.

The small starter has no external fonts/media. More complex scenes need an
explicit local asset loading scheme; don't disable the request block just to
allow arbitrary remote fonts. Browser capture API reference:
https://playwright.dev/docs/screenshots

Alternative production engine: https://github.com/heygen-com/hyperframes. Verify
the installed version and current official documentation instead of treating
research screenshots as a maintained CLI manual.
