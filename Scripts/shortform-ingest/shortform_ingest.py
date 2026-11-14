#!/usr/bin/env python3
"""Short-form video ingest helper (TikTok, YouTube Shorts, Reels).

Turns a short-form video URL into the raw material for a claw-work research
note: metadata, a timestamped transcript, and candidate frames to review.
Choosing the key frames and writing the summary stay a human/LLM step.

Subcommands (run in order; each is idempotent and skips finished work):

  download    URL...                  yt-dlp video + .info.json into WORK/raw/
  transcribe  [--engine faster|openai] whisper transcript with timestamps -> WORK/tx/
  candidates  [--early-seconds 3]     scene-change + dense early + uniform frames,
                                      plus contact sheets -> WORK/cand/<slug>/
  grab        SLUG T [T ...]          full-res 720w screenshots at exact times
                                      -> WORK/shots/<slug>/<TTT.T>s.jpg
  scaffold    SLUG --platform P --out DOCS_ROOT [--shot T:caption ...]
                                      write transcript.md, README.md skeleton, and
                                      copy chosen shots into
                                      DOCS_ROOT/<platform>/<slug>/

WORK defaults to ./shortform-work (override with --work). Raw video stays in
WORK and must never be committed.
"""
import argparse
import glob
import json
import os
import re
import shutil
import subprocess
import sys


def run(cmd, **kw):
    print("+", " ".join(cmd), file=sys.stderr)
    return subprocess.run(cmd, check=True, **kw)


def fmt(t):
    t = float(t)
    return f"{int(t // 60)}:{t % 60:04.1f}"


def shot_name(t):
    return f"{float(t):05.1f}s.jpg"


def slugs(work, only=None):
    vids = sorted(os.path.basename(f)[:-4] for f in glob.glob(f"{work}/raw/*.mp4"))
    return [v for v in vids if not only or v in only]


def info(work, slug):
    p = f"{work}/raw/{slug}.info.json"
    return json.load(open(p)) if os.path.exists(p) else {}


# ---------------------------------------------------------------- download
def creator_slug(inf):
    # TikTok "uploader" is the @handle (uploader_id is numeric); YouTube's is the
    # display name ("Wanderlust Alley" -> "wanderlustalley"). The video id keeps its case.
    name = inf.get("uploader") or inf.get("channel") or "unknown"
    return re.sub(r"[^a-z0-9._-]+", "", name.lower())


def cmd_download(a):
    raw = f"{a.work}/raw"
    tmp = f"{a.work}/raw/.incoming"
    os.makedirs(tmp, exist_ok=True)
    for url in a.urls:
        run(["yt-dlp", "--no-playlist", "--no-progress", "--write-info-json",
             "-f", "mp4/bestvideo*+bestaudio/best", "--merge-output-format", "mp4",
             "-o", f"{tmp}/%(id)s.%(ext)s", url])
        for ij in glob.glob(f"{tmp}/*.info.json"):
            vid = os.path.basename(ij)[:-len(".info.json")]
            slug = f"{creator_slug(json.load(open(ij)))}-{vid}"
            os.replace(ij, f"{raw}/{slug}.info.json")
            os.replace(f"{tmp}/{vid}.mp4", f"{raw}/{slug}.mp4")
            print(slug)


# ---------------------------------------------------------------- transcribe
def load_pcm(work, slug):
    import numpy as np
    os.makedirs(f"{work}/wav", exist_ok=True)
    pcm = f"{work}/wav/{slug}.pcm"
    if not os.path.exists(pcm):
        run(["ffmpeg", "-loglevel", "error", "-y", "-i", f"{work}/raw/{slug}.mp4",
             "-ac", "1", "-ar", "16000", "-f", "s16le", pcm])
    return np.fromfile(pcm, dtype=np.int16).astype(np.float32) / 32768.0


def cmd_transcribe(a):
    os.makedirs(f"{a.work}/tx", exist_ok=True)
    model = None
    for slug in slugs(a.work, a.only):
        out = f"{a.work}/tx/{slug}.json"
        if os.path.exists(out) and not a.force:
            continue
        audio = load_pcm(a.work, slug)
        if a.engine == "faster":
            from faster_whisper import WhisperModel
            model = model or WhisperModel(a.model, device="cpu", compute_type="int8")
            segs, inf = model.transcribe(audio, beam_size=5, vad_filter=a.vad,
                                         condition_on_previous_text=not a.vad)
            segs = [dict(start=round(s.start, 2), end=round(s.end, 2), text=s.text.strip())
                    for s in segs]
            lang, prob = inf.language, inf.language_probability
        else:
            import whisper
            model = model or whisper.load_model(a.model)
            r = model.transcribe(audio, fp16=False)
            segs = [dict(start=round(s["start"], 2), end=round(s["end"], 2),
                         text=s["text"].strip()) for s in r["segments"]]
            lang, prob = r.get("language"), None
        json.dump(dict(engine=a.engine, model=a.model, vad=a.vad, language=lang, prob=prob,
                       segments=segs), open(out, "w"), indent=1)
        print(slug, lang, len(segs), "segments", flush=True)


