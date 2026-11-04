# Ready-to-use mathematics container

As a user or agent, I want a non-root Lean container with mathlib already available,
so the first proof check works offline without installation or compilation of dependencies.

Acceptance: pinned compatible Lean/mathlib and archive/manifest hashes; no startup downloads;
successful mathlib proof, rejected false proof, Lean program, arbitrary UID, docker run/exec,
shell and actual interactive diagnostics; source profile explicitly separate; real size/time/RAM results.
<!-- pdd-story-prompts: ["prompts/container_shell.prompt"] -->
