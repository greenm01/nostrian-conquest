use crate::{CoreGameData, EncounterDispositionReason, Order, ShipLosses};
use nc_data::{ShipCounts, slowest_speed, troop_transports};

pub(super) const IDX_DD: usize = 0;
pub(super) const IDX_CA: usize = 1;
pub(super) const IDX_BB: usize = 2;
pub(super) const IDX_SB: usize = 3;
pub(super) const IDX_SC: usize = 4;
pub(super) const IDX_TT: usize = 5;
pub(super) const IDX_ET: usize = 6;

const AS_DD: u32 = 10;
const AS_CA: u32 = 30;
const AS_BB: u32 = 90;
const AS_SB: u32 = 100;
const AS_SC: u32 = 0;
const AS_TT: u32 = 0;
const AS_ET: u32 = 0;

/// A starbase's attack strength, which is also what it counts for in the
/// decision to fight.
pub(crate) const STARBASE_ATTACK_STRENGTH: u32 = AS_SB;

const DS_DD: u32 = 8;
const DS_CA: u32 = 30;
const DS_BB: u32 = 100;
const DS_SB: u32 = 120;
const DS_SC: u32 = 10;
const DS_TT: u32 = 10;
const DS_ET: u32 = 20;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum BattleRole {
    IncumbentDefender,
    GuardingDefender,
    Attacker,
    Neutral,
}

#[derive(Clone, Debug, Default)]
pub(super) struct FleetCombatState {
    pub(super) counts: [u32; 7],
    pub(super) crippled: [u32; 7],
}

/// Hits a side took in a fleet battle but has not yet spent on a damage
/// step, per fire pool.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) struct CarriedHits {
    pub(super) suppression: u32,
    pub(super) execution: u32,
}

/// How a side left the battle.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Withdrawal {
    /// Its ROE failed, or it had no armed ship left.
    pub(super) reason: EncounterDispositionReason,
    /// The other side caught it in a pursuit exchange.
    pub(super) pursued: bool,
}

#[derive(Clone, Debug)]
pub(super) struct TaskForce {
    pub(super) empire: u8,
    pub(super) fleet_indices: Vec<usize>,
    pub(super) state: FleetCombatState,
    pub(super) role: BattleRole,
    /// Set once the side has broken off: it is out of the battle.
    pub(super) withdrew: Option<Withdrawal>,
    pub(super) engaged_in_battle: bool,
    /// Hits carried against this side from earlier exchanges of this
    /// battle. They belong to the side, not to its fleets: a fleet that
    /// leaves takes none with them, and the battle's end drops them.
    pub(super) carried_hits: CarriedHits,
}

impl FleetCombatState {
    pub(super) fn total_combat_as(&self) -> u32 {
        self.suppression_as() + self.execution_as()
    }

    pub(super) fn suppression_as(&self) -> u32 {
        [IDX_CA, IDX_BB]
            .into_iter()
            .map(|idx| {
                let as_value = fleet_class_as(idx);
                self.nominal_count(idx) * as_value + self.crippled[idx] * crippled_as(as_value)
            })
            .sum()
    }

    pub(super) fn execution_as(&self) -> u32 {
        [IDX_DD, IDX_SB]
            .into_iter()
            .map(|idx| {
                let as_value = fleet_class_as(idx);
                self.nominal_count(idx) * as_value + self.crippled[idx] * crippled_as(as_value)
            })
            .sum()
    }

    pub(super) fn is_mixed(&self) -> bool {
        let mut kinds = 0;
        if self.counts[IDX_DD] > 0 {
            kinds += 1;
        }
        if self.counts[IDX_CA] > 0 {
            kinds += 1;
        }
        if self.counts[IDX_BB] > 0 {
            kinds += 1;
        }
        kinds >= 2
    }

    pub(super) fn has_units(&self) -> bool {
        self.counts.iter().any(|&c| c > 0)
    }

    pub(super) fn nominal_count(&self, idx: usize) -> u32 {
        self.counts[idx].saturating_sub(self.crippled[idx])
    }

    /// The side has ships, as opposed to starbases only.
    pub(super) fn has_ships(&self) -> bool {
        self.counts
            .iter()
            .enumerate()
            .any(|(idx, &count)| idx != IDX_SB && count > 0)
    }

    /// The speed the side's ships can make together: that of the slowest
    /// class still present, as the fleet's maximum speed is recomputed
    /// after the battle.
    pub(super) fn ship_speed(&self) -> u8 {
        let count = |idx: usize| self.counts[idx].min(u32::from(u16::MAX)) as u16;
        slowest_speed(&ShipCounts {
            battleships: count(IDX_BB),
            cruisers: count(IDX_CA),
            destroyers: count(IDX_DD),
            empty_transports: count(IDX_TT),
            loaded_transports: 0,
            etacs: count(IDX_ET),
            scouts: count(IDX_SC),
        })
    }
}

/// The attack strength of a fleet's warships at full strength: the
/// yardstick the decision to fight shares with the battle.
pub(crate) fn warship_attack_strength(destroyers: u16, cruisers: u16, battleships: u16) -> u32 {
    u32::from(destroyers) * AS_DD + u32::from(cruisers) * AS_CA + u32::from(battleships) * AS_BB
}

