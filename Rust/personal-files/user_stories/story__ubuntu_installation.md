<!-- pdd-story-prompts: prompts/build_bash.prompt, prompts/install_bash.prompt -->
<!-- pdd-story-dev-units: prompts/build_bash.prompt, prompts/install_bash.prompt -->

# Find the current local Ubuntu application

As an Ubuntu user, I want Personal Files rebuilt from this repository and
installed as a desktop application so I can open it from Applications and
find it after pressing Super and searching for Personal Files.

Source: the user's 2026-11-08 request. Personal machine paths are omitted.
Status: approved by the user on 2026-11-08 ("approve please").

## Acceptance

1. Build the native Ubuntu release from this subproject's current sources;
   the installed executable MUST match that build and work independently of
   the source checkout. Build output MUST remain outside the repository.
2. The current user's Applications registry MUST contain a visible Personal
   Files entry with its icon and an executable target. Searching for Personal
   Files MUST return that entry. Launching it MUST open the native browser.
3. Installation MUST preserve user preferences and permit already-running
   windows to finish normally. An existing window may need an ordinary restart.
4. Verification MUST distinguish desktop registry/search evidence from an
   actual Super-key interaction, and MUST NOT claim an unperformed UI check.
