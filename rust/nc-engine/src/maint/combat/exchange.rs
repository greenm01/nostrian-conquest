use crate::maint::encounter::rules::speed_roll_catches;
use crate::{GameRng, RNG_TAG_COMBAT};

use super::state::{
    CarriedHits, FleetCombatState, IDX_BB, IDX_CA, IDX_DD, IDX_SB, IDX_TT, fleet_auxiliary_order,
    fleet_battery_line_order, fleet_class_ds, fleet_combat_line_order, fleet_combat_line_present,
    fleet_target_order,
};

pub(super) const GROUND_AS_BATTERY: u32 = 90;
/// A battery takes two steps, like a ship: this many hits cripple it, and as
/// many again destroy it (t202).
pub(super) const GROUND_DS_BATTERY: u32 = 66;
pub(super) const GROUND_AS_ARMY: u32 = 10;
pub(super) const GROUND_DS_ARMY: u32 = 10;
pub(super) const COMBAT_GUARDRAIL_MAX_ROUNDS: u32 = 64;

const COLUMN_DISADVANTAGED: usize = 0;
const COLUMN_PRESSED: usize = 1;
const COLUMN_EVEN: usize = 2;
const COLUMN_ADVANTAGED: usize = 3;
const COLUMN_OVERWHELMING: usize = 4;

pub(super) const COMBAT_KIND_FLEET: u64 = 1;
pub(super) const COMBAT_KIND_PURSUIT: u64 = 2;
pub(super) const COMBAT_KIND_BOMBARD: u64 = 3;
pub(super) const COMBAT_KIND_INVASION_SUPPRESSION: u64 = 4;
pub(super) const COMBAT_KIND_INVASION_SOFTEN: u64 = 5;
pub(super) const COMBAT_KIND_GROUND: u64 = 6;
pub(super) const COMBAT_KIND_BLITZ_COVER: u64 = 7;
pub(super) const COMBAT_KIND_BLITZ_GROUND: u64 = 8;
pub(super) const COMBAT_KIND_BLITZ_LANDING: u64 = 9;
pub(super) const COMBAT_KIND_PURSUIT_CATCH: u64 = 10;
pub(super) const COMBAT_KIND_ALLOCATION: u64 = 11;
pub(super) const COMBAT_KIND_CAPTURE: u64 = 12;

/// Planetary combat's ratio CRT: one row per d10 value and one column per
/// force-ratio band, in quarters. Ships firing at the batteries and the
/// batteries' fire at ships read it in full; ships firing at the garrison and
/// the batteries' fire at descending transports read it in twelfths, a third
/// of its strength (t202).
const CRT_MULTIPLIER_X12: [[u8; 5]; 10] = [
    [0, 1, 2, 3, 4],
    [1, 2, 3, 4, 5],
    [1, 2, 4, 5, 6],
    [2, 3, 4, 5, 6],
    [2, 3, 4, 6, 7],
    [2, 4, 5, 6, 7],
    [3, 4, 5, 6, 8],
    [3, 4, 6, 7, 8],
    [4, 5, 6, 7, 8],
    [4, 6, 7, 8, 10],
];

/// Fleet battles' effectiveness in quarters of the side's AS, indexed by the
/// modified d10: 0-2 give 25%, 3-5 give 50%, 6-9 give 100%.
const FLEET_EFFECTIVENESS_X4: [u8; 10] = [1, 1, 1, 2, 2, 2, 4, 4, 4, 4];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct ExchangeResolution {
    pub(super) roll: u8,
    pub(super) critical: bool,
    pub(super) hits: u32,
}

fn apply_nominal_hits_to_fleet_classes(
    state: &mut FleetCombatState,
    hits: &mut u32,
    target_order: &[usize],
) -> bool {
    let mut progress = false;
    for &idx in target_order {
        let ds = fleet_class_ds(idx);
        // A class with nothing left to cripple is skipped; a class that
        // stands but cannot be afforded covers the ones behind it, and the
        // hits carry (t206).
        if ds == 0 || state.nominal_count(idx) == 0 {
            continue;
        }
        if *hits < ds {
            break;
        }
        let reducible = state.nominal_count(idx).min(*hits / ds);
        state.cripple(idx, reducible);
        *hits -= reducible * ds;
        progress = true;
    }
    progress
}

