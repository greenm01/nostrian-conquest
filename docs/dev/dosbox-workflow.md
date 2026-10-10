# DOSBox-X ECMAINT Testing Workflow

This documents the standard procedure for running `ECMAINT.EXE` through
DOSBox-X in a headless environment.

## Prerequisites

- `dosbox-x` binary in your `PATH` (the SDL2 version, e.g., `dosbox-x-sdl2`, is strongly recommended on Linux as SDL1 may segfault in headless environments)
- `xvfb-run` for headless X11 (if not using the dummy driver below)
- Python 3 plus `python-pexpect` if you want to use the debugger-driven helper
  scripts under `tools/`
- A scenario directory containing the game files to process

On Arch/CachyOS, the practical package names are:

```bash
sudo pacman -S python-pexpect dosbox-x
```

## Headless Execution Tip

For a faster and simpler headless setup (avoiding `xvfb-run`), use the SDL dummy video driver by setting the following environment variable:

```bash
export SDL_VIDEODRIVER=dummy
```

This allows `dosbox-x` (SDL2 version) to run without an X11 display or virtual framebuffer.

## Key Principle

ECMAINT modifies files **in-place** and creates `.SAV` backup copies. Always
copy your fixture files to a `/tmp/` working directory before running ECMAINT.
Never run ECMAINT directly against repo fixture directories.

## Best-Known ECGAME Launch

For `ECGAME`, the current best-known local recipe is:

1. initialize a real game directory first
2. place a full WWIV-style `CHAIN.TXT` in that same game directory
3. mount that game directory as `C:`
4. run plain `ECGAME`

Use:

```bash
tools/run_ecgame.sh /path/to/game_dir [player_number] [caller_alias]
```

Important rules:

- prefer `CHAIN.TXT` over `DOOR.SYS` for local launch attempts
- run plain `ECGAME`, not `ECGAME /L`
- do not rely on `ECGAME C:\CHAIN.TXT`
- keep `CHAIN.TXT` in the mounted working directory itself
- if the host path contains spaces, the helper now creates a temporary symlink
  automatically before mounting it in DOSBox-X

If the default video backend is wrong for your desktop, override it:

```bash
SDL_VIDEODRIVER_OVERRIDE=x11 tools/run_ecgame.sh /path/to/game_dir
```

or:

```bash
SDL_VIDEODRIVER_OVERRIDE=wayland tools/run_ecgame.sh /path/to/game_dir
```

For returning-player probing, caller identity matters:

- classic `ECGAME` does not treat `PLAYER.DAT` assigned-player fields alone as
  sufficient for returning-player login
- the generated `CHAIN.TXT` alias must match the persisted player handle well
  enough to escape the first-time menu
- the local launcher now lets you pass that alias explicitly:

```bash
tools/run_ecgame.sh /path/to/game_dir 2 SYSOP
```

or:

```bash
ECGAME_CALLER_ALIAS=SYSOP tools/run_ecgame.sh /path/to/game_dir 2
```

Public corroboration for this path:

- modern BBS operators have reported getting Esterian Conquest running under
  `DOSBox-X` with `CHAIN.TXT`, while `DOOR.SYS`/`DORINFO1.DEF` were less
  reliable
- WWIV door docs also note that older doors often require `CHAIN.TXT` to be
  copied directly into the door game directory before launch

What actually worked locally:

- `ECGAME` still returned to `C:\>` when using a synthetic local `CHAIN.TXT`
  with remote-style modem values
- the turning point was switching `CHAIN.TXT` to true local-console settings:
  - line 15 `remote = 0`
  - line 20 `user baud = 0`
  - line 21 `COM port = 0`
  - line 31 `COM baud = 0`
- after that change:
  - DOS saw the generated `CHAIN.TXT`
  - the old `ERRORS.TXT` line `could not find a Door File in path: \N`
    disappeared
  - `CHAIN.TXT` was therefore accepted by the parser

This is the main local-play rule to preserve:

- for local console play, use a WWIV-style `CHAIN.TXT` with local values, not
  remote modem values

## Standard Procedure

For most new mechanics, prefer the repo harness first:

```bash
python3 tools/ncmaint_oracle.py prepare /tmp/ecmaint-oracle
# submit one controlled order family or mutate one narrow field family
python3 tools/ncmaint_oracle.py run /tmp/ecmaint-oracle
```

That captures pre/post snapshots under `/tmp/ecmaint-oracle/.oracle/` and
prints byte-diff clusters for the core `.DAT` files plus report files.

For a known preserved scenario family, use the replay form:

```bash
python3 tools/ncmaint_oracle.py replay-known fleet-order /tmp/ecmaint-fleet-oracle
```

