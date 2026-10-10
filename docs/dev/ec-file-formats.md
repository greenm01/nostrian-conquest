# EC Classic File Formats

Every byte of the classic data files, as ECMAINT, ECGAME, and ECUTIL use them. The Rust codecs in `rust/nc-data/src/classic/` follow this.

## CONQUEST.DAT

One 2085-byte (0x825) record. ECMAINT, ECGAME, and ECUTIL each load it through
the same unit routine (ECMAINT `3225:000D`, ECGAME `413F:000D`, ECUTIL
`0E0E:000D`). That routine opens the file as a typed file of 0x825-byte
records. If that read fails, it treats the file as the older 10-byte layout:
it reads bytes 0x00..0x09, zeroes 0x0A..0x824, and rewrites the full record.
ECMAINT writes the record back after maintenance (dump 0x15B3). ECUTIL writes
it from F1 (Initialize) and F2 (Maintenance Days). ECGAME never writes it
except through that legacy-upgrade path.

Evidence key: `[P]` probe or preserved fixture pair, `[C]` code only, `[F]`
constant across every fixture. Addresses: ECMAINT and ECGAME are offsets in
`tools/unlzexe/ecmaint_640k.bin` (seg:off = offset + 0x8140, DS = 0x3529,
record at DS:190C) and `tools/unlzexe/ecgame_640k.bin` (offset = seg*16+off,
DS = 0x44A1, record at DS:8C5C). ECUTIL addresses are physical addresses in a
live DOSBox-X image of `original/v1.5/ECUTIL.EXE` sitting at its main menu
(captured with `-set "memory file=..."`; DS = 0x11BE, record at DS:4468,
SETUP at DS:5760). The repo's `ECUTILU.EXE` and `EC_UNLOCKED/ECUTIL.EXE` are
still packed and have no readable code.

| Offset | Width | Type | Name | Meaning | Evidence |
|---|---|---|---|---|---|
| 0x000 | 2 | u16 | `game_year` | Current game year. ECUTIL F1 writes 3000 (phys 0x9EB4). ECMAINT adds 1 after each maintenance pass (0x2AD9). When ECMAINT has just restored a crashed run from its `.SAV` backups (flag DS:169A, set at 0x26E91), it subtracts 1 before the passes if the year exceeds 3000 (0x2A61..0x2A70). ECGAME and ECMAINT read it for display and age arithmetic. | [P] fixture pairs advance by the pass count; [C] |
| 0x002 | 1 | u8 | `player_count` | Number of empire slots: 4, 9, 16, or 25, set by ECUTIL F1 options A..D (phys 0xA1BC..0xA1E5). ECMAINT sizes DATABASE.DAT from it (0x234E8). ECUTIL bounds its PLAYER.DAT loops by it. | [C]; 4 in every fixture |
| 0x003 | 7 | bool[7] | `maintenance_days` | Sunday through Saturday. ECMAINT runs a scheduled pass only when byte `3 + weekday` is nonzero, where weekday comes from DOS GetDate with Sunday = 0 (0x27179..0x27189). A second flag (DS:169C) bypasses the check. ECUTIL writes 0 or 1: F1 copies its in-memory day table (phys 0x9ED0) and F2 toggles that table with Space (phys 0xACCC..0xACDF) and saves it with Enter (phys 0xAD73). | [P] F2 probes below; [C] |
| 0x00A | 50 | u16[25] | `previous_production` | Slot *i* holds empire *i*'s production (PLAYER +0x52) as it stood at the start of the most recent maintenance pass. ECMAINT fills all 25 slots before each pass (0x27A5..0x27E7). ECMAINT's ranking report (0x2DA1) and ECGAME's rankings screen (0x36D13) print slots 1..`player_count` as "Previous Year's Information in Parentheses." ECUTIL F1 writes 100 to all 25 slots (phys 0x9EF5). | [P] every ecmaint pre/post fixture pair; [C] |
| 0x03C | 25 | u8[25] | `previous_planet_count` | Slot *i* holds empire *i*'s planet count (PLAYER +0x50) at the start of the most recent pass. Writer ECMAINT 0x27D8; readers ECMAINT 0x2DBE and ECGAME 0x36D30. ECUTIL F1 writes 1 to all 25 slots (phys 0x9EFF). | [P] fixture pairs; [C] |
| 0x055 | 2000 | zero[2000] | `reserved` | No program reads it. ECUTIL F1 zeroes it (phys 0x9F0B..0x9F26, loop to 2000), and so does the legacy-upgrade path. The codec regenerates zeros. | [C]; [F] zero in all 48 fixtures and the shipped sample |

Slots past `player_count` in both snapshot arrays hold garbage after any
maintenance run (see the bugs below). No program reads them, but a
byte-exact oracle comparison sees them. Under this project's DOSBox-X
configuration the garbage is stable; for a 4-empire game every post-maintenance
fixture holds:

- `previous_production[5..25]`: `FFFF 0000 0000 0000 3374 0000 0000 0375 2065 0000 047E 7420 0000 0000 0000 0000 0000 0000 863B FCFE 8B28`
- `previous_planet_count[5..25]`: `FF 00 00 00 C2 00 00 08 6F 00 01 6F 00 00 00 00 00 00 6A 8D 35`

The shipped `original/v1.5/CONQUEST.DAT`, made on another machine, holds
different garbage in the same slots.

Probes (ECUTIL driven headless with DOSBox-X `AUTOTYPE` against
`fixtures/ecutil-init/v1.5`, whose day bytes are all 0x01):

- `F2 Enter F10` rewrote bytes 0x03..0x09 from `01 01 01 01 01 01 01` to all
  `00`.
- `F2 Space ... Enter F10` set Sunday to `01` and left the other days `00`.

### CONQUEST.DAT classic bugs

1. **Snapshot loop ignores the player count.** ECMAINT's snapshot routine
   (0x27A5) runs `for i := 1 to 25` over the PLAYER pointer table at DS:16B0.
   The PLAYER.DAT loader (0x25F31..0x25F51) allocates pointers only for the
   records it reads, so entries past `player_count` hold whatever bytes
   occupied DS at load time. The loop dereferences them and copies
   PLAYER +0x52 and +0x50 from arbitrary memory: the interrupt table, DOS
   memory, VGA memory, or the BIOS. In the preserved ECMAINT dump, entry 5
   points at `AD05:05FE` (0xFFFF/0xFF from the VGA window) and entry 9 at
   `0A05:0525` (0x3374/0xC2). The garbage depends on the machine and changes
   nothing in play.
