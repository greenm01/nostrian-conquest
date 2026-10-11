use crate::{GameRng, RNG_TAG_COMBAT};

use super::state::{
    CarriedHits, FleetCombatState, IDX_BB, IDX_CA, IDX_DD, IDX_SB, IDX_TT, fleet_auxiliary_order,
    fleet_class_ds, fleet_combat_line_order, fleet_combat_line_present, fleet_target_order,
};

pub(super) const GROUND_AS_BATTERY: u32 = 90;
pub(super) const GROUND_DS_BATTERY: u32 = 20;
pub(super) const GROUND_AS_ARMY: u32 = 10;
pub(super) const GROUND_DS_ARMY: u32 = 10;
pub(super) const COMBAT_GUARDRAIL_MAX_ROUNDS: u32 = 64;

const COLUMN_DISADVANTAGED: usize = 0;
const COLUMN_PRESSED: usize = 1;
const COLUMN_EVEN: usize = 2;
const COLUMN_ADVANTAGED: usize = 3;
const COLUMN_OVERWHELMING: usize = 4;

pub(super) const COMBAT_KIND_FLEET: u64 = 1;
pub(super) const COMBAT_KIND_WITHDRAWAL: u64 = 2;
pub(super) const COMBAT_KIND_BOMBARD: u64 = 3;
pub(super) const COMBAT_KIND_INVASION_SUPPRESSION: u64 = 4;
pub(super) const COMBAT_KIND_INVASION_SOFTEN: u64 = 5;
pub(super) const COMBAT_KIND_GROUND: u64 = 6;
pub(super) const COMBAT_KIND_BLITZ_COVER: u64 = 7;
pub(super) const COMBAT_KIND_BLITZ_GROUND: u64 = 8;
pub(super) const COMBAT_KIND_BLITZ_LANDING: u64 = 9;

/// Planetary combat's ratio CRT, multipliers in quarters: one row per d10
/// value and one column per force-ratio band.
const CRT_MULTIPLIER_X4: [[u8; 5]; 10] = [
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
const FLEET_DIE_MODIFIER_CAP: u8 = 2;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum RoundActionKind {
    Fight,
    Withdraw,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct RoundAction {
    pub(super) empire: u8,
    pub(super) target_empire: u8,
    pub(super) kind: RoundActionKind,
}

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
        if ds == 0 || *hits < ds {
            break;
        }
        let reducible = state.nominal_count(idx).min(*hits / ds);
        if reducible == 0 {
            continue;
        }
        state.crippled[idx] += reducible;
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
        if ds == 0 || *hits < ds {
            break;
        }
        let destroyed = state.crippled[idx].min(*hits / ds);
        if destroyed == 0 {
            continue;
        }
        state.crippled[idx] -= destroyed;
        state.counts[idx] -= destroyed;
        *hits -= destroyed * ds;
        progress = true;
    }
    progress
}

pub(super) fn rule_threshold_satisfied(roe: u8, friendly_as: u32, enemy_as: u32) -> bool {
    match roe {
        0 => false,
        1 => enemy_as == 0,
        2 => friendly_as >= enemy_as.saturating_mul(4),
        3 => friendly_as >= enemy_as.saturating_mul(3),
        4 => friendly_as >= enemy_as.saturating_mul(2),
        5 => friendly_as.saturating_mul(2) >= enemy_as.saturating_mul(3),
        6 => friendly_as >= enemy_as,
        7 => friendly_as.saturating_mul(3) >= enemy_as.saturating_mul(2),
        8 => friendly_as.saturating_mul(2) >= enemy_as,
        9 => friendly_as.saturating_mul(3) >= enemy_as,
        _ => true,
    }
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

fn shifted_combat_column(base_column: usize, shift: i32) -> usize {
    (base_column as i32 + shift).clamp(0, COLUMN_OVERWHELMING as i32) as usize
}

fn hits_from_multiplier(as_total: u32, multiplier_x4: u8) -> u32 {
    (as_total.saturating_mul(multiplier_x4 as u32)).div_ceil(4)
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
    let multiplier_x4 = CRT_MULTIPLIER_X4[roll as usize][column];
    ExchangeResolution {
        roll,
        critical: roll == 9 && as_total > 0,
        hits: hits_from_multiplier(as_total, multiplier_x4),
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

/// A fleet battle side's die modifier: +1 for a mixed fleet and +1 for a
/// starbase, at most +2. The withdrawal exchange takes none.
pub(super) fn fleet_die_modifier(state: &FleetCombatState) -> u8 {
    let modifier = u8::from(state.is_mixed()) + u8::from(state.counts[IDX_SB] > 0);
    modifier.min(FLEET_DIE_MODIFIER_CAP)
}

/// One side's volley in the landing battle: its armies fire on the shared
/// flat table, unmodified, and the result is in quarter armies of damage
/// (an army has AS 10 and DS 10, so a quarter is 2.5 hits). There is no
/// critical.
pub(super) fn landing_volley(roll: u8, armies: u32) -> u32 {
    armies * u32::from(FLEET_EFFECTIVENESS_X4[usize::from(roll)])
}

/// Planetary combat's ratio-CRT exchange for orbital fire: bombardment,
/// invasion suppression and return fire, and blitz landing fire.
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
    mixed: bool,
) -> ExchangeResolution {
    let mut shift = 0i32;
    if mixed {
        shift += 1;
    }
    let column = shifted_combat_column(base_combat_column(our_as, enemy_as), shift);
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
    )
}

fn apply_critical_hit_to_fleet(state: &mut FleetCombatState) -> bool {
    for idx in fleet_target_order() {
        if state.crippled[idx] > 0 {
            state.crippled[idx] -= 1;
            state.counts[idx] -= 1;
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

fn apply_focus_fire_hits_to_fleet_classes(
    state: &mut FleetCombatState,
    hits: &mut u32,
    target_order: &[usize],
) -> bool {
    let mut progress = false;
    for &idx in target_order {
        let ds = fleet_class_ds(idx);
        if ds == 0 || *hits < ds {
            continue;
        }

        // First, finish off any already crippled ships (cost 1x DS)
        if state.crippled[idx] > 0 {
            let finished = state.crippled[idx].min(*hits / ds);
            if finished > 0 {
                state.crippled[idx] -= finished;
                state.counts[idx] -= finished;
                *hits -= finished * ds;
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

        if !apply_focus_fire_hits_to_fleet_classes(state, &mut execution_hits, target_order) {
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

/// Battery fire on a blitz landing. It is focus fire aimed at the descending
/// transports first, then their escorts; a critical sinks one more ship.
pub(super) fn apply_landing_fire_to_fleet(
    state: &mut FleetCombatState,
    mut hits: u32,
    critical: bool,
) {
    let target_order = [IDX_TT, IDX_DD, IDX_CA, IDX_BB];
    while hits > 0 && apply_focus_fire_hits_to_fleet_classes(state, &mut hits, &target_order) {}

    if !critical {
        return;
    }
    if let Some(idx) = target_order
        .into_iter()
        .find(|&idx| state.crippled[idx] > 0)
    {
        state.crippled[idx] -= 1;
        state.counts[idx] -= 1;
    } else if let Some(idx) = target_order
        .into_iter()
        .find(|&idx| state.nominal_count(idx) > 0)
    {
        state.counts[idx] -= 1;
    }
}

pub(super) fn fleet_state_changed(before: &FleetCombatState, after: &FleetCombatState) -> bool {
    before.counts != after.counts || before.crippled != after.crippled
}

pub(super) fn scalar_hits_with_critical(resolution: ExchangeResolution) -> u32 {
    resolution.hits + u32::from(resolution.critical)
}
