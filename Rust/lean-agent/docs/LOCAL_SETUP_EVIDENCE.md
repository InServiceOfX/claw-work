# Local five-harness deployment evidence

This supplements, rather than replaces, the earlier Linux x86-64 results in
../VERIFICATION.md. Current environment: Apple Silicon macOS, Docker Desktop
Linux ARM64 VM (about 16 GiB RAM, 14 CPUs), Rust 1.94.0, Python 3.9.6.
All inputs used here were public upstream source or authored synthetic fixtures.
No private source/model/report data was published or placed in a Docker build.

## Images

- Stable `claw-work/lean-agent:lean4.35.0-mathlib`: native ARM64, Lean 4.35.0
  and pinned mathlib. Image ID
  `sha256:afe4501f07b1b75084abc50a56b1cae287672e0e6a337d8dd28a1427af5217dc`.
  Docker image inspect Size: 2,939,484,701 bytes; image ls reports 13.4 GB disk usage.
  Built and accepted locally, not published.
- Separate source profile: `claw-work/lean-agent:source-c029d915d1`.
  Image ID `sha256:dc5140417d7709fcdfcc939bbb8ba042412bdef2e10c97807ab76ac2fc47a984`;
  Docker image inspect Size: 1,108,701,857 bytes; image ls reports 6.37 GB disk usage.
  Native ARM64. These different Docker storage views must not be conflated.
  Public upstream revision `c029d915d12bd1a1199881cefcae07f00ec9f568`;
  git-archive SHA-256 `cbeb7696e203fadd303a9fbb76734f78631f53b8cdd973b6a0104bc970c30b3e`.
  Built with six jobs. Reports Lean 4.37.0-pre, Lake 5.0.0-src and no embedded
  commit ID. Revision/hash are external build-input evidence, not binary attestation.
  Source smoke passed: version/Lake, valid Std proof, false theorem rejection,
  program execution, arbitrary non-root UID, read-only rootfs and no networking.

The ARM64 stable release checksum was obtained from the official GitHub v4.35.0
release asset digest, and is verified during build. The compatible mathlib revision
and dependency manifest are unchanged. Both platform metadata profiles are checked
exactly by the Rust checker; unknown platforms, drift and extra fields are rejected.
The full mathlib cache is retained. Default acceptance imports a representative
narrow module. Umbrella `import Mathlib` did not complete promptly in this VM and
was interrupted; it is not claimed to fit the default worker budget. Optional
umbrella stress testing remains separate from ordinary smoke acceptance.

## Deterministic tests

- Rust formatting, locked all-target tests and Clippy with warnings denied passed.
  Twelve ordinary Rust tests passed; nine heavy Docker tests are opt-in.
- Fifteen Python deployment/policy tests passed. They cover no-write previews,
  identical reruns, self-contained references, modified/foreign/dangling-symlink
  conflicts, symlink parents, rollback, unrelated configuration preservation,
  fixed Docker restrictions, local-only endpoints, LSP transport construction,
  deadline cleanup and public git-archive context selection/cleanup.
- macOS test fixtures canonicalize only the OS temporary parent. Untrusted
  review paths still reject symlinks; no production input guard was relaxed.
- Strict deterministic PDD lint: six subproject prompts, no warnings/errors.
  This is structural lint, not semantic story validation.
- Actual Docker context canary passed for the exact seven-file allowlist.
- Actual built-image archive/config/layer canary passed on the native ARM64 image,
  including compressed layers; this is defense in depth, not a universal privacy proof.
- Nine live Rust Docker tests passed: independent proof acceptance, changed or missing
  statements/definitions, admissions, unexpected/native-oracle axioms, deadline/output
  bounds, SIGTERM cleanup and diagnostics/edit/goal LSP feedback. The native-oracle
  regression additionally checks that the independent judge was actually reached.
- Six live Python launcher tests passed: version, narrow mathlib proof feedback,
  false theorem rejection, requested program execution, internal timeout and actual
  unpolluted framed LSP initialization/TERM cleanup.
- Stable container smoke passed: representative mathlib proof, false statement,
  program, arbitrary non-root UID, shell status/argument forwarding and persistent
  docker exec. Separate source-profile smoke passed as described above.
- All six named model obligations in identity/summation/withdrawal passed the strict
  checker. Source correspondence remains unproved in every report.

An early test poll saw the image tag before Docker had finished unpacking it and
metadata preflight safely returned an error. After the explicit build completed,
doctor and the full acceptance run above passed; no error was promoted to success.

## Descriptive native measurements

Five samples, 2 CPU / 4096 MiB worker limits, warm/uncontrolled host page cache,
no cache reset or cold-boot claim. The narrow synthetic addition/omega fixture
measures raw Lean, not the strict export/comparator pipeline.

| Measurement | Observed result |
|---|---:|
| New container + Lean/Lake version, median client wall | 180 ms |
| New container + raw proof, median client wall | 463 ms |
| Persistent-container exec + raw proof, median client wall | 389 ms |
| Fresh raw Lean process wall | 0.32–0.33 s |
| Fresh raw Lean CPU, median user / system | 0.21 / 0.11 s |
| Fresh raw Lean peak RSS | about 1.01 GiB |
| Persistent idle cgroup sample | 868 KiB, 0.00% CPU |

The metrics output now records actual host OS/architecture and image platform,
rather than hard-coding the earlier Linux x86-64 environment. Docker Desktop VM,
host/daemon overhead, shared mappings and build caches are separate costs. These
samples do not establish optimality, a universal speedup or a correctness proof.

## Local installation and discovery

Three shared commands are installed under the standard local bin directory:
`claw-lean`, `claw-lean-container`, `claw-lean-setup`. Two self-contained skill
bundles, `lean-proof` and `lean-review`, are copied into each required skill root.
No existing user configuration, credentials, provider policy, permission bypass,
MCP server or Git hook was changed. Installer status reports all entries unchanged
on an identical rerun.

- Codex: local app-server `skills/list` returned both skills with user scope and
  enabled=true. No thread/model turn was started.
- Grok Build: `grok inspect --json` discovered both from the shared user skill root.
- Hermes: `hermes skills list` reported both as local and enabled.
- Pi: the installed native skill parser loaded both bundles from the shared skill
  root with no Lean-skill diagnostics. This is parser validation, not a live model
  session or a complete custom-profile startup test.
- Claude Code: both bundles and references installed in its standard user skill
  root, frontmatter/reference validation passed. No native inspector or model
  conversation was used to claim end-to-end loader certification.

Restart/reload existing harness sessions to refresh their skill inventory. Discovery
and successful deterministic tools do not prove that an AI model follows instructions.
No paid/remote proof conversation, automatic provider selection or upload was performed.

## Operational limits and publication

The PDD planner ran in the exact subproject scope. Apply recorded accepted intent
and updated Product Intent, but its model-backed incremental architecture operation
was interrupted at the 30-second execution bound. It did not complete automated
prompt generation, sync, story contracts or semantic validation. Prompts, an
independent authored story, architecture mappings and accumulated tests were
maintained by the invoking coding harness. No successful automated PDD sync is claimed.

Docker images are local only. Build caches/intermediates and optional high-memory
imports are not included in runtime size figures. No global Docker prune occurred.
A prospective complete-index privacy scan passed using a separate temporary
GIT_INDEX_FILE, and the real index was left unstaged during that validation run.
The initial implementation/validation run did not commit, push, publish images or
execute remote CI. Any subsequent source commit is a separate, explicitly authorized
action; it does not imply permission to push or publish images. Human publication
review is still required. Model proofs, source correspondence and specification
review remain separate.