2. **ECUTIL F2 does not load the current schedule.** The editor shows and saves
   its own seven-byte table at ECUTIL DS:4C9A. That table starts as whatever
   memory held at launch, which is zero in a fresh DOSBox-X session; F1 sets it
   to all 1. Saving with F2 therefore overwrites the schedule in
   CONQUEST.DAT with that table. The probe above disabled every day. This
   likely explains the "day codes" 0xCA, 0x0A, and 0x26 in `RE_NOTES.md`:
   they were untoggled days carrying stale memory. ECMAINT tests only for
   nonzero.

## SETUP.DAT

One 522-byte (0x20A) record of installation settings. ECUTIL owns it: it
creates the file with defaults when none exists (phys 0xCBBD..0xCCFB), and F4
(Program Options) and F5 (Modem/Com Port) rewrite it. ECGAME reads it at
startup (0x3B156..0x3B1B5) and never writes it. ECMAINT reads it at startup
(0x27388) and rewrites it only when the version tag is wrong.

Addresses follow the CONQUEST.DAT conventions. SETUP sits at ECMAINT DS:2D68,
ECGAME DS:A556, and ECUTIL DS:5760.

| Offset | Width | Type | Name | Meaning | Evidence |
|---|---|---|---|---|---|
| 0 | 2 | ascii | `tag` | `"EC"`. ECGAME (0x3EABA) and ECUTIL (phys 0x89DA) treat a file without it as uninitialized. | [C]; [F] |
| 2 | 3 | ascii digits | `version` | `"151"`: the program version string (`"1.51"`) with the dot removed. ECMAINT requires exactly `EC151` (0x2743E..0x2748E). ECUTIL restamps `110` and `111` to the current version (phys 0x8A81..0x8B23). | [C]; [F] |
| 5 | 4 | u8[4] | `com_irq` | IRQ for COM1..COM4, range 0..7. ECGAME hooks interrupt vector IRQ+8 for the active port (0x3EAE3, 0x3EB75). Default `4 3 4 3`. ECUTIL F5 edits it (phys 0xDCA6..0xDCF5) and restores the defaults (phys 0xDD04). | [P] RE_NOTES F5 diff; [C] |
| 9 | 4 | bool[4] | `com_hw_flow_control` | Hardware flow control for COM1..COM4. When it is set, ECGAME waits for CTS before sending each byte (0x3F59B). Default 1. ECUTIL F5 toggles it (phys 0xDD2C..0xDD42). | [P] RE_NOTES F5 diff; [C] |
| 13 | 499 | zero[499] | `reserved` | No program reads it. Every initializer zeroes it: ECUTIL create (phys 0xCCF0) and upgrade (phys 0x8A43), ECMAINT tag repair (0x274EB), and ECGAME's in-memory repair (0x3EB27). The codec regenerates zeros. | [C]; [F] zero in all fixtures |
| 512 | 1 | bool | `snoop` | Mirror a remote caller's session on the local screen. ECGAME turns local output off only when this is 0 and the caller is remote (0x380E5, 0x3B63D). ECUTIL F4 C toggles it (phys 0xD18D); default 1. | [P] RE_NOTES F4 diff; [C] |
| 513 | 2 | u16 | `max_key_gap_minutes` | Idle limit between keystrokes. ECGAME copies the word to its inactivity limit (0x3B1C9); 0 disables the timeout (0x38004). ECUTIL F4 F enters a value from 0 to 255 (phys 0xD2A5); default 10. | [P] RE_NOTES F4 diff; [C] word read |
| 515 | 1 | bool | `remote_timeout` | Apply the key-gap timeout to remote callers (ECGAME 0x37FFD). ECUTIL F4 E toggles it (phys 0xD234); default 1. | [P] RE_NOTES F4 diff; [C] |
| 516 | 1 | bool | `local_timeout` | Apply the key-gap timeout to local users (ECGAME 0x37FEF). ECUTIL F4 D toggles it (phys 0xD1F9); default 0. | [P] RE_NOTES F4 diff; [C] |
| 517 | 1 | u8 | `minimum_time_minutes` | ECGAME raises the session's remaining time to at least this many minutes (0x3949C..0x394AC). ECUTIL F4 G sets it (phys 0xD316); default 0. | [P] RE_NOTES F4 diff; [C] |
| 518 | 2 | u16 | `purge_after_turns` | "Purge messages & reports after." ECUTIL F4 A stores 0..255 as a word (phys 0xD0EF); default 0. Neither ECGAME nor ECMAINT reads it. | [P] RE_NOTES F4 diff; [C] no reader |
| 520 | 2 | u16 | `autopilot_after_turns` | When it is nonzero, ECMAINT sets the autopilot flag (PLAYER +0x6D) on a human empire (PLAYER +0x00 = 1) once `game_year - PLAYER.last_run_year (+0x4E) >= value` (0x20658..0x20693). ECUTIL F4 B stores 0..255 as a word (phys 0xD165); default 0. | [P] RE_NOTES F4 diff; [C] word read |

ECUTIL's numeric input returns a byte, so ECUTIL always writes 0 to the high
bytes at 514, 519, and 521. The readers still load full words at 513 and 520.

ECGAME seeds the settings with its own defaults before loading the file
(0x9715: snoop 1, key gap 5, remote 1, local 1, minimum 0). The file replaces
them.

### SETUP.DAT classic quirks

1. **Purge does nothing in v1.5.** ECUTIL stores `purge_after_turns`, but no
   load, store, or indexed access touches offset 518 in ECGAME or ECMAINT.
2. **ECMAINT resets the COM settings on any tag mismatch.** If the first five
   bytes are not `EC151`, ECMAINT rewrites bytes 0..511 (tag, IRQs `4 3 4 3`,
   flow control 1, reserved zeros) and writes the file back (0x27493..0x2751B).
   Bytes 512..521 survive. A v1.50 file therefore loses its COM settings.
