# Personal Files macOS relocation — 2026-11-14

Canonical repository: claw-work. Project: `Rust/personal-files-macos/`.
Ubuntu remains unchanged at `Rust/personal-files/`.

## Provenance and preservation

Transferred all tracked files from the Monoclaw project at commit `dd6a868`,
including source, locked dependencies, scripts, PDD prompts/architecture, user
stories and acceptance tests. SHA-256 comparisons verified every copied file
before the documentation adjustments. No private repository history was
imported. Removed the personal absolute path in the original port request;
updated canonical-location and sibling-project guidance. Other historical
verification receipts retain their dates and describe earlier locations.

No runtime Rust, scripts, public interfaces, dependencies or bundle identity
changed as part of relocation. Preserve `io.monoclaw.PersonalFiles.macOS` so
existing macOS registration, saved credentials and local preferences remain
compatible. Preferences, user documents, models, provider state, generated
outputs and build caches are outside the source transfer.

## Workflow

The requested change is repository placement and rebuild, not new product
behavior or regeneration. PDD read-only plan `let-s-move-perosnal-files-macos-out-of-monoclaw--37a8e384`
was invoked at the source scope; no provider-backed apply/sync or semantic checks are claimed. The
existing prompt/story/test source is carried forward unchanged.

Run the repository public-index privacy gate and review all transferred text
before the local migration commit. No push or publication is authorized.

## Verification

Relocated build/test and installed-app evidence is recorded in PROGRESS.md and
docs/VERIFICATION.md. Earlier Monoclaw evidence is historical, not a substitute
for verification from this checkout.

The full relocated verify.sh run passed, including 31 deterministic tests and
three opt-in native test groups. A forced package release recompile from the
new checkout passed its native New Window fixture and was installed with verified
signature/executable byte equality. Application name resolution and Spotlight
metadata continued to select the installed bundle. The prior app is recoverable.

The whole-index privacy gate also passed. Eight unchanged existing Ubuntu PNG
icons needed exact review hashes added to reviewed-binaries.json; this migration
reviewed those small folder/star assets and added their metadata entries only.
