---
id: et03gvm9
date: 2026-10-10
kind: investigation
status: investigating
tags: [investigation]
---
# Conquest income, inherited assets, and bombardment economics

Keep NC's two income-free conversion passes and preserve surviving local assets, including unfinished builds. Conquest already rewards preparation with intact industry, savings and reusable ships. The half-income alternative adds reward earlier without solving a demonstrated pacing problem. Bombardment chiefly destroys savings; smooth growth repairs its production damage. A larger problem needs a separate decision: Rust's Scorch destroys potential permanently. Classic preserves potential and allows rebuilding. The owner leans toward classic Scorch; that preference is not yet a complete rule approval.

## t086 and recommendation to Claude

This is item4 of the [economy review](0t08owtu-economy-review-decisions-and-campaign-pacing-scope.md), with economic consequences for item5. It uses approved [smooth fractional accounting](../decisions/ubnjcchu-fractional-economy-accounting-smooth-growth-and-a-colony-head-start.md) and [starbase economics](../decisions/iq6sfxde-starbases-keep-their-growth-and-capacity-bonuses-without-stacking.md). No Rust edits, cargo, engine runs or new DOS probes. Code observations were read at d76f71b1. Exact arithmetic controls describe the approved economy, whose implementation is pending.

1. Keep 0/0/full conversion. A world captured in N receives no new growth or revenue in N+1 or N+2, then resumes in N+3. At P=K=100 and tax50, half yield in the second pass adds25 points; ten-pass revenue becomes425 instead of400. The forecast must say which pass next earns income.
2. Keep surviving production, local goods, docked assets and queue progress with the captured world. Do not reset it to colony P3, mint a capture bonus, forgive debt, or refund inherited work. Existing money can fund next-pass builds during conversion. A CA with12 points still owed completes from inherited goods50 in N+1, leaving38. Empty treasury delays that same completion to N+4 under 0/0/full, or N+3 under 0/half/full.
3. Keep the existing queue and its remaining bill visible through the existing build display and cancellation command. Report that assets and queued work were inherited. No new control or table column. Cancellation releases an unpaid commitment; it does not refund spent points. Fix misleading refund wording with the queue implementation.
4. Do not add upkeep or repair bills. Permanent hull and army losses, travel, temporary reduced strength, lost worlds and destroyed assets remain war's costs. These controls support retaining conversion timing, not a claim that campaign snowballing is solved. Bring Scorch and the bombardment rates to the owner as item5 before concluding the war-cost review.

These are proposed recommendations, not an ADR or a claimed joint approval. Claude agreed to include Scorch as a named question in item5 and requested the classic reference correction below.

## Capture contract and classic comparison

NC's successful Invade and Blitz branches in [assault.rs](../../../rust/nc-engine/src/maint/combat/assault.rs) change owner, conversion counter and surface forces. They do not clear the queue or reduce production or goods on capture. Invade clears batteries; Blitz retains surviving batteries. Earlier combat can destroy docked assets. Captured dock contents are only what survived, not a guaranteed intact fleet. This does not transfer surviving commissioned enemy fleets or bases to the conqueror.

The [build pass](../../../rust/nc-engine/src/maint/economics/build.rs) spends the world's local budget before income, without a conversion-status gate. The [planet pass](../../../rust/nc-engine/src/maint/economics/planets.rs) advances status0 and1 without growth or income. Thus conversion suspends new output, not the use of saved goods. Orders written after the capture can spend what remains; no second economic pass occurs in the capture year. Recapture resets conversion, but does not duplicate goods or paid work. Repeated yearly recaptures yield no fresh income in either schedule.

Classic's status progression is also0,1,2, but its old-status scales are0,0.5,1. That is a documented difference, not parity. More importantly, [classic capture §2.9](../../dev/ec-battle-spec.md) clears the queue and destroys assets even without Scorch. Invade retains only20–39.9% of production and normally10–49.9% of goods; Blitz uses survivor-dependent retention. Scorch causes additional losses. Comparing only conversion scales understates NC's existing capture reward. The [economy reference](../../dev/ec-economy-reference.md) now points to these ordinary mission losses.

Keep these NC differences deliberate. Do not import classic's random capture destruction merely to pay for an earlier income date. Preserving industry makes careful conquest worthwhile and distinguishes Invade from destructive Bombard. Fix the misleading ECMAINT-parity comments in both capture and conversion when the selected rule is implemented. The manual's “approximately two turns” should become the exact N/N+1/N+2/N+3 schedule.

