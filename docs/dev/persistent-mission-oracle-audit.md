# ECMAINT Persistent Mission Audit

Controlled standing-mission probes comparing Rust maintenance against classic `ECMAINT`.

Current takeaways from this probe set:

- All three standing missions keep their order on arrival and stop the fleet: `current_speed` becomes `0`.
- `PatrolSector` targets the sector, so it settles in deep space at depth `0`. `GuardBlockadeWorld` targets the world and settles in orbit at depth `1.0` (`19=81 1a..1e=00`).
- `GuardStarbase` takes its depth from the starbase: a starbase in deep space makes classic clear `mission_aux[0]` to `00` and settle the guard at depth `0`.
- Classic does this by moving the starbase: a base in orbit with no friendly world beneath it climbs to deep space, and its guards follow its depth. Rust does not move starbases yet (t037), so the guard-starbase case differs.

Scope of this probe set:

- `PatrolSector`
- `GuardStarbase`
- `GuardBlockadeWorld`
- controlled axial `speed=3` arrivals with one post-arrival maintenance tick

The goal is not full mission-combat semantics. It is to pin down:

- whether classic preserves the standing order after arrival
- whether classic preserves the fleet speed after arrival
- what classic writes into the `0x19..0x1e` arrival-state scratch window
- whether the next maintenance tick still treats the fleet as armed on that order

| case | rust arrival | classic arrival | arrival byte match | turn-by-turn match |
| --- | ---: | ---: | --- | --- |
| patrol-speed3-axial | 2 | 2 | yes | yes |
| guard-starbase-speed3-axial | 2 | 2 | no | no |
| guard-blockade-speed3-axial | 2 | 2 | yes | yes |

## patrol-speed3-axial

- order code: `3`
- speed: `3`
- start: `8,10`
- target: `11,10`
- initial Rust ETA: `2`
- Rust arrival turn: `2`
- Classic arrival turn: `2`
- Rust arrival bytes: `19=00 1a=00 1b=00 1c=00 1d=00 1e=00`
- Classic arrival bytes: `19=00 1a=00 1b=00 1c=00 1d=00 1e=00`
- Rust post-arrival trace: `11,10 order=patrol spd=0 aux=[00, 00] eta=arrived 19=00 1a=00 1b=00 1c=00 1d=00 1e=00`
- Classic post-arrival trace: `11,10 order=patrol spd=0 aux=[00, 00] eta=arrived 19=00 1a=00 1b=00 1c=00 1d=00 1e=00`
- turn-by-turn match: `yes`

| turn | Rust | Classic |
| ---: | --- | --- |
| 0 | `8,10 order=patrol spd=3 aux=[00, 00] eta=2 19=81 1a=00 1b=00 1c=00 1d=00 1e=00` | `8,10 order=patrol spd=3 aux=[00, 00] eta=2 19=81 1a=00 1b=00 1c=00 1d=00 1e=00` |
| 1 | `10,10 order=patrol spd=3 aux=[00, 00] eta=1 19=00 1a=00 1b=00 1c=00 1d=00 1e=00` | `10,10 order=patrol spd=3 aux=[00, 00] eta=1 19=00 1a=00 1b=00 1c=00 1d=00 1e=00` |
| 2 | `11,10 order=patrol spd=0 aux=[00, 00] eta=arrived 19=00 1a=00 1b=00 1c=00 1d=00 1e=00` | `11,10 order=patrol spd=0 aux=[00, 00] eta=arrived 19=00 1a=00 1b=00 1c=00 1d=00 1e=00` |
| 3 | `11,10 order=patrol spd=0 aux=[00, 00] eta=arrived 19=00 1a=00 1b=00 1c=00 1d=00 1e=00` | `11,10 order=patrol spd=0 aux=[00, 00] eta=arrived 19=00 1a=00 1b=00 1c=00 1d=00 1e=00` |

## guard-starbase-speed3-axial