fn apply_destroyed_hits_to_fleet_classes(
    state: &mut FleetCombatState,
    hits: &mut u32,
    target_order: &[usize],
) -> bool {
    let mut progress = false;
    for &idx in target_order {
        let ds = fleet_class_ds(idx);
        if ds == 0 || state.crippled_count(idx) == 0 {
            continue;
        }
        if *hits < ds {
            break;
        }
        let destroyed = state.crippled_count(idx).min(*hits / ds);
        state.finish_crippled(idx, destroyed);
        *hits -= destroyed * ds;
        progress = true;
    }
    progress
}

fn base_combat_column(our_as: u32, enemy_as: u32) -> usize {
    if enemy_as == 0 || our_as >= enemy_as.saturating_mul(3) {
        COLUMN_OVERWHELMING
    } else if our_as.saturating_mul(2) < enemy_as {
        COLUMN_DISADVANTAGED
    } else if our_as < enemy_as {
        COLUMN_PRESSED
    } else if our_as.saturating_mul(2) < enemy_as.saturating_mul(3) {
        COLUMN_EVEN
    } else {
        COLUMN_ADVANTAGED
    }
}

fn hits_from_multiplier(as_total: u32, multiplier_x4: u8) -> u32 {
    (as_total.saturating_mul(multiplier_x4 as u32)).div_ceil(4)
}

/// A fleet battle's luck: one d10 per side and exchange, the draws of a
/// pursuit, and the stream that breaks ties when a side's losses are shared
/// among its fleets. Maintenance seeds them from the battle context; tests
/// script them.
pub(super) trait BattleDice {
    fn d10(&self, kind: u64, round: u32, acting_empire: u8, target_empire: u8) -> u8;
    /// Two draws in [0, 1) for `pursuer`'s chase of `fleeing` in `round`:
    /// the intercept arc, then the roll against it.
    fn pursuit(&self, kind: u64, round: u32, pursuer: u8, fleeing: u8) -> (f64, f64);
    /// The stream that breaks allocation ties for `empire`'s side after its
    /// exchange of `round`. Scripted dice get a fixed stream unless they
    /// script one.
    fn allocation(&self, kind: u64, round: u32, empire: u8) -> GameRng {
        GameRng::from_context(
            0,
            RNG_TAG_COMBAT,
            &[kind, u64::from(round), u64::from(empire)],
        )
    }
}

/// The dice of a maintenance battle: every draw is keyed by the campaign
/// seed and the battle context, so a seed always replays the same battle.
pub(super) struct SeededDice {
    pub(super) campaign_seed: u64,
    pub(super) game_year: u16,
    pub(super) coords: [u8; 2],
}

impl SeededDice {
    fn rng(&self, kind: u64, round: u32, acting: u8, target: u8) -> GameRng {
        GameRng::from_context(
            self.campaign_seed,
            RNG_TAG_COMBAT,
            &[
                self.game_year as u64,
                self.coords[0] as u64,
                self.coords[1] as u64,
                kind,
                round as u64,
                acting as u64,
                target as u64,
            ],
        )
    }
}

impl BattleDice for SeededDice {
    fn d10(&self, kind: u64, round: u32, acting_empire: u8, target_empire: u8) -> u8 {
        self.rng(kind, round, acting_empire, target_empire)
            .roll_d10()
    }

    fn pursuit(&self, kind: u64, round: u32, pursuer: u8, fleeing: u8) -> (f64, f64) {
        let mut rng = self.rng(kind, round, pursuer, fleeing);
        let arc = rng.next_real();
        (arc, rng.next_real())
    }

    fn allocation(&self, kind: u64, round: u32, empire: u8) -> GameRng {
        self.rng(kind, round, empire, 0)
    }
}