## Exact conversion controls

Let potential be K, production P and integer percentage tax t. Ordinary growth coefficient is `a=(100-min(t,95))/400`, emergency penalty coefficient `b=max(0,t-65)/500`. At full conversion, `P'=clamp(P+a*(K-P)-b*P,0,K)` and `income=t*P'/100`. A qualifying base multiplies the growth term by its approved taper, not income or the penalty.

The comparison keeps NC's frozen status0 pass. At status1, baseline stays frozen; the half candidate uses `P'=clamp(P+0.5*a*(K-P)-b*P,0,K)` and `income=0.5*t*P'/100`. This explicitly tests half growth and income with the full emergency penalty. It is not an emulator of classic's entire formula or its status0 penalty behavior. Neither schedule changes prices or capture losses. The recommendation retains the baseline, so no new emergency-conversion exception is needed.

All table entries use tax50, no base, no purchases, no later attacks and no starting goods. “Ten passes” means N+1 through N+10, not ten income-producing passes.

| Potential / initial production | Ten-pass income: 0/0/full | Ten-pass income: 0/half/full | Production after ten: baseline / half |
| --- | ---: | ---: | ---: |
| 10 / 10 | 40.00 | 42.50 | 10.00 / 10.00 |
| 20 / 10 | 57.03 | 61.12 | 16.56 / 16.78 |
| 100 / 10 | 193.24 | 210.07 | 69.08 / 71.01 |
| 100 / 50 | 285.13 | 305.59 | 82.82 / 83.89 |
| 100 / 100 | 400.00 | 425.00 | 100.00 / 100.00 |

At P=K=100 and tax65, the second-pass bonus is32.5. The benefit is mostly earlier use of one half-year of output. On developing worlds the half-growth pass also leaves a decaying production lead. The JSON includes tax70, twenty-pass totals, fractional reserves and debt. There is no random economic variable here, so exact arithmetic is more useful than Monte Carlo.

An inherited goods balance200 is200 local points transferred, not newly created wealth. At production100 without a base, capacity limits spending to100 in a pass. A captured partially paid CA owing12 retains its three points of earlier work, rather than becoming a free15-point hull. Existing cancellation clears the slot without adding goods in [mutators_planets.rs](../../../rust/nc-data/src/directory/mutators_planets.rs). The abort screen's “fully refunded” wording is misleading: it releases outstanding commitments, not paid progress.

## War costs and conquest reward

The approved [ground calibration](4lasivnr-ground-combat-and-recovery-turns-campaign-calibration.md) found that 12 loaded transports with3 CA softening reach at least90% capture against10 starting armies, after the orbital gate and battery prerequisites are satisfied. This commits129 points and loses an expected8.98 points of armies per attempt. Without softening,20 armies commit140 and lose17.15. The defender's initial10 armies cost20.

The129 is not consumed by each conquest. Transports and escorts are reusable; surviving armies become the occupation force. Conversely,8.98 is not a complete war bill: it excludes orbital losses, battery-clearing losses, interception and the chance of losing the captured world. It is an unconditional attempt expectation, not the cost conditional on victory. Do not divide it by revenue and call that a campaign payback time.

A mature P100/tax50 world produces50 per income pass. Once fully converted, ownership produces a100-point annual revenue swing between two otherwise unchanged empires:50 denied to one and50 gained by the other. During the conversion delay the old owner still loses its50 each pass. Captured savings and surviving dock assets add an immediate transfer. This is already a strong strategic prize. Classic's half pass is not needed simply to make conquering rewarding.

Travel and RT remain real costs without a bill. CA/TT ideal orbit-to-orbit travel is approximately `(distance+2)/5` years: distance3,8,13 corresponds to1,2,3 years before interception and rounding. Crippled ships have reduced strength in later contacts until recovery. Winning intact is profitable; trading away the assault fleet can be disastrous. That rewards reconnaissance and concentration without a maintenance menu. It also permits a leader with local superiority to reuse the same fleet. Conversion delays alone cannot prove that snowballing is controlled; actual map campaigns must measure repeat conquests, counterattacks, RT exposure and permanent losses.

## Item5 bridge: bombardment at the existing rates

