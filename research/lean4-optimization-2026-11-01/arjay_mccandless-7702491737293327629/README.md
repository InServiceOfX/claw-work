# Lean vs $12 server?!  #coding #programming

| | |
|---|---|
| Source | <https://www.tiktok.com/@arjay_mccandless/video/7702491737293327629?_r=1&_t=ZP-9B32vwwlDyw> |
| Creator | Arjay McCandless (@arjay_mccandless) |
| Uploaded | 2026-10-30 09:02 PDT |
| Short link sent | <https://www.tiktok.com/t/ZPL5rt6W3/> |
| Views / likes at ingest | 126.7k / 12.5k (2026-11-01) |
| Duration | 87s |
| Hashtags | #coding #programming |
| Ingested | by yeung-assistant (yt-dlp + faster-whisper `small`, language `en`) |

**Caption (verbatim):**

> Lean vs $12 server?!  #coding #programming

## Summary

Arjay McCandless (Ernest remembered him as "Arjun"; the on-screen handle and
TikTok channel are **Arjay McCandless / @arjay_mccandless**, GitHub
`arjaythedev`) runs the **$12 server challenge**: build a small social-media API
on SQLite that holds as many concurrent users as possible on a 1 vCPU / 2 GB
VPS. This video covers the entry by GitHub **@hargup** (Harsh Gupta),
`submissions/lean-derive-hargup`, **PR #153**. Instead of writing C, Rust, or
assembly directly, he specified the app in **Lean 4**, proved properties of the
stored data, and generated the C application layer from that spec. The proved
invariant lets `GET /feed` read the last 20 rows of SQLite's posts B-tree directly
(bypassing the SQL parser, planner, and bytecode VM), which took the feed from
16.7 µs to 5.4 µs (3.1x). With io_uring on top, Arjay measured 280,000
concurrent users held 5 minutes at 26,683 req/s.

Full research on the submission, with verified links and the leaderboard
numbers, is in
[`docs/research/lean4-optimization/README.md`](../../../research/lean4-optimization/README.md).

## Key takeaways

- Lean is used as a **spec + proof + code generator**, not as the runtime: a
  Lean program emits `app_gen.c`, and the proofs justify a workload-specific
  fast path.
- The win comes from a **proved data invariant** (post ids are 1..n with no
  gaps, `created_at` only goes up, every author exists). Under it, the feed
  order equals reverse rowid order, so 20 seeks + 20 counts become "last 20 rows + one
  index range scan".
- The invariant is **checked at runtime** (once at startup and after every
  insert). If a check fails, the server falls back to plain SQL. The fast path
  is "proven safe" only while the check holds.
- Arjay is explicit that you could write the same C by hand ("feed.c by hand
  = app_gen.c, same code"). Lean's contribution is the proof that the
  optimization is safe for this workload.
- io_uring (multishot accept/recv, 10 rings with `SO_REUSEPORT`, one SQLite
  transaction per batch) is shared by most top entries. It is not Lean-specific.

## Verbatim phrases and script

- "Somebody used mathematical proofs to build the most efficient web server
  for my $12 server challenge and the results are insane." (0:00)
- "Instead of writing their server in one of those languages directly, this
  person started with Lean 4." (0:18)
- "they used it to describe how the application should work, prove certain
  properties about the data, and then use those proofs to generate optimized
  C code." (0:32)
- "This developer proved that as long as certain conditions hold, those posts
  are already stored in the exact order they need." (0:45)
- "This reduced time to load the feed from 16.7 microseconds all the way down
  to 5.4, roughly three times faster." (1:02)
- "Technically, you could write these exact optimizations in C yourself
  without ever touching Lean. Lean just lets you prove that these
  optimizations are safe for your specific workload." (1:08)
- "They also made some pretty clever optimizations with io_uring [whisper:
  "IOU ring"], which basically all of our top submissions did." (1:17)
- "this submission supported over 280,000 concurrent users or 26,000 requests
  per second." (1:21)

Transcript corrections: "IOU ring" = io_uring; "B tree" = B-tree; "coming a
lot recently" is likely "coming up a lot recently".

## On-screen text

Every link, handle, and data card, verbatim (read by eye from 720px frames):

- **0:00.2–0:01.4** card `L∃VN Plan.lean`: `theorem feedPlan_eq` / `(h : Inv db) :` / `plan = feed`, then a check mark.
- **0:01.8–0:04** `Plan.lean` → `app_gen.c` (`int feed(...) {` / `BtreeLast(c);` / `...` / `}`) → `web server` / `one binary`; `$12 · 1vCPU` tag.
- **0:05.5** browser URL **`benchmarks.arjaythedev.com`**, page `$12 Server`, badge `1 vCPU`, "How many users can a $12 server hold? 1 vCPU Synteq VPS." Ranking (scores blurred in the video):
  1. `lean-derive-hargup`
  2. `c-iouring-public`
  3. `crystal-epoll-faustinoaq`
  4. `rust-epoll-scooter1337`
  5. `c-iouring-unorig-username`
