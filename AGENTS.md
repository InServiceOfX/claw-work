# Agent instructions

This is a public monorepo for agent-authored tools and experiments.

- Cursor bots and hosted Grok Bot/Grok Bits automation work on feature branches. They do not commit or push to master/main or merge their own work. Human-directed local harnesses follow BOTS.md.
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

Read [BOTS.md](BOTS.md). Only Cursor bots and hosted Grok Bot/Grok Bits
automation must use `yeung-assistant`, feature branches, and pull requests.
Human-directed local harnesses, including Codex, Claude Code, Grok Build,
Hermes-Agent, and Pi, use the human's configured Git/GitHub identity. They
do not need that bot account or a mandatory pull request. When the human
explicitly asks, they may merge locally and ordinary non-force push to
`master`. Privacy, verification, and preservation still apply.

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
