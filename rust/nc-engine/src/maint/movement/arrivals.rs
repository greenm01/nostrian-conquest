use super::super::{
    ColonizationEvent, DiplomaticEscalationEvent, Mission, MissionEvent, MissionOutcome,
    MovementEvents, PendingObservationEvent, PlanetIntelEvent, PlanetIntelSource,
};
use super::salvage::queue_salvage_resolution;
use crate::{CoreGameData, Order};
use nc_data::build_runtime_planet_intel_snapshot;

fn queue_local_intrusion_escalation(
    movement_events: &mut MovementEvents,
    owner_empire_raw: u8,
    defender_empire_raw: u8,
) {
    if owner_empire_raw != 0 && defender_empire_raw != 0 && owner_empire_raw != defender_empire_raw
    {
        movement_events
            .diplomatic_escalation_events
            .push(DiplomaticEscalationEvent {
                left_empire_raw: owner_empire_raw,
                right_empire_raw: defender_empire_raw,
                stardate_week: None,
            });
    }
}

pub(super) fn handle_fleet_arrival(
    game_data: &mut CoreGameData,
    movement_events: &mut MovementEvents,
    to_remove: &mut [bool],
    fleet_idx: usize,
    order_kind: Order,
    owner_empire: u8,
    coords: [u8; 2],
) -> Result<(), Box<dyn std::error::Error>> {
    match order_kind {
        Order::ColonizeWorld => {
            movement_events.colonization_events.push(ColonizationEvent {
                fleet_idx,
                coords,
                owner_empire,
            });
        }
        Order::ScoutSector => {
            movement_events.mission_events.push(MissionEvent {
                fleet_idx,
                owner_empire_raw: owner_empire,
                kind: Mission::ScoutSector,
                outcome: MissionOutcome::Arrived,
                abort_reason: None,
                planet_idx: None,
                location_coords: Some(coords),
                target_coords: Some(coords),
                stardate_week: None,
            });
        }
        Order::ScoutSolarSystem => {
            let planet_idx = game_data
                .planets
                .records
                .iter()
                .position(|planet| planet.coords_raw() == coords);
            let intel_event = planet_idx.map(|planet_idx| PlanetIntelEvent {
                planet_idx,
                viewer_empire_raw: owner_empire,
                source: PlanetIntelSource::ScoutSolarSystem,
                source_fleet_idx: Some(fleet_idx),
                observed_snapshot: build_runtime_planet_intel_snapshot(
                    game_data,
                    owner_empire,
                    game_data.conquest.game_year(),
                    planet_idx,
                    PlanetIntelSource::ScoutSolarSystem,
                ),
                stardate_week: None,
            });
            movement_events
                .pending_observation_events
                .push(PendingObservationEvent {
                    fleet_idx,
                    owner_empire_raw: owner_empire,
                    kind: Mission::ScoutSolarSystem,
                    outcome: MissionOutcome::Succeeded,
                    planet_idx,
                    location_coords: coords,
                    target_coords: coords,
                    intel_event,
                });
        }
        Order::ViewWorld => {
            let planet_idx = game_data
                .planets
                .records
                .iter()
                .position(|planet| planet.coords_raw() == coords);
            let intel_event = planet_idx.map(|planet_idx| PlanetIntelEvent {
                planet_idx,
                viewer_empire_raw: owner_empire,
                source: PlanetIntelSource::ViewWorld,
                source_fleet_idx: Some(fleet_idx),
                observed_snapshot: build_runtime_planet_intel_snapshot(
                    game_data,
                    owner_empire,
                    game_data.conquest.game_year(),
                    planet_idx,
                    PlanetIntelSource::ViewWorld,
                ),
                stardate_week: None,
            });
            movement_events
                .pending_observation_events
                .push(PendingObservationEvent {
                    fleet_idx,
                    owner_empire_raw: owner_empire,
                    kind: Mission::ViewWorld,
                    outcome: if planet_idx.is_some() {
                        MissionOutcome::Succeeded
                    } else {
                        MissionOutcome::Failed
                    },
                    planet_idx,
                    location_coords: coords,
                    target_coords: coords,
                    intel_event,
                });
        }
        Order::Salvage => {
            let planet_idx = game_data
                .planets
                .records
                .iter()
                .position(|planet| planet.coords_raw() == coords);
            queue_salvage_resolution(
                game_data,
                movement_events,
                to_remove,
                fleet_idx,
                owner_empire,
                planet_idx,
                coords,
            )?;
        }
        Order::GuardStarbase => {
            movement_events.mission_events.push(MissionEvent {
                fleet_idx,
                owner_empire_raw: owner_empire,
                kind: Mission::GuardStarbase,
                outcome: MissionOutcome::Arrived,
                abort_reason: None,
                planet_idx: None,
                location_coords: Some(coords),
                target_coords: Some(coords),
                stardate_week: None,
            });
        }
        Order::GuardBlockadeWorld => {
            let planet_idx = game_data
                .planets
                .records
                .iter()
                .position(|planet| planet.coords_raw() == coords);
            if let Some(planet_idx) = planet_idx {
                let defender_empire = game_data.planets.records[planet_idx].owner_empire_slot_raw();
                queue_local_intrusion_escalation(movement_events, owner_empire, defender_empire);
            }
            movement_events.mission_events.push(MissionEvent {
                fleet_idx,
                owner_empire_raw: owner_empire,
                kind: Mission::GuardBlockadeWorld,
                outcome: MissionOutcome::Arrived,
                abort_reason: None,
                planet_idx,
                location_coords: Some(coords),
                target_coords: Some(coords),
                stardate_week: None,
            });
        }
        Order::RendezvousSector => {
            movement_events.mission_events.push(MissionEvent {
                fleet_idx,
                owner_empire_raw: owner_empire,
                kind: Mission::RendezvousSector,
                outcome: MissionOutcome::Arrived,
                abort_reason: None,
                planet_idx: None,
                location_coords: Some(coords),
                target_coords: Some(coords),
                stardate_week: None,
            });
        }
        Order::MoveOnly => {
            movement_events.mission_events.push(MissionEvent {
                fleet_idx,
                owner_empire_raw: owner_empire,
                kind: Mission::MoveOnly,
                outcome: MissionOutcome::Succeeded,
                abort_reason: None,
                planet_idx: None,
                location_coords: Some(coords),
                target_coords: Some(coords),
                stardate_week: None,
            });
        }
        Order::PatrolSector => {
            movement_events.mission_events.push(MissionEvent {
                fleet_idx,
                owner_empire_raw: owner_empire,
                kind: Mission::PatrolSector,
                outcome: MissionOutcome::Arrived,
                abort_reason: None,
                planet_idx: None,
                location_coords: Some(coords),
                target_coords: Some(coords),
                stardate_week: None,
            });
        }
        Order::SeekHome => {
            movement_events.mission_events.push(MissionEvent {
                fleet_idx,
                owner_empire_raw: owner_empire,
                kind: Mission::SeekHome,
                outcome: MissionOutcome::Succeeded,
                abort_reason: None,
                planet_idx: game_data
                    .planets
                    .records
                    .iter()
                    .position(|planet| planet.coords_raw() == coords),
                location_coords: Some(coords),
                target_coords: Some(coords),
                stardate_week: None,
            });
        }
        _ => {}
    }

    Ok(())
}

