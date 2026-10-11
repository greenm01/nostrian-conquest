use super::super::{
    Mission, MissionEvent, MissionOutcome, MovementEvents, SalvageFailureReason,
    SalvageResolvedEvent,
};
use super::stepper::set_fleet_to_local_hold;
use crate::{CoreGameData, ProductionItemKind};

pub(super) fn queue_salvage_resolution(
    game_data: &mut CoreGameData,
    movement_events: &mut MovementEvents,
    to_remove: &mut [bool],
    fleet_idx: usize,
    owner_empire_raw: u8,
    planet_idx: Option<usize>,
    coords: [u8; 2],
) -> Result<(), Box<dyn std::error::Error>> {
    // Empty fleets are culled at turn start; a salvage fleet reaching this
    // point should always carry at least one unit.  The one exception is a
    // fleet that loses all ships to ROE pursuit fire mid-transit on the same
    // turn it arrives — the cull cannot catch that, so we fall through rather
    // than panic.
    debug_assert!(
        game_data.fleets[fleet_idx].has_any_force(),
        "salvage dispatched to empty fleet (fleet_idx={fleet_idx}) -- should have been culled at turn start"
    );

    let salvage_event =
        resolve_salvage_arrival(game_data, fleet_idx, owner_empire_raw, planet_idx)?;
    match salvage_event {
        SalvageResolvedEvent::Succeeded { .. } => {
            movement_events.mission_events.push(MissionEvent {
                fleet_idx,
                owner_empire_raw,
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
                owner_empire_raw,
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
    owner_empire_raw: u8,
    planet_idx: Option<usize>,
) -> Result<SalvageResolvedEvent, Box<dyn std::error::Error>> {
    let coords = game_data.fleets[fleet_idx].current_location_coords_raw();
    let Some(planet_idx) = planet_idx else {
        let fleet = &mut game_data.fleets[fleet_idx];
        set_fleet_to_local_hold(fleet);
        return Ok(SalvageResolvedEvent::Failed {
            fleet_idx,
            owner_empire_raw,
            planet_idx: None,
            coords,
            reason: SalvageFailureReason::NoPlanetAtTarget,
            stardate_week: None,
        });
    };

    if game_data.planets[planet_idx].owner_empire_slot_raw() != owner_empire_raw {
        let fleet = &mut game_data.fleets[fleet_idx];
        set_fleet_to_local_hold(fleet);
        return Ok(SalvageResolvedEvent::Failed {
            fleet_idx,
            owner_empire_raw,
            planet_idx: Some(planet_idx),
            coords,
            reason: SalvageFailureReason::PlanetNotOwned,
            stardate_week: None,
        });
    }

    let recovered_points = fleet_salvage_value(&game_data.fleets[fleet_idx]);
    let current_stored = game_data.planets[planet_idx].stored_production_points();
    game_data.planets[planet_idx]
        .set_stored_production_points(current_stored.saturating_add(recovered_points));

    Ok(SalvageResolvedEvent::Succeeded {
        fleet_idx,
        fleet_number: game_data.fleets[fleet_idx].local_slot_word_raw() as u8,
        owner_empire_raw,
        planet_idx,
        coords,
        recovered_points,
        stardate_week: None,
    })
}

fn fleet_salvage_value(fleet: &nc_data::Fleet) -> u32 {
    let total_cost = u32::from(fleet.destroyer_count())
        * purchase_cost(ProductionItemKind::Destroyer)
        + u32::from(fleet.cruiser_count()) * purchase_cost(ProductionItemKind::Cruiser)
        + u32::from(fleet.battleship_count()) * purchase_cost(ProductionItemKind::Battleship)
        + u32::from(fleet.scout_count()) * purchase_cost(ProductionItemKind::Scout)
        + u32::from(fleet.troop_transport_count()) * purchase_cost(ProductionItemKind::Transport)
        + u32::from(fleet.etac_count()) * purchase_cost(ProductionItemKind::Etac)
        + u32::from(fleet.army_count()) * purchase_cost(ProductionItemKind::Army);
    total_cost / 2
}

fn purchase_cost(kind: ProductionItemKind) -> u32 {
    match kind {
        ProductionItemKind::Destroyer => 5,
        ProductionItemKind::Cruiser => 15,
        ProductionItemKind::Battleship => 45,
        ProductionItemKind::Scout => 15,
        ProductionItemKind::Transport => 5,
        ProductionItemKind::Etac => 20,
        ProductionItemKind::GroundBattery => 20,
        ProductionItemKind::Army => 2,
        ProductionItemKind::Starbase => 50,
        ProductionItemKind::Unknown(_) => 0,
    }
}
