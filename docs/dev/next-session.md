# Next Session

Keep this file short. Historical detail belongs in
[archive/next-session-archive.md](archive/next-session-archive.md), not here.

## Handoff from Claude to Codex (2026-10-11)

**Heads.** `main` = `origin/main` at this commit; nothing uncommitted on
main. The last full suite ran on the t061 merge (`a9c2a710`): 3195 passed,
0 failed, 11 ignored. Commits after it change only the sysop PDF, todo.md and
this file.

**Live work.** Three Claude subagents were running at handoff. They stop
when Claude stands down; their work stays in the worktrees. The worktrees
are in Claude's session scratchpad under `/tmp` (`git worktree list` shows
the paths), which is on **tmpfs: a reboot loses uncommitted work**. Each
brief is at `briefs/<task>.md` beside the worktrees.

| Task | Branch, worktree | State at handoff |
| --- | --- | --- |
| t099 | `t099`, `wt-t099` (from `7348955f`) | ~20 files uncommitted, no commits |
| t090 | `t090`, `wt-t090` | `0c1c8f6f` (price campaign driver) + 3 uncommitted; also detached worktrees `variants/{bb30,bb60,etac15,etac25}` |
| t101 | `t101`, `wt-t101` (from `9a9ca001`) | 1 file uncommitted |

To take one over: read its brief, review the diff, finish, commit signed,
merge. After t090, remove the four variant worktrees and their cargo pools
(`~/.cache/cargo-slots/nostrian-conquest@{bb30,bb60,etac15,etac25}`).

**Merge routine.** For each branch: every commit `%G? = G`; `git merge
--no-ff -S`; full suite from `rust/` with `nice -n 10 cargo-slot
nostrian-conquest cargo test --workspace --no-fail-fast`; if a `.typ`
changed, `python3 scripts/refresh_manual_revision_date.py --doc
player|sysop` and commit the PDF; close with `zk tasks -p ls id:tNNN` then
`zk tasks do N` (re-read the line number after each close), `zk index`;
`zk list docs/notes --broken-links` reports 22 pre-existing notes; push;
`git worktree remove` and delete `~/.cache/cargo-slots/nostrian-conquest@wt-tNNN`.
`zk tasks add` prepends the date itself. The highest task ID is t101.

**Signing.** Every commit is GPG-signed by Mason Green's key; the agent
cache expires (default 10 min, max 2 h), and a cold cache makes `git
commit -S` time out at pinentry. The GitHub SSH key sits in the same
gpg-agent. When either is cold, ask the owner to unlock it in a terminal
(`echo x | gpg --clearsign >/dev/null`, `ssh -T git@github.com`). Never
commit unsigned. To sign an agent's unsigned commits, cherry-pick them with
`-S` onto the branch's base and compare trees; a rebase would also rewrite
main commits the branch merged.

**Queue after the three tasks.** t024: the Windows nc-sysop packages
(x64, x86, Win7+ x86, as beta.2 was), built on `ssh bean` (PowerShell;
sync by git bundle over scp, since `git pull` hangs there), signed and
uploaded from crunch; ask the owner before uploading. Then the beta.3
re-release: rebuild and replace its assets and deploy the website, keeping
the version v1.0.0-beta.3.

**Owner decisions.** None block the queue. For the owner's review:
t061's choices (a new hosted game arms its timer instead of running at
once; a host that was down runs one year on return; hosted league games
put absent players on autopilot after three turns, as local games do), and
t090's price recommendation when it lands (prices change only with the
owner's approval). Rulings for t099 are in
[q0a0mhm2](../notes/plans/q0a0mhm2-classic-follow-ups-from-the-october-2026-oracle-work.md#t099).

**Other agents.** The owner's own agent pushes signed commits to nc-bridge,
nc-herald and nc-dropfile; fetch before each merge.
