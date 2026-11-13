# lean4-docker

A small image + script for the "Lean reference model → kernel-checked plan proof →
emitted C" workflow analysed in
[`docs/research/lean4-optimization/lean-derive-hargup-analysis.md`](../../docs/research/lean4-optimization/lean-derive-hargup-analysis.md).

- `Dockerfile`: Debian bookworm-slim, `build-essential`, elan, one pinned Lean toolchain
  (build arg `LEAN_TOOLCHAIN`, default `leanprover/lean4:v4.35.0`), non-root user `lean`,
  workdir `/work`.
- `check.sh` (installed as `lean-check`): runs `lake build`, greps for proof escape hatches
  (`sorry`, `admit`, `native_decide`, `decide +native`, `implemented_by`, `extern`, `unsafe`,
  `axiom`), runs `#print axioms` on named theorems and fails on anything beyond
  `propext / Classical.choice / Quot.sound`, optionally runs the emitter and `cmp`s the output
  against a checked-in golden file.

The `leanprovercommunity/lean4` Docker Hub image is stale (tags last updated 2024-08-20,
per `https://hub.docker.com/v2/repositories/leanprovercommunity/lean4/tags`), so we build our own.

## Build

```bash
cd Scripts/lean4-docker
docker build -t lean4-check:4.35.0 --build-arg LEAN_TOOLCHAIN=leanprover/lean4:v4.35.0 \
  --build-arg UID=$(id -u) --build-arg GID=$(id -g) .
```

Match `LEAN_TOOLCHAIN` to the project's `lean-toolchain` file. If they differ, elan will try
to download the project's toolchain at run time (needs network).

## Run

Proof-check and codegen-check hargup's submission (from the `submissions/lean-derive-hargup`
directory of the twelve-dollar-server-challenge repo):

```bash
cd submissions/lean-derive-hargup
docker run --rm -v "$PWD":/work -w /work/spec lean4-check:4.35.0 \
  lean-check \
    --theorems "Feed.Db.feedPlan_eq Feed.Db.postPlan_eq Feed.Db.feed_eq_tail \
                Feed.Db.checkInv_sound Feed.Db.inv_addPost Feed.Db.inv_like Feed.Db.rangeCounts_eq" \
    --emit "lake exe derive /tmp/out" \
    --generated /tmp/out/app_gen.c --golden ../gen/app_gen.c
```

Interactive shell:

```bash
docker run --rm -it -v "$PWD":/work lean4-check:4.35.0 bash
```

## Status

Docker is not installed on the bot box, so **the image itself has not been built there**.
The exact steps (elan-init with `--default-toolchain none`, `elan toolchain install
leanprover/lean4:v4.35.0`) and `check.sh` were run directly on the box on 2026-11-01 against
hargup's spec at upstream commit `cfa7003`: `lake build` OK (18 jobs), no escape hatches, all
7 theorems above depend only on the 3 standard axioms, regenerated `app_gen.c` MATCH
(sha256 `4c0fee6d…b70a`).