- order code: `4`
- speed: `3`
- start: `8,8`
- target: `11,8`
- initial Rust ETA: `None`
- Rust arrival turn: `2`
- Classic arrival turn: `2`
- Rust arrival bytes: `19=81 1a=00 1b=00 1c=00 1d=00 1e=00`
- Classic arrival bytes: `19=00 1a=00 1b=00 1c=00 1d=00 1e=00`
- Rust post-arrival trace: `11,8 order=guard_starbase spd=0 aux=[01, 01] eta=unreachable 19=81 1a=00 1b=00 1c=00 1d=00 1e=00`
- Classic post-arrival trace: `11,8 order=guard_starbase spd=0 aux=[00, 01] eta=unreachable 19=00 1a=00 1b=00 1c=00 1d=00 1e=00`
- turn-by-turn match: `no`

| turn | Rust | Classic |
| ---: | --- | --- |
| 0 | `8,8 order=guard_starbase spd=3 aux=[01, 01] eta=unreachable 19=81 1a=00 1b=00 1c=00 1d=00 1e=00` | `8,8 order=guard_starbase spd=3 aux=[01, 01] eta=unreachable 19=81 1a=00 1b=00 1c=00 1d=00 1e=00` |
| 1 | `10,8 order=guard_starbase spd=3 aux=[01, 01] eta=unreachable 19=00 1a=00 1b=00 1c=00 1d=00 1e=00` | `10,8 order=guard_starbase spd=3 aux=[00, 01] eta=unreachable 19=00 1a=00 1b=00 1c=00 1d=00 1e=00` |
| 2 | `11,8 order=guard_starbase spd=0 aux=[01, 01] eta=unreachable 19=81 1a=00 1b=00 1c=00 1d=00 1e=00` | `11,8 order=guard_starbase spd=0 aux=[00, 01] eta=unreachable 19=00 1a=00 1b=00 1c=00 1d=00 1e=00` |
| 3 | `11,8 order=guard_starbase spd=0 aux=[01, 01] eta=unreachable 19=81 1a=00 1b=00 1c=00 1d=00 1e=00` | `11,8 order=guard_starbase spd=0 aux=[00, 01] eta=unreachable 19=00 1a=00 1b=00 1c=00 1d=00 1e=00` |

## guard-blockade-speed3-axial

- order code: `5`
- speed: `3`
- start: `8,8`
- target: `11,8`
- initial Rust ETA: `None`
- Rust arrival turn: `2`
- Classic arrival turn: `2`
- Rust arrival bytes: `19=81 1a=00 1b=00 1c=00 1d=00 1e=00`
- Classic arrival bytes: `19=81 1a=00 1b=00 1c=00 1d=00 1e=00`
- Rust post-arrival trace: `11,8 order=guard_blockade spd=0 aux=[01, 00] eta=arrived 19=81 1a=00 1b=00 1c=00 1d=00 1e=00`
- Classic post-arrival trace: `11,8 order=guard_blockade spd=0 aux=[01, 00] eta=arrived 19=81 1a=00 1b=00 1c=00 1d=00 1e=00`
- turn-by-turn match: `yes`

| turn | Rust | Classic |
| ---: | --- | --- |
| 0 | `8,8 order=guard_blockade spd=3 aux=[01, 00] eta=unreachable 19=81 1a=00 1b=00 1c=00 1d=00 1e=00` | `8,8 order=guard_blockade spd=3 aux=[01, 00] eta=unreachable 19=81 1a=00 1b=00 1c=00 1d=00 1e=00` |
| 1 | `10,8 order=guard_blockade spd=3 aux=[01, 00] eta=unreachable 19=00 1a=00 1b=00 1c=00 1d=00 1e=00` | `10,8 order=guard_blockade spd=3 aux=[01, 00] eta=unreachable 19=00 1a=00 1b=00 1c=00 1d=00 1e=00` |
| 2 | `11,8 order=guard_blockade spd=0 aux=[01, 00] eta=arrived 19=81 1a=00 1b=00 1c=00 1d=00 1e=00` | `11,8 order=guard_blockade spd=0 aux=[01, 00] eta=arrived 19=81 1a=00 1b=00 1c=00 1d=00 1e=00` |
| 3 | `11,8 order=guard_blockade spd=0 aux=[01, 00] eta=arrived 19=81 1a=00 1b=00 1c=00 1d=00 1e=00` | `11,8 order=guard_blockade spd=0 aux=[01, 00] eta=arrived 19=81 1a=00 1b=00 1c=00 1d=00 1e=00` |
