use std::path::Path;

use nc_data::{
    Coords, FleetDetachSelection, fleet_notation, recompute_max_speed, set_armies_aboard,
    set_troop_transports,
};

use crate::commands::runtime::with_runtime_game_mut_and_export;

pub(crate) fn set_fleet_ships(
    dir: &Path,
    fleet_record_index_1_based: usize,
    scouts: u16,
    battleships: u16,
    cruisers: u16,
    destroyers: u16,
    transports: u16,
    armies_loaded: u16,
    etacs: u16,
) -> Result<(), Box<dyn std::error::Error>> {
    if armies_loaded > transports {
        return Err(format!(
            "loaded armies ({armies_loaded}) cannot exceed transports ({transports})"
        )
        .into());
    }

    let ships = with_runtime_game_mut_and_export(dir, |data| {
        let fleet = data
            .fleets
            .get_mut(fleet_record_index_1_based - 1)
            .ok_or_else(|| {
                format!("fleet record index out of range: {fleet_record_index_1_based}")
            })?;
        fleet.ships.scouts = scouts;
        fleet.ships.battleships = battleships;
        fleet.ships.cruisers = cruisers;
        fleet.ships.destroyers = destroyers;
        set_troop_transports(fleet, transports);
        set_armies_aboard(fleet, armies_loaded);
        fleet.ships.etacs = etacs;
        recompute_max_speed(fleet);
        if fleet.speed > fleet.max_speed {
            fleet.speed = fleet.max_speed;
        }
        Ok(fleet_notation(fleet))
    })?;

    println!(
        "Fleet {} ships set: {}",
        fleet_record_index_1_based,
        none_if_empty(ships)
    );
    Ok(())
}

pub(crate) fn set_fleet_location(
    dir: &Path,
    fleet_record_index_1_based: usize,
    x: u8,
    y: u8,
) -> Result<(), Box<dyn std::error::Error>> {
    with_runtime_game_mut_and_export(dir, |data| {
        let fleet = data
            .fleets
            .get_mut(fleet_record_index_1_based - 1)
            .ok_or_else(|| {
                format!("fleet record index out of range: {fleet_record_index_1_based}")
            })?;
        fleet.sector = Coords::new(x, y);
        Ok(())
    })?;

    println!(
        "Fleet {} current location set to ({},{})",
        fleet_record_index_1_based, x, y
    );
    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn detach_fleet_to_new_record(
    dir: &Path,
    player_record_index_1_based: usize,
    donor_fleet_record_index_1_based: usize,
    battleships: u16,
    cruisers: u16,
    destroyers: u16,
    full_transports: u16,
    empty_transports: u16,
    scouts: u16,
    etacs: u16,
    donor_speed: Option<u8>,
    new_fleet_roe: u8,
) -> Result<(), Box<dyn std::error::Error>> {
    let (result, ships) = with_runtime_game_mut_and_export(dir, |data| {
        let result = data
            .detach_ships_to_new_fleet(
                player_record_index_1_based,
                donor_fleet_record_index_1_based,
                FleetDetachSelection {
                    battleships,
                    cruisers,
                    destroyers,
                    full_transports,
                    empty_transports,
                    scouts,
                    etacs,
                    ..FleetDetachSelection::default()
                },
                donor_speed,
                new_fleet_roe,
            )
            .map_err(|err| err.to_string())?;
        let ships = fleet_notation(&data.fleets[result.new_fleet_record_index_1_based - 1]);
        Ok((result, ships))
    })?;

    println!(
        "Detached fleet {} -> new fleet {} for player {}: {} donor_speed={} roe={}",
        result.donor_fleet_record_index_1_based,
        result.new_fleet_record_index_1_based,
        player_record_index_1_based,
        none_if_empty(ships),
        donor_speed
            .map(|value| value.to_string())
            .unwrap_or_else(|| "-".to_string()),
        new_fleet_roe
    );
    Ok(())
}

/// A fleet's notation, or `none` for a fleet without ships.
fn none_if_empty(ships: String) -> String {
    if ships.is_empty() {
        "none".to_string()
    } else {
        ships
    }
}
