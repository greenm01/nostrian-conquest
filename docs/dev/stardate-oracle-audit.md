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

Open: classic sends no reports to an empire in civil disorder; NC still
sends such an empire a planet's alert when its world is attacked.

Probe setup matters: the order's world flag (mission aux 0) must be set as
ECGAME sets it (`Order::targets_world`). With it clear, classic's world
orders stop in the sector and report from there.

Not covered: Guard Starbase's base over a non-friendly world (the sweep's
`--deep-base`), colonizing an occupied world, encounters with other fleets,
and the NC routing extensions in `ec-movement-spec.md`.
