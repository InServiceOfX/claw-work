# AI NEWS: Kimi K3.1 weights OUT, open source computer use, SOTA image model…

- Creator: Matthew Berman
- Canonical video: https://www.youtube.com/watch?v=-M62vWrZh54
- Publication date (YouTube date field): 2026-11-05
- Duration: 19:53
- Research date: 2026-11-07
- Evidence scope: Full ASR transcript + three chronological visual review clips

## What this covers

A **multi-story AI model-news roundup**, not a weight-space-programming tutorial.
Keep separate from WSP research; cross-link only where topically useful.

"Open source" in the video's framing does not establish each artifact's licence.
Open weights, source-available inference code, and OSI-compliant licensing are
distinct. Video largely reads provider/blog/demo claims; no independent executed
benchmark.

## Verified on-screen caveat

**~03:33 NorthCUA slide**: Research results are NOT tied to exact checkpoint
exports and are not fresh evaluations of repository files. This limits
transferability of benchmarks to downloadable weights but does not invalidate
underlying research.

## Timestamped inventory

| Time | Story | On-screen details | Local-relevance note |
|---|---|---|---|
| 00:00–02:53 | Kimi K3.1 Pro | 3T total params; host estimates ~1.2TB | Not local-laptop viable |
| 02:53–04:35 | NorthCUA Pro/Mini/Nano | 397B total / 17B active (Pro); "9B Nano consumer hardware" is host claim, no Mac support established | Desktop-agent candidate; sandbox required |
| 04:35–07:14 | Pruna P-H3-Image | BF16 ~55GB / int8 ~30GB / w4a8+int8VAE ~20GB GPU | No automatic Mac compatibility |
| 07:14–08:23 | Sarvam PRISM | FP8 NVIDIA ~120GB (B200/H200 class) | Not Mac-viable |
| 08:23–10:02 | Hydrogen 1.0 scientific | Demo/benchmark claims | Research lead; no validation |
| 10:02–10:51 | Perplexity Embed v2 Context | 9B / 0.6B variants mentioned | **RAG candidate**: 0.6B may fit local |
| 10:51–11:50 | Qwen3.5-4B-GLB-3D-Coder | 4B params, Three.js/GLB output | Narrow 3D asset tool |
| 11:50–12:17 | Frontier Pass | Paid membership tier | Promotion, not research |
| 12:17–14:20 | Pika H3 Flex | API tier prices visible on-screen | API pricing ≠ proof no open weights; verify primary terms |
| 14:20–15:18 | Apriel Voice 27B | 27B params | Voice-agent lead; not "universally local" |
| 15:18–16:20 | DiffuSpace Clover-35B-A3B | 35B total / 3B active | 3B active ≠ only 3B in memory |
| 16:20–17:35 | Liquid AI LFM3 | 1.7B/2B claimed on-device, ~1GB 4-bit | **Local candidate**: no independent hardware proof |
| 17:35–18:14 | Baidu Unlimited OCR 2 | Long-document OCR | **OCR candidate**: verify accuracy, memory, security |
| 18:14–19:53 | GPT-6.1 Terra | Proprietary/API | Closed; treat prices/rankings as claims |

## Local-relevance shortlist

| Candidate | Size | Status | Action |
|---|---|---|---|
| pplx-embed-v2-context (0.6B) | 0.6B | Promotional (demo shown) | Verify licence, runtime, retrieval quality |
| Liquid AI LFM3 (4-bit) | ~1GB claimed | Promotional (blog claim) | Verify actual model files, hardware tests |
| Unlimited OCR 2 | Unknown | Promotional (demo) | Verify accuracy on long docs, security review |
| NorthCUA Nano | "9B" host claim | Promotional | No Mac support established; sandbox required |

**Measured vs promotional**: All items above are promotional (provider/blog/demo
claims). None have independent executed benchmarks in this review.

## Evidence and uncertainty

- Full visual review completed 2026-11-07 via three chronological clips.
- ASR caption transcript reviewed in full.
- No continuous audio listening; no independent human transcription.
- ASR errors: "Northka/North Ka", "Kimmy", "Quen". Canonical spellings from
  on-screen text and description links.
- Raw unedited 1080p screenshots retained in private archive; six commentary
  frames selected for repository.
- Full annotated transcript delivered privately (not in repository).

## Validation scope

No dependency installed, no project code executed, no benchmark reproduced, no
weights downloaded. Each claim requires primary-source verification before
implementation. Do not invent owner links from watch results—verify against
primary sources.
