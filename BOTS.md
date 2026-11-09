# Rules for bots

## GitHub identity and workflow scope

The `yeung-assistant` account, mandatory feature-branch, and pull-request
rules below apply **only to Cursor bots and hosted Grok Bot/Grok Bits
automation**, until the repository owner explicitly changes this scope.
Those runs act as `yeung-assistant`, not as the human's personal account.
They use `bots/<topic>` branches and pull requests, and they do not push to
`master`/`main` or merge their own pull requests.

Human-directed local harnesses — including Codex, Claude Code, Grok Build,
Hermes-Agent, Pi, and other local AI tooling — use the human's configured
Git/GitHub identity. They do not need `yeung-assistant` or a mandatory
pull-request workflow, and they must not switch accounts merely because they
are AI agents. Local refers to the human's harness/workflow, not where model
inference runs. Local Grok Build is distinct from Cursor and hosted Grok
Bot/Grok Bits automation.

When the human explicitly asks a local harness to merge, it may merge locally
and perform an ordinary, non-force push to `master`/`main` without opening a
pull request. Do not force-push `master`/`main`, publish unrelated work, or
push without that explicit request. Privacy, verification, and preservation
requirements still apply to every agent.

## Shared contribution rules

Cursor bots and hosted Grok Bot/Grok Bits automation:

- Never push to `master`; use `bots/<topic>` branches and pull requests. Never merge your own pull request.

Everyone:

- This repo is **PUBLIC**, so never commit secrets, tokens, passwords, account numbers, financial data, personal info, employer-confidential info, private IPs, or local paths.
- Material from the private repo `InServiceOfX/Monoclaw` (or `claw-portfolio`) comes here only after a privacy check.

## Git workflow for Cursor and hosted Grok automation

- Periodically run `git checkout master && git pull`.
- Do all work on your own non-master branch.
- Regularly pull master and rebase onto it: `git fetch origin && git rebase origin/master`.
- After rebasing a branch that's already pushed, use `git push --force-with-lease` only, and only on your own unmerged branches.
- Never force-push `master` or anyone else's branch.