3. **ECGAME stamps a stale version.** When the tag lacks `"EC"`, ECGAME repairs
   the record in memory and copies `"EC150"` into it (0x3EB07, string at
   DS:1982). It never writes the repair to disk.

## PLANETS.DAT

One 97-byte (0x61) record per world, in map order. ECUTIL F1 writes the file;
ECMAINT and ECGAME load every record onto the heap and write all of them back.
The pointer tables are ECMAINT DS:1712, ECGAME DS:4E98, and ECUTIL DS:02F1
(records in place, `DS:02F1 + 0x61 * n`). Inside ECMAINT's planetary battle the
target world is copied to DS:0517 and written back from there. In Pascal terms:

```text
x, y: Byte; potential: Byte; production, goods: Real; name: String[20];
queue: array[1..10] of Word; dock: array[1..8] of LongInt;
armies, batteries: Word; status, owner, spare1, spare2, scorch: Byte
```

Addresses follow the CONQUEST.DAT conventions. An ECMAINT dump offset is
`seg*16 + off - 0x8140`, so the yearly planet pass `110a:747f` in the other
specs is 0x103DF here. ECGAME addresses are offsets in `ecgame_640k.bin`.
`tools/unlzexe/ECGAMEU.EXE` has corrupted bytes in places (load-image offset
0x689A reads `b8 16 dc 47` where the live dump has `b8 16 00 50`), so read
ECGAME from the dump. ECUTIL addresses are physical, as in the CONQUEST.DAT
section.

Every real in PLANETS, FLEETS, and BASES is a Turbo Pascal 6-byte `Real`:
exponent byte first (bias 129, 0 means 0.0), then a 39-bit mantissa
little-endian with the sign in the top bit of the last byte. The Rust `Real48`
type uses this layout.

The fixture column below summarizes all 880 records in the 44 PLANETS.DAT
files under `fixtures/` and `original/v1.5`.

| Offset | Width | Type | Name | Meaning | Evidence |
|---|---|---|---|---|---|
| 0x00 | 1 | u8 | `x` | Sector x, 1-based. ECUTIL writes it (phys 0x969A) and nothing changes it. ECMAINT's coordinate-to-world lookup reads it (0x29E85), as do ECGAME's lists and reports. | [C]; [F] |
| 0x01 | 1 | u8 | `y` | Sector y, 1-based. ECUTIL phys 0x96A4; read with `x`. | [C]; [F] |
| 0x02 | 1 | u8 | `potential_production` | Ceiling for present production. ECUTIL sets it by the generator's cell class: homeworld 100 (phys 0x97A5), class 2 `Random(41) + 50` (0x9811), class 3 `Random(51) + 30` (0x982B), class 4 `Random(141) + 10` (0x9845). ECMAINT reads it as the growth target in the yearly pass (0x1083B, 0x109F9) and in the planetary battle (0xCC61); ECGAME reads it for display. Nothing writes it after ECUTIL. | [C]; [F] constant across every pre/post pair |
| 0x03 | 6 | real | `production` | Present production ("factories"), P. The yearly pass grows it toward `potential`, rounds it to 0.001 (0x10A2E), and raises it to at least 1.0 (0x10B51). Colonizing sets 1.0 (0x783B). A scorched world loses 51–70% a year (0x10CA6..0x10CEA), and capture halves it if the world was scorched (0xFDCA). Bombard damage writes it through DS:0517 (0xF492, 0xFC92). ECUTIL writes 0, or 100.0 for a homeworld (phys 0x97AE). Rust's "homeworld seed" test `raw[0x03] == 0x87` reads this real's exponent: it holds for any 64 <= P < 128. There is no flag byte. | [C]; [P] ECUTIL fixtures read P = 100; scorch probe below |
| 0x09 | 6 | real | `stored_goods` | Stored production points, G. The yearly pass charges completed builds against it (0x105FB..0x10825; unit costs 5, 15, 45, 15, 5, 20, 50, 100 for queue slots 1..8, 2 an army, 20 a battery), adds the year's income (0x10BCD), and caps it at 65535 (0x10BFC). Salvage adds the fleet's yield (0x83B5). A scorched world keeps 10% (0x10C11..0x10CA2), and capture halves it if the world was scorched. ECGAME only reads it. ECUTIL writes 0, or 35.0 for a homeworld (phys 0x97E1). Byte 0x0E is the mantissa's top byte: the "economy marker", "accumulator", and "tax byte" in older notes are all this real. | [C]; [P] ECUTIL fixtures read G = 35; scorch probe below |
| 0x0F | 21 | String[20] | `name` | Length byte, then up to 20 characters. ECUTIL stores `"Unowned"`, or `"Not Named Yet"` for a homeworld (phys 0x976D, 0x9806; store limit 0x14). ECMAINT stores `"Not Named Yet"` on colonizing (0x7873). ECGAME's rename prompt reads "Name this world (20 characters or less)" (0x2DDB2) and stores with limit 20 (0x2E43A). Bytes past the length are stale: a Pascal string store leaves them alone, and no program reads them. ECUTIL leaves whatever its buffer held, the same bytes per record slot in every new game, and a renamed world keeps the tail of its old name (`original/v1.5` record 13 reads `"foosville"` then `" Yet"`). | [C]; [P] poison probe below |
| 0x24 | 20 | u16[10] | `build_queue` | Units ordered this year, indexed by kind: 1 destroyer, 2 cruiser, 3 battleship, 4 scout, 5 troop transport, 6 ETAC, 7 starbase, 8 reserved, 9 army, 10 ground battery. ECMAINT addresses slot *k* as `+0x22 + 2k`. ECGAME adds an order to its slot (0x11D5F..0x11D76), refuses slot 8 ("There is no unit #8 in Esterian Conquest (The 8th slot is reserved for future use.)"), and clears the queue on Abort (0x122B0) and when a starbase leaves orbit through Starbase Command (0x22D56). ECMAINT's yearly pass moves slots 1..8 into the dock (0x105FB), adds slots 9 and 10 to armies and batteries, and zeroes all ten (0x10D41). Capture zeroes it (0xFA5D). The rogue AI adds orders directly (0x1DB6E..0x1DDAB). ECUTIL zeroes it (phys 0x96C1). | [C]; [P] build fixtures |
| 0x38 | 32 | u32[8] | `stardock` | Built units waiting to be commissioned, kinds 1..8 as above. ECMAINT addresses slot *k* as `+0x34 + 4k`. The yearly pass adds the queue and saturates each count at 65535 (0x10614..0x10694). ECGAME commissioning decrements the slots (0x132AB, 0x1339D, scouts 0x143A4, starbases 0x13948). Bombard and capture damage write them through DS:0517 (0xF084, 0xF66C, 0xFDBC, 0xFE2A), and the rogue AI commissions from them (0x1E827..0x1EA01). Slot 8 (IPBM in the bombard report, 0xA108) can be filled only by editing the file. ECUTIL zeroes it (phys 0x96E7). | [C]; [P] build fixtures |
| 0x58 | 2 | u16 | `armies` | Armies on the surface. The yearly pass adds queue slot 9 and saturates at 65535 (0x1070C..0x10742). Colonizing sets 1 (0x782A); Salvage adds the fleet's armies (0x83DF); ground combat and capture write it through DS:0517 (0xE2E9, 0xF182, 0xFA6A). ECGAME Load and Unload write it (0x1673F, 0x16B01). ECUTIL writes 0, or 10 for a homeworld (phys 0x97CD). The high byte 0x59 is 0 in every fixture. | [C]; [P] invade and bombard fixtures |
| 0x5A | 2 | u16 | `ground_batteries` | The yearly pass adds queue slot 10 and saturates at 65535 (0x1079C..0x107D2). Bombard and blitz damage write it through DS:0517 (0xE1AE, 0xFB34, 0xFD4B). ECUTIL writes 0, or 4 for a homeworld (phys 0x97D7). The high byte 0x5B is 0 in every fixture. | [C]; [P] bombard fixtures |
| 0x5C | 1 | u8 | `status` | Years under the current owner, capped at 2. The yearly pass steps 0 to 1 to 2 (0x10436..0x1045F) and scales the year's income by 0, 0.5, and 1.0 for the value it found (0x10483). Capture sets 0 (0xFA41); colonizing sets 2 (0x7858). ECGAME prints "Just captured - factories will produce nothing this year", "Recently captured - factories being converted for empire use", or "Regular planet - factories fully functional" (0x2864A, 0x29435). ECUTIL writes 2 for a homeworld and 0 elsewhere (phys 0x97C4, 0x970E). Rust's `conversion_countdown` models this field at the wrong offset (0x5E). | [C]; [P] RE_NOTES probe: 0 and 1 return to 2 |
| 0x5D | 1 | u8 | `owner` | Owning empire, 0 for none. Colonizing (0x781B) and capture (0xFA49) write it, and ECMAINT recounts PLAYER +0x50 from it each run (0x1411). ECUTIL writes 0 and then hands each empire a homeworld (phys 0x9904). ECGAME only reads it. | [C]; [P] |
| 0x5E | 1 | u8 | `spare1` | Unused. ECUTIL zeroes it (phys 0x9720). No load, store, or indexed access touches it in ECMAINT, ECGAME, or ECUTIL. ECMAINT passes any value through untouched and is not affected by it. | [C]; [P] poison probe; [F] zero in all fixtures |
| 0x5F | 1 | u8 | `spare2` | Unused, like 0x5E. ECUTIL zeroes it (phys 0x9729). | [C]; [P] poison probe; [F] zero in all fixtures |
| 0x60 | 1 | u8 | `scorch_earth` | Scorch-Earth order, 0 or 1. ECGAME sets it from Planet Command (0x10E86) and clears it on Rescind (0x10DE4). Each year while it is set, ECMAINT skips growth and income: G keeps 10% and P keeps `Random(20)/100 + 0.30` of itself (0x1082D, 0x10C11..0x10CEA); the build queue still completes. On capture ECMAINT halves P and G, empties the dock, and clears the flag (0xFDCA..0xFE38). ECUTIL zeroes it (phys 0x9776). | [C]; [P] scorch probe below |

