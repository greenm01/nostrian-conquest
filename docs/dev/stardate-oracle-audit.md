# ECMAINT Stardate Audit

`tools/ncmaint_stardate_audit.py` runs every case of
`tools/ncmaint_mission_sweep.py` (empire 1's first fleet given one of the 15
orders, at speeds 1 and 3, from 3 or 6 sectors diagonally off its target)
through classic `ECMAINT` and `nc-cli maint-rust` year by year. Each year it
compares:

- the fleet's motion state: sector, in-sector fractions, depth, speed, order,
  target, aux bytes and ROE, byte for byte;
- every empire's reports in order: recipient, sender, stardate week and
  full text.

Classic saves only at year end, so the fractions and depth pin each year's
motion to the week, and the report weeks and order show the in-year
sequence. Classic's combat dice are not reproducible: a combat report keeps
its place and week, but its numbers are masked, and a case stops counting
once combat outcomes diverge. Run it on a machine with nix:

```
nix shell nixpkgs#dosbox-x -c python3 tools/ncmaint_stardate_audit.py --years 10 --jobs 12
```

## Results (2026-10-30)

Fleet motion matches in every case up to combat, and Bombard, Invade and
Blitz strike in the same week as classic. The remaining differences are
NC's on purpose:

| orders | classic | NC |
|---|---|---|
| Bombard, Invade, Blitz (6–8) | One prose mission report to the attacker and one to a human defender, in the strike week. | The same reports in the same week, as structured briefings (`nc-reports.md`). |
| Bombard (6) | The fleet holds after one bombing run. | The fleet bombards again every year (player manual, Hostile Missions). |
| Join (13) | The joining fleet reports "We have joined the 2nd Fleet and are now merging with them." in the week it joins. | One Fleet Command Center "Join mission summary" in week 1 (`nc-reports.md`, Compact Join Summary). |
| Rendezvous (14) | "We have arrived at the our rendezvous point…" | Drops classic's stray "the". |

Fixed by this audit:

- A View World fleet's Move report carries the week it climbed out of the
  system (9436cdde).
- A scout on station over a world reports in week 1 of each later year, not
  week 22.
- Scout Solar System over an unowned world gives classic's short report:
  "We are in extended orbit around the world in System(x,y). The world is
  unowned but has a potential value of N points."
- View World over an unowned world says "unowned but has a potential".
- Bombard, Invade and Blitz send no report in the year the fleet enters the
  target sector, as in classic. Reaching orbit takes about half a year at
  speed 3, so the strike often lands the next year.
- A capture report gives the garrison, as classic's "After the smoke
  cleared, we had N armies" does.
- A guard joining a starbase that has climbed out to deep space reports
  from the sector (`--deep-base`), as classic does.

- Reports go only to empires with a human player, as in classic: none to
  rogues or to an empire in civil disorder (every classic fixture without a
  seated player has an empty RESULTS.DAT). An empire that falls or whose
  crews defect that year still gets its final reports.

Probe setup matters: the order's world flag (mission aux 0) must be set as
ECGAME sets it (`Order::targets_world`). With it clear, classic's world
orders stop in the sector and report from there.

Join (13) and Rendezvous: both merge in the week the fleets come
together, as in classic, so from that week on the joiner fights as part
of its host. NC still sends one report a year (owner, 2026-10-30): the Join
summary is dated by the first join and gives each join's week ("Fleets 3
and 4 merged into Fleet 2 (week 14)."), and the Rendezvous report names
each arrival's week when they differ. A join that waits on a host that
merged away is retargeted after the ticks and merges at year end.

Re-audit 2026-10-31 (all 60 cases, plus `--missions 14 --rendezvous`,
which sends fleet 2 to the rendezvous too): every merge lands in classic's
week (join weeks 5 and 26; rendezvous y3w49 and y5w28). The remaining
differences are wording only: NC's Join summary in place of classic's
report, classic's "at the our rendezvous point" typo, NC's "absorbing fleet
2" for classic's "absorbing the 2nd Fleet", and no separate "waiting"
report when the first rendezvous fleet arrives in the same year the second
merges. Bombard, Invade and Blitz at speed 3 differ only in combat dice.

Not covered: colonizing an occupied world, rendezvous merges of several
fleets, and the NC routing extensions in `ec-movement-spec.md`.

## Encounters (2026-10-30)

`tools/ncmaint_encounter_audit.py` sends empire 1's fleet through a sector
where empire 2's fleet patrols, under each pairing of diplomacy and ROE, and
tallies both engines' report sequences over many runs. Sightings are rolls,
so compare rates and weeks, not single runs. Classic seeds its rolls from
the clock: runs started together roll alike, so the tool runs them one at a
time. Each Rust run gets its own campaign seed.

- Motion matches in every run until a battle.
- Sighting rates and weeks agree: a patrol sights a lone cruiser crossing
  its sector in about 3 runs in 10 in both engines, in any week the cruiser
  is there (NC 18 of 64 seeds; classic 5 of 16).
- Once a sighting happens, both send the same sequence in the same week:
  contact, identification, then the interception or battle reports. NC
  sends contact and identification as one "Sensor contact -- detected and
  identified" report where classic sends two ("Sensor contact shows an alien
  fleet … Closing to check it out…", then "We have located and
  identified…"), and its battle reports are structured briefings. The one
  report carries what classic's two do: the alien's speed ("a translight
  speed of 3", or "sublight speed" at rest), whose fleet it is, its size,
  and the decision ("Ignoring alien fleet.", "Accelerating to engage the
  enemy.", or the ROE-avoidance report in its place).
- A fleet that is not closing (ROE 0 on an ordinary order, or Guard
  Starbase at its post) only reaches contact, as in classic: its report
  gives the speed but not the owner or size ("We could not identify it.").
  A later sighting by a closing fleet of the same empire sends the
  identification in its own week. NC has no sighting numbers, so it drops
  classic's "designating this sighting as FS-n". "Sublight" for a fleet at
  rest fits both classic captures; the rule itself is not in the RE notes.
- Re-audit 2026-10-31 (16 runs a case): sighting rates and weeks still
  agree, and the one report gives classic's speed wording ("a translight
  speed of 3", "sublight speed") and decision lines. Classic's contact-only
  report reads "We are designating this sighting as FS-16 and notifying all
  other fleets in the area"; NC's says "We could not identify it."
- Patrol needs warships, as in classic. A patrol that has none, or loses
  its last one in battle, aborts and holds position: "Since we have lost
  all of our warships in combat, we must abort our mission of patrolling
  Sector(x,y). We are holding position and awaiting orders."
