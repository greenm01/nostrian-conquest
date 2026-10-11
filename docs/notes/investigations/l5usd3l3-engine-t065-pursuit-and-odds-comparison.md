---
id: l5usd3l3
date: 2026-10-11
kind: investigation
status: investigating
tags: [investigation, combat]
---
# Engine t065 pursuit and odds comparison

The new `engine-t065` preset models one fresh encounter with after-first-exchange ROE, automatic unarmed withdrawal, speed-gated pursuit, ordinary screening and DD DS8. Eight of nine supplied engine cases pass the existing comparison. All win-rate and casualty-tolerance checks pass; the remaining failure is rare mutual destruction in the mixed-fleet case. An initial apparent ROE failure came from a stale engine harness file and disappears in Claude's regenerated data. Keep both files as evidence. No Rust changes or cargo runs were made.

## t091 and inputs

Claude supplied engine odds for main7c31f736: first4,000 trials per case, then20,000 with the corrected harness. The JSON uses map seed1515, combat seeds starting at1, and up to six maintenance years to obtain the first reported battle. Two trials of equal10CA produced no battle within that window, so that row has19,998 observations. Odds are conditional on recorded battles; this does not validate encounter frequency or map pacing.

Files:

- [nc_engine_odds_t065.json](../../../tools/combat-model/nc_engine_odds_t065.json): corrected20,000-trial input. SHA256 `2f918dc9ca7460693f59dc184fcbd03f8e46318349f9c476ab6960ce3b894220`.
- [nc_engine_odds_t065_before_harness_fix.json](../../../tools/combat-model/nc_engine_odds_t065_before_harness_fix.json): original4,000-trial input. SHA256 `0e1c46f61a654bf949123fde25368cfbef01acef6450f711c027abef472ef7d5`.
- [nc_t065_comparison.json](../../../tools/combat-model/nc_t065_comparison.json) and [table](../../../tools/combat-model/nc_t065_comparison.md):20,000 model samples per case, seed20261010, compared against corrected engine data.
- [before-fix comparison](../../../tools/combat-model/nc_t065_comparison_before_harness_fix.json) and [table](../../../tools/combat-model/nc_t065_comparison_before_harness_fix.md): same model run compared against the stale input.
- [mixed-fleet follow-up](../../../tools/combat-model/nc_t065_mixed_probe.json) and [table](../../../tools/combat-model/nc_t065_mixed_probe.md):100,000 additional model trials, independent seed20261011. This is a targeted uncertainty check after a failure, not a replacement chosen to make the main comparison pass.

Older CRT and t060 results remain unchanged. Sources are the merged [fleet battle](../../../rust/nc-engine/src/maint/combat/fleet_battle.rs), [pursuit exchange](../../../rust/nc-engine/src/maint/combat/exchange.rs), [intercept formula](../../../rust/nc-engine/src/maint/encounter/rules.rs) and [harness](../../../rust/nc-fuzz/examples/combat_odds.rs). The new code extends the existing [sweep](../../../tools/combat-model/sweep.py), [rules](../../../tools/combat-model/rules.py) and [model](../../../tools/combat-model/fleet_model.py).

## Preset contract and scope

The preset inherits t060's flat25/50/100 effectiveness, battle-local carry, full DS finishing, mixed/base modifiers, natural-9 free kill and critical target order. It changes ROE to after each exchange, enables unarmed escape and uses speed pursuit instead of the old withdrawal exchange. Both sides leaving means no pursuit. A side containing a base does not withdraw, matching the current engine; a base without ships cannot pursue. The default unit variant is `dd-10-8-5`. Historical baseline DD DS5 remains available and is not overwritten.

Pursuit uses the slowest surviving ship class: DD/SC6, CA/TT5, BB4, ETAC3. Starbases are excluded from ship speed. Losing the last slow hull can change the next pursuit chance; crippling a hull does not change its speed. An armed ship-bearing pursuer catches a slower fleeing side with certainty. Otherwise the engine tests `U2 <= sin(pi*U1)*pursuer_speed/fleeing_speed`, with the contact already established and no awareness term. Under independent uniform draws its integrated chance is `2*pursuer_speed/(pi*fleeing_speed)`. Equal speeds give63.661977%; BB speed4 pursuing DD speed6 gives42.441318%. A faster DD pursuing CA catches certainly.

The model branches on this integrated probability. A successful catch fires one ordinary exchange, preserving this encounter's hit banks and ordinary target screening. Failed catch ends without another exchange. It models one fresh escape episode and one aggregated fleet per side. It does not persist episode allowances across ticks, model detection or movement contact, or reproduce seeded engine draws. The full per-fleet, per-opposing-empire episode contract remains engine state, not a claim of this single-encounter model. Repeated-contact callers must not assume this preset enforces a previously spent episode allowance.

