use crate::{
    ContactReportSource, CoreGameData, EncounterDispositionEvent, EncounterDispositionReason,
    Mission, Order, PlanetIntelEvent, PlanetIntelSource, ScoutContactEvent, ShipLosses,
    StarbaseAlert,
};

use super::state::{
    FleetCombatState, IDX_BB, IDX_CA, IDX_DD, IDX_ET, IDX_SC, IDX_TT, fleet_state_from_records,
};
use nc_data::troop_transports;

/// Who made a sighting; a base also says what it makes of the fleet.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ContactObserver {
    Fleet(usize),
    Base(usize, StarbaseAlert),
}

/// The report of an observer sighting alien fleet `target_fleet_idx` at
/// `coords` in week `tick`; `identified` when it made out whose fleet it is.
#[allow(clippy::too_many_arguments)]
pub(crate) fn push_fleet_contact(
    scout_contact_events: &mut Vec<ScoutContactEvent>,
    planet_intel_events: &mut Vec<PlanetIntelEvent>,
    game_data: &CoreGameData,
    observer: ContactObserver,
    target_fleet_idx: usize,
    coords: [u8; 2],
    tick: u8,
    identified: bool,
) {
    let target = &game_data.fleets[target_fleet_idx];
    let target_empire = target.owner.0;
    let (small_vessels, medium_vessels, large_vessels) = vessel_size_summary(
        &fleet_state_from_records(game_data, &[target_fleet_idx], &[]),
    );
    let target_fleet_number = single_named_fleet_number(game_data, &[target_fleet_idx]);
    let (viewer_empire, source, reporting_fleet_number, reporting_initial, armies, in_system) =
        match observer {
            ContactObserver::Fleet(idx) => {
                let fleet = &game_data.fleets[idx];
                let source = contact_reporting_kind(fleet.order)
                    .map(ContactReportSource::FleetMission)
                    .unwrap_or(ContactReportSource::Fleet(fleet.fleet_number.0 as u8));
                (
                    fleet.owner.0,
                    source,
                    Some(fleet.fleet_number.0 as u8),
                    ship_counts_from_fleet(fleet),
                    u32::from(fleet.ships.loaded_transports),
                    fleet.depth.round() == 1,
                )
            }
            ContactObserver::Base(idx, alert) => {
                let base = &game_data.bases[idx];
                (
                    base.owner.0,
                    ContactReportSource::Starbase {
                        id: base.base_id.0,
                        alert,
                    },
                    None,
                    ShipLosses::default(),
                    0,
                    base.depth.round() == 1,
                )
            }
        };
    scout_contact_events.push(ScoutContactEvent {
        viewer_empire,
        source,
        reporting_fleet_number,
        reporting_initial,
        reporting_loaded_armies_initial: armies,
        coords,
        target_empire,
        target_fleet_number,
        small_vessels,
        medium_vessels,
        large_vessels,
        target_speed: target.speed,
        in_system,
        identified,
        decision: None,
        stardate_week: Some(tick),
    });
    // Only an identified fleet says whose world this is.
    if !identified {
        return;
    }
    push_civil_disorder_contact_intel(
        planet_intel_events,
        game_data,
        coords,
        viewer_empire,
        target_empire,
        tick,
    );
}

/// Observer fleet `fleet_idx` identified alien fleet `target_fleet_idx` and
/// did not fight it: its ROE forbids the fight (`RoeDeclined`, "we are
/// avoiding this enemy fleet") or the intercept failed (`InterceptFailed`,
/// "We are unable to intercept the enemy").
pub(crate) fn push_no_engagement(
    encounter_disposition_events: &mut Vec<EncounterDispositionEvent>,
    game_data: &CoreGameData,
    fleet_idx: usize,
    target_fleet_idx: usize,
    coords: [u8; 2],
    tick: u8,
    reason: EncounterDispositionReason,
) {
    let fleet = &game_data.fleets[fleet_idx];
    let (small_vessels, medium_vessels, large_vessels) = vessel_size_summary(
        &fleet_state_from_records(game_data, &[target_fleet_idx], &[]),
    );
    encounter_disposition_events.push(EncounterDispositionEvent::NoEngagement {
        fleet_idx,
        owner_empire: fleet.owner.0,
        mission: mission_kind_for_order(Some(fleet.order)),
        coords,
        friendly_initial: ship_counts_from_fleet(fleet),
        friendly_loaded_armies_initial: u32::from(fleet.ships.loaded_transports),
        target_empire: game_data.fleets[target_fleet_idx].owner.0,
        target_fleet_number: single_named_fleet_number(game_data, &[target_fleet_idx]),
        small_vessels,
        medium_vessels,
        large_vessels,
        reason,
        stardate_week: Some(tick),
    });
}