That materializes the known accepted pre-maint state, runs `ECMAINT`, and then
compares the result against the preserved post-maint fixture.

To validate the oracle path itself against preserved fixtures, use:

```bash
python3 tools/ncmaint_oracle.py replay-preserved fleet-order /tmp/ecmaint-fleet-pre-direct
```

That copies the preserved pre-maint fixture directly, runs `ECMAINT`, and
compares the result against the preserved post-maint fixture.

### 1. Prepare the working directory

```bash
SCENARIO=/tmp/ecmaint-test
rm -rf "$SCENARIO"
mkdir -p "$SCENARIO"
cp fixtures/some-fixture/v1.5/* "$SCENARIO/"
```

### 2. Run ECMAINT

```bash
xvfb-run -a dosbox-x \
  -defaultconf \
  -nopromptfolder \
  -defaultdir "$SCENARIO" \
  -set "dosv=off" \
  -set "machine=vgaonly" \
  -set "core=normal" \
  -set "cputype=386_prefetch" \
  -set "cycles=fixed 3000" \
  -set "xms=false" \
  -set "ems=false" \
  -set "umb=false" \
  -set "output=surface" \
  -c "mount c $SCENARIO" \
  -c "c:" \
  -c "ECMAINT /R" \
  -c "exit"
```

The `/R` flag runs maintenance in non-interactive mode.

### 3. Check for errors

```bash
# Check if ERRORS.TXT was created (indicates a problem)
cat "$SCENARIO/ERRORS.TXT" 2>/dev/null

# Check DOSBox-X exit
echo $?
```

Common error messages:
- `"Fleet assigned to an unknown starbase"` — Guard Starbase lookup failed
- `"Game file(s) missing or failed integrity check!"` — cross-file integrity
  failure (usually caused by mixing files from different game states)

### 3b. Dump The Unpacked Live Image

When `ECMAINT.EXE` is LZEXE-packed, the DOSBox-X debugger is the fastest path
to the real program image.

For the current token-path work, you can prepare the standard live-debug
scenario with:

```bash
python3 tools/prepare_ecmaint_token_debug_case.py
```

That rebuilds:

- `/tmp/ecmaint-debug-token`
- the raw two-base Starbase 2 repro
- zero-length `PLAYER.TOK`

Use that directory as the default debugger target when breaking on the token
helpers at `2000:96c4`, `2000:9cb9`, and `2000:9e1e`.

Important for live DOSBox-X breakpoints: those `2000:` addresses are the
raw-import/Ghidra segment numbers, not the segment values shown by the live
debugger. The working translation for this dump is PSP-relative, using the
unpacked program PSP `0814` plus the raw-import segment base:

```text
2000:96c4 -> 2814:96c4
2000:9cb9 -> 2814:9cb9
2000:9e1e -> 2814:9e1e
```

Those PSP-relative breakpoints can display under different normalized segment
values when DOSBox-X stops. For example, breaking on `2814:96c4` surfaced as
`3159:0274`, but it resolves to the same linear address.

Current headless-debugger caveat: `2814:96c4` is the first clean token-path
code stop. If that breakpoint is deleted and execution continues with only
`2814:9cb9` and `2814:9e1e` armed, DOSBox-X currently falls into repeated
`Illegal Unhandled Interrupt Called 6` logging before either later breakpoint
surfaces. Treat that as a debugger/runtime interaction problem, not an address
translation failure.

Terminal caveat: do not use `foot` for DOSBox-X debugger capture work here.
In local runs, `foot` can hide or fail to flush the debugger's live
stop/register prompt output (`CS=` and related break banners) even when
DOSBox-X is actually waiting for input. That makes both manual breakpoint work
and `pexpect`-style automation look hung or promptless. Use a different
terminal emulator when you need reliable live debugger I/O or `MEMDUMP.BIN`
capture.

Launch into the debugger at program entry:

```bash
env SDL_VIDEODRIVER=dummy SDL_AUDIODRIVER=dummy \
dosbox-x \
  -defaultconf \
  -nopromptfolder \
  -nogui \
  -nomenu \
  -defaultdir "$SCENARIO" \
  -set "dosv=off" \
  -set "machine=vgaonly" \
  -set "core=normal" \
  -set "cputype=386_prefetch" \
  -set "cycles=fixed 3000" \
  -set "xms=false" \
  -set "ems=false" \
  -set "umb=false" \
  -set "output=surface" \
  -c "mount c $SCENARIO" \
  -c "c:" \
  -c "DEBUGBOX ECMAINT /R"
```

Inside the debugger:

