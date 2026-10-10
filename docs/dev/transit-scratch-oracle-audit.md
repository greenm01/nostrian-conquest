# ECMAINT Transit Scratch Audit

Controlled in-transit probes comparing Rust maintenance against classic `ECMAINT`, focused on the movement scratch windows `0x0d..0x12` and `0x19..0x1e`.

Current takeaways from this probe set:

- `0x0d..0x12` and `0x13..0x18` hold the fleet's in-sector x and y fractions, and `0x19..0x1e` its depth, all as Turbo Pascal 6-byte reals.
- In deep-space transit the depth is `0`, so `0x19..0x1e` reads all zero. A fleet still climbing out of orbit in its first year leaves a small residue such as `5b 00 00 00 00 40`.
- Sector orders complete in the year the fleet enters the target sector, anywhere in it.

| case | transit turns | classic transit `19..1e` all zero | transit byte match | visible movement match |
| --- | --- | --- | --- | --- |
| move-only-speed3-horizontal | `1, 2` | yes | yes | yes |
| move-only-speed3-diagonal | `1, 2` | yes | yes | yes |
| move-only-speed1-diagonal | `1, 2, 3, 4` | no | yes | yes |
| move-only-speed3-shallow | `1, 2` | yes | yes | yes |
| patrol-speed3-axial | `1` | yes | yes | yes |
| guard-starbase-speed3-axial | `1` | yes | yes | yes |
| guard-blockade-speed3-axial | `1` | yes | yes | yes |

## move-only-speed3-horizontal

- order code: `1`
- speed: `3`
- start: `10,10`
- target: `16,10`
- transit turns checked: `1, 2`
- classic transit `19..1e` all zero: `yes`
- transit byte match: `yes`

| turn | Rust | Classic |
| ---: | --- | --- |
| 0 | `10,10 order=move spd=3 aux=[00, 00] eta=3 0d..12=80/00/00/00/00/00 19..1e=81/00/00/00/00/00` | `10,10 order=move spd=3 aux=[00, 00] eta=3 0d..12=80/00/00/00/00/00 19..1e=81/00/00/00/00/00` |
| 1 | `12,10 order=move spd=3 aux=[00, 00] eta=2 0d..12=7f/c0/fe/ff/ff/7f 19..1e=00/00/00/00/00/00` | `12,10 order=move spd=3 aux=[00, 00] eta=2 0d..12=7f/c0/fe/ff/ff/7f 19..1e=00/00/00/00/00/00` |
| 2 | `15,10 order=move spd=3 aux=[00, 00] eta=1 0d..12=7f/c0/fc/ff/ff/7f 19..1e=00/00/00/00/00/00` | `15,10 order=move spd=3 aux=[00, 00] eta=1 0d..12=7f/c0/fc/ff/ff/7f 19..1e=00/00/00/00/00/00` |
| 3 | `16,10 order=hold spd=0 aux=[00, 00] eta=arrived 0d..12=7b/00/64/d8/89/1d 19..1e=00/00/00/00/00/00` | `16,10 order=hold spd=0 aux=[00, 00] eta=arrived 0d..12=7b/00/64/d8/89/1d 19..1e=00/00/00/00/00/00` |

## move-only-speed3-diagonal

- order code: `1`
- speed: `3`
- start: `10,10`
- target: `16,16`
- transit turns checked: `1, 2`
- classic transit `19..1e` all zero: `yes`
- transit byte match: `yes`

| turn | Rust | Classic |
| ---: | --- | --- |
| 0 | `10,10 order=move spd=3 aux=[00, 00] eta=3 0d..12=80/00/00/00/00/00 19..1e=81/00/00/00/00/00` | `10,10 order=move spd=3 aux=[00, 00] eta=3 0d..12=80/00/00/00/00/00 19..1e=81/00/00/00/00/00` |
| 1 | `11,11 order=move spd=3 aux=[00, 00] eta=2 0d..12=80/30/68/e6/09/6a 19..1e=00/00/00/00/00/00` | `11,11 order=move spd=3 aux=[00, 00] eta=2 0d..12=80/30/68/e6/09/6a 19..1e=00/00/00/00/00/00` |
| 2 | `14,14 order=move spd=3 aux=[00, 00] eta=1 0d..12=7c/00/47/00/8c/11 19..1e=00/00/00/00/00/00` | `14,14 order=move spd=3 aux=[00, 00] eta=1 0d..12=7c/00/47/00/8c/11 19..1e=00/00/00/00/00/00` |
| 3 | `16,16 order=hold spd=0 aux=[00, 00] eta=arrived 0d..12=7c/00/2c/d1/30/0d 19..1e=00/00/00/00/00/00` | `16,16 order=hold spd=0 aux=[00, 00] eta=arrived 0d..12=7c/00/2c/d1/30/0d 19..1e=00/00/00/00/00/00` |
| 4 | `16,16 order=hold spd=0 aux=[00, 00] eta=arrived 0d..12=7c/00/2c/d1/30/0d 19..1e=00/00/00/00/00/00` | `16,16 order=hold spd=0 aux=[00, 00] eta=arrived 0d..12=7c/00/2c/d1/30/0d 19..1e=00/00/00/00/00/00` |

