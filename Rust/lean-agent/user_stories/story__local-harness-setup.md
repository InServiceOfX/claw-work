# Local Lean access from every harness

As a user of Claude Code, Codex, Grok Build, Hermes and Pi, I want the same
local Lean tools and discoverable instructions in each harness, so I can draft,
check and review proofs without duplicating setup or silently uploading input.

- An explicit build prepares pinned Lean/mathlib and a separate public-source image.
- Previewing installation changes nothing. Applying it preserves other tools,
  credentials, provider policies and user-edited skills; conflicts fail closed.
- Both installed skills can resolve their references away from the source checkout.
- A valid independent obligation passes; false, admitted or changed statements do not.
- Offline proof checking never chooses a model or claims arbitrary source verification.
- Native loader discovery and a paid/remote agent conversation are different evidence.

<!-- pdd-story-prompts: ["prompts/setup_python.prompt", "prompts/workflow_markdown.prompt"] -->
