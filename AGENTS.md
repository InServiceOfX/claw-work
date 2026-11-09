# Agent instructions

This is a public monorepo for agent-authored tools and experiments.

- Work on feature branches. Never commit/push to master/main or merge your own work.
- Preserve other agents' changes. Never bulk-stage untracked files.
- Read PRIVACY.md before writing examples, fixtures, documentation or reports.
- No secrets, private data, personal absolute paths, agent transcripts or private
  code/model/review artifacts in Git, images, CI logs or published output.
- Use synthetic/public examples. Keep private review workspaces outside Git trees.
- A passing secret scanner is not a complete privacy review.
- Install the optional local hook explicitly with `git config core.hooksPath .githooks`.
  Do not change another user's Git configuration automatically.
- Before publication run the repository privacy gate against the entire index,
  inspect staged diffs, and audit Docker contexts and image exports separately.
- Do not push, publish images or install global skills without authorization.

## Local agents and GitHub identity

Read [BOTS.md](BOTS.md) for the scoped identity policy. Only Cursor bots and
Grok Bot/Grok Bits automation must use `yeung-assistant`. Human-directed local
Codex, Claude Code, Grok Build, Hermes-Agent, Pi and other local harnesses use
the human's configured Git/GitHub identity; they do not need that bot account
or a separate identity exception for an already-authorized commit/push.
Privacy, feature-branch and human-merge requirements still apply to everyone.

## PDD

Use Prompt-Driven Development for explicitly prompt-owned parts. Ordinary requests
are valid intent input. The human owns meaning/acceptance; agents own file mechanics.
Run `pdd --no-core-dump intent plan` in the exact subproject scope before mutation;
apply the approved meaning with its exact ID. Keep operational .pdd state untracked.
Do not commit raw requests containing private paths. Maintain versioned prompts,
independent stories and accumulated positive/negative tests before implementation.
Report PDD command failures and actual test evidence separately.

The Lean toolkit is scoped to Rust/lean-agent/. Its .pddrc and architecture.json
own that product. The root privacy story and gate are deliberate integration work;
do not regenerate unrelated monorepo projects.
