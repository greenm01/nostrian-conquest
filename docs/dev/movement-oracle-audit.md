# ECMAINT Movement Audit

Controlled `MoveOnly` probes comparing Rust maintenance against classic `ECMAINT`.

Current takeaways from this probe set:

- Classic moves fleets in 52 ticks a year; each tick a fleet covers `speed / 52` sectors in a straight line. The model is in `docs/dev/ec-movement-spec.md`.
- A `MoveOnly` order completes in the year the fleet enters the target sector, and the fleet ETA reports that year.
- On arrival classic sets the order to `hold` and the speed to `0`.
- A fleet that starts at rest in orbit spends its first sector of movement climbing out, so speed `v` covers `v - 1` sectors in its first year.

| case | speed | start | target | rust ETA | rust arrival | classic arrival | trace match |
| --- | ---: | --- | --- | ---: | ---: | ---: | --- |
| speed3-horizontal | 3 | `10,10` | `16,10` | 3 | 3 | 3 | yes |
| speed3-diagonal | 3 | `10,10` | `16,16` | 3 | 3 | 3 | yes |
| speed6-diagonal | 6 | `10,10` | `16,16` | 2 | 2 | 2 | yes |
| speed1-diagonal | 1 | `10,10` | `13,13` | 5 | 5 | 5 | yes |
| speed3-shallow | 3 | `10,10` | `16,12` | 3 | 3 | 3 | yes |
| speed3-steep | 3 | `10,10` | `12,16` | 3 | 3 | 3 | yes |

## speed3-horizontal

- speed: `3`
- start: `10,10`
- target: `16,10`
- initial Rust ETA: `3`
- Rust arrival turn: `3`
- Classic arrival turn: `3`
- turn-by-turn match: `yes`

| turn | Rust | Classic |
| ---: | --- | --- |
| 0 | `10,10 order=move spd=3 eta=3` | `10,10 order=move spd=3 eta=3` |
| 1 | `12,10 order=move spd=3 eta=2` | `12,10 order=move spd=3 eta=2` |
| 2 | `15,10 order=move spd=3 eta=1` | `15,10 order=move spd=3 eta=1` |
| 3 | `16,10 order=hold spd=0 eta=arrived` | `16,10 order=hold spd=0 eta=arrived` |

## speed3-diagonal

- speed: `3`
- start: `10,10`
- target: `16,16`
- initial Rust ETA: `3`
- Rust arrival turn: `3`
- Classic arrival turn: `3`
- turn-by-turn match: `yes`

| turn | Rust | Classic |
| ---: | --- | --- |
| 0 | `10,10 order=move spd=3 eta=3` | `10,10 order=move spd=3 eta=3` |
| 1 | `11,11 order=move spd=3 eta=2` | `11,11 order=move spd=3 eta=2` |
| 2 | `14,14 order=move spd=3 eta=1` | `14,14 order=move spd=3 eta=1` |
| 3 | `16,16 order=hold spd=0 eta=arrived` | `16,16 order=hold spd=0 eta=arrived` |

## speed6-diagonal

- speed: `6`
- start: `10,10`
- target: `16,16`
- initial Rust ETA: `2`
- Rust arrival turn: `2`
- Classic arrival turn: `2`
- turn-by-turn match: `yes`

| turn | Rust | Classic |
| ---: | --- | --- |
| 0 | `10,10 order=move spd=6 eta=2` | `10,10 order=move spd=6 eta=2` |
| 1 | `14,14 order=move spd=6 eta=1` | `14,14 order=move spd=6 eta=1` |
| 2 | `16,16 order=hold spd=0 eta=arrived` | `16,16 order=hold spd=0 eta=arrived` |

## speed1-diagonal

- speed: `1`
- start: `10,10`
- target: `13,13`
- initial Rust ETA: `5`
- Rust arrival turn: `5`
- Classic arrival turn: `5`
- turn-by-turn match: `yes`

| turn | Rust | Classic |
| ---: | --- | --- |
| 0 | `10,10 order=move spd=1 eta=5` | `10,10 order=move spd=1 eta=5` |
| 1 | `10,10 order=move spd=1 eta=4` | `10,10 order=move spd=1 eta=4` |
| 2 | `11,11 order=move spd=1 eta=3` | `11,11 order=move spd=1 eta=3` |
| 3 | `11,11 order=move spd=1 eta=2` | `11,11 order=move spd=1 eta=2` |
| 4 | `12,12 order=move spd=1 eta=1` | `12,12 order=move spd=1 eta=1` |
| 5 | `13,13 order=hold spd=0 eta=arrived` | `13,13 order=hold spd=0 eta=arrived` |
| 6 | `13,13 order=hold spd=0 eta=arrived` | `13,13 order=hold spd=0 eta=arrived` |

## speed3-shallow

- speed: `3`
- start: `10,10`
- target: `16,12`
- initial Rust ETA: `3`
- Rust arrival turn: `3`
- Classic arrival turn: `3`
- turn-by-turn match: `yes`

| turn | Rust | Classic |
| ---: | --- | --- |
| 0 | `10,10 order=move spd=3 eta=3` | `10,10 order=move spd=3 eta=3` |
| 1 | `12,11 order=move spd=3 eta=2` | `12,11 order=move spd=3 eta=2` |
| 2 | `15,12 order=move spd=3 eta=1` | `15,12 order=move spd=3 eta=1` |
| 3 | `16,12 order=hold spd=0 eta=arrived` | `16,12 order=hold spd=0 eta=arrived` |

## speed3-steep

- speed: `3`
- start: `10,10`
- target: `12,16`
- initial Rust ETA: `3`
- Rust arrival turn: `3`
- Classic arrival turn: `3`
- turn-by-turn match: `yes`

| turn | Rust | Classic |
| ---: | --- | --- |
| 0 | `10,10 order=move spd=3 eta=3` | `10,10 order=move spd=3 eta=3` |
| 1 | `11,12 order=move spd=3 eta=2` | `11,12 order=move spd=3 eta=2` |
| 2 | `12,15 order=move spd=3 eta=1` | `12,15 order=move spd=3 eta=1` |
| 3 | `12,16 order=hold spd=0 eta=arrived` | `12,16 order=hold spd=0 eta=arrived` |