Let H be hits that actually reach industry after military targets. Each exchange applies `u=floor(H/10)`, removes up to u production and up to100u nonnegative goods. The same hits damage both resources; goods do not absorb the hit budget before production. Fractional-safe subtraction must retain untouched fractions and must not erase negative imported goods. This is a consequence of approved accounting, not a new attack bonus.

| Exposed hit sequence | Production lost | Goods lost from500 | Additional revenue lost over20 passes |
| --- | ---: | ---: | ---: |
| 9 | 0 | 0 | 0 |
| 10 | 1 | 100 | 3.258 |
| 30 | 3 | 300 | 9.773 |
| 9,9,9 | 0 | 0 | 0 |
| 100 | 10 | 500 | 32.578 |

These controls start at P=K=100, tax50, no base. They condition on exposed hits; they are not the odds or price of getting a fleet through the orbital and battery defenses. Approved [battery shielding](../decisions/mk1ie4e3-batteries-shield-the-stardock-and-failed-landings-hold-orbit.md) still applies. Planetary remainders expire each exchange; the approved fleet and ground carry rules do not change that.

For one lost production point, the revenue deficit after n full passes is `3.5*(1-(7/8)^n)`. Its infinite total is3.5, versus up to100 goods destroyed immediately. With an eligible base the total is13/6, approximately2.167. Losing reserves therefore dominates this example; production recovers, and bombing an already empty treasury yields much less. Capacity loss may delay a specific order and is not priced by this revenue sum.

Do not increase both damage rates merely because RT makes surviving ships free to recover. Existing reserve destruction is already large. Item5 still needs whole-mission raid-cost controls before a final rate recommendation, especially for unguarded savings and repeated low-hit exchanges. No conclusion here establishes that a cheap bombardment can always reach those reserves.

## Item5 named question: Scorch preserves potential or destroys it?

Current [Rust Scorch](../../../rust/nc-data/src/directory/mutators_planets.rs) immediately zeros potential, production and goods, and clears queue and dock. Potential0 means the world never economically recovers. This can remove the prize entirely before an expected invasion. It is a much larger denial effect than either conversion schedule and cannot be ignored in a war-cost assessment. Existing menu access is enough; no new economic order is needed to review it.

[Classic Scorch](../../dev/ec-economy-reference.md) is an order flag processed during maintenance: production retains a random30–49%, goods become `min(G/10,sqrt(G))`, and ordinary growth/income are skipped. Goods retain10% only up to G100; above that the square-root branch leaves less. Ordinary capture damage still applies; capture then additionally halves production and goods, empties the dock and clears the flag. Potential survives. The classic yearly build pass precedes this destruction, another timing fact to document if selected.

The owner said they lean toward classic after this finding. Treat that as the preferred Scorch candidate, not approval of classic capture income, ordinary capture destruction, or the whole classic economy. My recommendation for the next discussion is recoverable denial: preserve potential. We must specify flag lifetime, maintenance timing, repeated decay, capture handling and the existing menu's wording before asking for approval. Imported negative goods also need a defined behavior: do not run a square root on debt or let destruction forgive it.

A simple control shows the distinction. Under approved smooth growth at tax50, a captured world with P0,K100 reaches P12.5 on its first productive pass; P0,K0 stays0 forever. This does not reproduce classic Scorch's random retention. It isolates the consequence of destroying potential.

## Evidence and validation

- [conquest_review.py](../../../tools/economy-model/conquest_review.py) writes [conquest_review_results.json](../../../tools/economy-model/conquest_review_results.json). Exact fractions, rounded only in output. One inherited order, builds before income; no extra orders, implicit credit or global wallet.
- [test_conquest_review.py](../../../tools/economy-model/test_conquest_review.py) checks conversion dates, half growth/penalty, inherited work, delayed income spending, debt/fraction/capacity conservation, recapture, bombardment remainders, permanent potential loss and the geometric revenue formula.
- `python3 -m unittest discover -s tools/economy-model -p 'test_*.py'`:47 tests pass, including9 new checks. No new engine or classic-oracle validation is claimed. Classic statements cite existing decoded/probed references.
- No combat Monte Carlo was rerun. Prior ground costs retain their stated assumptions. Full campaign evidence remains necessary before changing prices or claiming that war costs or denial are optimally balanced.
