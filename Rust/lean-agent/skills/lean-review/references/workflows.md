# Private review runbook

1. Confirm which inputs and which properties the human selected. Obtain consent
   before private data reaches the harness's remote model. Offline Docker only
   controls container networking, not the surrounding AI provider.
2. Run `claw-lean inventory INPUT`. It inventories bounded text without executing
   input hooks/build scripts. Review skipped files and truncation before proceeding.
3. Run `claw-lean prepare INPUT --output NEW_PRIVATE_WORKSPACE`. Keep outputs
   outside Git trees and use canonical paths, not symlink aliases.
   On macOS, /tmp and /var are aliases; obtain a canonical temp parent using
   `python3 -c 'import tempfile,pathlib; print(pathlib.Path(tempfile.gettempdir()).resolve())'`.
4. Record independently intended semantics, assumptions, boundaries and exclusions.
   Author independent challenge/solution Lean files and a schema-1 manifest. The
   adjacent manifest.json is a synthetic template, not an attestation for real input.
5. Final check: `claw-lean check --manifest MANIFEST --output NEW_REPORT_DIR`.
   Reports distinguish checked/rejected/unknown model obligations. Manual mappings
   are caller-supplied and unvalidated; compare current snapshot hashes yourself.
6. A source bug requires a concrete source reproducer, not just a failed proof.
   A patch requires authorization; never silently rewrite the reviewed checkout.
7. For an optimization, derive the model relation first, prove it, handle machine
   arithmetic/effects separately and benchmark actual representative release code.
8. State model results separately from source correspondence and specification
   review. No automatic arbitrary-source semantics or TLC-style exhaustive state
   explorer, temporal fairness/liveness checker or business-intent oracle is bundled.

Never mount home, credentials or the Docker socket; never run input lakefiles on
the host. Lean metaprograms are executable untrusted code. Raw feedback is not
independent proof acceptance. Docker shares the host kernel, so it is not a complete
sandbox for hostile inputs. Keep private source, models, manifests, diagnostics and
reports out of Git, images, transcripts and published output. No command uploads to
a model, selects a provider, pushes a branch or publishes an image.
