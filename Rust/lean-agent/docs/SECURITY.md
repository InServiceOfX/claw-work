# Security and proof trust

## Do not confuse three different claims
1. **Model proof:** the named Lean statements match the independent challenge and
   pass the selected image's comparison, dependency-axiom policy and kernel replay.
2. **Source correspondence:** the program implements that model. This toolkit
   currently reports **unproved**. Caller-provided mappings/hashes are retained but
   not validated by the proof checker; separately compare actual inventory snapshots.
3. **Specification validity:** the properties represent the intended requirements.
   Caller review attestation is recorded, not certified by Lean or the toolkit.

No status establishes correctness of arbitrary source, absence of all bugs, complete
requirements, application-level performance, or temporal/concurrency properties
that have not actually been modeled and proved.

## Strict pipeline
- Read bounded challenge/solution source into a fresh private workspace.
- Elaborate/export each independently in separate non-root, no-network, read-only
  rootfs containers. Only the respective source file is mounted. Do not execute host
  build files or reuse caller-created compiled imports. No credential/home/socket mount.
- Export the expected theorem dependency closure and a fixed upstream primitive/axiom
  set. Pinned Init is exported at image build time as a separate primitive baseline.
- A third container sees only NDJSON exports and checker-generated configuration.
  It first compares the challenge's primitives against the immutable Init export,
  then compares expected challenge/solution statements and definitions, audits the
  solution's transitive axioms, and replays declarations with Lean's default kernel.
- Only propext, Classical.choice and Quot.sound are permitted. An admission or
  unapproved native/compiler oracle in a selected dependency closure cannot pass.
  Unrelated unused declarations are outside that obligation scope.
- Source-generated output/zero exits/success markers do not authorize acceptance.
  Parse/export/comparison failure never means checked. A failed tactic does not prove
  a source bug. Timeouts/output limits/runtime failures are unknown, not proof successes.

Upstream comparator's `--inadvisably-no-sandbox` disables its **nested** bubblewrap
layer only. It is run inside the separately constrained export-only Docker container;
this flag is NOT a general claim that disabling sandboxing is safe for source builds.
No caller .olean is loaded by the judge. The primitive guard prevents equal forged
primitive definitions in two caller exports from becoming their own baseline.

## Runtime limits and residual risk
Default checking: 2 CPUs, 4 GiB RAM with no extra swap, 256 processes, 768 MiB tmpfs,
64 MiB per output stream, 120 seconds per export/judge phase. Preflight is separately
bounded. Host Rust memory also retains bounded exports. SIGINT/SIGTERM triggers own-
container cleanup and a non-passing result. A timeout inside each Docker run provides
a backstop if the host client is killed; Docker auto-removal is enabled. CLI signal
handling is explicitly installed by the check command, not automatically by embedding
the Rust library. SIGKILL, daemon/host failure, Docker bugs or
kernel compromise cannot guarantee immediate cleanup. Never run a global prune as cleanup.

Docker shares the host kernel; this is not a hardened hostile multi-tenant service.
Treat the Docker daemon/context, OS, selected immutable image, pinned Lean/toolchain,
exporter/comparator/kernel and dependency supply chain as trusted. A local Unix socket
can itself be forwarded elsewhere; the operator must select a genuinely local daemon.
For higher-risk inputs use an independently hardened VM/worker.

The challenge's MEANING and abstraction boundary remain human review obligations,
even after its primitive definitions are anchored to the pinned image. No machine can
infer that a conveniently weakened/vacuous model satisfies the intended business rule.

## Privacy and reproducibility
Outputs are fresh owner-only directories outside Git trees, with symlink and overwrite
checks. A Git-tree check is a useful guard, not proof a path is not cloud-synced or public.
Select storage policy yourself. Root publication scans read indexed BLOBS, not repaired
working copies; they fail on symlinks/submodules, binary/oversized/unreadable input and
known artifacts/secret patterns, but cannot detect every private identifier or fact.
Existing Git history, commit messages and external CI logs are not scanned by this
index gate; review those publication surfaces separately.

Docker context uses an exact six-file allowlist. Source builds use a separately selected
sanitized public git archive. Image canary tests cover config and saved layers, including
gzip/zstd compression. Canaries are synthetic; do not pass real secrets as CLI arguments.
The archive scanner is a canary test utility, not a full image-secret detector or an
untrusted-archive ingestion service; run it with the documented external time bound.

Ubuntu digest, Lean archive checksum and mathlib commit/manifest are pinned. OS APT
security packages and public precompiled mathlib cache transport are NOT a byte-for-byte
rebuild guarantee. The actual immutable image ID is recorded for execution; this is not
a signed provenance certificate. Matching self-reported pin metadata does not authenticate
an arbitrary image; build/review the recipe and select an approved image ID. The bundled
release GMP archive is version-probed as 6.3.0 at build time, separately from the system
package check; upstream release packaging remains trusted. Upstream artifacts and native extensions remain supply-
chain trust. The public source recipe is a separate version, not a cache-compatible alias.

No runtime command invokes a provider or uploads inputs. Your external agent/model
traffic is a DIFFERENT boundary: configure and approve it before processing private input.
Nothing here authorizes publication, bulk staging of review files, or global installation.