fn crippled_as(as_value: u32) -> u32 {
    as_value / 2
}

pub(super) fn fleet_state_from_records(
    game_data: &CoreGameData,
    fleet_indices: &[usize],
    starbases: u32,
) -> FleetCombatState {
    let mut state = FleetCombatState::default();
    for &idx in fleet_indices {
        let fleet = &game_data.fleets[idx];
        state.counts[IDX_DD] += fleet.ships.destroyers as u32;
        state.counts[IDX_CA] += fleet.ships.cruisers as u32;
        state.counts[IDX_BB] += fleet.ships.battleships as u32;
        state.counts[IDX_SC] += fleet.ships.scouts as u32;
        state.counts[IDX_TT] += troop_transports(fleet) as u32;
        state.counts[IDX_ET] += fleet.ships.etacs as u32;
    }
    state.counts[IDX_SB] = starbases;
    state
}

fn fleet_class_as(idx: usize) -> u32 {
    match idx {
        IDX_DD => AS_DD,
        IDX_CA => AS_CA,
        IDX_BB => AS_BB,
        IDX_SB => AS_SB,
        IDX_SC => AS_SC,
        IDX_TT => AS_TT,
        IDX_ET => AS_ET,
        _ => 0,
    }
}

pub(super) fn fleet_class_ds(idx: usize) -> u32 {
    match idx {
        IDX_DD => DS_DD,
        IDX_CA => DS_CA,
        IDX_BB => DS_BB,
        IDX_SB => DS_SB,
        IDX_SC => DS_SC,
        IDX_TT => DS_TT,
        IDX_ET => DS_ET,
        _ => 0,
    }
}

pub(super) fn fleet_target_order() -> [usize; 7] {
    [IDX_DD, IDX_SC, IDX_TT, IDX_ET, IDX_CA, IDX_BB, IDX_SB]
}

pub(super) fn fleet_combat_line_order() -> [usize; 4] {
    [IDX_DD, IDX_CA, IDX_BB, IDX_SB]
}

pub(super) fn fleet_auxiliary_order() -> [usize; 3] {
    [IDX_SC, IDX_TT, IDX_ET]
}

pub(super) fn fleet_combat_line_present(state: &FleetCombatState) -> bool {
    fleet_combat_line_order()
        .into_iter()
        .any(|idx| state.counts[idx] > 0)
}

fn task_force_role(
    game_data: &CoreGameData,
    empire: u8,
    coords: [u8; 2],
    fleet_indices: &[usize],
) -> BattleRole {
    if let Some(planet) = game_data
        .planets
        .iter()
        .find(|p| p.sector.to_array() == coords)
    {
        if planet.owner.0 == empire {
            return BattleRole::IncumbentDefender;
        }
    }
    let guarding = fleet_indices.iter().any(|&idx| {
        matches!(
            game_data.fleets[idx].order,
            Order::GuardStarbase | Order::GuardBlockadeWorld
        )
    });
    if guarding {
        BattleRole::GuardingDefender
    } else if !fleet_indices.is_empty() {
        BattleRole::Attacker
    } else {
        BattleRole::Neutral
    }
}

pub(super) fn new_task_force(
    game_data: &CoreGameData,
    empire: u8,
    coords: [u8; 2],
    fleet_indices: Vec<usize>,
    starbases: u32,
) -> TaskForce {
    let role = task_force_role(game_data, empire, coords, &fleet_indices);
    let state = fleet_state_from_records(game_data, &fleet_indices, starbases);
    TaskForce {
        empire,
        fleet_indices,
        state,
        role,
        withdrew: None,
        engaged_in_battle: false,
        carried_hits: CarriedHits::default(),
    }
}

pub(super) fn ship_losses_from_states(
    before: &FleetCombatState,
    after: &FleetCombatState,
) -> ShipLosses {
    ShipLosses {
        destroyers: before.counts[IDX_DD].saturating_sub(after.counts[IDX_DD]),
        cruisers: before.counts[IDX_CA].saturating_sub(after.counts[IDX_CA]),
        battleships: before.counts[IDX_BB].saturating_sub(after.counts[IDX_BB]),
        scouts: before.counts[IDX_SC].saturating_sub(after.counts[IDX_SC]),
        transports: before.counts[IDX_TT].saturating_sub(after.counts[IDX_TT]),
        etacs: before.counts[IDX_ET].saturating_sub(after.counts[IDX_ET]),
    }
}

pub(super) fn ship_counts_from_state(state: &FleetCombatState) -> ShipLosses {
    ShipLosses {
        destroyers: state.counts[IDX_DD],
        cruisers: state.counts[IDX_CA],
        battleships: state.counts[IDX_BB],
        scouts: state.counts[IDX_SC],
        transports: state.counts[IDX_TT],
        etacs: state.counts[IDX_ET],
    }
}

pub(super) fn tf_has_any_units(tf: &TaskForce) -> bool {
    tf.state.counts.iter().any(|&count| count > 0)
}

pub(super) fn planet_idx_at_coords(game_data: &CoreGameData, coords: [u8; 2]) -> Option<usize> {
    game_data
        .planets
        .iter()
        .position(|planet| planet.sector.to_array() == coords)
}
