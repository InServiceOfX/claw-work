# Short-form video tooling

Research notes from @itstundealao demos of open-source video and audio tools. All figures are from single on-screen demos, not independent benchmarks.

## Handy — cjpais/Handy

- Video: [7702529514181627169](https://www.tiktok.com/@itstundealao/video/7702529514181627169)
- Repo: https://github.com/cjpais/Handy
- License: **MIT**
- Description: Free, open source, extensible speech-to-text application that works completely offline.
- Provenance: Repo name "Handy" displayed with clone dialog visible. Owner text small/low-legibility but repo confirmed via GitHub API.

### Demo observations (creator's single run)

- Dictation described as "fast and accurate"
- Runs locally (offline)

### README notes (not independently tested)

Repository describes offline speech-to-text. No paid API required per documentation.

---

## supoclip — FujiwaraChoki/supoclip

- Video: [7698767504046394646](https://www.tiktok.com/@itstundealao/video/7698767504046394646)
- Repo: https://github.com/FujiwaraChoki/supoclip
- License: **AGPL-3.0** (link only, do not vendor)
- Description: Open-source OpusClip alternative for automated short-form clip extraction from long videos.
- Provenance: Clone URL `github.com/FujiwaraChoki/supoclip` legible on-screen. Video-model reported `tundealao/supoclip` (conflict), but frame evidence definitively favors FujiwaraChoki.

### Demo observations (creator's single run)

- ~20 minutes to produce 10 clips
- Used paid transcription (AssemblyAI) and LLM keys in the demo

### README notes (not independently tested)

The upstream README documents a local mode using:
- Local Whisper (no paid transcription)
- Ollama for LLM inference

This local path was not demonstrated in the video; documented here from README inspection only.

### Preliminary static review

Examined `requirements.txt` and README. Declares dependencies on:
- `openai`, `assemblyai` (optional paid APIs per README)
- `whisper` (local alternative per README)
- `moviepy`, `yt-dlp`, standard Python ML stack

No code was executed; dependency versions not audited.

---

## OpenMontage — calesthio/OpenMontage

- Video: [7693581657772756246](https://www.tiktok.com/@itstundealao/video/7693581657772756246)
- Repo: https://github.com/calesthio/OpenMontage
- License: **AGPL-3.0** (link only, do not vendor)
- Description: "World's first open-source, agentic video production system" — 12 pipelines, 100+ tools, 700+ skill/knowledge files.
- Provenance: Repo header `calesthio / OpenMontage` and install URL both legible. Video-model reported `tundealao/OpenMontage` (conflict), but frame evidence definitively favors calesthio.

### Demo observations (creator's single run)

- ~$12 agent session cost displayed
- $9.41 fal.ai image generation
- Output: synthetic persona trailer

### README notes (not independently tested)

The upstream README documents:
- Free/stock asset paths
- Multiple optional paid providers (fal.ai, Replicate, ElevenLabs, etc.)
- Local alternatives for some components

The demo showed a paid path. Free/local modes exist per docs but were not demonstrated.

### Preliminary static review

Examined README. Monorepo structure with many optional integrations. AGPL-3.0 with strong copyleft.

---

## VoiceStudio — debpalash/VoiceStudio (SOURCE-UNCONFIRMED)

- Video: [7699878190784777505](https://www.tiktok.com/@itstundealao/video/7699878190784777505)
- Candidate repo: https://github.com/debpalash/VoiceStudio
- License: AGPL-3.0 (if repo confirmed)
- Description: Open-source ElevenLabs alternative — voice cloning, dubbing, TTS in 646 languages.
- Provenance: **CONFLICT**. Low-res on-screen prompt URL reads as `debpalash/VoiceStudio`. Video-model reported `tundealao/VoiceStudio`. **Retains SOURCE-UNCONFIRMED** — do not treat as creator endorsement without primary verification.

### Demo observations (creator's single run)

- Cloned the creator's own voice
- Local app running on Apple Silicon
- Model downloads included Kokoro/Whisper

### README notes (not independently tested)

Repository exists at `debpalash/VoiceStudio` (verified live). Describes fully local operation.

---

## claude-watch / claude-video (SOURCE-UNCONFIRMED)

- Video: [7679101376772721942](https://www.tiktok.com/@itstundealao/video/7679101376772721942)
- Candidate repos:
  - https://github.com/taoufik123-collab/claude-watch (MIT, 1.1K stars)
  - https://github.com/bradautomates/claude-video (MIT, 20K stars)
- Provenance: **UNKNOWN**. On-screen file list shows commit message "release: v0.2.0 — fork as taoufik/claude-watch" but owner not legible. Video-model reported `Taufik123/claude-watch` (different casing). Fork genealogy unresolved.

### Demo observations

- Tool stalled until a Whisper API key was added
- Analyzes video frames + transcript (not native video model)

### README notes (from bradautomates/claude-video, higher-activity candidate)

Describes:
- yt-dlp/caption paths (free)
- Frame extraction for Claude vision analysis
- Not all modes require paid Whisper API

Neither candidate repo confirmed as the one displayed; both remain possibilities.

---

## Comparison to baseline: yt-dlp + ffmpeg + local Whisper

A minimal local ingest pipeline using only OSS tools:

```sh
yt-dlp -o "%(id)s.%(ext)s" "$URL"
ffmpeg -i video.mp4 -ar 16000 -ac 1 audio.wav
whisper audio.wav --model small --output_format txt
```

This baseline:
- No paid APIs
- No AGPL concerns
- Manual clip selection required
- No automated virality scoring or assembly

The tools above add automation, scoring, and assembly. Trade-off: complexity, licensing, and optional paid dependencies.

---

## License notes

| Repo | License | Implication |
|------|---------|-------------|
| cjpais/Handy | MIT | Permissive |
| FujiwaraChoki/supoclip | AGPL-3.0 | Strong copyleft; network use triggers distribution |
| calesthio/OpenMontage | AGPL-3.0 | Same |
| debpalash/VoiceStudio | AGPL-3.0 | Same |
| taoufik123-collab/claude-watch | MIT | Permissive |
| bradautomates/claude-video | MIT | Permissive |

**AGPL repos linked, not vendored.** Do not copy code into non-AGPL projects without review.

---

## Excluded from theme documentation

| Repo | Reason |
|------|--------|
| kaankiziltug/logo-design-skill | Low priority — branding skill, not video tooling. Source displayed (NAME DISPLAYED; owner INFERRED). Listed in [SOURCES.md](../SOURCES.md) but no theme doc written. |