/// Whether a pursuer whose ships make `pursuer_speed` catches a side
/// fleeing at `fleeing_speed`, by classic's intercept rule (`1b05:03b1`): a
/// faster pursuer catches for certain, otherwise the speed roll decides.
/// The fleeing side is in contact and running, so the intercept's
/// awareness branch does not apply.
pub(super) fn pursuit_catches(pursuer_speed: u8, fleeing_speed: u8, draws: (f64, f64)) -> bool {
    pursuer_speed > fleeing_speed
        || speed_roll_catches(pursuer_speed, fleeing_speed, draws.0, draws.1)
}

/// The d10 for one side's volley. The battle context keys it, so a seed
/// always replays the same battle.
pub(super) fn seeded_combat_roll(
    campaign_seed: u64,
    game_year: u16,
    coords: [u8; 2],
    combat_kind: u64,
    round: u32,
    acting_empire: u8,
    target_empire: u8,
) -> u8 {
    let mut rng = GameRng::from_context(
        campaign_seed,
        RNG_TAG_COMBAT,
        &[
            game_year as u64,
            coords[0] as u64,
            coords[1] as u64,
            combat_kind,
            round as u64,
            acting_empire as u64,
            target_empire as u64,
        ],
    );
    rng.roll_d10()
}

#[allow(clippy::too_many_arguments)]
fn seeded_exchange_resolution(
    campaign_seed: u64,
    game_year: u16,
    coords: [u8; 2],
    combat_kind: u64,
    round: u32,
    acting_empire: u8,
    target_empire: u8,
    as_total: u32,
    column: usize,
    divisor: u32,
) -> ExchangeResolution {
    let roll = seeded_combat_roll(
        campaign_seed,
        game_year,
        coords,
        combat_kind,
        round,
        acting_empire,
        target_empire,
    );
    let multiplier = CRT_MULTIPLIER_X12[roll as usize][column];
    ExchangeResolution {
        roll,
        critical: roll == 9 && as_total > 0,
        hits: (as_total.saturating_mul(u32::from(multiplier))).div_ceil(divisor),
    }
}

/// One side's volley in a fleet battle. The die modifier moves the roll
/// along the effectiveness table, at most to 9; only an unmodified 9 is a
/// critical.
pub(super) fn fleet_volley(roll: u8, die_modifier: u8, as_total: u32) -> ExchangeResolution {
    let index = usize::from(roll.saturating_add(die_modifier).min(9));
    ExchangeResolution {
        roll,
        critical: roll == 9 && as_total > 0,
        hits: hits_from_multiplier(as_total, FLEET_EFFECTIVENESS_X4[index]),
    }
}

/// A fleet battle side's die modifier: +1 for a starbase, +1 for a
/// surprise first exchange, and +1 for a heavy line (t206).
pub(super) fn fleet_die_modifier(state: &FleetCombatState, surprise: bool, heavy_line: bool) -> u8 {
    u8::from(state.counts[IDX_SB] > 0) + u8::from(surprise) + u8::from(heavy_line)
}

/// The heavy-line bonus (t206): a side whose cruisers, battleships and
/// starbases cost at least twice the enemy's fights with +1 on the die.
/// Neither side gets it with no heavies at all.
pub(super) fn heavy_line_outweighs(own_heavy_cost: u32, enemy_heavy_cost: u32) -> bool {
    own_heavy_cost > 0 && own_heavy_cost >= 2 * enemy_heavy_cost
}

/// One side's volley in the landing battle: its armies fire on the shared
/// flat table, unmodified, and the result is in quarter armies of damage
/// (an army has AS 10 and DS 10, so a quarter is 2.5 hits). There is no
/// critical.
pub(super) fn landing_volley(roll: u8, armies: u32) -> u32 {
    armies * u32::from(FLEET_EFFECTIVENESS_X4[usize::from(roll)])
}