- **0:06.5–0:09** GitHub card **`twelve-dollar-server-challenge`**, "open challenge · anyone can submit", "you, forking it"; mock feed `@maya ♥ 12`, `@tom ♥ 4`, `@li ♥ 31` (illustrative).
- **0:12–0:15** `users` → `server` `$12/mo`: `SQLite in-process`, `1 core`, `memory`; chips `1 vCPU`, `2 GB RAM`.
- **0:16–0:19** "top of the leaderboard": C, Rust, then Assembly (asm), Rust, C.
- **0:20–0:24** (key frame) logos C / Rust / asm crossed out, tag `L∃VN 4`; card: GitHub **`@hargup`**, path **`submissions/lean-derive-hargup`**, **`PR #153`** "new submission".
- **0:24–0:27** `Plan.lean`: `import Model` / `theorem feedPlan_eq` / `(h : Inv db) :` / `db.feedPlan = db.feed := by` / `…`; progress bar `Lean kernel` `checking...`.
- **0:28–0:33** browser URL **`openai.com/index/building-scientific-understandir`** (truncated; the full page is <https://openai.com/index/building-scientific-understanding-with-academic-communities/>), heading "The Exponential Time Hypothesis is false"; highlighted "result that was subsequently verified in Lean"; chips `major labs`, `verified in L∃VN`. (Context only, unrelated to the submission.)
- **0:34–0:39** files: `Model.lean` (feed · getPost / newPost · like), `Contract.lean` (routes · statuses / JSON), `Invariant.lean` (`theorem checkInv_sound` ✓, `theorem inv_addPost` ✓, `theorem inv_like` ✓) → `Emit.lean` `generator`.
- **0:40–0:45** `GET /feed` `normal SQL plan`; `lookups` 0 → 3 → 20; `created_at` index ↔ `posts` table; "newest 20" / "20 seeks".
- **0:46–0:51** `@hargup` `L∃VN proved:` `Db.Inv`: ✓ "post ids are 1...n, no gaps"; ✓ "created_at only goes up"; ✓ "every author exists". Sample table `id / created_at / author`: `1 09:00:01 u7`, `2 09:00:04 u3`, `3 09:00:09 u7`, ⋮, `n 12:41:55 u2`; "id ↓ time ↓".
- **0:52** `normal plan` "20 seeks + 20 counts".
- **0:54–0:57** `posts B-tree`, highlighted leaf "last 20 rows →"; "+1 likes-index range scan → all 20 counts".
- **0:58–1:03** stack `app` → `SQL text`, `parser / planner`, `bytecode VM` (`query engine`, crossed out) → `B-tree` → `pager + WAL` "still in a transaction" → `disk`; `direct` arrow; chips `sqlite3BtreeTableMoveto`, `BtreeNext`; "writes still use plain SQL".
- **1:04–1:08** `GET /feed time per request`: `through SQL` **16.7 µs**, `direct B-tree` **5.4 µs**; "**3.1×** faster feed".
- **1:10–1:13** `feed.c`: `feed_tail(db) {` / `BtreeLast(c);` / `Prev(c) ×19;` / `}` **=** `app_gen.c` (same code) ← `L∃VN Plan.lean`; "by hand"; "same code".
- **1:14–1:17** `L∃VN Inv db → feedPlan = feed` ✓; `at startup` / `every insert` → `check Inv db` → "✓ holds B-tree path" / "✗ fails plain SQL"; "fast path proven safe".
- **1:18–1:21** `app` / SQ, CQ rings / `kernel`; `multishot` "accept + recv"; `10 rings` `SO_REUSEPORT`; `1 SQLite txn` "per batch"; chips `C io_uring`, `Rust`, `Crystal`.
- **1:22–1:27** `1 vCPU` "hold 5:00"; 112,000 → **280,000** "concurrent users · held 5 min"; 5,718 → **26,683 req/s**; `p95 141 ms`, `p99 316 ms`, `errors 0`.

Links and handles seen: `benchmarks.arjaythedev.com`, `twelve-dollar-server-challenge`,
`@hargup`, `submissions/lean-derive-hargup`, `PR #153`,
`openai.com/index/building-scientific-understandir…`, and the leaderboard
entries above. The most useful frame is **0:21.5**
([screenshots/021.5s.jpg](screenshots/021.5s.jpg)).

## Screenshots

**0:00.9**: Opening Plan.lean theorem feedPlan_eq card

![0:00.9: Opening Plan.lean theorem feedPlan_eq card](screenshots/000.9s.jpg)

**0:03.5**: Plan.lean proved -> app_gen.c -> one-binary web server

![0:03.5: Plan.lean proved -> app_gen.c -> one-binary web server](screenshots/003.5s.jpg)

**0:05.5**: benchmarks.arjaythedev.com leaderboard, lean-derive-hargup at #1

![0:05.5: benchmarks.arjaythedev.com leaderboard, lean-derive-hargup at #1](screenshots/005.5s.jpg)

**0:06.5**: twelve-dollar-server-challenge repo card

![0:06.5: twelve-dollar-server-challenge repo card](screenshots/006.5s.jpg)

**0:08.5**: open challenge, anyone can submit; mock social feed

![0:08.5: open challenge, anyone can submit; mock social feed](screenshots/008.5s.jpg)

**0:14.0**: $12/mo server: SQLite in-process, 1 core, 1 vCPU, 2 GB RAM

![0:14.0: $12/mo server: SQLite in-process, 1 core, 1 vCPU, 2 GB RAM](screenshots/014.0s.jpg)

**0:17.5**: Top of the leaderboard: C, Assembly, Rust

![0:17.5: Top of the leaderboard: C, Assembly, Rust](screenshots/017.5s.jpg)

**0:21.5**: KEY: GitHub @hargup, submissions/lean-derive-hargup, PR #153

![0:21.5: KEY: GitHub @hargup, submissions/lean-derive-hargup, PR #153](screenshots/021.5s.jpg)

**0:26.0**: Plan.lean feedPlan_eq checked by the Lean kernel

![0:26.0: Plan.lean feedPlan_eq checked by the Lean kernel](screenshots/026.0s.jpg)

**0:28.0**: openai.com ETH-is-false page (verified in Lean)

![0:28.0: openai.com ETH-is-false page (verified in Lean)](screenshots/028.0s.jpg)

**0:32.0**: Same page, highlight: verified in Lean, major labs

![0:32.0: Same page, highlight: verified in Lean, major labs](screenshots/032.0s.jpg)

**0:38.5**: Model.lean, Contract.lean, Invariant.lean theorems, Emit.lean generator

![0:38.5: Model.lean, Contract.lean, Invariant.lean theorems, Emit.lean generator](screenshots/038.5s.jpg)

**0:44.5**: GET /feed normal SQL plan: 20 lookups

![0:44.5: GET /feed normal SQL plan: 20 lookups](screenshots/044.5s.jpg)

**0:48.5**: @hargup Lean proved Db.Inv (3 properties)

![0:48.5: @hargup Lean proved Db.Inv (3 properties)](screenshots/048.5s.jpg)

**0:50.2**: Db.Inv with sample rows: id and time both increase

![0:50.2: Db.Inv with sample rows: id and time both increase](screenshots/050.2s.jpg)

**0:52.5**: normal plan: 20 seeks + 20 counts

![0:52.5: normal plan: 20 seeks + 20 counts](screenshots/052.5s.jpg)

**0:56.5**: posts B-tree: last 20 rows + one likes-index range scan

![0:56.5: posts B-tree: last 20 rows + one likes-index range scan](screenshots/056.5s.jpg)

**1:02.5**: Bypass SQL text/parser/bytecode VM; sqlite3BtreeTableMoveto, BtreeNext

![1:02.5: Bypass SQL text/parser/bytecode VM; sqlite3BtreeTableMoveto, BtreeNext](screenshots/062.5s.jpg)

**1:08.5**: GET /feed 16.7 us through SQL vs 5.4 us direct B-tree, 3.1x

![1:08.5: GET /feed 16.7 us through SQL vs 5.4 us direct B-tree, 3.1x](screenshots/068.5s.jpg)

**1:12.5**: feed.c by hand = app_gen.c from Plan.lean (same code)

![1:12.5: feed.c by hand = app_gen.c from Plan.lean (same code)](screenshots/072.5s.jpg)

**1:16.5**: Inv db -> feedPlan = feed; checked at startup and every insert

![1:16.5: Inv db -> feedPlan = feed; checked at startup and every insert](screenshots/076.5s.jpg)

**1:20.5**: io_uring: multishot, 10 rings SO_REUSEPORT, 1 SQLite txn per batch

![1:20.5: io_uring: multishot, 10 rings SO_REUSEPORT, 1 SQLite txn per batch](screenshots/080.5s.jpg)

**1:26.8**: 280,000 concurrent users held 5 min, 26,683 req/s, p95 141 ms, p99 316 ms, 0 errors

![1:26.8: 280,000 concurrent users held 5 min, 26,683 req/s, p95 141 ms, p99 316 ms, 0 errors](screenshots/086.8s.jpg)

## How to apply

- Use the research note,
  [`docs/research/lean4-optimization/`](../../../research/lean4-optimization/README.md),
  which assesses a "Lean 4 for performance-critical code" workflow based on
  this spec → prove → generate pattern.
- Content angle: "proof-backed optimization" with a before/after µs bar chart
  and a leaderboard reveal is a strong technical short format. Note the
  structure: hook claim, context, "how", one number, honest caveat, final score.

## Transcript

Full timestamped transcript: [transcript.md](transcript.md).
