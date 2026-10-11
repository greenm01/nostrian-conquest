---
id: twexjjwm
date: 2026-10-10
kind: investigation
status: investigated
tags: [investigation, combat, assault, collateral]
---
# Assault collateral and planetary support after the ground decision

Put batteries first and protect the dock until an exchange starts with no batteries. Keep the planetary support CRT and Bombard's economic damage in this decision. Changing those rates would change siege balance far more than the collateral correction. An orbital defeat triggers withdrawal; a failed landing alone does not. A surviving assault fleet keeps its position on Hold, with an explicit failure report. It does not automatically acquire a blockade order. These are recommendations for decision 5, not another ADR.

## t072

Review delayed battery shielding, the support table, industrial damage and failed-assault behavior. Use exact bombardment controls. Preserve chronological tick resolution from t052; do not restore a mission-class priority or annual ownership limit. No Rust edits and no cargo.

## Proposed contract

At each bombardment exchange, snapshot whether batteries stand. If they do, spend its planetary hit budget only on batteries at 20 hits each. Discard the excess, including hits left after destroying the last battery. There is no planetary carry. The dock becomes eligible at the next exchange that starts without batteries. It is not exposed during the exchange that breaks the battery line.

Keep the current three-exchange bombardment shape. The first two exchanges suppress military assets. The third can reach armies and the economy only if it starts without batteries. Clearing the final battery in exchange three does not retroactively permit breakthrough. Invade's suppression and Blitz's cover already target batteries; this proposal does not make them attack the dock or industry.

Player wording: “While batteries stand, bombardment targets them. Docked ships become exposed in the next exchange after the batteries are destroyed.” This avoids the ambiguity of “once the batteries are gone,” which could mean halfway through an exchange.

Retain existing dock item DS and slot order for now. This fixes docked assets acting as armor for batteries. It does not claim to remove every allocation-order choice; dock-slot collateral priority is still visible in the current allocator.

## Exact bombardment controls

The model enumerates independent d10 rolls across the three exchanges, including return fire and changes to surviving attackers. Shots use the force at round start; destroying the firing ships does not cancel their already computed volley. It retains current bombard AS (DD 5, CA 30, BB 135 nominal), execution fire from batteries, natural-9 criticals, and current ratio modifiers. Fleet AS/DS and prices remain unchanged. It has no ROE checks inside the three bombardment exchanges, matching the current planetary routine. It assumes the orbital gate was already won.

| Attacking fleet; planet | Current: attacker loss / dock loss | Delayed battery-first: attacker loss / dock loss |
| --- | ---: | ---: |
| 1 BB; 2 batteries, empty dock | 22.72 / 0 | 22.72 / 0 |
| 1 BB; 2 batteries, docked BB | 25.69 / 38.25 | 22.72 / 22.28 |
| 6 CA; 3 batteries, docked BB | 82.35 / 33.47 | 80.24 / 7.61 |
| 3 BB; 3 batteries, docked BB | 40.50 / 45.00 | 40.50 / 40.50 |

All losses are expected build points. A docked BB costs 45. The world also starts with 10 armies, 500 stored points and production 100. Adding the docked BB no longer raises the attacking BB's losses: the dock no longer delays destruction of the guns firing back. Protecting the dock for the entire breaking exchange also matters: same-exchange spillover would destroy 26.07 dock points in the 6-CA case, compared with 7.61 under the delayed shield. Attacker losses are 80.24 in both battery-first variants, but 82.35 under the current dock-first rule.

These are exact model expectations, not sampled confidence intervals. The tool has not been compared with Rust bombardment output. Tests check the last-battery boundary, the third-exchange gate, dock-independent battery protection, and a closed-form undefended target.

## Keep support fire separate

Changing support to flat 25/50/100 while adding the delayed shield cuts 6 CA's losses against the three-battery world from 80.24 to 41.44 points. It cuts the lone BB's losses against two batteries from 22.72 to 5.71. This would substantially weaken battery deterrence, even with their printed AS unchanged.