Probes (ECMAINT under this project's DOSBox-X configuration, clock fixed per
run):

- Poison. In ten fixtures (econ, build, starbase, starbase-lift, move, fleet,
  fleet-battle, invade, bombard-heavy, results-interleaved) every record got
  `0x5E = A5`, `0x5F = 5A`, and `C3` in every name byte past the length.
  Every output file matched the unpoisoned run except at those bytes, and
  every poisoned byte survived. A world colonized in the fleet fixture took
  `"Not Named Yet"` and kept `C3` in 0x1D..0x23.
- Scorch. `ecmaint-econ-pre` with `0x60 = 1` on world 15 (P 100, G 35) and
  world 14 (P 100, G 66), three clocks each: G became 3.5 and 6.6, P became
  39–47, the flag stayed 1, and the control runs left P and G unchanged.

### PLANETS.DAT classic quirks

1. **Saturating counts in wide fields.** The dock slots are u32, but the yearly
   pass clamps them at 65535, as it clamps armies and batteries.
2. **Slot 8 is dead.** ECGAME will not queue kind 8, yet ECMAINT carries a cost
   (100) for it, moves it into dock slot 8, and reports dock slot 8 as IPBMs.
3. **Production floor writes.** The nearest-world scorers behind Seek Home
   (ECMAINT 0x1D166, ECGAME 0x17830) divide by P and store a floor into the
   candidate world while computing: 1.0 in ECMAINT (0x1D2D3), 0.1 in ECGAME
   (0x179A6). Candidates are the fleet owner's worlds, whose P the yearly pass
   already keeps at 1.0 or more, so only a world scorched on capture (P halved
   below 1.0) can be raised early.
4. **Stale name tails.** See `name`. A byte-exact comparison must either carry
   the tail or ignore it; no program reads it.

## FLEETS.DAT

