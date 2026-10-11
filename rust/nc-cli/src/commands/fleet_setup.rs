use std::path::Path;

use nc_data::{
    Coords, FleetDetachSelection, recompute_max_speed, set_armies_aboard, set_troop_transports,
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

    with_runtime_game_mut_and_export(dir, |data| {
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
        Ok(())
    })?;

    println!(
        "Fleet {} ships set: SC={} BB={} CA={} DD={} TT={} loaded_armies={} ETAC={}",
        fleet_record_index_1_based,
        scouts,
        battleships,
        cruisers,
        destroyers,
        transports,
        armies_loaded,
        etacs
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
    let result = with_runtime_game_mut_and_export(dir, |data| {
        data.detach_ships_to_new_fleet(
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
        .map_err(|err| err.to_string().into())
    })?;

    println!(
        "Detached fleet {} -> new fleet {} for player {}: BB={} CA={} DD={} fullTT={} emptyTT={} SC={} ETAC={} donor_speed={} roe={}",
        result.donor_fleet_record_index_1_based,
        result.new_fleet_record_index_1_based,
        player_record_index_1_based,
        battleships,
        cruisers,
        destroyers,
        full_transports,
        empty_transports,
        scouts,
        etacs,
        donor_speed
            .map(|value| value.to_string())
            .unwrap_or_else(|| "-".to_string()),
        new_fleet_roe
    );
    Ok(())
}
