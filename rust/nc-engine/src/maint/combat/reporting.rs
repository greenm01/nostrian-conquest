use crate::{
    ContactReportSource, CoreGameData, Mission, Order, PlanetIntelEvent, PlanetIntelSource,
    ScoutContactEvent, ShipLosses,
};

use super::super::FleetBattlePerspective;

use super::state::{
    BattleRole, FleetCombatState, IDX_BB, IDX_CA, IDX_DD, IDX_ET, IDX_SC, IDX_TT, TaskForce,
};

pub(super) fn push_contact_event_for_task_force(
    scout_contact_events: &mut Vec<ScoutContactEvent>,
    planet_intel_events: &mut Vec<PlanetIntelEvent>,
    game_data: &CoreGameData,
    coords: [u8; 2],
    task_force: &TaskForce,
    target_task_force: &TaskForce,
) {
    let (small_vessels, medium_vessels, large_vessels) =
        vessel_size_summary(&target_task_force.state);
    let target_fleet_number =
        single_named_fleet_number(game_data, &target_task_force.fleet_indices);

    for &idx in &task_force.fleet_indices {
        let fleet = &game_data.fleets.records[idx];
        let source = contact_reporting_kind(fleet.standing_order_kind())
            .map(ContactReportSource::FleetMission)
            .unwrap_or(ContactReportSource::Fleet(fleet.local_slot_word_raw() as u8));
        scout_contact_events.push(ScoutContactEvent {
            viewer_empire_raw: fleet.owner_empire_raw(),
            source,
            reporting_fleet_number: Some(fleet.local_slot_word_raw() as u8),
            reporting_initial: ship_counts_from_fleet(fleet),
            reporting_loaded_armies_initial: u32::from(fleet.army_count()),
            coords,
            target_empire_raw: target_task_force.empire,
            target_fleet_number,
            small_vessels,
            medium_vessels,
            large_vessels,
            stardate_week: None,
        });
        push_civil_disorder_contact_intel(
            planet_intel_events,
            game_data,
            coords,
            fleet.owner_empire_raw(),
            target_task_force.empire,
        );
    }

    for base in game_data.bases.records.iter().filter(|base| {
        base.coords_raw() == coords
            && base.owner_empire_raw() == task_force.empire
            && base.base_id_raw() != 0
    }) {
        scout_contact_events.push(ScoutContactEvent {
            viewer_empire_raw: task_force.empire,
            source: ContactReportSource::Starbase(base.base_id_raw()),
            reporting_fleet_number: None,
            reporting_initial: ShipLosses::default(),
            reporting_loaded_armies_initial: 0,
            coords,
            target_empire_raw: target_task_force.empire,
            target_fleet_number,
            small_vessels,
            medium_vessels,
            large_vessels,
            stardate_week: None,
        });
        push_civil_disorder_contact_intel(
            planet_intel_events,
            game_data,
            coords,
            task_force.empire,
            target_task_force.empire,
        );
    }
}

fn ship_counts_from_fleet(fleet: &nc_data::FleetRecord) -> ShipLosses {
    ShipLosses {
        destroyers: u32::from(fleet.destroyer_count()),
        cruisers: u32::from(fleet.cruiser_count()),
        battleships: u32::from(fleet.battleship_count()),
        scouts: u32::from(fleet.scout_count()),
        transports: u32::from(fleet.troop_transport_count()),
        etacs: u32::from(fleet.etac_count()),
    }
}

fn push_civil_disorder_contact_intel(
    planet_intel_events: &mut Vec<PlanetIntelEvent>,
    game_data: &CoreGameData,
    coords: [u8; 2],
    viewer_empire_raw: u8,
    target_empire_raw: u8,
) {
    if viewer_empire_raw == 0
        || target_empire_raw == 0
        || game_data.empire_campaign_state(target_empire_raw)
            != Some(nc_data::CampaignState::CivilDisorder)
    {
        return;
    }
    let Some((planet_idx, _planet)) =
        game_data
            .planets
            .records
            .iter()
            .enumerate()
            .find(|(_, planet)| {
                planet.coords_raw() == coords && planet.owner_empire_slot_raw() == target_empire_raw
            })
    else {
        return;
    };
    if planet_intel_events.iter().any(|event| {
        event.planet_idx == planet_idx
            && event.viewer_empire_raw == viewer_empire_raw
            && event.source == PlanetIntelSource::CivilDisorderContact
    }) {
        return;
    }
    push_planet_intel(
        planet_intel_events,
        planet_idx,
        viewer_empire_raw,
        PlanetIntelSource::CivilDisorderContact,
    );
}

pub(super) fn single_named_fleet_number(
    game_data: &CoreGameData,
    fleet_indices: &[usize],
) -> Option<u8> {
    let named_fleets = fleet_indices
        .iter()
        .filter_map(|idx| game_data.fleets.records.get(*idx))
        .filter(|fleet| {
            fleet.destroyer_count() > 0
                || fleet.cruiser_count() > 0
                || fleet.battleship_count() > 0
                || fleet.scout_count() > 0
                || fleet.troop_transport_count() > 0
                || fleet.etac_count() > 0
        })
        .map(|fleet| fleet.local_slot_word_raw() as u8)
        .filter(|fleet_number| *fleet_number != 0)
        .collect::<Vec<_>>();

    if named_fleets.len() == 1 {
        Some(named_fleets[0])
    } else {
        None
    }
}

