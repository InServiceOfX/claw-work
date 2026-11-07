# @itstundealao TikTok catalog research — 2026-11-06

Public account research: [Tunde Alao (@itstundealao)](https://www.tiktok.com/@itstundealao) on TikTok. Notes are paraphrased summaries of publicly visible posts; short quotes are attributed.

## Coverage

| Item | Count |
|------|-------|
| Videos on profile (2026-11-06 ~21:15 PT) | 274 |
| IDs enumerated via yt-dlp | 262 |
| Gap (cause UNKNOWN) | 12 |
| Transcribed (faster-whisper `small`, CPU) | 90 |
| Frame-inspected for on-screen sources | 22 |
| Captions only (not transcribed) | 172 older posts |
| Video-model reviews completed | 4 |

**Disclaimer:** 172 older posts were caption-screened only, not proven irrelevant. The 12-video gap and remaining videos have not been semantically screened. No claim of exhaustive coverage.

## Source-verified repositories

Repositories where the upstream owner/path was **legible on-screen** (clone URL, repo header, or breadcrumb). All links independently verified live 2026-11-07.

| Video | Repo | License | Provenance |
|-------|------|---------|------------|
| [7698767504046394646](https://www.tiktok.com/@itstundealao/video/7698767504046394646) | [FujiwaraChoki/supoclip](https://github.com/FujiwaraChoki/supoclip) | AGPL-3.0 | Clone URL legible |
| [7693581657772756246](https://www.tiktok.com/@itstundealao/video/7693581657772756246) | [calesthio/OpenMontage](https://github.com/calesthio/OpenMontage) | AGPL-3.0 | Header + install URL legible |
| [7702529514181627169](https://www.tiktok.com/@itstundealao/video/7702529514181627169) | [cjpais/Handy](https://github.com/cjpais/Handy) | MIT | Name displayed; owner low-legibility |
| [7684295430821235971](https://www.tiktok.com/@itstundealao/video/7684295430821235971) | [JuliusBrussee/caveman](https://github.com/JuliusBrussee/caveman) | Apache-2.0 | Header legible |
| [7689488245217824022](https://www.tiktok.com/@itstundealao/video/7689488245217824022) | [666ghj/MiroFish](https://github.com/666ghj/MiroFish) | AGPL-3.0 | Clone URL legible |
| [7693954418621844758](https://www.tiktok.com/@itstundealao/video/7693954418621844758) | [ayghri/i-have-adhd](https://github.com/ayghri/i-have-adhd) | MIT | Name displayed; owner inferred |
| [7694691465393507607](https://www.tiktok.com/@itstundealao/video/7694691465393507607) | [DietrichGebert/ponytail](https://github.com/DietrichGebert/ponytail) | MIT | Name displayed; owner inferred |
| [7699127837030157570](https://www.tiktok.com/@itstundealao/video/7699127837030157570) | [kaankiziltug/logo-design-skill](https://github.com/kaankiziltug/logo-design-skill) | MIT | Name displayed; owner inferred |
| [7683190353330982166](https://www.tiktok.com/@itstundealao/video/7683190353330982166) | [emilkowalski/skills](https://github.com/emilkowalski/skills) (apple-design/) | MIT | Breadcrumb partially legible |

## Source-unconfirmed candidates

| Video | Display evidence | Candidate repos | Status |
|-------|------------------|-----------------|--------|
| [7699878190784777505](https://www.tiktok.com/@itstundealao/video/7699878190784777505) | URL in prompt (low-res) | [debpalash/VoiceStudio](https://github.com/debpalash/VoiceStudio) (AGPL-3.0) | **CONFLICT** — video-model read `tundealao/VoiceStudio`; frame read `debpalash`. Retains SOURCE-UNCONFIRMED. |
| [7679101376772721942](https://www.tiktok.com/@itstundealao/video/7679101376772721942) | Commit msg "fork as taoufik/claude-watch" | [taoufik123-collab/claude-watch](https://github.com/taoufik123-collab/claude-watch) (MIT) or [bradautomates/claude-video](https://github.com/bradautomates/claude-video) (MIT) | **UNKNOWN** — owner not legible on-screen. |

## Already documented (link, not duplicated)

The [7686156516189080854](https://www.tiktok.com/@itstundealao/video/7686156516189080854) video demoing [latent-spaces/brag](https://github.com/latent-spaces/brag) (MIT) is documented in the existing research merged in PR #5:

- [research/tiktok-mcp-and-skills-2026-11-06/themes/04-itstundealao-brag.md](../tiktok-mcp-and-skills-2026-11-06/themes/04-itstundealao-brag.md)

## Theme documents

| Theme | File |
|-------|------|
| Short-form video tooling (supoclip, OpenMontage, VoiceStudio, claude-watch) | [themes/01-shortform-video-tooling.md](themes/01-shortform-video-tooling.md) |
| Token/output-shaping skills (caveman, ponytail, i-have-adhd) | [themes/02-token-output-shaping.md](themes/02-token-output-shaping.md) |
| "Fact or Hype?" format pattern | [themes/03-fact-or-hype-format.md](themes/03-fact-or-hype-format.md) |

## Source ledger and catalog

- [SOURCES.md](SOURCES.md) — provenance ledger for all source-linked videos
- [CATALOG.md](CATALOG.md) — 63 reviewed videos with screening status (trimmed from 262 enumerated)

## Screenshot index

| File | Description |
|------|-------------|
| `01-supoclip-clone-url.jpg` | Clone URL `github.com/FujiwaraChoki/supoclip` visible |
| `02-openmontage-repo-header.jpg` | Repo header `calesthio / OpenMontage` visible |
| `03-handy-clone-dialog.jpg` | Handy repo page with clone dialog |
| `04-caveman-repo-header.jpg` | Repo header `JuliusBrussee / caveman` visible |

## Limitations

1. **Coverage gaps:** 12 videos not enumerated; 172 older posts caption-only.
2. **Whisper transcription errors:** Automated transcripts may have errors.
3. **No code execution:** README claims not independently tested.
4. **Single-run anecdotes:** Cost/timing figures are the creator's single demo, not benchmarks.
5. **Source conflicts:** VoiceStudio and claude-watch owner attribution unresolved.
6. **Correlation not causation:** Format/views correlation observed, not a causal claim.

## Privacy notes

- No private information, employer details, or personal paths.
- Screenshots are frame extracts from public TikTok videos, EXIF-free.
- No raw transcripts beyond brief attributed quotes.
