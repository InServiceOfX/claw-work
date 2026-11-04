# Local Docker and five-harness setup

## Requirements and boundaries

- macOS or Linux, Python 3.9+, Rust/Cargo and a **local Unix-socket Docker daemon**.
- At least 16 GiB Docker memory is recommended for building both profiles concurrently;
  allow tens of GiB of image/build-cache storage and time for the public downloads.
- Stable profile: Lean 4.35.0 + pinned mathlib, native linux/arm64 or linux/amd64.
  Separate checksums come from the official upstream release assets. The mathlib
  revision and dependency manifest remain identical. The entire mathlib cache is
  bundled, but default smoke checks use a narrow mathlib import. Umbrella import
  stress testing is opt-in with LEAN_AGENT_FULL_IMPORT=1 and may need more RAM.
  The setup command chooses the
  host-native platform by default; use --platform explicitly when cross-building.
- Source profile: an explicitly reviewed **public, clean** Lean checkout, archived
  at a chosen commit. It is core-only, not a replacement for stable mathlib.
- Build time needs network access for public dependencies. Check time never pulls,
  installs a toolchain, selects an AI model or uploads to a provider.
- Custom harness homes/profiles are not automatically inferred. The installer
  targets the standard locations under --home; configure nonstandard profiles
  deliberately rather than assuming they share the default skill library.

## Build and validate

From Rust/lean-agent:

```sh
cargo build --release --locked
python3 scripts/setup.py build-image
# Optional, separate newer Lean development profile:
python3 scripts/setup.py build-image --source /path/to/public/lean4 --jobs 4
python3 scripts/setup.py smoke
```

Source setup records selected revision, archive SHA-256, platform and tag on stdout.
It passes only a git-archive export as the named source context, never .git,
untracked files or local modifications. Review that upstream revision before use.
The main context is the exact .dockerignore allowlist; skills/tools/reports are not
copied into the images. No images are pushed. Source-profile base/package downloads
and current build dependencies are not a bit-for-bit reproducibility guarantee.

Test the source tag printed by build-image separately:

```sh
bash tests/source_smoke.sh claw-work/lean-agent:source-REVISION10
```

macOS Docker Desktop may need to be started manually. Cross-architecture emulation
is optional and slower; it is not the recommended Apple Silicon deployment path.

## Preview, install, verify

The user must authorize local installation. Build the release binary first:

```sh
python3 scripts/setup.py install          # no writes
python3 scripts/setup.py install --apply  # explicitly authorized local writes
claw-lean-setup status
claw-lean doctor
claw-lean-container version
```

Add ~/.local/bin to PATH yourself if it is not already present. No shell startup
file, harness configuration, API credential, model policy, MCP registration,
permission bypass or Git hook is modified.

Installed tools are symlinks to this checkout's release binary and scripts. Keep
the checkout in place; rebuild the binary after updating Rust source. Installed
skills are self-contained copies with bundled references, not fragile links back
through the repository's layout. An identical rerun is a no-op. Any modified/foreign
skill or conflicting tool is an error before any payload is written. Review and
move aside an old managed copy yourself before a deliberate upgrade; there is no
force-overwrite option. Failure rolls back this run's newly created payloads
(parent directories may remain). Unrelated tools/configuration remain untouched.

Use --home with an existing canonical test home to preview/test isolated installs.
This intentionally does not imply that the actual clients use that alternate home.

## Harness discovery

| Harness | Installed skill root | Use |
|---|---|---|
| Claude Code | ~/.claude/skills | /lean-proof or /lean-review, or natural-language selection |
| Codex | ~/.agents/skills | $lean-proof or $lean-review, or natural-language selection |
| Grok Build | ~/.agents/skills | /lean-proof or /lean-review; inspect with grok inspect |
| Hermes Agent | ~/.hermes/skills | skills_list then skill_view; ask for lean-proof / lean-review |
| Pi | ~/.agents/skills | /skill:lean-proof or /skill:lean-review |

Restart clients after installation; Pi can also /reload. Existing sessions may
retain an older skill inventory. Native discovery is affected by user/profile
configuration and project trust; file placement alone is not an end-to-end agent test.
Inspectors/loaders can validate discovery without spending model tokens. No paid
agent conversation or remote-model proof session is required by setup.

Example request in any harness:

> Use lean-proof to prove that adding zero preserves a natural number. Keep the
> independent statement fixed, use the local container, and report the strict
> checker result separately from raw elaboration.

For selected code:

> Use lean-review on this file. First identify independent intended properties,
> keep review artifacts outside Git, and do not claim a model proof verifies source.

## Commands and evidence levels

- claw-lean inventory INPUT: bounded JSON source inventory, no code execution.
- claw-lean prepare INPUT --output NEW_PRIVATE_DIR: private read-only evidence snapshot.
- claw-lean-container --input-dir PRIVATE_DIR check Solution.lean: bounded raw feedback.
- claw-lean-container --input-dir PRIVATE_DIR run Program.lean: untrusted Lean program.
- claw-lean-container --input-dir PRIVATE_DIR --timeout-seconds 3600 server:
  framed stdin/stdout LSP, no TTY; document URIs under /input.
- claw-lean check --manifest MANIFEST --output NEW_REPORT_DIR: independent strict
  model gate; exit 0 checked, 1 rejected, 2 unknown/error.
- claw-lean-setup smoke: checks all three public fixtures into temporary private
  directories and removes only its own temporary output.

Raw feedback mounts the explicitly selected directory read-only; select a narrow
proof directory, not a checkout/home. It exposes no arbitrary Docker flags. It has
a wall-clock deadline and resource limits, but stdout/stderr are streamed; the
calling harness should also bound captured output. The stable-image interface is
not a source-profile wrapper. For core-only source experiments use the constrained
Docker invocation in the README/source smoke test.

On macOS, /tmp and /var are system symlinks. The Rust input policy rejects symlink
components intentionally. Obtain a canonical parent before choosing a fresh output:

```sh
python3 -c 'from pathlib import Path; import tempfile; print(Path(tempfile.gettempdir()).resolve())'
```

Never weaken path checks just to use an alias. Tests canonicalize the OS temporary
parent without following symlinks in selected review input.

A proof draft loop defaults to five attempts at 120 seconds each, then reports the
remaining goals. The skills prescribe this budget to the harness; there is no
autonomous model daemon or hidden provider. Challenge specification review and
source/model correspondence remain explicit independent obligations.
