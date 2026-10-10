---
id: ld2f252j
date: 2026-10-10
kind: adr
status: accepted
tags: [adr, combat, balance]
---
# Flat combat effectiveness with battle-local carry

## Context

The owner approved combat decision 1 on 2026-10-10 after the joint Claude
and Codex recommendation and the [t058 controlled sweep](../investigations/qjev5wpq-review-of-the-t055-combat-sweep-semantics.md#t058).
The ratio-column CRT amplifies force superiority twice. Discarding small
hit budgets creates per-exchange damage thresholds; residual rolls replace
those thresholds with extra lotteries.

## Decision

Use the EotS effectiveness table without force-ratio columns:

| Unmodified d10 | Effectiveness |
| --- | ---: |
| 0–2 | 25% |
| 3–5 | 50% |
| 6–9 | 100% |

The unmodified mean is 62.5%. Qualitative modifiers and critical behavior
are separate choices; this approval does not import every switch from the
sweep's `flat-carry` preset.

Carry unused incoming hits separately for each target side and fire pool
within one `fight_encounter`, including its withdrawal/pursuit exchange.
Debit the budget when it buys a damage step. Clear all unspent hits when
the encounter ends for any reason, including the 64-round cap. Every new
encounter starts at zero, even against the same empire in the same tick.

If future participation rules let part of a side leave, its remaining side
keeps the budget. Do not duplicate it or transfer it to departing fleets.
There is no empire-wide or layer-wide bank. Carry neither adds to nor
reverses completed cripple and destruction steps. Persistent hull damage
remains decision 3.

A nominal ship costs full printed DS to cripple. Finishing a crippled
ship costs full printed DS again. Execution fire that destroys a nominal
ship must pay both steps. Do not halve the finishing cost.

## Alternatives and consequences

The [sweep tables](../../../tools/combat-model/nc_sweep_results.md) compare
EotS-rate full and half DS, intermediate 0.45-rate full and half DS, and
VBAM-rate carry with half DS. With the tested candidate settings, the
selected baseline takes 4.79 exchanges for equal 10-CA sides and gives
11 CA against 10 about 67.1% victory. These are model results, not promises
for an engine that retains other combat rules.

The 0.45/half-DS alternative is credible but adds less memorable
percentages and a finishing exception without a clear role improvement.
VBAM-rate carry/half DS gives 11 against 10 about 77% victory: a strong
advantage, not near-certainty. Its destroyers improve substantially with
DS 8, so their weakness at baseline values is not an argument against
all lower-rate designs.

Carry rewards sustained contact. Sequential small attacks cannot pool
sub-threshold progress across encounters, and withdrawal can forfeit
unspent hits. Reports and the manual must explain this reset. Combat
exchanges resolve within a tick; extra exchanges cost no movement time.
Carry removes discard thresholds, not defeat or finite-horizon limits.

## Scope and implementation

Accepted policy; engine implementation is pending. The
[combat spec](../../dev/nc-combat-spec.md) still describes the existing
ratio CRT until that implementation and its manual changes land.

This approval does not change hull values or prices, ROE, pursuit,
screening, participation, surprise, persistent damage, repair, or ground
combat. DD 10/8 remains a decision-7 playtest. Do not apply fleet sweep
odds to planetary assaults or silently change their specialized damage
rules.

Implementation must test carry spending, separate fire pools, encounter
reset after withdrawal and the cap, and a fresh subsequent encounter.
Update the player manual's combat explanation and Appendix B alongside
the engine, and compare the chosen implemented rule combination against
the model. The [source synthesis](../investigations/ebrogywq-nc-combat-design-review-against-its-sources.md#decisions-for-the-user)
tracks the remaining owner decisions. This refines
[NC combat stays](b5u6lekb-rust-keeps-its-own-combat-system.md).

## Implementation (t060)

The engine now fights fleet battles on this table: each side's volley reads
the modified d10 and deals `ceil(AS × effectiveness)` hits. The decision left
qualitative modifiers to implementation. The former column shifts became die
modifiers on the same conditions: +1 for a mixed fleet (any two of DD, CA,
BB) and +1 for a side with a starbase, at most +2, with the modified roll
capped at 9. The withdrawal exchange, formerly fixed at the Pressed column,
reads the table unmodified. The unmodified-9 critical still destroys one
extra unit. Unspent hits carry per side and fire pool on the side's task
force, so they end with `fight_encounter`. Finishing already cost full DS.
Planetary combat keeps the ratio CRT, and execution fire, target order,
screening, ROE timing, and withdrawal rules are unchanged. The engine's
`combat_odds` example gives 11 CA against 10 a 67.5% victory rate (2,000
trials, 95% interval 65.4–69.5%).
