# Lean 4 for performance- and memory-critical code: the `lean-derive-hargup` submission

Research started from a TikTok Ernest sent (2026-11-01): Arjay McCandless,
"Lean vs $12 server?!", notes in
[`docs/shortform/tiktok/arjay_mccandless-7702491737293327629/`](../../shortform/tiktok/arjay_mccandless-7702491737293327629/README.md).
Goal: identify the submission, explain what Lean 4
([leanprover/lean4](https://github.com/leanprover/lean4)) was actually used for,
and decide whether a Monoclaw workflow, skill, or agent tool for "using Lean 4 to
optimize code where performance and memory matter" is worth building.

All links below were opened and returned HTTP 200 on 2026-11-01, except
`theoric.com` (HTTP 500). Times are Pacific.

## 1. The submission

| | |
|---|---|
| Challenge | **$12 server challenge**, <https://github.com/arjaythedev/twelve-dollar-server-challenge> ("Implement one small API on SQLite in any language; we benchmark it on a $12 server (1 vCPU, 2 GB)") |
| Organizer | Arjay McCandless: TikTok [@arjay_mccandless](https://www.tiktok.com/@arjay_mccandless), GitHub `arjaythedev`, live board <https://benchmarks.arjaythedev.com> |
| Entry | **`lean-derive-hargup`**: <https://github.com/arjaythedev/twelve-dollar-server-challenge/tree/main/submissions/lean-derive-hargup> |
| PR | **#153 "Lean 4 derived C-Code"**: <https://github.com/arjaythedev/twelve-dollar-server-challenge/pull/153>. Opened 2026-10-24 00:48 PDT, merged 2026-10-30 06:48 PDT. 27 files, +3,922 lines. Scored at head `fd2c149`. |
| Author | **Harsh Gupta**, GitHub [@hargup](https://github.com/hargup). Profile lists X [@hargup13](https://x.com/hargup13) and blog `http://theoric.com` (returned 500 when checked). |
| Leaderboard | <https://github.com/arjaythedev/twelve-dollar-server-challenge/blob/main/LEADERBOARD.md> |

### What it is

The README says the API is "specified in **Lean 4**: the domain, a global
invariant over the stored data, the read plans (with proofs that they compute
the specified results), and the wire contract. A Lean program then
**generates the C application layer** from that specification." The build runs
on the benchmark box, so Lean's kernel checks the theorems on every build. The
generator writes `gen/app_gen.c`, and `build.sh` refuses to build if the result
differs from the checked-in copy.

Lean package `spec/` (toolchain `leanprover/lean4:v4.35.0`, no dependencies beyond core):

| Lean file | Role | Becomes in `app_gen.c` |
|---|---|---|
| `Model.lean` | users/posts/likes; `feed`, `getPost`, `newPost`, `like` as plain functions | meaning of each endpoint; test oracle |
| `Invariant.lean` | `Db.Inv`, startup check `checkInv`, per-insert check `insertOk` | `app_check_inv`, post-insert check |
| `Plan.lean` | `feedPlan`, `postPlan` over B-tree primitives | `feed_tail`, `post_by_id`, `range_counts`, `user_name` |
| `Contract.lean` | routes, statuses, error bodies, JSON shapes | route matcher, constant responses, serializers |
| `Schema.lean` | `schema.sql` as data | field positions, SQL text, `PRAGMA table_info` check |
| `Emit.lean` | the generator | fills `Feed/templates/app.c` |

Kernel-checked theorems ("no `sorry`, only the standard axioms"):

| Theorem | Statement |
|---|---|
| `Db.feed_eq_tail` | `Inv db → db.feed = (db.posts.reverse.take 20).filterMap db.view` |
| `Db.feedPlan_eq` | `Inv db → db.feedPlan = db.feed` |
| `Db.postPlan_eq` | `db.postPlan id = db.getPost id` |
| `Db.checkInv_sound` | `db.checkInv = true → Inv db` |
| `Db.inv_addPost` | `Inv db → db.insertOk p = true → Inv (db.addPost p)` |
| `Db.inv_like` | likes preserve `Inv` |

`Db.Inv` = `dense` (post ids are exactly 1..n), `mono` (`created_at`
non-decreasing in rowid order), and `authors` (every author exists). Under it, feed
order equals reverse rowid order. The feed becomes "last 20 rows of the posts
B-tree + one range scan of `likes_post_id_idx`" instead of an index walk with
20 seeks and 20 counts. The C runs on SQLite's internal B-tree API
(`sqlite3BtreeTableMoveto`, `sqlite3BtreeIndexMoveto`, Next/Previous), compiled into the
amalgamation's translation unit. Writes stay as ordinary SQL. If the layout
differs from the schema, the invariant check fails, or another connection writes, the
server falls back to the spec's SQL for good.

**What Lean proves and what it doesn't** (the author says this plainly): "The plans are
proved against the model; the C emitted for them is tested, not proved." The
generator (`Emit.lean`) is not verified, the C runtime (io_uring, HTTP, JSON, JWT)
is hand-written, and the B-tree primitives are modelled, not verified against
SQLite. Byte-identical output against the SQL path (feed + 20,000 random posts)
plus the challenge's 42-check suite serve as the tests.

### Claimed and measured results

| Source | Claim |
|---|---|
| README (author) | `GET /feed` **16.7 µs → 5.4 µs** (reference SQL vs native B-tree plan, on the seed, byte-identical output). The video shows this as "3.1× faster feed". |
| PR #153 body (author) | "consume 19% less CPU per user than the rust-epoll-scooter1337 submission", #1 at the time |
| README local runs (author, Xeon E-2176G, loopback, not official) | 18.7 µs CPU/request, 579 MB peak (30 s ramp-down). Checkpoints in the event loop: 15.4–17.1 µs, 858–965 MB. Rust #82 in the same harness: 23.8 µs, 616 MB (19.3–21.3 µs, 676–713 MB). Tail p95 2.4 ms / p99 13 ms. |
| Organizer's benchmark agent (PR #153 comment) | **280,000 users held 5 min**, 26,683 req/s, p95 141 ms / p99 316 ms, 0 errors, 92 % app CPU (31 µs/request), **836 MB** peak. Tests 42/42. Fails at 290k+ with "connection refused" (connection limit plus CPU). The first run froze the box at a 320k probe (kernel OOM outside the cgroup, probably from 10 io_uring rings × 65,471 registered descriptors). |
| Organizer ruling | Arjay: "this is epic, allowed". On 2026-10-31 he ruled that reading through SQLite's B-tree API is allowed (rules 2/3). |
| LEADERBOARD.md (commit `feb3926`, 2026-11-01 09:38 PST) | Now tied **#2** at 280,000 (26,683 req/s, p95 141, p99 316, CPU 92 %, 31 µs/req, RAM **836 MB**), behind `c-iouring-pavlyhalim` #245 (295,000, 1,536 MB, 26 µs/req) and tied with Mojo #224 (1,597 MB) and Objective-C #154 (1,245 MB). It was #1 when the video was posted (the video's board shows it first). |

Memory: of the top-4 entries, `lean-derive` has the lowest peak memory (836 MB vs 1,245–1,597 MB).
Some lower-ranked C/Rust entries use less (`c-iouring-public` 625 MB,
`rust-epoll-scooter1337` 630 MB, `c-iouring-shivanshthapliyal` 638 MB). Memory
was not something Lean optimized. It follows from the io_uring and SQLite configuration.

Related entry: `lean-c-iouring-loafecho` (PR
[#239](https://github.com/arjaythedev/twelve-dollar-server-challenge/pull/239)
by `loafecho`, merged, "Lean-proved B-tree read plans, C io_uring server"). It
has a Lean 4.35.0 `spec/` but a hand-written C server. Scored 65,000, connection-capped
at about 65k at 28 % CPU (rank 104), so its score doesn't say much about the approach.

The video also shows OpenAI's
"[Working with academic communities to build scientific understanding](https://openai.com/index/building-scientific-understanding-with-academic-communities/)"
page (ETH refutation "subsequently verified in Lean"), only as context for
why Lean is in the news. It is unrelated to the submission.

## 2. Assessment: is a workflow worth building?

**Yes, but a narrow one.** The finding is real and reproducible (public code,
kernel-checked proofs, independently benchmarked), but it supports a specific
pattern, not "Lean makes code fast":

- The speedup comes from a **workload-specific algorithmic or plan change**
  (skip the query engine, exploit an ordering invariant). Lean contributes the
  **evidence that the change is safe**: an equivalence theorem
  `Inv → fastPlan = referencePlan` plus sound runtime checks for the
  invariant. Arjay says so himself: "you could write these exact optimizations
  in C yourself."
- The trust gap sits in the generator and the C. Lean proves the model, and the C is
  only tested. A workflow has to keep the **differential tests** (reference vs
  optimized, byte-identical outputs) as a first-class part, not an afterthought.
- Lean 4 does compile to C and can be fast, but nothing here shows
  Lean-the-runtime beating C/Rust. Writing hot loops in Lean itself is a
  separate and weaker case.
- Where it pays off: optimizations that rely on **data or usage invariants**
  (ordering, density, uniqueness, monotonicity, aliasing, bounds) that are
  easy to get subtly wrong and costly when wrong: query fast paths, cache
  layouts, index tricks, bit-packing, removing bounds checks, SIMD/loop
  rewrites with stated preconditions. It is overkill for ordinary tuning
  (profiling, allocation reduction, I/O batching), which needs a benchmark harness
  and no proofs.
- Cost: Lean proof engineering is slow even with LLM help. A useful agent
  tool has to make "state model → state fast plan → prove equality" cheap,
  and it must fall back gracefully (property tests) when a proof stalls.

Recommendation: build a **skill + small Docker-backed CLI**, not an autonomous
agent at first. Scope: "prove an optimized plan equals a reference spec under
stated invariants, then differential-test and benchmark the real code."

## 3. Workflow sketch (not built)

`spec → invariant → fast plan → proof → generate or hand-write → differential test → benchmark`

1. **Reference model** (`Model.lean`): the slow, obviously correct version as
   pure Lean functions over a simple data model (lists or maps). This is also the
   test oracle.
2. **Invariant** (`Invariant.lean`): the data or usage preconditions the
   optimization needs, plus an executable `check : Db → Bool` with
   `check_sound : check db = true → Inv db`, and preservation lemmas for each
   write (`inv_op`).
3. **Fast plan** (`Plan.lean`): the optimized algorithm in terms of modelled
   primitives (B-tree tail scan, index range, packed array, and so on), with
   `plan_eq : Inv db → fastPlan db = reference db`.
4. **Emit or bind**: either a Lean generator that writes C, Rust, or Python from the
   plan (hargup's approach), or a hand-written implementation tied to the plan
   by naming and tests. Generated code is checked in and diffed on every build.
5. **Runtime guard**: run `check` at startup and after writes. On failure,
   fall back to the reference path.
6. **Differential tests**: optimized implementation vs reference (SQL or
   naive code) on seeds plus random or property-generated inputs, byte-identical.
7. **Benchmark gate**: µs/op, CPU %, peak RSS (cgroup), before and after, on a
   pinned box. Report with caveats, as hargup's README does.
8. **Agent loop**: an LLM drafts steps 1–3. `lake build` in Docker is the
   verifier, so "no `sorry`, standard axioms only" is checked with
   `#print axioms`. Iterate on kernel errors, and stop after N failed proof attempts
   to fall back to property tests only.

Possible home: `shared/openclaw/skills/lean4-verified-optimization/SKILL.md`
+ `Scripts/lean4/` (Dockerfile + `lean-docker` wrapper). Pilot target:
re-derive `feedPlan_eq` from hargup's spec in our container, then apply the
pattern to one Monoclaw hot path.

### Lean in Docker (Ernest's preference over local elan)

Findings (image search done by the parent agent on 2026-11-01, Docker Hub date
re-checked here):

- **No official image from leanprover.** The `leanprover` namespace on Docker
  Hub is empty. On GHCR it only has the `velcom` and `lean-workbench` images.
- The closest is the community image
  [`leanprovercommunity/lean4`](https://hub.docker.com/r/leanprovercommunity/lean4)
  (Dockerfile: [leanprover-community/mathlib4 `.docker/lean/Dockerfile`](https://github.com/leanprover-community/mathlib4/blob/master/.docker/lean/Dockerfile)).
  It was last updated **2024-08-20** (Docker Hub API), so it is stale for Lean 4.35.
- Recommendation: **our own small, pinned image** that installs
  [elan](https://github.com/leanprover/elan) inside the container and pins the
  toolchain from the project's `lean-toolchain`, running as a non-root user.

Minimal Dockerfile (sketch, not yet built or tested):

```dockerfile
# Scripts/lean4/Dockerfile  (sketch)
FROM ubuntu:24.04
ARG LEAN_TOOLCHAIN=leanprover/lean4:v4.35.0
RUN apt-get update && apt-get install -y --no-install-recommends \
      curl ca-certificates git build-essential \
    && rm -rf /var/lib/apt/lists/*
RUN useradd -m -u 1000 lean
USER lean
ENV PATH=$HOME/.elan/bin:$PATH
# Pin elan-init by commit/SHA-256 in the real version.
RUN curl -sSfL https://raw.githubusercontent.com/leanprover/elan/master/elan-init.sh \
      | sh -s -- -y --default-toolchain "$LEAN_TOOLCHAIN" --no-modify-path \
    && lean --version && lake --version
WORKDIR /work
```

Build with the project's toolchain, then call `lean` or `lake` with a mounted workdir:

```bash
docker build -t monoclaw/lean4:v4.35.0 \
  --build-arg LEAN_TOOLCHAIN="$(cat spec/lean-toolchain)" Scripts/lean4
docker run --rm -u "$(id -u):$(id -g)" -v "$PWD":/work -w /work/spec \
  monoclaw/lean4:v4.35.0 lake build            # kernel-checks all theorems
docker run --rm -v "$PWD":/work -w /work/spec monoclaw/lean4:v4.35.0 \
  lake exe derive ../gen                        # run the generator
docker run --rm -v "$PWD":/work -w /work monoclaw/lean4:v4.35.0 \
  lean --run Check.lean                         # e.g. #print axioms Feed.Db.feedPlan_eq
```

Notes: tag the image by toolchain. Mount a named volume for `~/.cache` or
`.lake` if Mathlib is ever added (large). Agents call only the wrapper, so
they never need a host toolchain.

## 4. What was searched

- Resolved the TikTok short link with yt-dlp →
  `@arjay_mccandless/video/7702491737293327629`.
- `gh search repos twelve-dollar-server-challenge` → the challenge repo (589★
  on 2026-11-01). Read `LEADERBOARD.md`, PR #153 (body + comments), PR #239, and
  the submission's README, `spec/Feed/*.lean`, `lean-toolchain`, and `lakefile.toml`
  through the GitHub API.
- `gh api users/hargup` → name, X handle, blog.
- Web search for the OpenAI ETH page shown on screen.