pub(super) fn preferred_reporting_fleet_number(
    game_data: &CoreGameData,
    fleet_indices: &[usize],
) -> Option<u8> {
    fleet_indices
        .iter()
        .filter_map(|idx| game_data.fleets.records.get(*idx))
        .map(|fleet| fleet.local_slot_word_raw() as u8)
        .filter(|fleet_number| *fleet_number != 0)
        .min()
}

pub(super) fn preferred_reporting_fleet_index(
    game_data: &CoreGameData,
    fleet_indices: &[usize],
) -> Option<usize> {
    fleet_indices
        .iter()
        .copied()
        .filter(|idx| game_data.fleets.records.get(*idx).is_some())
        .filter(|idx| game_data.fleets.records[*idx].local_slot_word_raw() != 0)
        .min_by_key(|idx| game_data.fleets.records[*idx].local_slot_word_raw())
}

pub(super) fn loaded_armies_for_fleet_indices(
    game_data: &CoreGameData,
    fleet_indices: &[usize],
) -> u32 {
    fleet_indices
        .iter()
        .filter_map(|idx| game_data.fleets.records.get(*idx))
        .map(|fleet| fleet.army_count() as u32)
        .sum()
}

pub(super) fn report_perspective_for_mission(
    mission: Option<Mission>,
    role: BattleRole,
) -> FleetBattlePerspective {
    match mission {
        Some(Mission::GuardStarbase | Mission::GuardBlockadeWorld) => {
            FleetBattlePerspective::Intercepted
        }
        Some(
            Mission::MoveOnly
            | Mission::SeekHome
            | Mission::PatrolSector
            | Mission::ViewWorld
            | Mission::ColonizeWorld
            | Mission::ScoutSector
            | Mission::ScoutSolarSystem,
        ) => FleetBattlePerspective::Attacked,
        Some(
            Mission::BombardWorld
            | Mission::InvadeWorld
            | Mission::BlitzWorld
            | Mission::JoinAnotherFleet
            | Mission::RendezvousSector
            | Mission::Salvage,
        ) => FleetBattlePerspective::Intercepted,
        None => {
            if matches!(role, BattleRole::GuardingDefender) {
                FleetBattlePerspective::Intercepted
            } else {
                FleetBattlePerspective::Attacked
            }
        }
    }
}

pub(super) fn mission_kind_for_order(order: Option<Order>) -> Option<Mission> {
    match order? {
        Order::MoveOnly => Some(Mission::MoveOnly),
        Order::SeekHome => Some(Mission::SeekHome),
        Order::PatrolSector => Some(Mission::PatrolSector),
        Order::ViewWorld => Some(Mission::ViewWorld),
        Order::GuardStarbase => Some(Mission::GuardStarbase),
        Order::GuardBlockadeWorld => Some(Mission::GuardBlockadeWorld),
        Order::ScoutSector => Some(Mission::ScoutSector),
        Order::ScoutSolarSystem => Some(Mission::ScoutSolarSystem),
        Order::BombardWorld => Some(Mission::BombardWorld),
        Order::InvadeWorld => Some(Mission::InvadeWorld),
        Order::BlitzWorld => Some(Mission::BlitzWorld),
        _ => None,
    }
}

fn contact_reporting_kind(order: Order) -> Option<Mission> {
    match order {
        Order::MoveOnly => Some(Mission::MoveOnly),
        Order::SeekHome => Some(Mission::SeekHome),
        Order::PatrolSector => Some(Mission::PatrolSector),
        Order::ViewWorld => Some(Mission::ViewWorld),
        Order::ScoutSector => Some(Mission::ScoutSector),
        Order::ScoutSolarSystem => Some(Mission::ScoutSolarSystem),
        Order::BombardWorld => Some(Mission::BombardWorld),
        Order::InvadeWorld => Some(Mission::InvadeWorld),
        Order::BlitzWorld => Some(Mission::BlitzWorld),
        Order::GuardStarbase => Some(Mission::GuardStarbase),
        Order::JoinAnotherFleet => Some(Mission::JoinAnotherFleet),
        Order::RendezvousSector => Some(Mission::RendezvousSector),
        Order::GuardBlockadeWorld => Some(Mission::GuardBlockadeWorld),
        Order::Salvage => Some(Mission::Salvage),
        _ => None,
    }
}

pub(super) fn vessel_size_summary(state: &FleetCombatState) -> (u32, u32, u32) {
    let small =
        state.counts[IDX_DD] + state.counts[IDX_SC] + state.counts[IDX_TT] + state.counts[IDX_ET];
    let medium = state.counts[IDX_CA];
    let large = state.counts[IDX_BB];
    (small, medium, large)
}

pub(super) fn push_planet_intel(
    planet_intel_events: &mut Vec<PlanetIntelEvent>,
    planet_idx: usize,
    viewer_empire_raw: u8,
    source: PlanetIntelSource,
) {
    if viewer_empire_raw == 0 {
        return;
    }
    planet_intel_events.push(PlanetIntelEvent {
        planet_idx,
        viewer_empire_raw,
        source,
        source_fleet_idx: None,
        observed_snapshot: None,
        stardate_week: None,
    });
}

pub(super) fn mission_kind_for_fleet(
    fleet: usize,
    bombard_set: &std::collections::HashSet<usize>,
    invade_set: &std::collections::HashSet<usize>,
    blitz_set: &std::collections::HashSet<usize>,
) -> Option<Mission> {
    if blitz_set.contains(&fleet) {
        Some(Mission::BlitzWorld)
    } else if invade_set.contains(&fleet) {
        Some(Mission::InvadeWorld)
    } else if bombard_set.contains(&fleet) {
        Some(Mission::BombardWorld)
    } else {
        None
    }
}
