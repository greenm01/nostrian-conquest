//! Every planet and fleet sits on the 1-based map, one planet per sector.

use std::collections::BTreeMap;

use nc_data::{CoreGameData, map_size_for_player_count, sector_within_map};

use super::Violation;

pub fn check(state: &CoreGameData) -> Vec<Violation> {
    let map_size = map_size_for_player_count(state.conquest.player_count());
    let mut violations = Vec::new();
    let mut out_of_map = |what: String, coords: [u8; 2]| {
        if !sector_within_map(coords, map_size) {
            violations.push(Violation::CoordinateOutOfMap {
                what,
                coords,
                map_size,
            });
        }
    };
    for (idx, planet) in state.planets.iter().enumerate() {
        out_of_map(format!("planet {}", idx + 1), planet.coords_raw());
    }
    for (idx, fleet) in state.fleets.iter().enumerate() {
        out_of_map(
            format!("fleet record {}", idx + 1),
            fleet.current_location_coords_raw(),
        );
    }

    let mut seen = BTreeMap::new();
    for (idx, planet) in state.planets.iter().enumerate() {
        if let Some(first) = seen.insert(planet.coords_raw(), idx + 1) {
            violations.push(Violation::PlanetsShareSector {
                first_record: first,
                second_record: idx + 1,
                coords: planet.coords_raw(),
            });
        }
    }
    violations
}
