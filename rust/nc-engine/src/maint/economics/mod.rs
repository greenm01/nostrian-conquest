mod ai;
mod build;
mod planets;

use crate::CoreGameData;

pub fn process_autopilot_fleet_orders(
    game_data: &mut CoreGameData,
) -> Result<(), Box<dyn std::error::Error>> {
    ai::process_autopilot_fleet_orders(game_data)
}

pub fn process_autopilot_ai(
    game_data: &mut CoreGameData,
) -> Result<(), Box<dyn std::error::Error>> {
    ai::process_autopilot_ai(game_data)
}

pub(super) fn process_build_completion(
    game_data: &mut CoreGameData,
) -> Result<Vec<usize>, Box<dyn std::error::Error>> {
    build::process_build_completion(game_data)
}

pub(super) fn process_planet_economics(
    game_data: &mut CoreGameData,
    planets_with_builds: &[usize],
    newly_colonized_planets: &[usize],
) -> Result<(), Box<dyn std::error::Error>> {
    planets::process_planet_economics(game_data, planets_with_builds, newly_colonized_planets)
}

pub(super) fn recompute_player_planet_stats(game_data: &mut CoreGameData) {
    planets::recompute_player_planet_stats(game_data)
}

fn planet_has_friendly_starbase(
    game_data: &CoreGameData,
    owner_empire_raw: u8,
    coords: [u8; 2],
) -> bool {
    game_data.bases.records.iter().any(|base| {
        base.owner_empire_raw() == owner_empire_raw
            && base.coords_raw() == coords
            && base.base_id_raw() != 0
    })
}