One 54-byte (0x36) record per fleet. Records form one doubly linked chain per
empire, headed by PLAYER +0x40. ECGAME keeps its pointer table at DS:508E.
ECMAINT reads records as its unit loader walks the chains (0x25EE4) and works
on stack copies. Movement, encounter, and battle rules live in
[ec-movement-spec.md](ec-movement-spec.md),
[ec-encounter-spec.md](ec-encounter-spec.md), and
[ec-battle-spec.md](ec-battle-spec.md). The decode of 0x23..0x35 is in
[the t033 note](../notes/investigations/9msqq5e0-classic-move-orders-complete-on-entering-the-target-sector.md).

| Offset | Width | Type | Name | Meaning | Evidence |
|---|---|---|---|---|---|
| 0x00 | 2 | u16 | `fleet_number` | The fleet's number within its empire ("Fleet 1"). ECGAME reads it as a word wherever it names a fleet and renumbers it (0x16E5E). The starbase escort word in BASES 0x00 holds this value (0x5C8). | [C]; [F] shipped file numbers 1..4 per empire |
| 0x02 | 1 | u8 | `owner` | Owning empire. The unit loader copies it to the unit entry. | [C]; [F] |
| 0x03 | 2 | u16 | `next` | Record number of the next fleet in the owner's chain, 0 at the tail. ECGAME relinks it when it adds, deletes, or merges fleets (0x1E099..0x1E31A, 0x2C5CC). | [C]; [F] shipped chains |
| 0x05 | 2 | u16 | `record` | This record's own number, 1-based. | [C]; [F] |
| 0x07 | 2 | u16 | `prev` | Previous fleet in the chain, 0 at the head. Byte 0x08 is its high byte. | [C]; [F] |
| 0x09 | 1 | u8 | `max_speed` | Slowest ship type present. ECGAME recomputes it; ECMAINT sets it when it creates or merges fleets. | movement spec; t033 note |
| 0x0A | 1 | u8 | `speed` | Current speed. | movement spec |
| 0x0B | 1 | u8 | `x` | Sector x. | movement spec |
| 0x0C | 1 | u8 | `y` | Sector y. | movement spec |
| 0x0D | 6 | real | `fx` | In-sector x fraction; 0.5 is the center. | movement spec |
| 0x13 | 6 | real | `fy` | In-sector y fraction. | movement spec |
| 0x19 | 6 | real | `z` | Depth: 0 deep space, 1 orbit. | movement spec |
| 0x1F | 1 | u8 | `order` | Standing order 0..15 (0 Hold through 15 Salvage). | encounter spec |
| 0x20 | 1 | u8 | `target_x` | Target sector x. | movement spec |
| 0x21 | 1 | u8 | `target_y` | Target sector y. | movement spec |
| 0x22 | 1 | u8 | `target_z` | Target depth: 0 sector, 1 world. | movement spec |
| 0x23 | 2 | u16 | `mission_target` | Join host fleet number or guarded starbase id. Other orders leave stale bytes here (the shipped Guard/Blockade fleets hold `b046`, `9ab7`, ...). | t033 note |
| 0x25 | 1 | u8 | `roe` | Rules of engagement. | encounter spec |
| 0x26 | 2 | u16 | `battleships` | | t033 note |
| 0x28 | 2 | u16 | `cruisers` | | t033 note |
| 0x2A | 2 | u16 | `destroyers` | | t033 note |
| 0x2C | 2 | u16 | `transports_empty` | | t033 note |
| 0x2E | 2 | u16 | `transports_loaded` | One army each. | t033 note |
| 0x30 | 2 | u16 | `etacs` | | t033 note |
| 0x32 | 2 | u16 | `scouts` | | t033 note |
| 0x34 | 2 | u16 | `strength_sum` | Dead field. The only access that reaches the file is the Join merge, which adds the absorbed fleet's word to the host's with 16-bit wraparound (0x25410..0x2541E). The encounter strength routine (0x1CBA4) stores `2 DD + 7 CA + 21 BB` into its argument (0x1CC19), but every caller (0x13634, 0x13907, 0x13970, 0x1AC6C) passes a stack copy. Nothing reads it for a decision, ECGAME never touches it, and ECUTIL leaves stack garbage (`86fe`, `268a`, ... in the shipped file). | [C]; [P] poison probe: sentinels survived every run, and a three-fleet merge summed `7012 + 7013 + 7014` to `5039` |

## BASES.DAT

One 35-byte (0x23) record per starbase, in doubly linked chains headed by
PLAYER +0x44 (first) and +0x46 (last). [ec-starbase-spec.md](ec-starbase-spec.md)
holds the full evidence and the movement rules; this table restates the layout
with dump offsets. Every byte is accounted for.

| Offset | Width | Type | Name | Meaning | Evidence |
|---|---|---|---|---|---|
| 0x00 | 2 | u16 | `escort` | FLEETS 0x00 number of the Guard Starbase fleet riding with the base; 0 for none. The guard arrival writes it when it is 0 (0x4537); the escort carry matches it (0x5C8); the robot guard assignment requires 0 (0x1F197). Nothing clears it when the escort leaves. | [C][P] |
| 0x02 | 2 | u16 | `base_id` | "Starbase 1". A guarding fleet names it in FLEETS 0x23 (guard lookup 0x15043; unit loader 0x264EB). | [C][P] |
| 0x04 | 1 | u8 | `owner` | Owning empire. The loader's integrity check requires it to match the chain's player (0x26488). | [C][P] |
| 0x05 | 2 | u16 | `next` | Next base in the owner's chain, 0 at the tail (0x26582..0x266CC). | [C][P] |
| 0x07 | 2 | u16 | `record` | This record's own number, 1-based. | [C][P] |
| 0x09 | 2 | u16 | `prev` | Previous base, 0 at the head. | [C] |
| 0x0B | 1 | u8 | `x` | Sector x. | [C][P] |
| 0x0C | 1 | u8 | `y` | Sector y. | [C][P] |
| 0x0D | 6 | real | `fx` | In-sector x fraction (step reads 0x403..0x4C3, writes 0xCDE..0xD42). | [C][P] |
| 0x13 | 6 | real | `fy` | In-sector y fraction. | [C][P] |
| 0x19 | 6 | real | `z` | Depth: 0 deep space, 1 orbit. | [C][P] |
| 0x1F | 1 | u8 | `moving` | 1 while under way. The tick dispatcher sets and clears it (0x8C96, 0x8CE3); ECGAME sets it on Move and clears it on Halt. The step ignores it, and sighting multiplies by 1 either way (0x10D88). | [C][P] |
| 0x20 | 1 | u8 | `target_x` | Target sector x. | [C][P] |
| 0x21 | 1 | u8 | `target_y` | Target sector y. | [C][P] |
| 0x22 | 1 | u8 | `target_z` | Target depth: 1 orbit, 0 deep space. | [C][P] |