/// The batteries' fire at ships in orbit: bombardment and invasion return
/// fire, on the full ratio CRT (t202).
#[allow(clippy::too_many_arguments)]
pub(super) fn resolve_battery_exchange(
    campaign_seed: u64,
    game_year: u16,
    coords: [u8; 2],
    combat_kind: u64,
    round: u32,
    acting_empire: u8,
    target_empire: u8,
    battery_as: u32,
    enemy_as: u32,
) -> ExchangeResolution {
    let column = base_combat_column(battery_as, enemy_as);
    seeded_exchange_resolution(
        campaign_seed,
        game_year,
        coords,
        combat_kind,
        round,
        acting_empire,
        target_empire,
        battery_as,
        column,
        4,
    )
}

/// The batteries' fire at descending transports in a blitz, at a third of
/// the ratio CRT (t202): fleeting targets in the atmosphere are hard to hit.
#[allow(clippy::too_many_arguments)]
pub(super) fn resolve_landing_exchange(
    campaign_seed: u64,
    game_year: u16,
    coords: [u8; 2],
    combat_kind: u64,
    round: u32,
    acting_empire: u8,
    target_empire: u8,
    battery_as: u32,
    enemy_as: u32,
) -> ExchangeResolution {
    let column = base_combat_column(battery_as, enemy_as);
    seeded_exchange_resolution(
        campaign_seed,
        game_year,
        coords,
        combat_kind,
        round,
        acting_empire,
        target_empire,
        battery_as,
        column,
        12,
    )
}

/// Planetary combat's ratio-CRT exchange for ships' orbital fire at the
/// batteries: bombardment and invasion suppression, at full strength.
pub(super) fn resolve_orbital_exchange(
    campaign_seed: u64,
    game_year: u16,
    coords: [u8; 2],
    combat_kind: u64,
    round: u32,
    acting_empire: u8,
    target_empire: u8,
    our_as: u32,
    enemy_as: u32,
) -> ExchangeResolution {
    let column = base_combat_column(our_as, enemy_as);
    seeded_exchange_resolution(
        campaign_seed,
        game_year,
        coords,
        combat_kind,
        round,
        acting_empire,
        target_empire,
        our_as,
        column,
        4,
    )
}

/// Planetary combat's ratio-CRT exchange for fire at the garrison: blitz
/// cover fire and invasion softening. The landing battle itself is
/// [`landing_volley`].
pub(super) fn resolve_ground_exchange(
    campaign_seed: u64,
    game_year: u16,
    coords: [u8; 2],
    combat_kind: u64,
    round: u32,
    acting_empire: u8,
    target_empire: u8,
    our_as: u32,
    enemy_as: u32,
) -> ExchangeResolution {
    let column = base_combat_column(our_as, enemy_as);
    seeded_exchange_resolution(
        campaign_seed,
        game_year,
        coords,
        combat_kind,
        round,
        acting_empire,
        target_empire,
        our_as,
        column,
        12,
    )
}

fn apply_critical_hit_to_fleet(state: &mut FleetCombatState) -> bool {
    for idx in fleet_target_order() {
        if state.crippled_count(idx) > 0 {
            state.finish_crippled(idx, 1);
            return true;
        }
    }
    for idx in fleet_target_order() {
        if state.nominal_count(idx) > 0 {
            state.counts[idx] -= 1;
            return true;
        }
    }
    false
}

/// Focus fire finishes cripples and destroys nominal hulls at 2x DS. Ship
/// and starbase fire (`half_finish`) finishes a cripple at half DS rounded
/// up (t206), the finisher's price for what heavier fire crippled; battery
/// fire pays full DS.
fn apply_focus_fire_hits_to_fleet_classes(
    state: &mut FleetCombatState,
    hits: &mut u32,
    target_order: &[usize],
    half_finish: bool,
) -> bool {
    let mut progress = false;
    for &idx in target_order {
        let ds = fleet_class_ds(idx);
        if ds == 0 {
            continue;
        }

        // First, finish off any already crippled ships
        let finish = if half_finish { ds.div_ceil(2) } else { ds };
        if state.crippled_count(idx) > 0 && *hits >= finish {
            let finished = state.crippled_count(idx).min(*hits / finish);
            if finished > 0 {
                state.finish_crippled(idx, finished);
                *hits -= finished * finish;
                progress = true;
            }
        }

        if *hits < ds * 2 {
            continue;
        }

        // Then, destroy nominal ships (cost 2x DS)
        let cost = ds * 2;
        let destroyed = state.nominal_count(idx).min(*hits / cost);
        if destroyed > 0 {
            state.counts[idx] -= destroyed;
            *hits -= destroyed * cost;
            progress = true;
        }
    }
    progress
}

