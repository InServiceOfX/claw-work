// Original minimal local capture utility; no upstream plugin code is vendored.
import { chromium } from 'playwright';
import { readFile, mkdir } from 'node:fs/promises';
import { resolve, join } from 'node:path';
import { spawnSync } from 'node:child_process';

const [html, destination, seconds = '7', fpsText = '30'] = process.argv.slice(2);
const duration = Number(seconds), fps = Number(fpsText);
if (!html || !destination || !Number.isFinite(duration) || duration <= 0 || duration > 60 || !Number.isInteger(fps) || fps < 1 || fps > 60) {
  throw new Error('Usage: node render.mjs scene.html NEW_OUTPUT_DIR [seconds: 0..60] [fps: 1..60]');
}
const source = await readFile(resolve(html), 'utf8');
if (Buffer.byteLength(source) > 2_000_000) throw new Error('Scene exceeds 2 MB');
const encoder = spawnSync('ffmpeg', ['-version'], { stdio: 'ignore' });
if (encoder.error || encoder.status !== 0) throw new Error('FFmpeg is required');
const browser = await chromium.launch({ headless: true, ...(process.env.CANVAS_BROWSER_CHANNEL ? { channel: process.env.CANVAS_BROWSER_CHANNEL } : {}) });
try {
  const page = await browser.newPage({ deviceScaleFactor: 1, serviceWorkers: 'block' });
  await page.route('**/*', route => route.abort());
  const csp = `<meta http-equiv="Content-Security-Policy" content="default-src 'none'; script-src 'unsafe-inline'; style-src 'unsafe-inline'; img-src data:; font-src data:; connect-src 'none'">`;
  await page.setContent(csp + source, { waitUntil: 'load' });
  await page.evaluate(async () => { await document.fonts.ready; });
  const size = await page.evaluate(() => {
    const c = document.querySelector('canvas#scene');
    if (!c || typeof window.renderFrame !== 'function') throw new Error('Need canvas#scene and window.renderFrame(t)');
    return { width: c.width, height: c.height };
  });
  if ([size.width,size.height].some(n => !Number.isInteger(n) || n < 2 || n > 1920 || n % 2)) throw new Error('Canvas dimensions must be even, 2..1920');
  await page.setViewportSize(size);
  const output = resolve(destination);
  await mkdir(output); // Existing files, directories and symlinks fail closed.
  const frames = Math.ceil(duration * fps);
  for (let i = 0; i < frames; i++) {
    await page.evaluate(async t => { await window.renderFrame(t); }, i / fps);
    await page.locator('canvas#scene').screenshot({ path: join(output, `frame-${String(i).padStart(6,'0')}.png`), animations: 'disabled' });
  }
  const result = spawnSync('ffmpeg', [
    '-hide_banner','-loglevel','error','-nostdin','-n','-framerate',String(fps),
    '-i',join(output,'frame-%06d.png'),'-frames:v',String(frames),'-an',
    '-c:v','libx264','-crf','20','-pix_fmt','yuv420p','-movflags','+faststart',
    join(output,'video.mp4')
  ], { stdio: 'inherit' });
  if (result.error || result.status !== 0) throw new Error('FFmpeg failed; partial directory retained');
  console.log(join(output,'video.mp4'));
} finally {
  await browser.close();
}
