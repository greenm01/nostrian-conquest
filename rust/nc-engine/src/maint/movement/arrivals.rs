use super::super::{
    ColonizationEvent, DiplomaticEscalationEvent, Mission, MissionEvent, MissionOutcome,
    MovementEvents, PendingObservationEvent, PlanetIntelEvent, PlanetIntelSource,
};
use super::salvage::queue_salvage_resolution;
use crate::{CoreGameData, Order};
use nc_data::{MissionRetargetEvent, build_runtime_planet_intel_snapshot};

fn queue_local_intrusion_escalation(
    movement_events: &mut MovementEvents,
    owner_empire: u8,
    defender_empire: u8,
) {
    if owner_empire != 0 && defender_empire != 0 && owner_empire != defender_empire {
        movement_events
            .diplomatic_escalation_events
            .push(DiplomaticEscalationEvent {
                left_empire: owner_empire,
                right_empire: defender_empire,
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
                stardate_week: None,
            });
        }
        Order::ScoutSector => {
            movement_events.mission_events.push(MissionEvent {
                fleet_idx,
                owner_empire,
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
                .iter()
                .position(|planet| planet.sector.to_array() == coords);
            if planet_idx.is_some_and(|idx| game_data.planets[idx].owner.0 == owner_empire) {
                // ECMAINT's scout has nothing to learn at its own world: it
                // holds (`0000:5c18`).
                let fleet = &mut game_data.fleets[fleet_idx];
                fleet.order = Order::HoldPosition;
                movement_events
                    .retargets
                    .push(MissionRetargetEvent::TargetWorldOwned {
                        fleet_idx,
                        reporting_fleet_number: Some(fleet.fleet_number.0 as u8),
                        owner_empire,
                        mission: Mission::ScoutSolarSystem,
                        target_coords: coords,
                        coords: fleet.sector.to_array(),
                        in_system: fleet.depth.round() == 1,
                        follow_on: Order::HoldPosition,
                        stardate_week: None,
                    });
                return Ok(());
            }
            let intel_event = planet_idx.map(|planet_idx| PlanetIntelEvent {
                planet_idx,
                viewer_empire: owner_empire,
                source: PlanetIntelSource::ScoutSolarSystem,
                source_fleet_idx: Some(fleet_idx),
                observed_snapshot: build_runtime_planet_intel_snapshot(
                    game_data,
                    owner_empire,
                    game_data.conquest.game_year,
                    planet_idx,
                    PlanetIntelSource::ScoutSolarSystem,
                ),
                stardate_week: None,
            });
            movement_events
                .pending_observation_events
                .push(PendingObservationEvent {
                    fleet_idx,
                    owner_empire,
                    kind: Mission::ScoutSolarSystem,
                    outcome: MissionOutcome::Succeeded,
                    planet_idx,
                    location_coords: coords,
                    target_coords: coords,
                    intel_event,
                    stardate_week: None,
                    exit_week: None,
                });
        }
        Order::ViewWorld => {
            let planet_idx = game_data
                .planets
                .iter()
                .position(|planet| planet.sector.to_array() == coords);
            let intel_event = planet_idx.map(|planet_idx| PlanetIntelEvent {
                planet_idx,
                viewer_empire: owner_empire,
                source: PlanetIntelSource::ViewWorld,
                source_fleet_idx: Some(fleet_idx),
                observed_snapshot: build_runtime_planet_intel_snapshot(
                    game_data,
                    owner_empire,
                    game_data.conquest.game_year,
                    planet_idx,
                    PlanetIntelSource::ViewWorld,
                ),
                stardate_week: None,
            });
            movement_events
                .pending_observation_events
                .push(PendingObservationEvent {
                    fleet_idx,
                    owner_empire,
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
                    stardate_week: None,
                    exit_week: None,
                });
        }
        Order::Salvage => {
            // The tick's home-world check passed, so the fleet's empire owns
            // a world here.
            let planet_idx = game_data
                .planets
                .iter()
                .position(|planet| planet.sector.to_array() == coords)
                .ok_or("a Salvage fleet arrived where its empire has no world")?;
            queue_salvage_resolution(
                game_data,
                movement_events,
                to_remove,
                fleet_idx,
                owner_empire,
                planet_idx,
                coords,
            );
        }
        Order::GuardBlockadeWorld => {
            let planet_idx = game_data
                .planets
                .iter()
                .position(|planet| planet.sector.to_array() == coords);
            if let Some(planet_idx) = planet_idx {
                let defender_empire = game_data.planets[planet_idx].owner.0;
                queue_local_intrusion_escalation(movement_events, owner_empire, defender_empire);
            }
            movement_events.mission_events.push(MissionEvent {
                fleet_idx,
                owner_empire,
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
                owner_empire,
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
                owner_empire,
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
                owner_empire,
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
                owner_empire,
                kind: Mission::SeekHome,
                outcome: MissionOutcome::Succeeded,
                abort_reason: None,
                planet_idx: game_data
                    .planets
                    .iter()
                    .position(|planet| planet.sector.to_array() == coords),
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
    let owner_empire = game_data.fleets[fleet_idx].owner.0;
    let planet_idx = game_data
        .planets
        .iter()
        .position(|planet| planet.sector.to_array() == coords);
    if let Some(planet_idx) = planet_idx {
        let defender_empire = game_data.planets[planet_idx].owner.0;
        queue_local_intrusion_escalation(movement_events, owner_empire, defender_empire);
    }
    movement_events.mission_events.push(MissionEvent {
        fleet_idx,
        owner_empire,
        kind,
        outcome: MissionOutcome::Arrived,
        abort_reason: None,
        planet_idx,
        location_coords: Some(coords),
        target_coords: Some(coords),
        stardate_week: None,
    });
}

/// ECMAINT's View World action (0b5d:219c) leaves the fleet on a Move out of
/// the system: 0x22 = 0 at full speed. A fleet that has already climbed back
/// to depth 0 has also passed that Move's test, which stops it on Hold
/// (0b5d:00ec). Returns whether the Move resolved.
pub(super) fn complete_view_world(fleet: &mut nc_data::Fleet) -> bool {
    fleet.target_depth = 0;
    if fleet.depth.is_zero() {
        fleet.speed = 0;
        fleet.order = Order::HoldPosition;
        true
    } else {
        fleet.speed = fleet.max_speed;
        fleet.order = Order::MoveOnly;
        false
    }
}
