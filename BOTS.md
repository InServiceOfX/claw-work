# Rules for bots

- Act only as `yeung-assistant`.
- Never push to `master`; use `bots/<topic>` branches and PRs. Never merge your own PR.
- This repo is **PUBLIC**, so never commit secrets, tokens, passwords, account numbers, financial data, personal info, employer-confidential info, private IPs, or local paths.
- Material from the private repo `InServiceOfX/Monoclaw` (or `claw-portfolio`) comes here only after a privacy check.

## Git workflow

- Periodically run `git checkout master && git pull`.
- Do all work on your own non-master branch.
- Regularly pull master and rebase onto it: `git fetch origin && git rebase origin/master`.
- After rebasing an already-pushed branch, use `git push --force-with-lease` only, and only on your own unmerged branches.
- Never force-push `master` or anyone else's branch.
