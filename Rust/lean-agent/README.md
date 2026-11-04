# Lean agent toolkit

A local, Rust-first mathematical review toolkit and **mathlib-enabled** Lean 4
container. It checks named model obligations against an independent challenge.
It does **not** automatically translate arbitrary programs or verify that source
implements a model.

- [Verification and measurements](VERIFICATION.md)
- [Review and specification-derived improvement workflows](docs/WORKFLOWS.md)
- [Security/trust boundaries](docs/SECURITY.md)
- [Portable agent skill](skills/lean-review/SKILL.md) and [harness notes](docs/ADAPTERS.md)
- [Story → prompt → test traceability](docs/TRACEABILITY.md)
- [PDD operational status](PROGRESS.md)

## Build once, check offline
Requirements: native Linux x86_64, local Unix-socket Docker daemon/BuildKit, Rust
(tested with 1.97.1), and substantial build/cache disk space. Other platforms are
not accepted by the initial release recipe. Image and Rust dependencies are fetched
explicitly at build time; review commands never choose an AI provider or auto-pull.

```sh
# From this directory, not the monorepo root.
docker build --platform=linux/amd64 -t claw-work/lean-agent:lean4.35.0-mathlib .
cargo build --release --locked
target/release/claw-lean doctor
target/release/claw-lean check --manifest examples/identity/manifest.json \
  --output /tmp/lean-identity-report-new
```

Choose a NEW private report path outside all Git trees. The examples are public
synthetic source; real inputs, models and reports must stay in private workspaces.
CLI check exit codes: **0 checked**, **1 rejected**, **2 unknown/error**. JSON and
Markdown retain named obligations, assumptions, actual model hashes, image identity
and per-phase diagnostics/status. Source mappings are caller-supplied, not validated.

### Selected source
```sh
target/release/claw-lean inventory /path/to/selected/input
target/release/claw-lean prepare /path/to/selected/input \
  --output /tmp/private-lean-review-new
```
Inventory accepts a clone, file or directory; it does not execute its Git/package/build
instructions. See exclusions/limits in the output. An agent/human then chooses intended
properties and creates the model, independent challenge, solution and manifest.
There is no hidden model-provider API, automatic repository rewrite or blanket badge.

## Direct Docker use

```sh
image=claw-work/lean-agent:lean4.35.0-mathlib
docker run --rm --network=none --read-only --cap-drop=ALL \
  --security-opt=no-new-privileges --user "$(id -u):$(id -g)" \
  --memory=4g --memory-swap=4g --cpus=2 --pids-limit=256 \
  --tmpfs /tmp:rw,noexec,nosuid,nodev,size=768m \
  --mount "type=bind,src=$PWD/examples/identity,dst=/input,readonly" \
  "$image" check /input/Solution.lean
```

`check` and `run` are raw Lean elaboration/program interfaces, not strict proof
certificates. The Rust `check` command adds independent statement, primitive,
transitive axiom and kernel checks. Never mount a private checkout into a Docker
BUILD context; runtime review input is narrowly selected read-only source.

Entrypoint commands: `version`, `metadata`, `check FILE`, `run FILE`, `shell`,
`idle`, `server`, and internal `export`/`judge`. For a persistent container, use
the same restrictions with `--name CHOSEN_NAME -d … "$image" idle`, then
`docker exec CHOSEN_NAME lean /input/Solution.lean`. Explicitly remove only that
container afterwards. Use `shell -c '…'` for shell commands. Temporary writes go to
`/tmp`, not the read-only work directory.

For interactive proof feedback, attach `docker run -i … "$image" server` using
JSON-RPC/LSP framing (no TTY). The tested session opens an invalid theorem, observes
errors, edits it to a valid proof and requests `$/lean/plainGoal`. A shell is not a
proof REPL. No Lean Beam/MCP daemon is required or bundled in this profile.

## Build the separate public Lean source profile

The supplied upstream source revision can be built without publishing its Git
directory or unrelated host files. Use an audited public Lean checkout:

```sh
source_context=$(mktemp -d)
git -C /path/to/public/lean4 rev-parse HEAD
git -C /path/to/public/lean4 archive HEAD | tar -x -C "$source_context"
docker build --platform=linux/amd64 -f Dockerfile.source \
  --build-context source="$source_context" --build-arg JOBS=4 \
  -t claw-work/lean-agent:source-local .
bash tests/source_smoke.sh claw-work/lean-agent:source-local
```

Record the selected revision and source archive hash externally. Git archives omit
Git metadata, so this core-only binary does not embed a commit ID. The tested supplied
revision is documented in VERIFICATION.md. It builds at image-build time, not startup,
uses GMP ≥6.3.0 with FORCE_GMP=OFF, and copies bin/lib rather than the CMake build tree.

This **4.37.0-pre core-only** profile does not claim compatibility with the default
**Lean 4.35.0/mathlib** cache or the Rust checker's stable-profile metadata contract.
Use it for direct core Lean checking/program execution. A future matching mathlib
profile needs its own pins and acceptance evidence.

## Tests

```sh
cargo fmt --check
cargo test --locked --all-targets
cargo clippy --locked --all-targets -- -D warnings
LEAN_AGENT_LIVE=1 cargo test --tests -- --ignored --test-threads=1
bash tests/container_smoke.sh
bash tests/context_canary.sh
bash tests/image_canary.sh
bash tests/source_smoke.sh
cargo run --release --example spec_derived
cargo run --release --example container_metrics
```

Live tests are opt-in; ordinary CI does not download/build large images. The image
canary test scans an actual saved image, including compressed intermediate layers,
not just the final filesystem. BuildKit/compile caches can exceed runtime image sizes;
no global Docker prune is performed.

## Repository publication gates
From the monorepo root, build/cache the Rust dependencies, then optionally enable
the repository-local hook with `git config --local core.hooksPath .githooks`.
Nothing installs it automatically. The hook uses locked OFFLINE compilation and scans
the COMPLETE index plus reviewed Docker context. CI scans the committed index and
runs synthetic Rust tests; no secrets or artifact uploads are configured. CI has been
authored, not remotely executed/published by this work.

A scan is defense in depth, not proof of absence of all private facts. Review the
staged diff, image surface and publication text manually. See ../../PRIVACY.md.
No push, image publication, global skill or global hook installation is implicit.
