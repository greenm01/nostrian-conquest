---
id: b5u6lekb
date: 2026-10-10
kind: adr
status: accepted
tags: [adr, combat]
---
# Rust keeps its own combat system

## Context

Rust matches the classic binaries in movement, encounters, file layouts, and
timing. Task t036 decoded classic ECMAINT's battles
([ec-battle-spec.md](../../dev/ec-battle-spec.md)) and began porting them.
Classic's battle system is erratic and has many defects. Rust's battle and
assault mechanics were designed and tuned over a long period and are
documented in [nc-combat-spec.md](../../dev/nc-combat-spec.md) and the player
manual's combat chapters.

## Decision

Rust keeps its own combat system. It does not port classic's battle
mechanics. Classic still decides when units meet (the encounter loop,
sighting, and intercept in [ec-encounter-spec.md](../../dev/ec-encounter-spec.md))
and when a hostile fleet strikes (the tick its depth reaches orbit). Rust's
combat decides the fight and what an assault does to the order: bombardment
persists each year until the player changes the order, and invasion and
blitz resolve, win or lose, after which the fleet holds. There is no yearly
retry after a failed attack.

## Alternatives

Port classic's battles, minus its bugs. Rejected: the classic system is
worse than Rust's, and the project already chose its combat rules.

## Consequences

ec-battle-spec.md stays as a reference to classic's behaviour, not a
contract. [Rust fixes classic battle bugs](sb0sfjgt-rust-fixes-classic-battle-bugs.md)
no longer applies and is superseded. The yearly hostile retry added in t031
is removed under t036, with the manual and movement spec.

## Acceptance and connections

Accepted by the user on 2026-10-10.