A new base (0x1E353) starts with escort 0, target equal to its sector,
`target_z = 1`, `moving = 0`, and `fx = fy = 0.5`.

## PLAYER.DAT

One 110-byte (0x6E) record per empire, `player_count` records (CONQUEST 0x02).
ECMAINT reads every record into the heap at load (0x25EE4..0x25F82) and keeps
far pointers in the table at DS:16B0 (entry *i* at DS:16AC + 4*i). ECGAME
keeps the same kind of table at DS:4660 and the signed-on empire's index in
DS:8C5A. ECUTIL reads the file into an array at DS:4C34 + 0x6E*i at startup
(phys 0xDF49..0xDF8D), so fields that F1 (Initialize) does not write keep
whatever the previous file held. All three write whole records back.

Nothing in the record is opaque. Two areas hold stale bytes that no program
reads: the empire name past its length byte and the diplomacy slots past
`player_count`. The codec must carry them only for a byte-exact comparison;
otherwise it can regenerate them as zero. The shipped
`original/v1.5/PLAYER.DAT` holds them, and `fixtures/ecutil-init` inherits
them because F1 ran over that file:

- name tails after "In Civil Disorder" in `ecutil-init`, records 1..4:
  `27 73 EF`, `72 65 68`, `6C 61 6E`, `8C 46 F0`
- diplomacy slots 10..25 (0x5D..0x6C), records 1..4:
  `65 74 24 45 6E 74 65 72 20 74 68 65 20 6E 75 6D`,
  `20 63 6F 6E 74 61 69 6E 20 61 6E 79 20 65 6D 70`,
  `79 73 74 65 6D 2E 55 89 E5 81 EC 12 04 BF 41 22`,
  `45 0C 50 9A 9B 1A F8 33 88 46 F5 8A 46 F5 30 E4`

Slots 5..9 of those records are zero.