## move-only-speed1-diagonal

- order code: `1`
- speed: `1`
- start: `10,10`
- target: `13,13`
- transit turns checked: `1, 2, 3, 4`
- classic transit `19..1e` all zero: `no`
- transit byte match: `yes`

| turn | Rust | Classic |
| ---: | --- | --- |
| 0 | `10,10 order=move spd=1 aux=[00, 00] eta=5 0d..12=80/00/00/00/00/00 19..1e=81/00/00/00/00/00` | `10,10 order=move spd=1 aux=[00, 00] eta=5 0d..12=80/00/00/00/00/00 19..1e=81/00/00/00/00/00` |
| 1 | `10,10 order=move spd=1 aux=[00, 00] eta=4 0d..12=80/00/00/00/00/00 19..1e=5b/00/00/00/00/40` | `10,10 order=move spd=1 aux=[00, 00] eta=4 0d..12=80/00/00/00/00/00 19..1e=5b/00/00/00/00/40` |
| 2 | `11,11 order=move spd=1 aux=[00, 00] eta=3 0d..12=7e/00/cc/cc/13/54 19..1e=00/00/00/00/00/00` | `11,11 order=move spd=1 aux=[00, 00] eta=3 0d..12=7e/00/cc/cc/13/54 19..1e=00/00/00/00/00/00` |
| 3 | `11,11 order=move spd=1 aux=[00, 00] eta=2 0d..12=80/00/66/e6/09/6a 19..1e=00/00/00/00/00/00` | `11,11 order=move spd=1 aux=[00, 00] eta=2 0d..12=80/00/66/e6/09/6a 19..1e=00/00/00/00/00/00` |
| 4 | `12,12 order=move spd=1 aux=[00, 00] eta=1 0d..12=80/00/99/d9/0e/1f 19..1e=00/00/00/00/00/00` | `12,12 order=move spd=1 aux=[00, 00] eta=1 0d..12=80/00/99/d9/0e/1f 19..1e=00/00/00/00/00/00` |
| 5 | `13,13 order=hold spd=0 aux=[00, 00] eta=arrived 0d..12=78/00/00/52/ab/07 19..1e=00/00/00/00/00/00` | `13,13 order=hold spd=0 aux=[00, 00] eta=arrived 0d..12=78/00/00/52/ab/07 19..1e=00/00/00/00/00/00` |
| 6 | `13,13 order=hold spd=0 aux=[00, 00] eta=arrived 0d..12=78/00/00/52/ab/07 19..1e=00/00/00/00/00/00` | `13,13 order=hold spd=0 aux=[00, 00] eta=arrived 0d..12=78/00/00/52/ab/07 19..1e=00/00/00/00/00/00` |

## move-only-speed3-shallow

- order code: `1`
- speed: `3`
- start: `10,10`
- target: `16,12`
- transit turns checked: `1, 2`
- classic transit `19..1e` all zero: `yes`
- transit byte match: `yes`

| turn | Rust | Classic |
| ---: | --- | --- |
| 0 | `10,10 order=move spd=3 aux=[00, 00] eta=3 0d..12=80/00/00/00/00/00 19..1e=81/00/00/00/00/00` | `10,10 order=move spd=3 aux=[00, 00] eta=3 0d..12=80/00/00/00/00/00 19..1e=81/00/00/00/00/00` |
| 1 | `12,11 order=move spd=3 aux=[00, 00] eta=2 0d..12=7f/c0/6b/a2/73/4b 19..1e=00/00/00/00/00/00` | `12,11 order=move spd=3 aux=[00, 00] eta=2 0d..12=7f/c0/6b/a2/73/4b 19..1e=00/00/00/00/00/00` |
| 2 | `15,12 order=move spd=3 aux=[00, 00] eta=1 0d..12=7e/80/25/2c/42/79 19..1e=00/00/00/00/00/00` | `15,12 order=move spd=3 aux=[00, 00] eta=1 0d..12=7e/80/25/2c/42/79 19..1e=00/00/00/00/00/00` |
| 3 | `16,12 order=hold spd=0 aux=[00, 00] eta=arrived 0d..12=7a/00/d8/e1/47/1e 19..1e=00/00/00/00/00/00` | `16,12 order=hold spd=0 aux=[00, 00] eta=arrived 0d..12=7a/00/d8/e1/47/1e 19..1e=00/00/00/00/00/00` |

## patrol-speed3-axial

- order code: `3`
- speed: `3`
- start: `8,10`
- target: `11,10`
- transit turns checked: `1`
- classic transit `19..1e` all zero: `yes`
- transit byte match: `yes`

