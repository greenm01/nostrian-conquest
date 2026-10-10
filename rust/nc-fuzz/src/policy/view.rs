//! Read-only views of the live state that policies choose from.

use nc_data::{CoreGameData, FleetRecord, PlanetRecord, map_size_for_player_count};

pub fn map_size(state: &CoreGameData) -> u8 {
    map_size_for_player_count(state.conquest.player_count())
}

pub fn fleet(state: &CoreGameData, record: usize) -> &FleetRecord {
    &state.fleets.records[record - 1]
}

pub fn planet(state: &CoreGameData, record: usize) -> &PlanetRecord {
    &state.planets.records[record - 1]
}

/// 1-based records of the empire's fleets that still carry ships.
pub fn owned_fleets(state: &CoreGameData, empire_raw: u8) -> Vec<usize> {
    records_where(&state.fleets.records, |fleet| {
        fleet.owner_empire_raw() == empire_raw && fleet.has_any_force()
    })
}

/// 1-based records of the empire's planets.
pub fn owned_planets(state: &CoreGameData, empire_raw: u8) -> Vec<usize> {
    records_where(&state.planets.records, |planet| {
        planet.owner_empire_slot_raw() == empire_raw
    })
}

/// 1-based records of placed planets that pass `keep`.
pub fn planets_where(state: &CoreGameData, keep: impl Fn(&PlanetRecord) -> bool) -> Vec<usize> {
    records_where(&state.planets.records, |planet| {
        planet.coords_raw() != [0, 0] && keep(planet)
    })
}

pub fn foreign_fleet_coords(state: &CoreGameData, empire_raw: u8) -> Vec<[u8; 2]> {
    state
        .fleets
        .records
        .iter()
        .filter(|fleet| fleet.owner_empire_raw() != empire_raw && fleet.has_any_force())
        .map(FleetRecord::current_location_coords_raw)
        .collect()
}

/// The empire's owned planet at the fleet's location, if any.
pub fn owned_planet_at(state: &CoreGameData, empire_raw: u8, coords: [u8; 2]) -> Option<usize> {
    owned_planets(state, empire_raw)
        .into_iter()
        .find(|&record| planet(state, record).coords_raw() == coords)
}

fn records_where<T>(records: &[T], keep: impl Fn(&T) -> bool) -> Vec<usize> {
    records
        .iter()
        .enumerate()
        .filter(|(_, record)| keep(record))
        .map(|(idx, _)| idx + 1)
        .collect()
}