# ---------------------------------------------------------------- candidates
def scene_times(video, threshold):
    r = subprocess.run(["ffmpeg", "-hide_banner", "-i", video, "-vf",
                        f"select='gt(scene,{threshold})',showinfo", "-f", "null", "-"],
                       capture_output=True, text=True)
    return [float(x) for x in re.findall(r"pts_time:([0-9.]+)", r.stderr)]


def grab(video, t, out, width=720):
    run(["ffmpeg", "-loglevel", "error", "-y", "-ss", str(t), "-i", video, "-frames:v", "1",
         "-vf", f"scale={width}:-2", "-q:v", "3", out])


def cmd_candidates(a):
    for slug in slugs(a.work, a.only):
        video = f"{a.work}/raw/{slug}.mp4"
        d = f"{a.work}/cand/{slug}"
        os.makedirs(d, exist_ok=True)
        dur = float(info(a.work, slug).get("duration") or 60)
        scenes = scene_times(video, a.scene_threshold)
        # Dense early frames: lists/hooks often flash for ~1s in the first seconds,
        # which neither scene detection nor a 2s grid reliably catches.
        early = [round(i * a.early_step, 2) for i in range(int(a.early_seconds / a.early_step) + 1)]
        uniform = [round(i * a.step, 2) for i in range(int(dur / a.step) + 1)]
        # Last frames often hold the complete list/overlay.
        tail = [round(max(dur - x, 0), 2) for x in (1.5, 0.6)]
        times = sorted(t for t in set(early + uniform + [round(s + 0.3, 2) for s in scenes] + tail)
                       if t < dur - 0.2)
        for t in times:
            p = f"{d}/{t:06.2f}.jpg"
            if not os.path.exists(p):
                grab(video, t, p, width=a.width)
        # Contact sheets (5x4) for fast visual review. Each thumbnail is labelled
        # with its time; labels exist only on the review sheets, never on shots.
        frames = sorted(glob.glob(f"{d}/[0-9]*.jpg"))
        td = f"{d}/thumbs"
        os.makedirs(td, exist_ok=True)
        for f in frames:
            b = os.path.basename(f)
            run(["ffmpeg", "-loglevel", "error", "-y", "-i", f, "-vf",
                 f"scale=240:-2,drawtext=text='{b[:-4]}s':x=4:y=4:fontsize=20:"
                 "fontcolor=yellow:box=1:boxcolor=black@0.6", f"{td}/{b}"])
        for i in range(0, len(frames), 20):
            n = i // 20 + 1
            sd = f"{td}/sheet{n:02d}"
            os.makedirs(sd, exist_ok=True)
            for k, f in enumerate(frames[i:i + 20]):
                shutil.copy(f"{td}/{os.path.basename(f)}", f"{sd}/{k:02d}.jpg")
            run(["ffmpeg", "-loglevel", "error", "-y", "-i", f"{sd}/%02d.jpg",
                 "-vf", "tile=5x4", "-frames:v", "1", f"{d}/sheet_{n:02d}.jpg"])
            shutil.rmtree(sd)
        json.dump(dict(scenes=scenes, frames=[os.path.basename(f) for f in frames]),
                  open(f"{d}/candidates.json", "w"), indent=1)
        print(slug, f"{len(scenes)} scene changes, {len(frames)} candidate frames -> {d}")


def cmd_grab(a):
    d = f"{a.work}/shots/{a.slug}"
    os.makedirs(d, exist_ok=True)
    for t in a.times:
        grab(f"{a.work}/raw/{a.slug}.mp4", t, f"{d}/{shot_name(t)}", width=a.width)
    print(d)