| turn | Rust | Classic |
| ---: | --- | --- |
| 0 | `8,10 order=patrol spd=3 aux=[00, 00] eta=2 0d..12=80/00/00/00/00/00 19..1e=81/00/00/00/00/00` | `8,10 order=patrol spd=3 aux=[00, 00] eta=2 0d..12=80/00/00/00/00/00 19..1e=81/00/00/00/00/00` |
| 1 | `10,10 order=patrol spd=3 aux=[00, 00] eta=1 0d..12=7f/c0/fe/ff/ff/7f 19..1e=00/00/00/00/00/00` | `10,10 order=patrol spd=3 aux=[00, 00] eta=1 0d..12=7f/c0/fe/ff/ff/7f 19..1e=00/00/00/00/00/00` |
| 2 | `11,10 order=patrol spd=0 aux=[00, 00] eta=arrived 0d..12=7b/00/84/d8/89/1d 19..1e=00/00/00/00/00/00` | `11,10 order=patrol spd=0 aux=[00, 00] eta=arrived 0d..12=7b/00/84/d8/89/1d 19..1e=00/00/00/00/00/00` |
| 3 | `11,10 order=patrol spd=0 aux=[00, 00] eta=arrived 0d..12=7b/00/84/d8/89/1d 19..1e=00/00/00/00/00/00` | `11,10 order=patrol spd=0 aux=[00, 00] eta=arrived 0d..12=7b/00/84/d8/89/1d 19..1e=00/00/00/00/00/00` |

## guard-starbase-speed3-axial

- order code: `4`
- speed: `3`
- start: `8,8`
- target: `11,8`
- transit turns checked: `1`
- classic transit `19..1e` all zero: `yes`
- transit byte match: `yes`

| turn | Rust | Classic |
| ---: | --- | --- |
| 0 | `8,8 order=guard_starbase spd=3 aux=[01, 01] eta=unreachable 0d..12=80/00/00/00/00/00 19..1e=81/00/00/00/00/00` | `8,8 order=guard_starbase spd=3 aux=[01, 01] eta=unreachable 0d..12=80/00/00/00/00/00 19..1e=81/00/00/00/00/00` |
| 1 | `10,8 order=guard_starbase spd=3 aux=[01, 01] eta=unreachable 0d..12=7f/c0/fe/ff/ff/7f 19..1e=00/00/00/00/00/00` | `10,8 order=guard_starbase spd=3 aux=[00, 01] eta=unreachable 0d..12=7f/c0/fe/ff/ff/7f 19..1e=00/00/00/00/00/00` |
| 2 | `11,8 order=guard_starbase spd=0 aux=[01, 01] eta=unreachable 0d..12=80/00/00/00/00/00 19..1e=81/00/00/00/00/00` | `11,8 order=guard_starbase spd=0 aux=[00, 01] eta=unreachable 0d..12=7b/00/84/d8/89/1d 19..1e=00/00/00/00/00/00` |
| 3 | `11,8 order=guard_starbase spd=0 aux=[01, 01] eta=unreachable 0d..12=80/00/00/00/00/00 19..1e=81/00/00/00/00/00` | `11,8 order=guard_starbase spd=0 aux=[00, 01] eta=unreachable 0d..12=7b/00/84/d8/89/1d 19..1e=00/00/00/00/00/00` |

## guard-blockade-speed3-axial

- order code: `5`
- speed: `3`
- start: `8,8`
- target: `11,8`
- transit turns checked: `1`
- classic transit `19..1e` all zero: `yes`
- transit byte match: `yes`

| turn | Rust | Classic |
| ---: | --- | --- |
| 0 | `8,8 order=guard_blockade spd=3 aux=[01, 00] eta=unreachable 0d..12=80/00/00/00/00/00 19..1e=81/00/00/00/00/00` | `8,8 order=guard_blockade spd=3 aux=[01, 00] eta=unreachable 0d..12=80/00/00/00/00/00 19..1e=81/00/00/00/00/00` |
| 1 | `10,8 order=guard_blockade spd=3 aux=[01, 00] eta=unreachable 0d..12=7f/c0/fe/ff/ff/7f 19..1e=00/00/00/00/00/00` | `10,8 order=guard_blockade spd=3 aux=[01, 00] eta=unreachable 0d..12=7f/c0/fe/ff/ff/7f 19..1e=00/00/00/00/00/00` |
| 2 | `11,8 order=guard_blockade spd=0 aux=[01, 00] eta=arrived 0d..12=80/00/00/00/00/00 19..1e=81/00/00/00/00/00` | `11,8 order=guard_blockade spd=0 aux=[01, 00] eta=arrived 0d..12=80/00/00/00/00/00 19..1e=81/00/00/00/00/00` |
| 3 | `11,8 order=guard_blockade spd=0 aux=[01, 00] eta=arrived 0d..12=80/00/00/00/00/00 19..1e=81/00/00/00/00/00` | `11,8 order=guard_blockade spd=0 aux=[01, 00] eta=arrived 0d..12=80/00/00/00/00/00 19..1e=81/00/00/00/00/00` |

_Generated by `tools/ncmaint_transit_scratch_audit.py`._
