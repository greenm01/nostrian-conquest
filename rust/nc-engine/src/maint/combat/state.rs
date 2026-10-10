use std::collections::{BTreeMap, BTreeSet};

use crate::{CoreGameData, Order, ShipLosses};

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

const DS_DD: u32 = 5;
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

/// How a task force came to its battle.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Encounter {
    /// Forces sharing a location after movement: the ordinary fleet battle.
    InPlace,
    /// Fleets ordered away from a blockaded world, met before they move.
    Departing,
    /// The blockade meeting those departing fleets.
    Blockading,
}

#[derive(Clone, Debug, Default)]
pub(super) struct FleetCombatState {
    pub(super) counts: [u32; 7],
    pub(super) crippled: [u32; 7],
}

#[derive(Clone, Debug)]
pub(super) struct TaskForce {
    pub(super) empire: u8,
    pub(super) fleet_indices: Vec<usize>,
    pub(super) coords: [u8; 2],
    pub(super) state: FleetCombatState,
    pub(super) role: BattleRole,
    pub(super) encounter: Encounter,
    pub(super) withdrew_under_roe: bool,
    /// An anchored defender whose ROE declined to break formation against a
    /// contact that is not entering its system. It stays on station and only
    /// fires back if a willing enemy attacks it.
    pub(super) holding_formation: bool,
    pub(super) engaged_in_battle: bool,
    /// Guards/blockades get one free hold when ROE threshold fails (per spec).
    /// Set to true once this free hold has been used.
    pub(super) free_hold_used: bool,
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
        let fleet = &game_data.fleets.records[idx];
        state.counts[IDX_DD] += fleet.destroyer_count() as u32;
        state.counts[IDX_CA] += fleet.cruiser_count() as u32;
        state.counts[IDX_BB] += fleet.battleship_count() as u32;
        state.counts[IDX_SC] += fleet.scout_count() as u32;
        state.counts[IDX_TT] += fleet.troop_transport_count() as u32;
        state.counts[IDX_ET] += fleet.etac_count() as u32;
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
        .records
        .iter()
        .find(|p| p.coords_raw() == coords)
    {
        if planet.owner_empire_slot_raw() == empire {
            return BattleRole::IncumbentDefender;
        }
    }
    let guarding = fleet_indices.iter().any(|&idx| {
        matches!(
            game_data.fleets.records[idx].standing_order_kind(),
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

pub(super) fn has_patrol_order(game_data: &CoreGameData, fleet_indices: &[usize]) -> bool {
    fleet_indices
        .iter()
        .any(|&idx| game_data.fleets.records[idx].standing_order_kind() == Order::PatrolSector)
}

/// Orders that take a fleet into the solar system at its current location
/// instead of leaving it in the sector's deep space. View World is excluded:
/// it scans from the system edge and backs off into deep space.
fn fleet_enters_system(fleet: &nc_data::FleetRecord) -> bool {
    fleet.current_location_coords_raw() == fleet.standing_order_target_coords_raw()
        && matches!(
            fleet.standing_order_kind(),
            Order::GuardStarbase
                | Order::GuardBlockadeWorld
                | Order::BombardWorld
                | Order::InvadeWorld
                | Order::BlitzWorld
                | Order::ScoutSolarSystem
        )
}

pub(super) fn task_force_enters_system(game_data: &CoreGameData, task_force: &TaskForce) -> bool {
    task_force
        .fleet_indices
        .iter()
        .any(|&idx| fleet_enters_system(&game_data.fleets.records[idx]))
}

pub(super) fn task_force_blockades_system(
    game_data: &CoreGameData,
    task_force: &TaskForce,
) -> bool {
    task_force
        .fleet_indices
        .iter()
        .any(|&idx| fleet_blockades_system(&game_data.fleets.records[idx]))
}

/// A Guard/Blockade World fleet on station at its target world.
fn fleet_blockades_system(fleet: &nc_data::FleetRecord) -> bool {
    fleet.standing_order_kind() == Order::GuardBlockadeWorld && fleet_enters_system(fleet)
}

/// A fleet stopped at its location whose order moves it elsewhere this turn.
/// Fleets passing through mid-transit are not at rest (0x80) and do not count.
fn fleet_departs_location(fleet: &nc_data::FleetRecord) -> bool {
    fleet.standing_order_kind() != Order::HoldPosition
        && fleet.current_speed() > 0
        && fleet.movement_state_flag_raw() == 0x80
        && fleet.standing_order_target_coords_raw() != fleet.current_location_coords_raw()
}

pub(super) fn starbase_count_at(game_data: &CoreGameData, coords: [u8; 2], owner: u8) -> u32 {
    game_data
        .bases
        .records
        .iter()
        .filter(|b| {
            b.coords_raw() == coords && b.owner_empire_raw() == owner && b.active_flag_raw() != 0
        })
        .count() as u32
}

pub(super) fn build_task_forces_at_location(
    game_data: &CoreGameData,
    coords: [u8; 2],
) -> Vec<TaskForce> {
    let mut by_empire: BTreeMap<u8, Vec<usize>> = BTreeMap::new();
    for (idx, fleet) in game_data.fleets.records.iter().enumerate() {
        if fleet.current_location_coords_raw() == coords {
            by_empire
                .entry(fleet.owner_empire_raw())
                .or_default()
                .push(idx);
        }
    }

    let mut empires: BTreeSet<u8> = by_empire.keys().copied().collect();
    if let Some(planet) = game_data
        .planets
        .records
        .iter()
        .find(|p| p.coords_raw() == coords)
    {
        if planet.owner_empire_slot_raw() != 0
            && starbase_count_at(game_data, coords, planet.owner_empire_slot_raw()) > 0
        {
            empires.insert(planet.owner_empire_slot_raw());
        }
    }

    empires
        .into_iter()
        .map(|empire| {
            let fleet_indices = by_empire.remove(&empire).unwrap_or_default();
            let starbases = starbase_count_at(game_data, coords, empire);
            new_task_force(
                game_data,
                empire,
                coords,
                fleet_indices,
                starbases,
                Encounter::InPlace,
            )
        })
        .collect()
}

/// Locations where a Guard/Blockade World fleet is on station.
pub(super) fn blockaded_locations(game_data: &CoreGameData) -> BTreeSet<[u8; 2]> {
    game_data
        .fleets
        .records
        .iter()
        .filter(|fleet| fleet_blockades_system(fleet))
        .map(|fleet| fleet.current_location_coords_raw())
        .collect()
}

/// Task forces for the departure interception at a blockaded world, before
/// movement: each empire's fleets ordered away from the world, and each other
/// empire's blockading fleets. An empire with departing fleets fields only
/// those. Starbases do not intercept departures.
pub(super) fn build_departure_task_forces_at_location(
    game_data: &CoreGameData,
    coords: [u8; 2],
) -> Vec<TaskForce> {
    let mut departing: BTreeMap<u8, Vec<usize>> = BTreeMap::new();
    let mut blockading: BTreeMap<u8, Vec<usize>> = BTreeMap::new();
    for (idx, fleet) in game_data.fleets.records.iter().enumerate() {
        if fleet.current_location_coords_raw() != coords {
            continue;
        }
        if fleet_departs_location(fleet) {
            departing
                .entry(fleet.owner_empire_raw())
                .or_default()
                .push(idx);
        } else if fleet_blockades_system(fleet) {
            blockading
                .entry(fleet.owner_empire_raw())
                .or_default()
                .push(idx);
        }
    }
    blockading.retain(|empire, _| !departing.contains_key(empire));
    if blockading.is_empty() {
        return Vec::new();
    }

    let departing = departing
        .into_iter()
        .map(|(empire, fleets)| (empire, fleets, Encounter::Departing));
    let blockading = blockading
        .into_iter()
        .map(|(empire, fleets)| (empire, fleets, Encounter::Blockading));
    let mut task_forces: Vec<TaskForce> = departing
        .chain(blockading)
        .map(|(empire, fleet_indices, encounter)| {
            new_task_force(game_data, empire, coords, fleet_indices, 0, encounter)
        })
        .collect();
    task_forces.sort_by_key(|tf| tf.empire);
    task_forces
}

fn new_task_force(
    game_data: &CoreGameData,
    empire: u8,
    coords: [u8; 2],
    fleet_indices: Vec<usize>,
    starbases: u32,
    encounter: Encounter,
) -> TaskForce {
    let role = task_force_role(game_data, empire, coords, &fleet_indices);
    let state = fleet_state_from_records(game_data, &fleet_indices, starbases);
    TaskForce {
        empire,
        fleet_indices,
        coords,
        state,
        role,
        encounter,
        withdrew_under_roe: false,
        holding_formation: false,
        engaged_in_battle: false,
        free_hold_used: false,
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
        .records
        .iter()
        .position(|planet| planet.coords_raw() == coords)
}
