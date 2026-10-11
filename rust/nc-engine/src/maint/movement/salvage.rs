use super::super::{
    Mission, MissionEvent, MissionOutcome, MovementEvents, SalvageFailureReason,
    SalvageResolvedEvent,
};
use super::stepper::set_fleet_to_local_hold;
use crate::CoreGameData;
use nc_data::{Real48, has_any_force, stored_production_points};

pub(super) fn queue_salvage_resolution(
    game_data: &mut CoreGameData,
    movement_events: &mut MovementEvents,
    to_remove: &mut [bool],
    fleet_idx: usize,
    owner_empire: u8,
    planet_idx: Option<usize>,
    coords: [u8; 2],
) -> Result<(), Box<dyn std::error::Error>> {
    // Empty fleets are culled at turn start; a salvage fleet reaching this
    // point should always carry at least one unit.  The one exception is a
    // fleet that loses all ships to ROE pursuit fire mid-transit on the same
    // turn it arrives — the cull cannot catch that, so we fall through rather
    // than panic.
    debug_assert!(
        has_any_force(&game_data.fleets[fleet_idx]),
        "salvage dispatched to empty fleet (fleet_idx={fleet_idx}) -- should have been culled at turn start"
    );

    let salvage_event = resolve_salvage_arrival(game_data, fleet_idx, owner_empire, planet_idx)?;
    match salvage_event {
        SalvageResolvedEvent::Succeeded { .. } => {
            movement_events.mission_events.push(MissionEvent {
                fleet_idx,
                owner_empire,
                kind: Mission::Salvage,
                outcome: MissionOutcome::Succeeded,
                abort_reason: None,
                planet_idx,
                location_coords: Some(coords),
                target_coords: Some(coords),
                stardate_week: None,
            });
            movement_events.salvage_events.push(salvage_event);
            // Always remove the fleet on a successful salvage arrival.
            // Any fleet that reaches this point with zero ships was already
            // empty before the turn (culled) or lost ships mid-transit; either
            // way the mission is done and the record should be discarded.
            to_remove[fleet_idx] = true;
        }
        SalvageResolvedEvent::Failed { .. } => {
            movement_events.mission_events.push(MissionEvent {
                fleet_idx,
                owner_empire,
                kind: Mission::Salvage,
                outcome: MissionOutcome::Failed,
                abort_reason: None,
                planet_idx,
                location_coords: Some(coords),
                target_coords: Some(coords),
                stardate_week: None,
            });
            movement_events.salvage_events.push(salvage_event);
        }
    }

    Ok(())
}

fn resolve_salvage_arrival(
    game_data: &mut CoreGameData,
    fleet_idx: usize,
    owner_empire: u8,
    planet_idx: Option<usize>,
) -> Result<SalvageResolvedEvent, Box<dyn std::error::Error>> {
    let coords = game_data.fleets[fleet_idx].sector.to_array();
    let Some(planet_idx) = planet_idx else {
        let fleet = &mut game_data.fleets[fleet_idx];
        set_fleet_to_local_hold(fleet);
        return Ok(SalvageResolvedEvent::Failed {
            fleet_idx,
            owner_empire,
            planet_idx: None,
            coords,
            reason: SalvageFailureReason::NoPlanetAtTarget,
            stardate_week: None,
        });
    };

    if game_data.planets[planet_idx].owner.0 != owner_empire {
        let fleet = &mut game_data.fleets[fleet_idx];
        set_fleet_to_local_hold(fleet);
        return Ok(SalvageResolvedEvent::Failed {
            fleet_idx,
            owner_empire,
            planet_idx: Some(planet_idx),
            coords,
            reason: SalvageFailureReason::PlanetNotOwned,
            stardate_week: None,
        });
    }

    // The yield joins the world's stored goods and each loaded transport
    // lands its army (`0b5d:4d65`).
    let fleet = &game_data.fleets[fleet_idx];
    let recovered_points = fleet_salvage_value(fleet);
    let armies_landed = fleet.ships.loaded_transports;
    let fleet_number = fleet.fleet_number.0 as u8;
    let planet = &mut game_data.planets[planet_idx];
    let current_stored = stored_production_points(planet);
    planet.stored_goods =
        Real48::from_f64(f64::from(current_stored.saturating_add(recovered_points)));
    planet.armies = planet.armies.saturating_add(armies_landed);

    Ok(SalvageResolvedEvent::Succeeded {
        fleet_idx,
        fleet_number,
        owner_empire,
        planet_idx,
        coords,
        recovered_points,
        armies_landed,
        stardate_week: None,
    })
}

/// What an arriving Salvage fleet yields (`0b5d:4d65`): each ship's half
/// cost, rounded up. A transport is worth the same empty or loaded.
fn fleet_salvage_value(fleet: &nc_data::Fleet) -> u32 {
    let ships = &fleet.ships;
    let transports = u32::from(ships.empty_transports) + u32::from(ships.loaded_transports);
    u32::from(ships.battleships) * 23
        + u32::from(ships.cruisers) * 8
        + u32::from(ships.destroyers) * 3
        + transports * 3
        + u32::from(ships.etacs) * 10
        + u32::from(ships.scouts) * 8
}
