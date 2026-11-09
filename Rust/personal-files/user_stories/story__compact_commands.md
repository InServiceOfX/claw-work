<!-- pdd-story-prompts: prompts/commands_rust.prompt, prompts/ui_rust.prompt -->
<!-- pdd-story-dev-units: prompts/commands_rust.prompt, prompts/ui_rust.prompt -->

# Run short commands in the displayed directory

As a Personal Files user, I want a tiny command area at the bottom that I can
show or hide so I can run one or two short shell commands in the folder I am
browsing without opening a separate terminal.

Source: the user's 2026-11-08 request. This story is independent of current code.
Status: approved by the user on 2026-11-08 ("approve please").

## Acceptance

1. An obvious toggle MUST show and hide the bottom command area. Its initial
   layout SHOULD occupy about three text lines, with input and compact,
   scrollable output while keeping the file listing useful.
2. Enter or Run MUST execute an explicitly entered shell command in the
   displayed folder of the active tab at submission time. Spaces and Unicode
   in folder names MUST work without interpolating the folder into shell code.
3. Output, errors and completion/exit status MUST be visible. Ordinary shell
   syntax, such as pipelines and two commands joined with &&, MUST work.
4. Execution MUST NOT freeze navigation. Navigating, switching tabs, showing
   or hiding the panel MUST NOT execute or repeat a command. A running command
   MUST retain its originally submitted working directory.
5. Command input MUST retain normal editing/copy behavior. A command changing
   files MUST eventually be reflected in the browser's directory listing.
6. Command history and output MUST stay local and MUST NOT be written into
   repository artifacts. Tests MUST use disposable commands and directories.

This compact command runner does not promise a full interactive terminal for
programs such as editors or password prompts. That distinction is part of the
approved scope.
