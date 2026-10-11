# Next Session

Historical handoffs are in
[archive/next-session-archive.md](archive/next-session-archive.md).

## Codex takeover (2026-10-11)

Claude and her subagents have stood down. The inherited work is merged
and its worktrees removed. Final merged validation: 3212 Rust tests pass,
0 fail, 11 ignored; four Python evidence tests pass. The inherited work:

- t101: CLI maintenance uses the shared campaign runner and retains
  starbase, scorch and recovery events. Three failing-first regressions
  cover the omitted reports. The merged workspace passed 3198 tests.
- t099: classic AI merge eligibility, planetless civil-disorder conversion
  and Hold depth. The owner rejected classic's stale missing-base merge
  flag. Final review survives rogue conversion. Fleet-chain removal and
  intentional rogue disbanding have regressions. The merged workspace
  passed 3212 tests, 0 failures, 11 ignored. Both manual PDFs were rebuilt.
- t090: the 7020-game price matrix and tooling are in
  [tools/price-campaigns](../../tools/price-campaigns/README.md), with results
  in [n7fzd2vk](../notes/investigations/n7fzd2vk-unit-prices-on-generated-maps-with-hostile-openings.md).
  Keep prices and the opening unchanged. This is scripted-policy evidence,
  not human win odds. Most campaigns did not finish; the attacker protects
  home and favors colonies. The matrix predates t099, which is explicit in
  its provenance. Four Python integrity tests pass; a separate current-code
  release-build smoke run passed.

The initial unfinished files, raw price TSVs and validation logs are saved
outside tmpfs under the development host's local-state directory:
`~/.local/state/nostrian-conquest/handoffs/20261011T175210Z/`.
All 90 TSV hashes match committed provenance. Experimental price worktrees
and their cargo pools were removed after verifying their source backup.
Do not restart the old `run_matrix.sh`: it deletes its output directory.

## Next work

Run `zk queue`: t024 is next. Build the Windows nc-sysop x64, x86 and
Win7+ x86 packages on `ssh bean`, then refresh beta.3's release assets and
website without changing the version. Windows SSH responds; its checkout
is `dev/nostrian-conquest` under the SSH user's home. Sync by a git bundle over scp;
Claude found that remote `git pull` hangs. Check its working tree before
updating it. Prepare and verify the packages before asking the owner to
approve uploading/replacing assets and deploying the site.

Packaging preflight: `build_release_bundle.py` currently reads binaries
from `rust/target`, while cargo-slot builds into its selected pool. Make
that lookup honor the build's target directory before preparing archives;
do not package a stale binary from a previous build.

All commits must be signed. A cold GPG/SSH agent cache can block signing or
pushing; ask the owner to unlock it instead of committing unsigned. Fetch
before merging: the owner's other agent may push nc-bridge, nc-herald or
nc-dropfile work. Use cargo-slot from `rust/`; never invent a target dir.
Task IDs are allocated above all IDs in todo and monthly done files.

The broader invalid-order abort differences found by t099 remain outside
its scope; see [q0a0mhm2](../notes/plans/q0a0mhm2-classic-follow-ups-from-the-october-2026-oracle-work.md#t099).
