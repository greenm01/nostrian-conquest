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

On macOS, `brew install dosbox-x` (the formula; the cask is disabled) and
run the script directly. Flags beyond the case matrix:

- `--variant enemy|home|world|sector` aims the order at empire 2's world,
  empire 1's home world, the unowned world or deep space;
- `--base-move world|sector` creates a starbase at the target and orders it
  to move;
- `--retarget MISSION:X,Y[:YEAR]` gives fleet 1 a new order after a year,
  as a player would; the case fails if `fleet-order` and the classic patch
  change different motion fields;
- `--rendezvous N` sends fleets 2..N to the rendezvous too.

Cases faster than the fleet's ships are skipped. Hold (order 0) targets the
fleet's start; offset `d0` starts it on its target.

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

## Coverage audit (2026-10-31)

Run on the Mac mini with Homebrew DOSBox-X.

Matching classic:

- Hold in orbit, in deep space, and with a speed left on the order.
- Starbase moves to deep space, alone and with a guard, week by week.
  A base ordered to a world its empire does not own refuses in week 1
  ("Since we lost control of planet…"), in both engines.
- Order changes mid-route: Move to a new sector, to Patrol, to Seek Home,
  and a Move after a View.
- Guard/Blockade at an enemy world.
- Three fleets at a rendezvous: same weeks and hosts; wording as above.
- The full matrix (orders 0–15, speeds 1–6, offsets d3, d6, s3 and s6; 384
  cases, 64 skipped as too fast) adds no new kind of difference. Bombard,
  Invade and Blitz differ only once combat dice diverge. Join and Rendezvous
  differ in wording only.

Dice, not a bug: a guard escorting a base whose move is refused ends the
year at depth 51/52 or 1.0 depending on the unit order. Classic splits
about evenly over six runs at each speed.

Fixed (t220). Classic checks the target world's owner during the year, in
the fleet's dispatch after its step (`movement/target_world.rs`); NC did
not. Every case below now matches classic week by week:

| order and target | classic, and now NC | NC before |
|---|---|---|
| View our own world | Week 1: "Since we now own the world in System(x,y), we are aborting our viewing mission…", then a Move to that sector. | Kept viewing. |
| Scout System our own world | On arrival: "Since we now control System(x,y), we are aborting our scouting mission…", then Hold. | Scouted every year. |
| Colonize our own world | Week 1, before its first step: "Since we now own the world in System(x,y), we are holding our current position…", then Hold. | Flew on. |
| Colonize an occupied world | Reports in the week it enters the system, then turns to a Move and climbs out to the sector. | Reported on reaching orbit, and stayed there. |
| Bombard, Invade, Blitz our own world | Flies on as Guard/Blockade, or as a Move if unarmed or at ROE 0 (`ec-battle-spec.md` §2.1). | Cancelled to Hold at the start of the year. |

The check runs every tick, so a bombarding fleet whose sister fleet takes
the world guards it from the next tick. A Move that ends in orbit now
reports from "System(x,y)", as classic does. NC drops classic's
"posistion" typo.

Reports matched to classic at the owner's request (2026-10-31):

- The View report names an owned world (`analysis of planet "Name".`) and
  keeps "the world found within" for an unowned one.
- A fleet lost in its own assault gets classic's report: "We have lost all
  contact with the 1st Fleet shortly after it began its bombing run against
  planet "N" in System(x,y), currently owned by E. According to the fleet's
  last report, the target world was defended by … We have confirmed that
  they managed to destroy … Records show that the 1st Fleet was composed of
  …, with all hands presumed lost." Blitz reads "blitz attempt" and "owned
  by". Invade follows the spec ("invasion", §4); no classic run has shown
  it yet. The fleet's make-up stays in NC's notation (CA1), by the
  owner's choice (2026-10-31).

Re-audit after the fix (mini, 2026-10-31): the 64 default cases give only
the differences listed above, and Hold matches in all four. The `home`
variant for orders 5–9, 11 and 12 matches in every case except for the
"posistion" typo. The `enemy` variant matches apart from combat dice. When
the attacker dies, the lost-fleet report matches classic word for word,
except for the force notation and the masked damage.

NC's "Estimated production" for classic's "Estimated present production"
is deliberate (7bd10af9).

Probe artifact: `fleet-location` keeps orbit depth, so a start in an empty
sector sits at depth 1 and reports "System(x,y)". Both engines see the same
start, so the comparison holds, but the deep-space Hold cases start at depth
0.

Not covered: the NC routing extensions in `ec-movement-spec.md`, which
classic does not have.

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

## Alerts (2026-10-31)

Scratch probes at empire 1's home (16,13), 30 classic runs one at a time
and 30 to 40 NC runs on their own seeds, after the question of what a
starbase's or a fleet's alert does for the other fleets in the system. The
answer is the same in both engines: the sighting raises the empire's
awareness of that alien to "identified", so the other fleets skip their own
contact roll and go straight to interception when their own sighting roll
succeeds. No order changes. Earlier parallel classic runs shared a clock
seed and overstated the differences; the serial runs show these.

- Response after an alert agrees. A Hold fleet (DD4, ROE 6) in orbit
  attacks a bombarding cruiser 0 to 3 weeks after the starbase's alert in
  both engines (classic gaps 0:9 1:6 2:9 3:4 5:1 of 29; NC 0:5 1:7 2:12
  3:3 and four of 6 to 8 of 33), and the same after a Guard fleet's alert
  instead. With the cruiser holding in orbit and the base alerting, the
  gap is longer in both, classic median 4 weeks, NC 3.
- Chasing a faster raider agrees. A Guard/Blockade fleet (CA3 DD2, max 5)
  beside the base intercepts a bombarding destroyer (max 6) in classic
  28 of 30 runs and NC 36 of 40, in the same weeks. The battle's end
  differs, classic kills the destroyer in 16 of 28 and NC in 36 of 36,
  which is combat resolution and outside this audit.
- A raider's abort against a Hold fleet is NC's orbital gate
  (`ec-combat-reference.md`, "The orbital gate"): the owner's fleet with
  warships in orbit fights the attacker at the strike tick, and a raider
  whose ROE refuses the fight seeks home without a shot. Classic lets only
  the starbase block: the cruiser fights the destroyers in the encounter
  loop and bombards anyway in 29 of 30 runs (its own batteries then destroy
  it). NC bombards in 0 of 40. This was the one real difference. The
  owner then chose classic's gate and pairing
  ([bd8trz18](../notes/decisions/bd8trz18-battles-pair-one-fleet-with-one-target-as-classic-does.md),
  t226): only the starbase and its guards block a strike now.
- A Patrol fleet (DD4, ROE 0) and a starbase watching a cruiser move into
  orbit first report it in the same weeks: the base in week 33 (classic)
  or 34 (NC) at the median, the patrol in 4 (classic) and 7 (NC) of 30
  runs before the base does.