```text
BPINT 21 3D
RUN
DOS MCBS
MEMDUMPBIN 0814:0000 97EB0
```

Expected output:

- `MEMDUMP.BIN` in the scenario directory

Fallback when the debugger prompt does not surface locally:

If `DEBUGBOX` never gives you a usable live `CS=` prompt in this environment,
the reliable local fallback is DOSBox-X guest-RAM capture via `memory file`.
That avoids the prompt entirely and still yields the same PSP-sliced dump used
for later Ghidra work.

```bash
SCENARIO=/tmp/ecmaint-debug
MEMFILE=/tmp/ecmaint-runtime.mem
export SCENARIO MEMFILE

env SDL_VIDEODRIVER=dummy SDL_AUDIODRIVER=dummy \
dosbox-x \
  -defaultconf \
  -nopromptfolder \
  -nogui \
  -nomenu \
  -defaultdir "$SCENARIO" \
  -set "dosv=off" \
  -set "machine=vgaonly" \
  -set "core=normal" \
  -set "cputype=386_prefetch" \
  -set "cycles=fixed 3000" \
  -set "xms=false" \
  -set "ems=false" \
  -set "umb=false" \
  -set "output=surface" \
  -set "memory file=$MEMFILE" \
  -c "mount c $SCENARIO" \
  -c "c:" \
  -c "ECMAINT /R"

python3 - <<'PY'
import os
from pathlib import Path

guest = Path(os.environ["MEMFILE"]).read_bytes()
Path(os.environ["SCENARIO"], "MEMDUMP.BIN").write_bytes(
    guest[0x8140:0x8140 + 0x97EB0]
)
PY
```

Expected fallback output:

- guest RAM image at `$MEMFILE`
- carved `$SCENARIO/MEMDUMP.BIN`, equivalent to `MEMDUMPBIN 0814:0000 97EB0`

Useful sanity check:

```bash
strings -a "$SCENARIO/MEMDUMP.BIN" | rg "Runtime error|Borland|integrity check|Player.Dat"
```

### 4. Diff the results

```bash
# Binary diff of specific files
xxd "$SCENARIO/FLEETS.DAT" > /tmp/post-fleets.hex
xxd fixtures/some-fixture/v1.5/FLEETS.DAT > /tmp/pre-fleets.hex
diff /tmp/pre-fleets.hex /tmp/post-fleets.hex
```

### 5. Preserve the results

If the run produced useful results, copy the post-maint state to a fixture:

```bash
cp "$SCENARIO"/*.DAT fixtures/some-fixture-post/v1.5/
# Also preserve .SAV files if needed for analysis
cp "$SCENARIO"/*.SAV fixtures/some-fixture-post/v1.5/
```

## DOSBox-X Configuration Notes

The configuration flags are carefully chosen:

- `machine=vgaonly` — minimal video hardware emulation
- `core=normal` — interpreter core (most compatible)
- `cputype=386_prefetch` — matches era of original software
- `cycles=fixed 3000` — deterministic speed (no dynamic throttling)
- `xms=false`, `ems=false`, `umb=false` — no extended memory (DOS real mode only)
- `output=surface` — minimal display backend (works with xvfb)

## Files Created by ECMAINT

After a maintenance run, ECMAINT produces:

- `.SAV` files — backup copies of `.DAT` files before modification
- `RANKINGS.TXT` — player rankings (text)
- `ERRORS.TXT` — error log (only if errors occurred)
- Updated `.DAT` files — the post-maintenance game state

## Cross-File Integrity

ECMAINT performs cross-file integrity checks. If you construct a scenario by
mixing files from different game states (e.g., init PLAYER.DAT with original
CONQUEST.DAT), it may fail with an integrity error. When testing patches:

- Start from a single consistent baseline (either `original/` or `ecutil-init/`)
- Apply minimal targeted patches to that baseline
- Do not mix files from different baselines unless you have confirmed they pass
  integrity checks together

## Running Multiple Passes

To test persistence across turns, run ECMAINT twice:

```bash
# First pass
xvfb-run -a dosbox-x [flags] \
  -c "mount c $SCENARIO" -c "c:" -c "ECMAINT /R" -c "exit"

# Save first-pass state
cp "$SCENARIO"/*.DAT /tmp/pass1/

# Second pass (runs against the already-modified files)
xvfb-run -a dosbox-x [flags] \
  -c "mount c $SCENARIO" -c "c:" -c "ECMAINT /R" -c "exit"

# Diff pass1 vs pass2
diff <(xxd /tmp/pass1/FLEETS.DAT) <(xxd "$SCENARIO/FLEETS.DAT")
```
