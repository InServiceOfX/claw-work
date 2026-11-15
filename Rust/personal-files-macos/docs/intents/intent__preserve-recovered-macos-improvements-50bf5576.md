# Intent: Preserve recovered macOS improvements

<!-- pdd-intent-id: preserve-recovered-macos-improvements-50bf5576 -->
<!-- pdd-intent-sha256: 50bf557645d4f7a957c43edb884815fdc21e16aedce445adcfe5258a6f7b5ce1 -->

## Record

- Intent ID: `preserve-recovered-macos-improvements-50bf5576`
- Kind: `add`
- Supersedes: none
- Approval ID: `preserve-recovered-macos-improvements-50bf5576`
- Source kind: `inline`
- Source reference: `not applicable`
- Request SHA-256: `50bf557645d4f7a957c43edb884815fdc21e16aedce445adcfe5258a6f7b5ce1`
- Project scope: `subproject`
- Adoption scenario: `existing_pdd_subproject_change`

- Technology: `not stated`

## Original Request

> do that please
>
> Approved proposal: selectively port the recovered Personal Files macOS improvements onto the newer implementation: protect unreadable existing preferences from being overwritten, restore ordered tab sessions and the active tab after restart, preserve independent sorting for each tab, and stage/sign/verify the cached app before replacement. Preserve the newer native AppKit tabs, window tiling, one scan worker, stale-result protection, and exclusive no-overwrite installation. Bring the corresponding regression tests. Affected files are src/core.rs, src/model.rs, src/native.rs, and build.sh.

## Must Stay Unchanged

- None stated.

## Examples

- None stated.

## Candidate Product Areas

- Native AppKit browser
- Publish a complete staged bundle with Darwin renamex_np RENAME_EXCL.
- Single-worker navigation state
- Install current bundle per-user in ~/Applications/Personal Files.app.
- Private places and filesystem semantics