| Offset | Width | Type | Name | Meaning | Evidence |
|---|---|---|---|---|---|
| 0x00 | 1 | u8 | `mode` | 0 civil disorder (unowned), 1 human player, 0xFF rogue (played by ECMAINT). ECGAME sets 1 when a player joins (0x31646). ECUTIL sets 0 in F1 (phys 0xA224) and 0, 1, or 0xFF from the ownership editor (phys 0xC100..0xC52A). ECMAINT turns a civil-disorder empire into a rogue at the start of the orders pass when `ECMAINT /C` set DS:16A3 (0x2921A), or without /C when it has no planets (0x1D51A..0x1D627); /C also converts at the end of the run (0x1372..0x13CC). A rogue with no planets is marked enemy of every human empire (0x1D690). Only human empires keep their messages through the message pack (0x21045). | [C]; [F] |
| 0x01 | 25 | char[25] | `handle` | The owner's BBS log-on name, upper case, padded with spaces. ECGAME copies it on join (0x31616..0x3162E) and matches it against the caller at sign-on. ECUTIL upper-cases a typed name (phys 0xC4FB) and writes 25 spaces for unowned and rogue empires, as does ECMAINT's rogue conversion (0x1D5D1). | [C]; [F] |
| 0x1A | 1 | u8 | `page_lines` | Lines per screen for ECGAME's "More" pause: ECGAME pauses when the line count reaches `page_lines - 1` (or `- 2`, `- 5` on some screens) and never pauses when it is 0 or 1 (0xBD61, 0x17E85..0x17EC1, and six more sites). ECUTIL F1 writes 24 (phys 0x9A4E). No program writes another value; ECMAINT never touches it. | [C]; [P] ECMAINT kept a poisoned 0x33; [F] 24 everywhere |
| 0x1B | 21 | string[20] | `empire_name` | Pascal string: length byte at 0x1B, characters 0x1C..0x2F. Every writer stores it with a 20-character limit: ECGAME (0x2E934, 0x31EC6), ECUTIL (phys 0xA240), and ECMAINT when it renames an empire "Rogues" (0x13CC, 0x1D5B3). ECUTIL writes "In Civil Disorder" for unowned empires. Bytes past the length are stale. | [C]; [P] ECMAINT kept a poisoned 0x2F |
| 0x30 | 4 | i32 | `messages_first` | 1-based MESSAGES.DAT record of the first message waiting for this empire; 0 for none. ECMAINT tests it as a signed LongInt (0x20C3A..0x20C64), rebuilds it when it packs the message file (0x2106E..0x210A4), and zeroes it when it reinitializes the file (0x21767). ECGAME rewrites the signed-on empire's pair as the player reads and deletes mail (0x2FC04, 0x3076C, 0x3094E, 0x30AB6). | [C]; [P] a nonzero high byte at 0x33, or 5 with an empty MESSAGES.DAT, made ECMAINT log "Error reading message base" and zero 0x30..0x37 |
| 0x34 | 4 | i32 | `messages_last` | 1-based MESSAGES.DAT record of the last waiting message (0x2138C). Same writers as `messages_first`. | [C][P] as above |
| 0x38 | 4 | i32 | `results_first` | 1-based RESULTS.DAT record of the first report header for this empire; 0 for none (0x21E8C, 0x22188..0x221BD, 0x2428D). ECGAME clears it after the player reads the reports (0x2EB0A). | [C]; [P] fixture pairs; a nonzero 0x3B made ECMAINT log "Error reading results database" and rebuild the chain |
| 0x3C | 4 | i32 | `results_last` | 1-based RESULTS.DAT record of the last report header (0x22494, 0x24437..0x24482; ECGAME 0x2EB23). | [C][P] as above |
| 0x40 | 2 | u16 | `fleets_first` | FLEETS.DAT record heading this empire's fleet chain; 0 for none. ECMAINT's loader walks from it (0x26085); the pack renumbers it (0x1941); unlink and append maintain it (0x2461D, 0x1DE1A..0x1DFAB). ECGAME maintains it when it adds, deletes, or merges fleets (0x216C6, 0x2C62C). ECUTIL F1 writes the empire's first fleet. | [C]; [F] |
| 0x42 | 2 | u16 | `fleets_last` | Last fleet in the chain. No program reads it; ECMAINT's append walks the chain from the head (0x1DE34). Writers: ECMAINT pack (0x1A44), append (0x1DFAB), unlink (0x246BC); ECGAME (0x2173E); ECUTIL F1. | [C]; [F] |
| 0x44 | 2 | u16 | `bases_first` | BASES.DAT record heading the starbase chain; 0 for none. ECMAINT loader (0x26418), pack (0x1BC4), append (0x1E317), unlink (0x24776). ECGAME rewrites it after saving bases (0x2BEDB). | [C]; see [ec-starbase-spec.md](ec-starbase-spec.md) |
| 0x46 | 2 | u16 | `bases_last` | Last base in the chain. ECMAINT append reads it (0x1E331) and writes it (0x1E3E6, 0x1E4E3); the pack (0x1CC8) and unlink (0x247F4) write it. ECGAME never reads or writes it. | [C] |
| 0x48 | 2 | u16 | `ipbms_first` | IPBM.DAT record heading this empire's IPBM chain; 0 for none. It is a record number, not a count. ECMAINT's loader reads it (0x2679F..0x26902) and fails the integrity check when it points past the file; the pack renumbers it (0x1E3F..0x1E7D). ECGAME loads the chain from it at sign-on (0x2C707..0x2C74B). ECUTIL F1 writes 0. | [C]; [P] IPBM probes below; [F] 0 everywhere |
| 0x4A | 2 | u16 | `ipbms_last` | Last IPBM in the chain. Only the pack writes it (0x1F43), and only for empires with a chain; ECUTIL F1 writes 0 (phys 0x9C5B). No program reads it. | [C]; [P] ECMAINT kept a poisoned 0x4B when 0x48 = 0 |
| 0x4C | 1 | u8 | `homeworld` | 1-based PLANETS.DAT record of the empire's starting world. ECUTIL F1 writes it (phys 0x9929) and reads it while placing the starting fleets (phys 0x99E2). ECMAINT and ECGAME never read it. | [C]; [P] ECMAINT kept a poisoned value; [F] |
| 0x4D | 1 | u8 | `homeworld_copy` | ECUTIL F1 writes the same planet number (phys 0x9939). No program reads it. | [C]; [P] ECMAINT kept a poisoned value; [F] equals 0x4C |
| 0x4E | 2 | u16 | `last_year_on` | Game year of the empire's last sign-on; 0 before the first. ECGAME writes the current year at sign-on (0x2B7F5, 0x31202) and shows it as "Last year on". ECMAINT turns autopilot on when `year - last_year_on` reaches the inactivity limit at DS:2F70, if that limit is nonzero, for human empires (0x20666..0x20693). ECUTIL F1 writes 0. | [C]; [F] |
| 0x50 | 1 | u8 | `planet_count` | Planets this empire owns (PLANETS 0x5D). ECMAINT recounts it for every empire at the end of a run (0x13D4..0x141F) and adjusts it on capture (0x78C8, 0xFA2A, 0xFA3D). ECGAME reads it (0xAE14 and others). ECUTIL F1 writes 1. | [C]; [P] a poisoned 9 came back as 1 |
| 0x51 | 1 | u8 | `tax_rate` | Tax rate in percent. ECGAME sets it from the economy screen (0x10FFB) and writes 50 when a player joins (0x2E973). The rogue and autopilot AI sets it (0x1D90A, 0x1D91F). The economy pass reads it (0x108F9..0x10B77). ECUTIL F1 writes 0. | [C]; [F] |
| 0x52 | 2 | u16 | `production` | `Trunc` of the summed present production (PLANETS 0x03..0x08, real) of the empire's planets. ECMAINT recomputes it at the end of a run (0x13DD..0x13EA via 0x206F0); the rankings read it (0x27C0, 0x2D59), as does ECGAME (0xADF6, 0x36CCB). ECUTIL F1 writes 100. Byte 0x53 is its high byte, not a diplomacy slot. | [C]; [P] a poisoned 0x52/0x53 came back as 100 |
| 0x54 | 25 | u8[25] | `diplomacy` | Byte 0x53 + *k* is this empire's stance toward empire *k*: 0 neutral, 1 enemy. Code indexes it as `[ptr + k + 0x53]`. ECGAME sets 1 and 0 (0x9CB0, 0x9D86); ECMAINT reads it in the encounter (0x12952) and sets it for rogues (0x1D57F, 0x1D690, 0xFF2F, 0xFF3E). ECUTIL F1 zeroes slots 1..`player_count` (phys 0x9CB2). Slots past `player_count` are never read. | [C]; [P] ECMAINT kept a poisoned slot 13 |
| 0x6D | 1 | u8 | `autopilot` | 1 while ECMAINT plays a human empire. ECGAME toggles it (0x98C6); ECMAINT sets it after inactivity (0x20693) and reads it (0x206A3); ECUTIL clears it in F1 and in the ownership editor (phys 0x9C89, 0xC154, 0xC1AD). | [C]; [F] the sample's empire 1 holds 1 |

Probes for this table ran `ECMAINT /R` on
`fixtures/ecmaint-results-two-human-pre/v1.5` with poisoned bytes in empire 1:
0x1A, 0x2F, 0x4B, 0x4C, 0x4D, and diplomacy slot 13 survived; 0x50 and
0x52..0x53 came back recomputed. ECUTIL addresses come from a live DOSBox-X
image of `original/v1.5/ECUTIL.EXE` (it unpacks only with
`cputype=386_prefetch`).

### PLAYER.DAT classic quirks

1. **Three fields are write-only.** No program reads `fleets_last` (0x42),
   `ipbms_last` (0x4A), or `homeworld_copy` (0x4D), and ECMAINT and ECGAME
   never read `homeworld` (0x4C).
