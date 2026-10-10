# ECMAINT Hostile Arrival Audit

Controlled delayed hostile-world probes comparing Rust maintenance against classic `ECMAINT`.

Current takeaways from this probe set:

- `BombardWorld`, `InvadeWorld`, and `BlitzWorld` keep their order and travel speed while the fleet descends to orbit.
- A hostile fleet strikes on the tick its depth reaches `1.0`. If the attack fails, classic keeps the state from the tick before, so the depth stays just under `1.0` (for a one-sector straight path, `19=80 1a=b9 1b=ff 1c=ff 1d=ff 1e=7f`), and the fleet strikes again the next year. Rust does not copy that retry: its fleet stops in orbit (`19=81`, speed 0), where Bombard stands and Invade and Blitz hold.
- The strike year matches classic. NC combat decides the battle and the order afterward, so the turns after the strike differ from classic by design.

Scope of this probe set:

- one-sector delayed hostile arrivals
- `BombardWorld`, `InvadeWorld`, and `BlitzWorld`
- one follow-up ready-resolution tick after arrival
- `BlitzWorld` uses a strong combined-arms fleet so the second tick exercises a valid assault path

| case | rust arrival | classic arrival | arrival byte match | turn-by-turn match |
| --- | ---: | ---: | --- | --- |
| bombard-delayed | 1 | 1 | yes | no |
| invade-delayed | 1 | 1 | yes | yes |
| blitz-delayed-strong | 1 | 1 | yes | yes |

## bombard-delayed

- source scenario: `bombard`
- fleet record: `3`
- Rust arrival turn: `1`
- Classic arrival turn: `1`
- Rust arrival bytes: `19=80 1a=b9 1b=ff 1c=ff 1d=ff 1e=7f`
- Classic arrival bytes: `19=80 1a=b9 1b=ff 1c=ff 1d=ff 1e=7f`
- Rust resolution trace: `15,13 order=bombard spd=0 eta=arrived 19=81 1a=00 1b=00 1c=00 1d=00 1e=00`
- Classic resolution trace: `15,13 order=hold spd=0 eta=arrived 19=81 1a=00 1b=00 1c=00 1d=00 1e=00`
- turn-by-turn match: `no`

| turn | Rust | Classic |
| ---: | --- | --- |
| 0 | `16,13 order=bombard spd=3 eta=unreachable 19=81 1a=00 1b=00 1c=00 1d=00 1e=00` | `16,13 order=bombard spd=3 eta=unreachable 19=81 1a=00 1b=00 1c=00 1d=00 1e=00` |
| 1 | `15,13 order=bombard spd=3 eta=unreachable 19=80 1a=b9 1b=ff 1c=ff 1d=ff 1e=7f` | `15,13 order=bombard spd=3 eta=unreachable 19=80 1a=b9 1b=ff 1c=ff 1d=ff 1e=7f` |
| 2 | `15,13 order=bombard spd=0 eta=arrived 19=81 1a=00 1b=00 1c=00 1d=00 1e=00` | `15,13 order=hold spd=0 eta=arrived 19=81 1a=00 1b=00 1c=00 1d=00 1e=00` |

## invade-delayed

- source scenario: `invade`
- fleet record: `3`
- Rust arrival turn: `1`
- Classic arrival turn: `1`
- Rust arrival bytes: `19=80 1a=b9 1b=ff 1c=ff 1d=ff 1e=7f`
- Classic arrival bytes: `19=80 1a=b9 1b=ff 1c=ff 1d=ff 1e=7f`
- Rust resolution trace: `15,13 order=hold spd=0 eta=arrived 19=81 1a=00 1b=00 1c=00 1d=00 1e=00`
- Classic resolution trace: `15,13 order=hold spd=0 eta=arrived 19=81 1a=00 1b=00 1c=00 1d=00 1e=00`
- turn-by-turn match: `yes`

| turn | Rust | Classic |
| ---: | --- | --- |
| 0 | `16,13 order=invade spd=3 eta=unreachable 19=81 1a=00 1b=00 1c=00 1d=00 1e=00` | `16,13 order=invade spd=3 eta=unreachable 19=81 1a=00 1b=00 1c=00 1d=00 1e=00` |
| 1 | `15,13 order=invade spd=3 eta=unreachable 19=80 1a=b9 1b=ff 1c=ff 1d=ff 1e=7f` | `15,13 order=invade spd=3 eta=unreachable 19=80 1a=b9 1b=ff 1c=ff 1d=ff 1e=7f` |
| 2 | `15,13 order=hold spd=0 eta=arrived 19=81 1a=00 1b=00 1c=00 1d=00 1e=00` | `15,13 order=hold spd=0 eta=arrived 19=81 1a=00 1b=00 1c=00 1d=00 1e=00` |

## blitz-delayed-strong

- source scenario: `invade`
- fleet record: `3`
- Rust arrival turn: `1`
- Classic arrival turn: `1`
- Rust arrival bytes: `19=80 1a=b9 1b=ff 1c=ff 1d=ff 1e=7f`
- Classic arrival bytes: `19=80 1a=b9 1b=ff 1c=ff 1d=ff 1e=7f`
- Rust resolution trace: `15,13 order=hold spd=0 eta=arrived 19=81 1a=00 1b=00 1c=00 1d=00 1e=00`
- Classic resolution trace: `15,13 order=hold spd=0 eta=arrived 19=81 1a=00 1b=00 1c=00 1d=00 1e=00`
- turn-by-turn match: `yes`

| turn | Rust | Classic |
| ---: | --- | --- |
| 0 | `16,13 order=blitz spd=3 eta=unreachable 19=81 1a=00 1b=00 1c=00 1d=00 1e=00` | `16,13 order=blitz spd=3 eta=unreachable 19=81 1a=00 1b=00 1c=00 1d=00 1e=00` |
| 1 | `15,13 order=blitz spd=3 eta=unreachable 19=80 1a=b9 1b=ff 1c=ff 1d=ff 1e=7f` | `15,13 order=blitz spd=3 eta=unreachable 19=80 1a=b9 1b=ff 1c=ff 1d=ff 1e=7f` |
| 2 | `15,13 order=hold spd=0 eta=arrived 19=81 1a=00 1b=00 1c=00 1d=00 1e=00` | `15,13 order=hold spd=0 eta=arrived 19=81 1a=00 1b=00 1c=00 1d=00 1e=00` |
