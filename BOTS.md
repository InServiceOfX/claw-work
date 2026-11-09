# Rules for bots

## GitHub identity scope

The `yeung-assistant` account requirement applies **only to Cursor bots and
Grok bot automation, including Grok Bot and Grok Bits**, until the repository
owner explicitly changes this scope. Those runs must act as `yeung-assistant`,
not as the human's personal account.

Human-directed local agents and harnesses — including Codex, Claude Code,
Grok Build, Hermes-Agent, Pi and other local AI tooling — use the human's
configured Git/GitHub identity. They do not need `yeung-assistant` credentials,
must not switch accounts merely because they are AI agents, and must not add
an identity-only approval step to an already-authorized commit or push.
Local refers to the human's harness/workflow, not where model inference runs;
local Grok Build is distinct from Cursor/Grok bot automation.

This identity exception does not grant permission to publish unrelated work,
push to master/main or merge one's own work. All agents still follow privacy,
feature-branch, review and task-authorization rules.

## Shared contribution rules

- Never push to `master`; use `bots/<topic>` branches and PRs. Never merge your own PR.
- This repo is **PUBLIC**, so never commit secrets, tokens, passwords, account numbers, financial data, personal info, employer-confidential info, private IPs, or local paths.
- Material from the private repo `InServiceOfX/Monoclaw` (or `claw-portfolio`) comes here only after a privacy check.

## Git workflow

- Periodically run `git checkout master && git pull`.
- Do all work on your own non-master branch.
- Regularly pull master and rebase onto it: `git fetch origin && git rebase origin/master`.
- After rebasing an already-pushed branch, use `git push --force-with-lease` only, and only on your own unmerged branches.
- Never force-push `master` or anyone else's branch.