2. **ECGAME never maintains `bases_last`.** It rewrites `bases_first` after a
   base save (0x2BEDB) but leaves +0x46 alone; ECMAINT's base append reads it.
3. **Stale bytes survive ECUTIL F1.** F1 overwrites fields inside records it
   first loaded from the old PLAYER.DAT, so the name tail and the diplomacy
   slots past `player_count` carry the previous game's bytes.

## IPBM.DAT

One 32-byte (0x20) record per interplanetary ballistic missile, in one doubly
linked chain per empire headed by PLAYER +0x48 (first) and +0x4A (last). The
layout is a fleet record without speeds, order, mission, or ships: 0x00..0x08
match FLEETS, the position sits two bytes earlier than in FLEETS (no speed
bytes), and the target three bytes earlier (no order byte).

The feature is dormant. No program creates an IPBM record: ECUTIL F1
truncates the file (phys 0x9E70..0x9E8D), ECMAINT only rewrites records it
loaded and packs them, and ECGAME only reads the chain into memory at sign-on
(0x2C722..0x2C7C2) and frees it at exit (0x3F8F6). Stardock IPBMs (PLANETS
slot 8) never become IPBM.DAT records. Every fixture and the shipped sample
have an empty IPBM.DAT, and PLAYER +0x48/+0x4A are 0.

When records exist, ECMAINT treats them as units of kind 3 in its unit table
(loader 0x2675A..0x26A52; unit entry +0 owner, +1/+2 sector, +5 layer, +6
record number, +10 word 0x00). The tick step moves them at a fixed speed of 7
(0x79A) toward the target sector center, like a fleet (0x6B1..0x79A,
writeback 0xE3A..0xEA2). The encounter ignores them as observer and as target
(0x122DC), and the dispatcher's kind-3 arrival branch is empty (0x8F49):
an IPBM that reaches its target does nothing. ECMAINT opens IPBM.DAT for
the run only when some unit has kind 3 (0x28695..0x286EE). The pack (0x1D88..0x1F43)
copies each chain to IPBM.TMP in chain order and renumbers `next`, `record`,
and `prev`.

| Offset | Width | Type | Name | Meaning | Evidence |
|---|---|---|---|---|---|
| 0x00 | 2 | u16 | `ipbm_number` | Copied to unit entry +10 (0x26870), the slot that holds the fleet number for fleets. No kind-3 code reads it. By analogy with FLEETS 0x00 it numbers the missile within its empire. | [C] |
| 0x02 | 1 | u8 | `owner` | Owning empire. The loader requires it to match the chain's empire for every record except the head (0x26807, 0x26964; see bug 2). | [C]; [P] |
| 0x03 | 2 | u16 | `next` | Next IPBM in the chain, 0 at the tail. The loader and ECGAME follow it (0x26906, 0x2C7C2); the pack renumbers it (0x1F13). | [C][P] |
| 0x05 | 2 | u16 | `record` | This record's own number, 1-based. The loader copies it to unit entry +6 for every record after the head (0x26A4B); the save uses it as the seek target (0x2407B); the pack rewrites it (0x1EE1). | [C][P] |
| 0x07 | 2 | u16 | `prev` | Previous IPBM, 0 at the head. Only the pack touches it (0x1EFA). Byte 0x08 is its high byte. | [C][P] |
| 0x09 | 1 | u8 | `x` | Sector x (unit entry +1). | [C][P] |
| 0x0A | 1 | u8 | `y` | Sector y (unit entry +2). | [C][P] |
| 0x0B | 6 | real | `fx` | In-sector x fraction; position x = `x + fx`. | [C][P] |
| 0x11 | 6 | real | `fy` | In-sector y fraction. | [C][P] |
| 0x17 | 6 | real | `z` | Depth: 0 deep space, 1 orbit. The loader sets unit entry +5 to `z >= 0.5` (0x268B8). | [C] |
| 0x1D | 1 | u8 | `target_x` | Target sector; the step aims at `target_x + 0.5` (0x723). | [C][P] |
| 0x1E | 1 | u8 | `target_y` | Target sector y; aimed at `target_y + 0.5` (0x744). | [C][P] |
| 0x1F | 1 | u8 | `target_z` | Target depth, clamped to 1 by the step (0x779). | [C] |

Probes ran `ECMAINT /R` on `fixtures/ecmaint-post/v1.5` with hand-built
records (empire 1 unless noted; target 20 sectors east):

- One IPBM, `record = 1`: IPBM.DAT came back unchanged, 32 bytes.
- The same IPBM stored as record 2 with `record = 1` and PLAYER +0x48 = 2:
  it advanced from x 10.5 to 17.5 (speed 7 for the year) and the pack moved it
  to record 1 with PLAYER +0x48/+0x4A = 1/1.
- IPBM at its target: no change and no report.
- Head record owned by empire 3 in empire 1's chain: accepted.
- Second record owned by empire 3: "Game file(s) missing or failed
  integrity check", maintenance aborted.
- Empire 2's IPBM as record 1, empire 1's as record 2: empire 1's IPBM was
  lost. Both chains came back holding copies of empire 2's IPBM (owner 2),
  one of them partly advanced, and two runs advanced it by different amounts.
- Two IPBMs in one chain: ECMAINT never finished; IPBM.TMP passed 340 MB.

### IPBM.DAT classic bugs

1. **The save writes one record too far.** The fleet and base save helpers
   seek to `record - 1`; the IPBM helper (0x2405D) seeks to `record`. Each
   tick an IPBM's moved state lands in the following slot while the next
   read (0x23F8D) fetches the unmoved original. An IPBM in the file's last
   record never advances: its moved copy lands past the end and the pack
   drops it. Any other IPBM overwrites the next record in the file, whoever
   owns it. When that record is the next link in the same chain, the copy's
   `next` points at itself and the pack loops forever, filling the disk.
2. **The chain head escapes the owner check.** For fleets and bases the
   loader checks the owner whenever DS:16A4 is 0, which it always is. The
   IPBM head test is inverted (0x26807), so only records after the head are
   checked (0x26964).
3. **Arrival does nothing.** The dispatcher's kind-3 branch is empty and the
   encounter skips kind 3, so a missile that reaches its target sits there.