# ---------------------------------------------------------------- scaffold
def cmd_scaffold(a):
    inf = info(a.work, a.slug)
    tx = json.load(open(f"{a.work}/tx/{a.slug}.json"))
    d = os.path.join(a.out, a.platform, a.slug)
    os.makedirs(f"{d}/screenshots", exist_ok=True)
    url = inf.get("webpage_url") or inf.get("original_url") or ""
    title = inf.get("title") or a.slug
    ud = inf.get("upload_date") or ""
    date = f"{ud[:4]}-{ud[4:6]}-{ud[6:]}" if len(ud) == 8 else "unknown"
    engine = f"{'faster-whisper' if tx.get('engine', 'faster') == 'faster' else 'openai-whisper'} `{tx.get('model', 'small')}`"

    T = [f"# Transcript: {title}", "", f"Source: <{url}>  ",
         f"Engine: {engine}. Machine transcribed; correct obvious mishearings in README.md.", ""]
    if tx["segments"]:
        T += [f"- `[{fmt(s['start'])} - {fmt(s['end'])}]` {s['text']}" for s in tx["segments"]]
    else:
        T += ["*No speech detected (music only); see on-screen text in README.md.*"]
    open(f"{d}/transcript.md", "w").write("\n".join(T) + "\n")

    shots = []
    for s in a.shot or []:
        t, _, cap = s.partition(":")
        src = f"{a.work}/shots/{a.slug}/{shot_name(t)}"
        shutil.copy(src, f"{d}/screenshots/{shot_name(t)}")  # unedited copy
        shots.append((float(t), cap or "TODO caption"))

    readme = f"{d}/README.md"
    if os.path.exists(readme) and not a.force:
        print(f"{readme} exists; not overwritten (use --force)")
        return
    tags = re.findall(r"#(\w+)", inf.get("description") or "")
    L = [f"# {title}", "", "| | |", "|---|---|", f"| Source | <{url}> |",
         f"| Creator | {inf.get('channel') or inf.get('uploader')} (@{inf.get('uploader')}) |",
         f"| Uploaded | {date} |", f"| Duration | {inf.get('duration')}s |",
         f"| Hashtags | {' '.join('#' + t for t in tags) or '(none)'} |",
         f"| Ingested | by yeung-assistant (yt-dlp + {engine}, language `{tx.get('language')}`) |",
         "", "**Caption (verbatim):**", "",
         "> " + (inf.get("description") or "").strip().replace("\n", "\n> "), "",
         "## Summary", "", "TODO", "", "## Key takeaways", "", "- TODO", "",
         "## Verbatim phrases and script", "", "TODO", "",
         "## On-screen text", "", "TODO: transcribe every list item and key figure shown on screen.", "",
         "## Screenshots", ""]
    for t, cap in shots:
        L += [f"**{fmt(t)}**: {cap}", "", f"![{fmt(t)}: {cap}](screenshots/{shot_name(t)})", ""]
    L += ["## How to apply", "", "TODO", "", "## Transcript", "",
          "Full timestamped transcript: [transcript.md](transcript.md).", ""]
    open(readme, "w").write("\n".join(L))
    print(d)


def main():
    p = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    p.add_argument("--work", default="shortform-work", help="scratch dir (raw video lives here; never commit)")
    sp = p.add_subparsers(dest="cmd", required=True)

    s = sp.add_parser("download"); s.add_argument("urls", nargs="+"); s.set_defaults(f=cmd_download)

    s = sp.add_parser("transcribe")
    s.add_argument("--engine", choices=["faster", "openai"], default="faster")
    s.add_argument("--model", default="small")
    s.add_argument("--vad", action="store_true", help="VAD filter; helps over loud music")
    s.add_argument("--only", nargs="*"); s.add_argument("--force", action="store_true")
    s.set_defaults(f=cmd_transcribe)

    s = sp.add_parser("candidates")
    s.add_argument("--scene-threshold", type=float, default=0.3)
    s.add_argument("--early-seconds", type=float, default=3.0)
    s.add_argument("--early-step", type=float, default=0.2)
    s.add_argument("--step", type=float, default=2.0)
    s.add_argument("--width", type=int, default=480)
    s.add_argument("--only", nargs="*")
    s.set_defaults(f=cmd_candidates)

    s = sp.add_parser("grab"); s.add_argument("slug"); s.add_argument("times", nargs="+", type=float)
    s.add_argument("--width", type=int, default=720); s.set_defaults(f=cmd_grab)

    s = sp.add_parser("scaffold"); s.add_argument("slug")
    s.add_argument("--platform", required=True, choices=["tiktok", "youtube", "instagram"])
    s.add_argument("--out", required=True, help="docs root, e.g. docs/shortform")
    s.add_argument("--shot", action="append", help="TIME:caption, repeatable (grab first)")
    s.add_argument("--force", action="store_true"); s.set_defaults(f=cmd_scaffold)

    a = p.parse_args()
    a.f(a)


if __name__ == "__main__":
    main()