Flat support also changes bombardment's economic output. One BB against production 100, no batteries, no dock and no armies destroys 22.9 production under the current CRT, versus 7.9 with flat support. With ten armies shielding the surface, those means are 12.9 and 1.2. It is not a cosmetic simplification or a necessary consequence of the ground decision.

Keep support fire on the CRT in decision 5. Revisit the table jointly with battery AS/DS and bombardment weights in decision 7 if warranted. The earlier [ground calibration](4lasivnr-ground-combat-and-recovery-turns-campaign-calibration.md) already shows that replacing the softening table changes capture odds. Approximate ratio stability belongs to the approved landing rule, not to every step of an assault.

## Economic destruction and mission purpose

Keep economic destruction as Bombard's strategic role. Invade and Blitz preserve industry; they are routes to taking a productive world. Bombard can deny production even when the attacker cannot afford a landing force. That gives reconnaissance, orbital defense and counterattack a purpose beyond ownership counts.

State the current rate accurately. After eligible military losses, each complete ten remaining hits removes **one point of current production and 100 stored production points**, each capped by what exists. The two losses use the same residual hit count. Goods do not absorb hits before production; describing this as a literal goods-then-factories budget cascade is misleading. Potential production is not the quantity reduced here.

One undefended BB raid wipes all 500 stored points in the stated example and removes 22.9 current production on average. Three BB against ten armies remove 59.4 production on average. These are severe attacks. Do not silently change the rate inside a collateral patch. The [economy investigation](tkce84f0-a-simple-nc-economy-informed-by-classic-ec-vbam-and-ec4x.md), task t051, should examine the replenishment and recovery cost before choosing another rate. Existing reports should show production and goods destroyed separately. No new management screen is needed.

## Failure is not orbital defeat

A fleet beaten in the orbital gate follows decision 2: withdraw, climb out, retain its escape episode and face only the allowed speed/contact-gated pursuit. It does not gain assault permission. Do not add a compulsory battery volley for leaving.

A failed ground battle, or an Invade stopped by surviving batteries, does not itself defeat the surviving orbital fleet. Preserve today's Hold outcome and report the failure, retained loaded armies where applicable, and the fleet's location. The player may keep the position, withdraw, or issue Guard/Blockade next turn. Batteries do not initiate another battle merely because the fleet stays. Later enemy fleets use ordinary encounter rules. Holding is not immunity or a promise that the fleet will remain unchallenged until the next year.

Two distinctions matter for the manual. Hold is not Guard/Blockade and grants none of that order's special observation or engagement rules. Also, the 20x visibility factor belongs to an assault order at its target. When the mission ends on Hold it becomes 1x, while the ordinary layer-1 hull visibility factor of 5 remains. Staying in enemy orbit is deliberate, but calling that state a persistent 20x assault or an automatic blockade would be false.

## Joint recommendation and remaining approval

Claude and Codex agree on delayed battery-first protection; current support CRT; current economic damage with explicit reports and rate review in t051; and the distinction between losing orbit and failing a landing. The visibility and Hold/blockade wording above corrects the preliminary discussion. Owner approval is still required for decision 5. No engine behavior was changed here.

## Evidence and connections

- [Exact model](../../../tools/combat-model/assault_collateral.py), [results](../../../tools/combat-model/nc_assault_collateral.json), [tests](../../../tools/combat-model/test_assault_collateral.py).
- [Current assault implementation](../../../rust/nc-engine/src/maint/combat/assault.rs) and [encounter visibility](../../../rust/nc-engine/src/maint/encounter/rules.rs), read during the t039 migration before its phase-5 replacement. These links identify the inspected implementation, not a claim that the future tree remains identical.
- [Withdrawal and strength ADR](../decisions/kiswrzos-withdrawal-pursuit-and-one-strength-yardstick.md), [ground ADR](../decisions/59v787ut-defender-first-ground-combat-on-the-shared-table.md), and [source synthesis](ebrogywq-nc-combat-design-review-against-its-sources.md) define the scope and neighboring decisions.
