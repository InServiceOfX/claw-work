# Tiling regression fix and tabs — 2026-10-30

Exact implementation request:
> Can you implement now because I'm not able to tile for some reason with Personal Files application

Exact added scope:
> Also, as a user of Personal Files I want to be able to open up tabs in this Mac OS version of Personal Files just like any other typical Mac OS application, with Command T

Tabs read-only PDD plan: also-as-a-user-of-personal-files-i-want-to-be-ab-7e6daf61.
Existing tiling story is the acceptance oracle. New independent tabs story is
user_stories/story__macos_tabs.md. Affected outputs: native.rs and model.rs.
Baseline: 19 deterministic tests passed. Added a native regression that fails
because NSApplication.windowsMenu is nil. Existing minimum content size was
820x440, unnecessarily restricting layouts on narrower/shorter screens.
No provider-backed PDD bug/fix/apply/sync is claimed; their help was checked,
but generation would delegate to an unauthorized additional provider.
Source, independent regression and implementation are maintained here.