/// Apply a volley's hits pool by pool and return what each pool could not
/// spend on a damage step.
pub(super) fn apply_hits_to_fleet(
    state: &mut FleetCombatState,
    mut suppression_hits: u32,
    mut execution_hits: u32,
    critical_hits: u32,
) -> CarriedHits {
    // 1. Apply Suppression hits (Dispersed Fire)
    while suppression_hits > 0 {
        let (combat_line_order, auxiliary_order);
        let target_order: &[usize] = if fleet_combat_line_present(state) {
            combat_line_order = fleet_combat_line_order();
            &combat_line_order
        } else {
            auxiliary_order = fleet_auxiliary_order();
            &auxiliary_order
        };

        let mut progress =
            apply_nominal_hits_to_fleet_classes(state, &mut suppression_hits, target_order);

        if target_order
            .iter()
            .all(|&idx| state.nominal_count(idx) == 0)
        {
            progress |=
                apply_destroyed_hits_to_fleet_classes(state, &mut suppression_hits, target_order);
        }

        if !progress {
            break;
        }
    }

    // 2. Apply Execution hits (Focus Fire)
    while execution_hits > 0 {
        let (combat_line_order, auxiliary_order);
        let target_order: &[usize] = if fleet_combat_line_present(state) {
            combat_line_order = fleet_combat_line_order();
            &combat_line_order
        } else {
            auxiliary_order = fleet_auxiliary_order();
            &auxiliary_order
        };

        if !apply_focus_fire_hits_to_fleet_classes(state, &mut execution_hits, target_order, true) {
            break;
        }
    }

    // 3. Apply Critical hits
    for _ in 0..critical_hits {
        if !apply_critical_hit_to_fleet(state) {
            break;
        }
    }

    CarriedHits {
        suppression: suppression_hits,
        execution: execution_hits,
    }
}

/// Battery return fire on a fleet in orbit: execution fire that keeps the
/// pre-t206 rules, landing DD, CA, BB, SB and finishing a cripple at full
/// DS. What it cannot spend expires.
pub(super) fn apply_battery_hits_to_fleet(
    state: &mut FleetCombatState,
    mut hits: u32,
    critical_hits: u32,
) {
    while hits > 0 {
        let (battery_line_order, auxiliary_order);
        let target_order: &[usize] = if fleet_combat_line_present(state) {
            battery_line_order = fleet_battery_line_order();
            &battery_line_order
        } else {
            auxiliary_order = fleet_auxiliary_order();
            &auxiliary_order
        };
        if !apply_focus_fire_hits_to_fleet_classes(state, &mut hits, target_order, false) {
            break;
        }
    }
    for _ in 0..critical_hits {
        if !apply_critical_hit_to_fleet(state) {
            break;
        }
    }
}

/// Battery fire on a blitz landing. It is focus fire aimed at the descending
/// transports first, then their escorts; a critical sinks one more ship.
pub(super) fn apply_landing_fire_to_fleet(
    state: &mut FleetCombatState,
    mut hits: u32,
    critical: bool,
) {
    let target_order = [IDX_TT, IDX_DD, IDX_CA, IDX_BB];
    while hits > 0 && apply_focus_fire_hits_to_fleet_classes(state, &mut hits, &target_order, false)
    {
    }

    if !critical {
        return;
    }
    if let Some(idx) = target_order
        .into_iter()
        .find(|&idx| state.crippled_count(idx) > 0)
    {
        state.finish_crippled(idx, 1);
    } else if let Some(idx) = target_order
        .into_iter()
        .find(|&idx| state.nominal_count(idx) > 0)
    {
        state.counts[idx] -= 1;
    }
}

pub(super) fn scalar_hits_with_critical(resolution: ExchangeResolution) -> u32 {
    resolution.hits + u32::from(resolution.critical)
}