Sweep defaults now honor the preset's unit variant even for cases without destroyers, and engine comparison selects that same variant. Direct model callers must load the preset's `default_units`; supplying an explicit alternate unit table remains an experiment, not engine parity.

## Corrected comparison

| Case | Model left win | Engine left win | Comparison |
| --- | ---: | ---: | --- |
| 10CA vs10CA | 50.1% | 49.7% | Pass |
| 11CA vs10CA | 65.6% | 65.8% | Pass |
| 15CA vs10CA | 96.6% | 96.8% | Pass |
| 20CA vs10CA | 100.0% | 100.0% | Pass |
| 30DD vs10CA | 31.5% | 31.4% | Pass |
| 90DD vs10BB | 23.4% | 24.2% | Pass |
| 30CA vs10BB | 39.6% | 40.3% | Pass |
| 29CA+3DD vs30CA | 58.7% | 58.3% | Mutual-destruction check fails |
| 10CA ROE6 vs15CA | 0.0% | 0.0% | Pass |

“Pass” retains the existing criteria: overlapping95% intervals for each sampled outcome, and each side's mean destroyed cost within2% of its initial build points. It is not proof of identical distributions. The comparison checks all four outcomes, so a matching left-win rate cannot hide a mutual or unresolved discrepancy. Loss dispersion is absent from the engine file, so the loss check is an engineering tolerance rather than a statistical confidence test.

The mixed case has24 mutual destructions in20,000 model samples versus8 in20,000 engine samples. The independent larger model probe has103 in100,000:0.103%, Wilson95% interval0.08494–0.12489%. The engine estimate is0.040%; its interval narrowly fails to overlap that larger probe. All other outcome/loss tests in the main row pass. This is a rare but unresolved difference, not an established Rust defect or evidence to change the approved rules. A larger engine probe or event-level reproduction is the next diagnostic; no thresholds were relaxed here.

## Stale withdrawal result and remaining harness weakness

The first supplied file reported0% left wins,57.25% right wins and42.75% stalemates for10CA ROE6 versus15CA. Claude confirmed it was generated before the harness counted `PursuitFire` as withdrawal. The old classifier mislabeled1,710 caught withdrawals. With the corrected harness the outcome is0/100/0/0, matching the model. This was a measurement defect, not failed withdrawal balance.

An exact single-state enumeration of this ROE case gives1.6366197724 exchanges and destroyed costs66.1734959020 /4.2883946030. Every path breaks off after the first exchange; the extra0.63662 exchange is the equal-speed catch probability. The sampled model gives66.10/4.27 and the corrected engine about64.5/4.3. That is within the existing loss tolerance, but the difference is not demonstrated to be mere sampling noise. Event-level volley and catch counts would distinguish conditional contact/RNG effects from a resolution difference. Do not claim exact casualty agreement.

The current harness still chooses first battle reports by sector, then searches the whole year's disposition events there by empire, without matching the battle tick or fleet. That can associate a later disposition with an earlier battle. This is a source-level fragility, not the cause established for the stale file. Claude acknowledged it separately. Any follow-up should identify the encounter consistently, not only count additional event variants.

## Reproduction and checks

Run:

```sh
python3 tools/combat-model/sweep.py --rules engine-t065 --cases engine --trials 20000 --exact-limit 0 --no-ground --engine tools/combat-model/nc_engine_odds_t065.json --engine-rules engine-t065 --json tools/combat-model/nc_t065_comparison.json --markdown tools/combat-model/nc_t065_comparison.md
python3 tools/combat-model/sweep.py --rules engine-t065 --cases '29 CA + 3 DD vs 30 CA' --trials 100000 --seed 20261011 --exact-limit 0 --no-ground --json tools/combat-model/nc_t065_mixed_probe.json --markdown tools/combat-model/nc_t065_mixed_probe.md
python3 -m unittest discover -s tools/combat-model -p 'test_*.py'
```

Tests:88 run,87 pass and1 existing optional scientific-toolchain check skipped. Six new [t065 checks](../../../tools/combat-model/test_t065.py) cover the DS8 default, speed-law probabilities, surviving slow hulls, crippled speed, retained banks during pursuit, no extra volley on failed/mutual withdrawal, immobile bases, unarmed escape and comparison unit selection. No Rust edits or cargo. The copied JSON hashes match their supplied sources.

The comparison work is ready for review; the rare-outcome and event-association questions remain explicitly open. Connects to the approved [withdrawal and pursuit ADR](../decisions/kiswrzos-withdrawal-pursuit-and-one-strength-yardstick.md) and [DD durability ADR](../decisions/8pc4t661-destroyers-become-ds-8-for-the-first-playtest.md).
