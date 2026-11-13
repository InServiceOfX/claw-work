# lean-derive-hargup: what Lean 4 actually buys for performance-critical code

*Analysis by yeung-assistant for Ernest Yeung, 2026-11-01 (PT). Written for the question:
"does Harsh Gupta's (@hargup) Lean 4 submission to the Twelve Dollar Server Challenge generalize
into a workflow, skill or AI-agent tool for using Lean 4 to optimize performance- and
memory-critical code?"*

## Sources

| Ref | What | Where |
|---|---|---|
| [S] | Submission `submissions/lean-derive-hargup` (PR #153, merged), upstream commit `cfa7003` | https://github.com/arjaythedev/twelve-dollar-server-challenge/tree/main/submissions/lean-derive-hargup |
| [PR] | PR #153 discussion (benchmark agent + organiser) | https://github.com/arjaythedev/twelve-dollar-server-challenge/pull/153 |
| [L] | Leaderboard | `LEADERBOARD.md` at `cfa7003` |
| [LE] | Contrast entry `lean-c-iouring-loafecho` (PR #239) | `submissions/lean-c-iouring-loafecho` |
| [P1] | #1 entry `c-iouring-pavlyhalim` (PR #245) | `submissions/c-iouring-pavlyhalim` |
| [LOW] | Fork branch `lean-derive-lowmem-hargup` @ `100234f` | https://github.com/hargup/twelve-dollar-server-challenge |
| [MET] | Fork branch `lean-derive-metal-complete-hargup` @ `b9a1e99` | same fork |
| [B-*] | Theoric Labs blog posts by hargup (local copy of theoric repo) | `blog/<slug>/{post.md,index.html}` |
| [FAQ] | Lean FAQ, "What is the Trusted Code Base of Lean?" | https://lean-lang.org/faq/ |
| [VAL] | Lean reference, "Validating a Lean Proof" | https://lean-lang.org/doc/reference/latest/ValidatingProofs/ |
| [FFI] | Lean reference, "Foreign Function Interface" | https://lean-lang.org/doc/reference/latest/Run-Time-Code/Foreign-Function-Interface/ |
| [PIT] | leanprover-community pitfalls (`native_decide`) | https://github.com/leanprover-community/leanprover-community.github.io/blob/lean4/templates/extras/pitfalls.md |
| [AEN] / [CHA] / [HAX] / [DOJO] | Aeneas, Charon, hax, LeanDojo READMEs | github.com/AeneasVerif/aeneas, /AeneasVerif/charon, /cryspen/hax, /lean-dojo/LeanDojo |

File paths below without a prefix are relative to `submissions/lean-derive-hargup/` at `cfa7003`.

---

## TL;DR

1. **Architecture.** Lean is a *generator and proof checker*, not the runtime. An executable
   Lean model of the endpoints (`spec/Feed/Model.lean`), a data invariant (`Invariant.lean`),
   optimized "plans" over B-tree primitives with kernel-checked equivalence proofs (`Plan.lean`),
   and an emitter (`Emit.lean`) that fills ~45 `@{KEY}` placeholders in a **hand-written C
   template** (`spec/Feed/templates/app.c`, 402 lines). The runtime (io_uring, HTTP, JSON, JWT,
   SQLite glue; ~1,670 lines) is hand-written C. **The proofs are about Lean models of the plans,
   not about the C that runs.** The C plan bodies, the primitive semantics, the emitter, SQLite
   internals and gcc are all trusted.
2. **Where the wins come from.** Almost all throughput/latency comes from things any good C entry
   does (no framework, io_uring, group commit, in-TU SQLite B-tree access that skips the VDBE,
   constant-folded responses). #1 (pure C) beats it on throughput without using the invariant.
   The Lean-specific win is narrower: a *proved-safe* invariant (`ids dense, created_at monotone`)
   that turns the feed into a tail scan; on my box that path halves server CPU per `/feed`
   request vs. the SQL fallback (12.3 vs 24.9 µs). The memory win (836 MB vs 1,536 MB for #1) is
   runtime engineering, not Lean.
3. **Generalization.** Fits problems where an optimization is justified by a *discrete invariant
   over data* (ordering, density, sparsity pattern, bounded sizes, monotone time tags) and the
   fast path can be checked at runtime with a fallback. Fits poorly for floating-point numerics
   (integrator accuracy, reassociation), because Lean's `Float` is effectively opaque to the
   kernel [PIT]. For orbit propagators the sweet spot is the *discrete scaffolding*: ephemeris
   segment lookup, event/step bookkeeping, buffer sizing, packet framing, and op-order-preserving
   restructurings proven over an abstract number type.
4. **Workflow.** A `lean-derive` skill + a `lean-check` container tool (this PR ships the
   Dockerfile and checker): model → invariant + sound checker → plan + equivalence theorem →
   emit constants/tables into hand C → runtime invariant check + fallback → mandatory
   differential tests + A/B/B/A benchmark. Agents write proofs, models, harnesses; they must not
   be trusted for theorem statements, primitive semantics, C templates, or performance claims.
   **Go/no-go: conditional GO for a 2-week narrow pilot; NO-GO as a general "Lean makes code
   fast" strategy.**

---

## 0. What I ran (build / proof / codegen / differential)

Box: Debian trixie, 8 vCPU, no Docker → installed elan directly
(`elan-init.sh -y --default-toolchain none; elan toolchain install leanprover/lean4:v4.35.0`,
matching `spec/lean-toolchain`). `build.sh` hard-codes `LEAN=/opt/lean-4.35.0/bin`
(`install.sh` puts a sha-pinned tarball there); I pointed it at the elan toolchain instead.

| Check | Result |
|---|---|
| `lake build` (spec/) | OK, 18 jobs, ~3 s |
| `lake exe derive` → `app_gen.c` vs checked-in `gen/app_gen.c` | **byte-identical** (sha256 `4c0fee6d…b70a`); `build.sh` itself also `cmp`s |
| Escape hatches in `.lean` sources (`sorry`, `admit`, `native_decide`, `implemented_by`, `extern`, `unsafe`, `axiom`) | none |
| `#print axioms` on `feedPlan_eq`, `postPlan_eq`, `feed_eq_tail`, `checkInv_sound`, `inv_addPost`, `inv_like`, `rangeCounts_eq` (+2 more) | only `[propext, Classical.choice, Quot.sound]` |
| Full `build.sh` (SQLite 3.54.1 amalgamation, sha-pinned, + runtime) | OK, ~24 s |
| Challenge CI (`test/run.sh`, seeded DB) | passed=42, failed=0 |
| **My differential test**: server A (native plans) vs server B forced to the SQL fallback by a foreign write (`UPDATE users SET username=username WHERE id=1` → log "native read plans disabled") | 20,004 `/posts/:id` (random + ids 1, 250000, 500000, 500001): **0 mismatches**; `/feed` identical |
| Fork branch [MET] `lake build` + regenerate 4 files (`app_gen.c`, `job_gen.h`, `frame_gen.h`, `sqe_gen.h`) | OK, 26 jobs; all 4 MATCH checked-in `gen/` |

The repeatable checker is `Scripts/lean4-docker/check.sh` (this PR); its output on [S] is
reproduced in `Scripts/lean4-docker/README.md`.

---

## 1. Architecture and the trust boundary

### 1.1 Pipeline

```
spec/Feed/Model.lean      executable reference semantics (Lists in rowid order)
        │                 feed = mergeSort newerEq → filterMap view → take 20   (Model.lean:65,71)
spec/Feed/Invariant.lean  Inv = dense ids ∧ createdAt monotone ∧ authors exist (Invariant.lean:25-28)
        │                 checkInv : Bool, checkInv_sound : checkInv = true → Inv  (:33-41, :76)
        │                 inv_addPost / inv_like : writes preserve Inv given insertOk (:84-138)
spec/Feed/Plan.lean       plans over modeled B-tree primitives
        │                 tailRows, rangeCounts (= list filter), window              (:133-151)
        │                 feedPlan_eq : Inv db → feedPlan db = feed db               (:186)
        │                 postPlan_eq : postPlan db id = getPost db id (no Inv)      (:208)
spec/Feed/Contract.lean   status codes, error cases, JSON shapes, routes             (:163-169)
spec/Feed/Schema.lean     tables/columns/indexes as data, ordinals
spec/Feed/Emit.lean       C-literal escaping (cstr), @{KEY} fill, merged JSON pieces,
        │                 error responses with Content-Length, schema check, SQL text,
        │                 route matcher, decode tables → appGen fills ~45 keys    (:200-247)
        ▼
templates/app.c (402 lines, HAND-WRITTEN C) ──fill──► gen/app_gen.c (512 lines)
runtime/*.c (HAND-WRITTEN: uring 561, db 328, json 230, jwt 166, out 147, http 131, rt.h 71)
runtime/server.c (34): one TU #including sqlite3.c + app_gen.c + runtime → gcc -O2 -march=native
```

### 1.2 What is proved (plain language)

- `feed_eq_tail` (Plan.lean:117): if ids are dense and `created_at` is monotone in id order, the
  "sort all posts newest-first and take 20" spec equals "take the last 20 rows in id order,
  reversed".
- `feedPlan_eq` (:186): the concrete plan (tail scan over a bounded window + one like-count range
  scan + author lookup) equals the spec feed, **assuming `Inv`**.
- `rangeCounts_eq` (:140): the "count likes per post id in a contiguous id range" primitive equals
  per-post filtering, under the model's definition of the primitive.
- `postPlan_eq` (:208): `GET /posts/:id` via rowid seek + index range count equals the spec, with
  no invariant.
- `checkInv_sound` (Invariant.lean:76): the boolean scan used at startup implies `Inv`.
- `inv_addPost` / `inv_like` (:91, :138): inserting a post that passes `insertOk` (next id,
  non-decreasing timestamp, existing author) and inserting a like preserve `Inv`.

### 1.3 The trust boundary (skeptical reading)

What the Lean kernel checked is *only* the equivalence of Lean functions over Lean lists. The
following are outside it and trusted:

1. **The C plan bodies.** `feed_tail` (app.c:223-250), `post_by_id`, `range_counts` (:187),
   `app_check_inv` (:60-90) and the write-path `insertOk` check (:367,
   `id == prev_id + 1 && text_le(prev, created)`) are hand-written. The emitter never generates
   control flow for them; it fills constants. Nothing connects `feedPlan` (Lean) to `feed_tail`
   (C) except the author's care, naming, and tests. (In fairness, `feed_tail` re-checks density
   at read time: `if (id != hi - k) return -1;` at app.c:237, falling back to SQL.)
2. **Primitive semantics.** `rangeCounts` *is defined* as a filter over the likes list
   (Plan.lean:137). The claim that `sqlite3BtreeIndexMoveto` + `Next` over `likes_post_id_idx`
   computes that is an unproved modeling assumption about SQLite internals.
3. **String order.** Lean `String` `≤` (used by `newerEq`, `mono`) vs. C `text_le`
   (app.c:54, memcmp-style). They agree for byte-wise UTF-8 comparison of ISO timestamps, but
   this is an assumption, not a theorem.
4. **The emitter and template filling.** `cstr` escaping, `@{KEY}` replacement, Content-Length
   arithmetic and the generated route matcher are Lean code that is *run*, not proved. The
   golden-file `cmp` (build.sh) only proves determinism, not correctness. The challenge's
   42 CI tests and my differential test are what cover it.
5. **The Lean compiler** (for running `derive`), **gcc**, **SQLite 3.54.1** (internal B-tree APIs
   are only callable because the amalgamation is in the same TU: runtime/server.c), the kernel's
   io_uring, and the runtime's concurrency (checkpoint thread, `SQLITE_THREADSAFE=2`).
   The Lean FAQ states this explicitly: running Lean code natively extends the TCB with "the Lean
   compiler…, the Lean runtime…, and the code generation backend…, as well as any other code
   external to Lean that is linked in" [FAQ].
6. **The fallback trigger.** Correctness under foreign writers depends on the C detecting them
   (`BTREE_DATA_VERSION` check in runtime/db.c) and switching to SQL. I exercised this path and it
   worked, but it is not modeled.

Documentation drift (typical of agent-written repos, and worth a skill rule):
- README.md:144 claims "the feed and 20,000 random posts are byte-identical on the seed", but no
  such test exists in the repo (`tests/` only has `blast.c`, a load generator). My differential
  test reproduces the claim; the repo does not.
- README.md:18 says `THREADSAFE=0`; `build.sh` and README.md:102 use `SQLITE_THREADSAFE=2`.
- Model.lean:7 refers to `Feed.Oracle`, which does not exist.

**Bottom line:** this is *proof-guided engineering*, not verified compilation. The proofs give
real value (they tell you exactly which invariant makes the fast path legal, and the invariant is
then checked at runtime with a fallback), but a bug in `feed_tail` would not be caught by Lean.

### 1.4 Contrast: loafecho (#239)

[LE] proves the *same* theorem family (`feed_eq_tail`, `feedPlan_eq`, `postPlan_eq` under `Inv`,
`checkInv_sound`, `inv_addPost`, `inv_addLike` in `Spec/Theorems.lean`) but Lean is **not in the
build path at all**: its README says so and `build.sh` compiles only C (server.c 1,198 lines,
db.c 1,425 lines, liburing, PGO). It compensates with a `--selfcheck` differential mode. Its
65k ᶜ score is a connection cap, not CPU (fixed-file table sized from `RLIMIT_NOFILE`, server.c:1117;
LEADERBOARD.md:333). So the two Lean entries bracket the design space: hargup uses Lean as
*generator + proof*, loafecho as *proof only*. In both, the C that runs is hand-written; hargup's
generator removes a class of constant/format bugs, nothing more.

---

## 2. Where the performance and memory wins come from

Leaderboard (LEADERBOARD.md:46-47, :159):

| Entry | Max sustained | req/s | p99 ms | µs CPU/req | Peak MB |
|---|---|---|---|---|---|
| #1 c-iouring-pavlyhalim (hand C) | 295,000 | 28,013 | 117 | 26 | 1,536 |
| #2 lean-derive-hargup | 280,000 | 26,683 | 316 | 31 | 836 |
| #104 lean-c-iouring-loafecho | 65,000 ᶜ | 6,399 | 56 | 41 | 605 |

### 2.1 Attribution

| Win | Source | Lean-specific? |
|---|---|---|
| No framework; io_uring multishot accept/recv, provided buffers, direct descriptors, `DEFER_TASKRUN`, 10 rings (runtime/uring.c) | runtime | No; every top entry does this |
| Group commit: one transaction per batch, WAL + `synchronous=NORMAL`, checkpoint thread, 250 µs coalescing (runtime/db.c) | runtime | No |
| In-TU SQLite B-tree reads (`sqlite3BtreeTableMoveto`, `IndexMoveto`, `Last/Previous/Next`, `PayloadFetch`) that skip the VDBE | runtime + template | No; ruled allowed (PR #153: "this is epic, allowed"; LEADERBOARD.md ~:399), now common on the leaderboard ("SQLite B-tree reads" in many rows) |
| Pre-rendered error responses with Content-Length, merged JSON literals, schema ordinals | **emitter** | Codegen convenience: removes format bugs. Any templating language could do it; Lean lets it share definitions with the proofs |
| Feed as a bounded tail scan instead of sort/index walk | **invariant + proof** | **Yes, the one genuinely Lean-enabled optimization** |

**My micro-benchmark** (server pinned to 1 core, `tests/blast.c` on 6 other cores, 4 threads ×
16 connections × 10 s, seeded DB; server CPU from `/proc/<pid>/stat`; one run each, box noise
applies):

| Path | `/feed` req/s | `/feed` CPU/req | `/posts/:id` req/s | `/posts/:id` CPU/req |
|---|---|---|---|---|
| Native plans | 53,313 | 12.3 µs | 64,841 | 9.1 µs |
| SQL fallback (same binary, after foreign write) | 31,522 | 24.9 µs | 71,086 | 8.0 µs |

So the invariant-enabled feed plan is a real ~2x on the feed's CPU (README.md:83-84 reports
16.7 → 5.4 µs for the app part alone). The single-post plan shows **no win** over SQLite's
prepared statement here (within noise or slightly worse), which matches the fact that
`postPlan_eq` needs no invariant: it is the same seek SQLite does.

### 2.2 Versus #1 (pure C)

[P1] reaches higher throughput **without the density invariant**: it walks
`posts_created_at_id_idx` (a `(created_at DESC, id DESC)` index) for the 20 newest ids
(src/btree.h:81-85, :423), then one likes range scan, one posts range scan, and group-prefetched
author lookups. It also uses multishot *poll* instead of multishot recv (263 vs 360 bytes per
idle connection), 8 rings created on demand, `unix-excl` VFS with a WAL write-buffer shim
(src/db.h:3-12, :248, :414), a PCACHE2 wrapper, PGO, `-O3`, static linking, `mmap_size=0`,
8 MB cache, a nice-19 checkpointer and `mlock`. Lesson: a 20-entry walk of a sorted index is
already cheap; the invariant saves a sort that good C avoids another way. The marginal
performance value of the proof is small; its value is *licensing* a shortcut that would
otherwise be a silent correctness bet.

### 2.3 Memory

836 MB vs 1,536 MB comes from runtime choices: ring/buffer sizing, kernel socket memory at 280k
connections, page cache settings, and on #1's side probably `mlock` plus a larger cache. None of it
comes from Lean. Note the PR thread: the benchmark agent's first search **froze the box at 320k**,
with the kernel running out of memory outside the cgroup, probably from 10 rings × 65,471 direct
descriptors [PR]. Memory risk here is in the kernel and io_uring sizing, which Lean does not model.

### 2.4 The fork branches confirm this

- **[LOW] lowmem (6 commits):** proofs unchanged; Emit adds `hasRows`/`bufferClass` (OUT_BIG vs
  OUT_SMALL chosen from the JSON shape). Everything else is runtime: rings on demand (5 vs 10 at
  300k), 512 receive buffers, releasing drained input buffers, capped free lists, reusable
  cursor storage, 1 MiB page cache, WAL bound. `tests/optimization-results.md`: releasing input
  buffers cut idle memory **40.7 → 1.6 MiB per 10k connections**; cursor reuse −2.8 to −3.9 % CPU;
  `-O3` rejected; 250 µs coalescing kept. Adds `tests/edge_cases.py`.
- **[MET] metal-complete (24 commits):** the serious attempt at *type-directed codegen*. New
  `Bounds.lean` (`Shape.fixedBound`, a `Bounded` structure carrying `by decide` evidence, inline
  capacity 192), `Frame.lean` (generated framing union), `Sqe.lean` (`ReadyPlan` with a
  `complete` proof that field assignments cover the opcode's required SQE fields; generates
  `sqe_*` constructors), `Wire.lean` (typed `Ref` contexts, `PlainText` refinement), plus
  Contract/Schema lemmas (`PayloadRef`, `prefix_le`). It generates `app_gen.c`, `job_gen.h`,
  `frame_gen.h`, `sqe_gen.h`. `interventions/00-09` rank ideas (bounded responses 224 vs 1,056
  bytes, 32- vs 72-byte jobs, ~19x targeted framing, −99.2 % tail SQE publications, −87 %
  pwrite via WAL buffer, AVX2 escaping −2.5 % feed CPU). **End-to-end: +2.4 % req/s and
  −2.3 % CPU per request (5.685 → 5.59 µs) in A/B/B/A runs**, which the author calls modest,
  with no memory claim. Also adds `crash_durability.py` and `tests/metal/oracle.py`.

So the author's own follow-up shows that pushing more of the low-level structure into Lean types
gives safety (exhaustive SQE fields, bounded buffers proved at generation time) and single-digit
performance gains.

### 2.5 The author's stated methodology (blog)

- [B-why-lean-4-for-agentic-coding] (Dec 22 2025, index.html:68): fix types and invariants first,
  let agents do the boring proofs; "If the code compiles, we can blindly trust the implementation
  to be right". **This submission is a counterexample to that slogan as applied to emitted C**:
  the proofs compile, yet the running C is outside them.
- [B-we-rewrote-tinygrad-in-lean] (June 29 2026): Lean core behind Python via FFI ("Lean compiles
  down to C", :47); agent-driven deletion 250k → 60k LOC behind a pre-commit test hook (:56-65);
  phase gates; agents caught trying to "cheat" (:72); "we have not yet mathematically proven
  TGrad" (:76); faster in 4 of 5 benchmarks, up to 3x.
- [B-leandb] (Sep 13 2026): typed schema, refinement types, proof-carrying writes; decoding
  stored rows must re-validate (post.md ~:228 and "Preserving the Domain Model").
- [B-leanhttp] (Sep 21 2026, post.md:22): "Writing an application in Lean does not automatically
  make the application correct or formally verified. The hard part is preserving the semantics
  across boundaries." **This is the right framing, and the trust-boundary analysis above is
  exactly that boundary.**
- [B-leanapi] (Sep 28 2026): Express/FastAPI-style server (github.com/theoriclabs/leanapi).

Methodology in one line: *types/invariants first → agent fills proofs and code → tests and phase
gates at every boundary Lean can't see*. That generalizes. "Blind trust" does not.

---

## 3. Generalization: which problems fit

### 3.1 The reusable pattern

> **An optimization is legal only under a data invariant. Prove `Inv → fast = spec` in Lean,
> prove that a cheap runtime checker implies `Inv`, prove that mutations preserve `Inv`, then
> ship the fast path guarded by the checker with the reference path as fallback.**

Lean pays off when (a) the spec is cheap to write as a pure function, (b) the invariant is
discrete and checkable, (c) the fast path is a structural rewrite (ordering, indexing, batching,
bounds), and (d) the fast path's C is small enough for a human to audit against the Lean plan.

### 3.2 Fit / no-fit

| Problem kind | Fit | Why |
|---|---|---|
| Query/index plans over ordered data (this submission) | **Good** | Discrete invariants (sortedness, density, uniqueness); lists model them well |
| Fixed-size layouts, buffer bounds, framing/serialization (CCSDS-style packets, telemetry frames) | **Good** | [MET]'s `Bounded`/`Frame.lean` show generation-time bounds + generated encoders. Memory bounds become theorems |
| Exhaustive dispatch / state machines (mode logic, command tables, SQE field completeness) | **Good** | `ReadyPlan.complete`-style coverage proofs [MET] |
| Ephemeris/table lookup: pick a Chebyshev segment or interpolation interval from a sorted time grid | **Good** | Same shape as `feed_eq_tail`: with a uniform/monotone grid invariant, an O(1) index computation equals binary/linear search; checker = grid scan; fallback = search |
| Event-detection bookkeeping in a propagator (root bracketing, step acceptance/rejection logic, event queue ordering) | **Medium** | Discrete control logic is provable; the float predicates inside are not |
| Op-order-preserving restructurings (AoS→SoA, loop fusion, CSE, hoisting) of an integrator step | **Medium** | Provable *if* the model is generic over an abstract number type with uninterpreted `+`/`*`: if the same operations happen in the same order, equivalence holds for IEEE doubles too, without any float axioms |
| Floating-point numerics: reassociation, FMA contraction, vectorized reductions, RK/symplectic accuracy, step-size error control | **Poor** | Lean's `Float` is opaque to the kernel; properties about it essentially need `native_decide`, which trusts the compiler and every `extern`/`implemented_by` [PIT][VAL]. Proofs over exact reals don't transfer to doubles. Use differential testing with ULP/energy/orbit-element tolerances instead |
| Throughput from OS/hardware (io_uring, NUMA, cache tuning, SIMD) | **None** | Lean doesn't model it; per §2, that's where most of the speed was |
| Existing Rust code you want to verify in place | Different tool | Charon extracts a Rust crate to LLBC [CHA]; Aeneas translates it to a pure lambda calculus with a Lean backend [AEN]; hax translates "a large subset of Rust" into Lean/F*/Rocq (Lean via Aeneas) [HAX]. These verify the code you run, unlike hargup's model-plus-template approach. Not evaluated here |

### 3.3 Integration mechanics (verified)

- **Generator mode (hargup):** Lean runs at build time and emits C text. No Lean runtime in the
  product. Simplest to adopt in a C/C++ codebase.
- **FFI mode (TGrad):** Lean compiled to C and linked in. The FFI is officially "designed for
  internal use… unstable", and can't pass C structs by value; `@[extern]`/`@[export]`; objects
  are refcounted `lean_object*` with owned/borrowed conventions; module initializers must run
  first and foreign threads must call `lean_initialize_thread` [FFI]. For a hot inner loop in a
  spacecraft sim this is a poor fit (boxing, refcounts, init); for a cold, logic-heavy core it
  is workable.
- **AI tooling:** LeanDojo is a Python library for extracting proof data and interacting with
  Lean programmatically; the original is deprecated in favor of LeanDojo-v2 [DOJO]. Not
  required for this workflow: `lake build` + `#print axioms` is the oracle an agent needs.

---

## 4. Proposed workflow, skill, and agent tool

### 4.1 `lean-derive` workflow (6 stages, each with a gate)

| # | Stage | Artifact | Gate (machine-checked) | Human review |
|---|---|---|---|---|
| 1 | **Spec**: pure Lean model of the hot function, translated from the existing C/C++ reference | `Model.lean` | Differential test: Lean model (`lake exe`) vs existing implementation on ≥10⁵ random + edge inputs | Does the model mean the same thing as production? |
| 2 | **Invariant**: `Inv`, boolean `checkInv`, `checkInv_sound`, preservation per mutation | `Invariant.lean` | `lean-check` (no escape hatches, std axioms only) | Is `Inv` actually true of production data? Measure it |
| 3 | **Plan**: fast path over *explicitly modeled* primitives + `Inv → plan = spec` | `Plan.lean` | `lean-check`; **theorem-statement lock** (below) | Are primitive models faithful to the real primitive (SQLite, Eigen, hardware)? |
| 4 | **Emit**: generate constants, tables, layouts, dispatch, bounds into a C/C++ template; keep hand-written control flow minimal and 1:1 with the Lean plan | `Emit.lean`, `templates/` | Golden-file `cmp`; compile with `-Wall -Werror`; sanitizers | Line-by-line C vs Lean plan correspondence (the trust gap) |
| 5 | **Guard**: runtime `checkInv` at load + incremental on writes, fallback to reference path, foreign-change detection | template | Differential test fast vs forced-fallback (as I did here), fuzzing, crash tests | Fallback trigger coverage |
| 6 | **Measure**: A/B/B/A runs, CPU/request, peak RSS, tail latency; attribute each win | `results/` | Benchmark script; record the baseline "best hand C without invariant" | Is the win from Lean or from runtime tuning? (§2) |

### 4.2 Agent tool: `lean-check` (shipped in this PR)

`Scripts/lean4-docker/` builds a pinned-toolchain image (the Docker Hub
`leanprovercommunity/lean4` tags were last updated 2024-08-20, so we build our own):

```bash
docker build -t lean4-check:4.35.0 Scripts/lean4-docker
docker run --rm -v "$PWD":/work -w /work/spec lean4-check:4.35.0 \
  lean-check --theorems "Feed.Db.feedPlan_eq Feed.Db.inv_addPost" \
             --emit "lake exe derive /tmp/out" \
             --generated /tmp/out/app_gen.c --golden ../gen/app_gen.c
```

It runs `lake build`, flags `sorry/admit/native_decide/decide +native/implemented_by/extern/
unsafe/axiom`, fails on any axiom beyond `propext, Classical.choice, Quot.sound` (the check
[VAL] recommends), and does the golden `cmp`. Verified on [S] with elan directly (Docker image
not built: no Docker on the box).

Next increments (not built yet):
- **Theorem-statement lock:** hash each approved theorem's *statement* (via `#check`
  pretty-print) into `theorems.lock`; the agent may change proofs freely but any statement diff
  fails CI until a human re-approves. This is the cheapest guard against an agent "proving"
  a weakened theorem (cf. agents "cheating" in [B-we-rewrote-tinygrad-in-lean]).
- JSON output for agent consumption; a `differential` subcommand (fast vs fallback, as in §0).
- MCP wrapper exposing `lean_check`, `lean_differential`, `lean_bench`.

### 4.3 Where an LLM agent helps vs. must not be trusted

| Agent can own (machine-checked or cheap to verify) | Agent must not be trusted on |
|---|---|
| Writing and repairing proofs (the kernel checks them) | **Theorem statements** and the spec model: proving the wrong thing is invisible to the kernel |
| Translating reference code into a Lean model, given a differential harness | **Primitive semantics** (e.g. `rangeCounts` = filter): unproved assumptions about real libraries |
| Emitter boilerplate, golden files, C escaping helpers | **Hand-written C fast paths** and their correspondence to Lean plans |
| Differential tests, fuzzers, benchmark harnesses | **Fallback triggers** and concurrency (checkpoint threads, foreign writers) |
| Proposing candidate invariants and plans | **Performance and memory claims** (require A/B/B/A runs; cf. [MET]'s honest +2.4 %) |
| Bisecting regressions | **READMEs**: §1.3 lists three doc/code drifts in [S] |

### 4.4 Pilot (2 weeks, one engineer + agent)

Candidate: a discrete hot path in the spacecraft simulation stack where an invariant is
plausible, e.g. **ephemeris/interpolation segment lookup on a monotone time grid** or a
**telemetry/event ring buffer with bounded records**. (I have not seen the True Anomaly codebase;
pick whichever is actually hot in a profile.)

- Days 1-2: profile, pick the function, write `Model.lean`, differential harness vs current C++.
- Days 3-5: `Inv`, checker soundness, preservation, plan + equivalence proof (agent-heavy).
- Days 6-7: emit tables/constants into a C++ template; runtime guard + fallback.
- Days 8-10: differential (≥10⁶ cases, incl. forced fallback), sanitizers, A/B/B/A benchmark
  against (a) current code and (b) the best hand-written fast path *without* Lean.

**Success criteria:** std axioms only; 0 differential mismatches; fast path ≥1.5x CPU on the
target function **vs. (b)**, or a memory bound turned into a theorem that removes a runtime check
or allocation; hand-written C in the trust gap ≤ ~150 lines and reviewed line by line against the
plan; total Lean ≤ ~800 lines (hargup's spec is ~940 lines including the emitter).

### 4.5 Go / no-go

- **GO (conditional), narrow pilot:** the *invariant → proof → guarded fast path → differential
  test* discipline is sound, cheap with agents doing proofs, and builds reproducibly (3 s
  `lake build`). It is most valuable where a shortcut is correct only under a data invariant, and
  that's where it gave this submission its one real Lean-attributable win (~2x feed CPU).
- **NO-GO** as a general strategy for making code faster or leaner: in [S] and both fork
  branches, speed and memory come from systems engineering; the typed-codegen push in [MET]
  bought +2.4 % throughput. **NO-GO** for floating-point integrator numerics (use differential
  tolerance testing). **NO-GO** on any claim that "Lean-generated" means "verified C": the proofs
  stop at the template boundary.
- **Revisit** if a verified Lean → C path for the fast-path bodies becomes practical (e.g.
  generating the plan's control flow from the same Lean definition the theorem is about), which
  would close gap 1.3(1). For Rust code, look at Aeneas/hax [AEN][HAX] instead.
