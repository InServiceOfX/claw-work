# Preserve useful recovered work

Source: Ernest's 2026-11-02 approval, “do that please”, of the proposed selective
port of unreadable-preferences protection, tab-session restoration, independent
tab sorting, staged packaging and corresponding regression tests. The newer
native tabs, window tiling and exclusive installation remain the current app.

As a Personal Files user, I want useful earlier improvements retained when code
is consolidated so saved preferences, folder tabs and working builds survive.

## Acceptance
1. An unreadable existing preferences file is never overwritten with defaults,
   including by navigation, sorting, favoriting or direct preference saves. An
   actionable save error remains visible; successful reload enables saving again.
2. Reopening restores tab folders/order, active tab and independent sort choices.
   A supplied startup PATH starts one tab; invalid or missing saved paths do not
   discard other tabs/favorites. Old preference formats remain readable.
3. Failed navigation retains the saved successful folder. Restore/switch actions
   do not inflate visits. Source documents remain untouched.
4. A cached app remains unchanged after build, copy, signing or verification
   failure. Publication failure restores the previous app, or retains and reports
   it if a concurrent app prevents safe restoration. Installed apps are never
   silently overwritten. Paths with spaces work.
5. Newer native tab controls, one-worker stale-result protection and native
   window tiling continue to pass their existing accumulated acceptance tests.

<!-- pdd-story-prompts: prompts/core_rust.prompt, prompts/model_rust.prompt, prompts/build_bash.prompt -->
