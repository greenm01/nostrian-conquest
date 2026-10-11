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
    starbase_planets: &[usize],
) -> Result<(), Box<dyn std::error::Error>> {
    planets::process_planet_economics(game_data, planets_with_builds, starbase_planets)
}

pub(super) fn recompute_player_planet_stats(game_data: &mut CoreGameData) {
    planets::recompute_player_planet_stats(game_data)
}

/// The worlds that take ECMAINT's starbase growth bonus this year
/// (`1852:011b`): one of the owner's bases rests in orbit in the world's
/// sector, at depth 0.5 or more with its moving flag clear. ECMAINT tests
/// this before the year's ticks, so the caller takes it before movement.
pub(super) fn starbase_planets(game_data: &CoreGameData) -> Vec<usize> {
    game_data
        .planets
        .iter()
        .enumerate()
        .filter(|(_, planet)| {
            let owner = planet.owner.0;
            owner != 0
                && game_data.bases.iter().any(|base| {
                    base.owner.0 == owner
                        && base.base_id.0 != 0
                        && base.sector == planet.sector
                        && base.depth >= nc_data::Real48::HALF
                        && !base.moving
                })
        })
        .map(|(idx, _)| idx)
        .collect()
}
