# Canvas animation workflow

Procedural motion graphics using JavaScript, Playwright, and FFmpeg.

## Architecture

1. **Frame function**: Write a `frame(t)` function that draws a single frame to
   an HTML canvas given a timestamp `t` (in seconds).
2. **Frame capture**: Use Playwright (headless Chromium) to render each frame
   at 30fps (or higher).
3. **Mux to MP4**: Use FFmpeg to combine frames and audio into the final video.
4. **QA contact sheet**: Generate a contact sheet (grid of keyframes) for visual
   review before committing to a full render.

## JavaScript template

```javascript
const canvas = document.getElementById('canvas');
const ctx = canvas.getContext('2d');

function frame(t) {
    ctx.clearRect(0, 0, canvas.width, canvas.height);
    
    ctx.fillStyle = '#1a1a2e';
    ctx.fillRect(0, 0, canvas.width, canvas.height);
    
    ctx.fillStyle = '#eee';
    ctx.font = 'bold 72px sans-serif';
    ctx.textAlign = 'center';
    ctx.textBaseline = 'middle';
    
    const y = 960 + Math.sin(t * 2) * 50;
    ctx.fillText('Hello World', 540, y);
}
```

## HTML host

```html
<!DOCTYPE html>
<html>
<head>
    <style>
        body { margin: 0; background: #000; }
        #canvas { display: block; }
    </style>
</head>
<body>
    <canvas id="canvas" width="1080" height="1920"></canvas>
    <script src="animation.js"></script>
</body>
</html>
```

## Playwright capture script

```javascript
const { chromium } = require('playwright');
const fs = require('fs');

async function captureFrames(htmlPath, outputDir, duration, fps) {
    const browser = await chromium.launch();
    const page = await browser.newPage();
    await page.setViewportSize({ width: 1080, height: 1920 });
    await page.goto(`file://${htmlPath}`);
    
    const totalFrames = duration * fps;
    
    for (let i = 0; i < totalFrames; i++) {
        const t = i / fps;
        await page.evaluate((time) => frame(time), t);
        await page.screenshot({
            path: `${outputDir}/frame_${String(i).padStart(6, '0')}.png`,
            clip: { x: 0, y: 0, width: 1080, height: 1920 }
        });
    }
    
    await browser.close();
}

captureFrames(process.argv[2], process.argv[3], parseFloat(process.argv[4]), 30);
```

## FFmpeg mux command

```sh
ffmpeg -framerate 30 -i frame_%06d.png \
  -i audio.mp3 \
  -c:v libx264 -preset medium -crf 18 \
  -c:a aac -b:a 192k \
  -pix_fmt yuv420p \
  -r 30 \
  output.mp4
```

## Contact sheet for QA

Before full render, generate a contact sheet of keyframes:

```sh
ffmpeg -i output.mp4 -vf "select='not(mod(n\,30))',tile=4x4" -frames:v 1 contact_sheet.png
```

Review the contact sheet for:
- Text cut-off or positioning issues
- Color/contrast problems
- Layout bugs
- Timing of key visual changes

If issues found, fix the `frame(t)` function and regenerate.

## Syncing to audio

For voice-overs, use Whisper to get word timestamps:

```sh
whisper audio.mp3 --output_format json
```

Then land visual changes on specific words by checking `t` against the timestamp
array in your `frame(t)` function.

## Safe zone considerations

Keep important text within:
- Top: y > 150px
- Bottom: y < 1620px (1920 - 300)
- Left: x > 50px
- Right: x < 1030px (1080 - 50)