fn ship_counts_from_fleet(fleet: &nc_data::Fleet) -> ShipLosses {
    ShipLosses {
        destroyers: u32::from(fleet.ships.destroyers),
        cruisers: u32::from(fleet.ships.cruisers),
        battleships: u32::from(fleet.ships.battleships),
        scouts: u32::from(fleet.ships.scouts),
        transports: u32::from(troop_transports(fleet)),
        etacs: u32::from(fleet.ships.etacs),
    }
}

fn push_civil_disorder_contact_intel(
    planet_intel_events: &mut Vec<PlanetIntelEvent>,
    game_data: &CoreGameData,
    coords: [u8; 2],
    viewer_empire: u8,
    target_empire: u8,
    tick: u8,
) {
    if viewer_empire == 0
        || target_empire == 0
        || game_data.empire_campaign_state(target_empire)
            != Some(nc_data::CampaignState::CivilDisorder)
    {
        return;
    }
    let Some((planet_idx, _planet)) =
        game_data.planets.iter().enumerate().find(|(_, planet)| {
            planet.sector.to_array() == coords && planet.owner.0 == target_empire
        })
    else {
        return;
    };
    if planet_intel_events.iter().any(|event| {
        event.planet_idx == planet_idx
            && event.viewer_empire == viewer_empire
            && event.source == PlanetIntelSource::CivilDisorderContact
    }) {
        return;
    }
    push_planet_intel(
        planet_intel_events,
        planet_idx,
        viewer_empire,
        PlanetIntelSource::CivilDisorderContact,
    );
    if let Some(event) = planet_intel_events.last_mut() {
        event.stardate_week = Some(tick);
    }
}

pub(super) fn single_named_fleet_number(
    game_data: &CoreGameData,
    fleet_indices: &[usize],
) -> Option<u8> {
    let named_fleets = fleet_indices
        .iter()
        .filter_map(|idx| game_data.fleets.get(*idx))
        .filter(|fleet| {
            fleet.ships.destroyers > 0
                || fleet.ships.cruisers > 0
                || fleet.ships.battleships > 0
                || fleet.ships.scouts > 0
                || troop_transports(fleet) > 0
                || fleet.ships.etacs > 0
        })
        .map(|fleet| fleet.fleet_number.0 as u8)
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
        .filter_map(|idx| game_data.fleets.get(*idx))
        .map(|fleet| fleet.fleet_number.0 as u8)
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
        .filter(|idx| game_data.fleets.get(*idx).is_some())
        .filter(|idx| game_data.fleets[*idx].fleet_number.0 != 0)
        .min_by_key(|idx| game_data.fleets[*idx].fleet_number.0)
}

pub(super) fn loaded_armies_for_fleet_indices(
    game_data: &CoreGameData,
    fleet_indices: &[usize],
) -> u32 {
    fleet_indices
        .iter()
        .filter_map(|idx| game_data.fleets.get(*idx))
        .map(|fleet| fleet.ships.loaded_transports as u32)
        .sum()
}

pub(crate) fn mission_kind_for_order(order: Option<Order>) -> Option<Mission> {
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
    viewer_empire: u8,
    source: PlanetIntelSource,
) {
    if viewer_empire == 0 {
        return;
    }
    planet_intel_events.push(PlanetIntelEvent {
        planet_idx,
        viewer_empire,
        source,
        source_fleet_idx: None,
        observed_snapshot: None,
        stardate_week: None,
    });
}
