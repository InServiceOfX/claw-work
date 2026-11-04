# Verification — 2026-11-03

Local results on native Linux x86_64, Docker 29.1.3, Rust 1.97.1.
No remote CI execution, push, image publication, or global skill/hook installation
is claimed. This is a working first implementation, not a source-verification engine.

## Accepted local images

| Profile | Image ID | Docker-reported size |
|---|---|---:|
| Lean 4.35.0 + default mathlib | `sha256:024c3a5ed698498b4ff0a3cd44e84c013e0e74132b1457c3fcbbc75044eddcfa` | 9,874,001,436 bytes |
| supplied source, core-only 4.37.0-pre | `sha256:6bd02377d166c509c614b90945a8e31ff67acd493edb9e4dd1e21dd8c69127de` | 4,917,496,280 bytes |

Default public pins are in docker/pins.json. Mathlib is pinned at
56a005855ea17e0b0958f73ec801ae140b0a1679.
The source profile was built from a clean public git archive at
8c6cec8d7a5dde27cc2c735044967f14791d279e, with four build jobs and no Git metadata
in the source context. Its binary reports 4.37.0-pre and an empty commit string;
the revision is external build-input evidence, not an embedded binary attestation.
The runtime copies bin/lib/licenses, not the stage1 CMake tree.

These are local tags/IDs, not published registry images. Sizes exclude BuildKit
caches/intermediates. APT repository state is not frozen; see the reproducibility
limits in docs/SECURITY.md.

## Passed

- `cargo fmt --check`, `cargo clippy --locked --all-targets -- -D warnings`.
- **11 ordinary Rust tests** across process bounds, compressed image-canary scanning,
  inventory/privacy/snapshot guards, staged-index/worktree disagreement, manifest
  validation and synthetic Rust source bug/repair/equivalence cases.
- **9 opt-in live Rust tests**: expected named proof acceptance; direct/transitive
  admissions; unexpected axiom; absent/changed obligations; fake success text;
  changed definitions despite reflexive theorem; native-oracle rejection; timeout/
  output-limit non-success; SIGTERM cleanup; actual LSP diagnostics/edit/goal feedback.
  Several tests contain multiple negative cases.
- Default container smoke: valid full-mathlib proof, false theorem rejection, program
  execution, arbitrary UID 12345, read-only/no-network execution, shell argument/status
  forwarding, persistent Docker exec with mathlib lookup.
- Core source build smoke: version, Lake, valid Std proof, false theorem rejection,
  program execution, arbitrary UID, no network/read-only rootfs.
- Checksum-pinned release's bundled GMP archive interrogated as **6.3.0** at build
  time; the system package version alone is not used as evidence for that static archive.
- Prospective public-change index passed the complete indexed-blob privacy scan using
  a separate private GIT_INDEX_FILE; the actual index was not staged or mutated.
- Actual Docker context canary: exact **six-file** context; a nested excluded canary
  and .env are not transferred. The test caught and fixed an initially broad unignore.
- Actual built-image save canary: configuration and saved layers scanned, including
  gzip/zstd-compressed blobs. Synthetic excluded values absent. This is not a general
  proof that every possible private fact is absent.
- Deterministic PDD strict contract lint: 5 subproject prompts + 4 stories, and the
  separate root privacy prompt/story, **zero warnings/errors**. Structural lint does
  not imply successful semantic story validation or PDD sync.

All live tests are opt-in; ordinary CI intentionally does not build/fetch large images.

## Checked mathematical examples

All use the image above and the independent primitive/statement/axiom/kernel gate.
Source correspondence remains **unproved** in every report. Source mappings are
caller declarations, not authenticated source-code refinement evidence.

| Example | Named model obligations | Challenge / solution / judge wall time |
|---|---:|---|
| identity | 1 | 2.027 / 2.208 / 1.251 s |
| withdrawal boundary bug + repaired contract | 3 | 5.307 / 5.187 / 0.836 s |
| recurrence-derived summation + scoped cost model | 2 | 5.396 / 5.784 / 1.846 s |

These single-run phase times exclude separately bounded preflight and report I/O.
The withdrawal example also has a concrete Rust equality-boundary reproducer and
repair tests. The summation example has executable Rust agreement tests, but no
mechanized Lean-to-compiled-Rust proof.

## Resource observations, not an optimality claim

Five descriptive samples each, 2 CPU/4 GiB limit, warm/uncontrolled host page cache,
no cache flushing or cold-boot claim. Raw Lean elaboration uses the authored
`Mathlib.Data.Nat.Basic` addition/omega fixture; it is NOT the strict audit pipeline.

| Measurement | Result |
|---|---:|
| new container + Lean/Lake version, median wall | 282 ms |
| new container + raw proof, median client wall | 1,088 ms |
| persistent-container exec + raw proof, median client wall | 888 ms |
| raw Lean process wall, fresh-container range | 0.85–0.90 s |
| raw Lean CPU, fresh-container median user / system | 0.49 / 0.36 s |
| raw Lean maximum RSS, observed samples | about 1.02 GiB |
| persistent idle cgroup sample | 472 KiB, 0.00% CPU |

RSS includes mapped/shared resident pages, not just private heap. Idle cgroup stats
exclude Docker daemon/host overhead. A persistent container is not a persistent
elaborator; each exec imports again. Interactive LSP behavior was tested separately;
its latency/memory has not been benchmarked. The default mathlib profile is **not**
tiny: approximately 9.87 GB stored image and approximately 1 GiB RSS for this fixture.
No claim of globally smallest, fastest, least-energy or production-optimal behavior
is supported by these measurements.

`cargo run --release --example spec_derived` measured the public 1,000,000-term
summation fixture at approximately 2.13 ms/call for the optimization-barrier-preserved
iterative implementation and approximately 1 ns/call for the closed formula in this
run. The latter is coarse short-operation timing, not a reliable universal speedup.
Different repetition counts amortize timer cost; LLVM may eliminate iterative work
without the explicit barrier. These numbers are not an application benchmark,
memory improvement claim, source-correctness proof or compiler verification.

Reproduce with `examples/container_metrics.rs`, `examples/spec_derived.rs` and the
commands in README.md. Full raw diagnostics/review artifacts belong outside Git.

## Remaining boundaries

- PDD intent apply recorded approval but automated architecture generation timed out;
  no completed auto-generation, sync or semantic acceptance is claimed. See PROGRESS.md.
- No automatic arbitrary-language extraction/refinement, comprehensive bug finder,
  full TLC-style state exploration, liveness/fairness engine or requirement-validity proof.
- No complete secret detector, signed supply-chain provenance, byte-for-byte rebuild
  guarantee, hardened multi-tenant sandbox, cross-platform performance certification,
  or live native-loader tests for every named agent harness.
- CI and local opt-in hook are authored. Publication review and human merge remain
  separate steps; do not bulk-stage private review directories or PDD/provider state.