/// A Bombard/Invade/Blitz fleet entered its target sector this year.
pub(super) fn handle_hostile_target_entry(
    game_data: &CoreGameData,
    movement_events: &mut MovementEvents,
    fleet_idx: usize,
    order_kind: Order,
    coords: [u8; 2],
) {
    let kind = match order_kind {
        Order::BombardWorld => Mission::BombardWorld,
        Order::InvadeWorld => Mission::InvadeWorld,
        Order::BlitzWorld => Mission::BlitzWorld,
        _ => return,
    };
    let owner_empire = game_data.fleets.records[fleet_idx].owner_empire_raw();
    let planet_idx = game_data
        .planets
        .records
        .iter()
        .position(|planet| planet.coords_raw() == coords);
    if let Some(planet_idx) = planet_idx {
        let defender_empire = game_data.planets.records[planet_idx].owner_empire_slot_raw();
        queue_local_intrusion_escalation(movement_events, owner_empire, defender_empire);
    }
    movement_events.mission_events.push(MissionEvent {
        fleet_idx,
        owner_empire_raw: owner_empire,
        kind,
        outcome: MissionOutcome::Arrived,
        abort_reason: None,
        planet_idx,
        location_coords: Some(coords),
        target_coords: Some(coords),
        stardate_week: None,
    });
}

pub(super) fn set_view_world_completion_hold(fleet: &mut nc_data::FleetRecord) {
    // ViewWorld is a one-shot mission: after the observation fires, revert the
    // fleet to HoldPosition so the player must issue new orders. ECMAINT's
    // View World action clears the mission aux bytes and lifts the fleet back
    // out to depth 0.
    fleet.set_current_speed(0);
    fleet.set_standing_order_kind(Order::HoldPosition);
    fleet.set_mission_aux_bytes([0, 0]);
    fleet.set_depth(nc_data::Real48::ZERO);
}
