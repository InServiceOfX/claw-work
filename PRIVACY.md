# Public boundary

This repository is intended to be public and shareable.

## Allowed

Source, portable configuration, authored synthetic fixtures, public upstream pins,
architecture and instructions that describe behavior rather than private data.

## Never publish

Credentials, tokens, private keys, private source or business models, personal paths
or identifiers, private reports/diagnostics, financial/account data, copied books,
agent transcripts, caches or provider configuration. This applies to Git history,
commit messages, image layers/history/labels, build contexts, CI logs and artifacts.

Use environment variables and portable placeholders, not personal absolute paths.
Do not copy another repository's history or private deployment configuration.
Do not assume untracked files or ignored files are safe to include in a Docker context.
.gitignore is convenience, not access control.

## Independent gates

1. Scan the complete Git index (not only working-tree copies), then inspect the staged
   diff and publication text. The local hook is opt-in; CI also runs this scan.
2. Audit the exact allowlisted Docker context and exported image/layers separately.
   Synthetic canary tests must verify that excluded private inputs cannot be copied.

The checks reject known secret patterns, personal absolute paths and forbidden
artifact paths. They cannot recognize every possible private fact. Human review
remains necessary; this is not a claim of perfect data-loss prevention.

## Runtime review boundary

Keep review source, proof models, manifests, reports and logs in an explicitly
selected private workspace outside Git trees. Inputs are read-only; no host project
build scripts are executed. Checker containers default to no network, non-root,
read-only rootfs, dropped capabilities, no Docker socket or credential/home mount,
bounded scratch space and deadlines.

Docker shares the host kernel and is not a complete sandbox for hostile code.
An external AI harness's model traffic is separate from container networking:
obtain approval before sending private code, models or diagnostics to a remote model.
No toolkit command silently chooses an AI provider or uploads input.

Do not publish a report, push a branch, publish an image, or install a global skill
merely because a local review succeeded.
