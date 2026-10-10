mod geometry;
mod placement;
mod scoring;

use nc_data::{
    CoreGameData, GameRng, GameStateBuilder, GameStateMutationError, PlanetRecord, RNG_TAG_MAPGEN,
    is_valid_player_count, map_size_for_player_count,
};
use placement::{generate_homeworlds, generate_neutral_worlds};
use scoring::{all_systems_unique, score_map};

const REROLL_CANDIDATES: usize = 64;
const HOMEWORLD_EDGE_MARGIN: f32 = 2.0;
const HOMEWORLD_MIN_DISTANCE_RATIO: f32 = 0.28;
const LOCAL_WORLD_COUNT_PER_PLAYER: usize = 2;
const EARLY_RADIUS: f32 = 5.5;
const CONTESTED_GAP_LIMIT: f32 = 2.75;
const NEUTRAL_MIN_SPACING: f32 = 1.6;
const NEUTRAL_EDGE_RING_THRESHOLD: f32 = 2.0;
const LOCAL_WORLD_EDGE_CLEARANCE_WEIGHT: f32 = 1.0;
const FRONTIER_WORLD_EDGE_CLEARANCE_WEIGHT: f32 = 1.9;
const FRONTIER_WORLD_EDGE_RING_PENALTY: f32 = 6.0;
const MAP_EDGE_RING_PENALTY: f32 = 4.5;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GeneratedWorld {
    pub coords: [u8; 2],
    pub potential_production: u8,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MapMetrics {
    pub score: f32,
    pub early_count_range: u8,
    pub early_value_range: u16,
    pub contested_worlds: u8,
    pub min_homeworld_spacing: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GeneratedMap {
    pub seed: u64,
    pub map_size: u8,
    pub homeworld_coords: Vec<[u8; 2]>,
    pub neutral_worlds: Vec<GeneratedWorld>,
    pub metrics: MapMetrics,
}

pub fn build_seeded_new_game(
    player_count: u8,
    year: u16,
    seed: u64,
) -> Result<CoreGameData, GameStateMutationError> {
    ensure_valid_player_count(player_count)?;
    let generated = generate_map(player_count, seed);
    let mut data = GameStateBuilder::new()
        .with_player_count(player_count)
        .with_year(year)
        .with_homeworld_coords(generated.homeworld_coords.clone())
        .build_joinable_new_game_baseline()?;

    for (idx, world) in generated.neutral_worlds.iter().enumerate() {
        let record_index = player_count as usize + idx;
        if let Some(planet) = data.planets.records.get_mut(record_index) {
            seed_unowned_world(planet, *world, record_index + 1);
        }
    }

    Ok(data)
}

pub fn build_seeded_initialized_game(
    player_count: u8,
    year: u16,
    seed: u64,
) -> Result<CoreGameData, GameStateMutationError> {
    ensure_valid_player_count(player_count)?;
    let generated = generate_map(player_count, seed);
    let mut data = GameStateBuilder::new()
        .with_player_count(player_count)
        .with_year(year)
        .with_homeworld_coords(generated.homeworld_coords.clone())
        .build_initialized_baseline()?;

    for (idx, world) in generated.neutral_worlds.iter().enumerate() {
        let record_index = player_count as usize + idx;
        if let Some(planet) = data.planets.records.get_mut(record_index) {
            seed_unowned_world(planet, *world, record_index + 1);
        }
    }

    Ok(data)
}

/// The builder would otherwise clamp the count and mismatch the generated map.
fn ensure_valid_player_count(player_count: u8) -> Result<(), GameStateMutationError> {
    if is_valid_player_count(player_count) {
        Ok(())
    } else {
        Err(GameStateMutationError::InvalidPlayerCount { player_count })
    }
}

pub fn generate_map(player_count: u8, seed: u64) -> GeneratedMap {
    let map_size = map_size_for_player_count(player_count);
    let mut best_map = None;
    let mut best_score = f32::MIN;

    for reroll in 0..REROLL_CANDIDATES {
        let candidate_seed =
            seed ^ ((player_count as u64) << 32) ^ ((reroll as u64) << 48) ^ 0xEC15_1000_0000_0000;
        let mut rng = GameRng::from_context(
            seed,
            RNG_TAG_MAPGEN,
            &[player_count as u64, reroll as u64, candidate_seed],
        );
        let homeworld_coords = generate_homeworlds(player_count, map_size, &mut rng);
        let neutral_worlds = generate_neutral_worlds(
            player_count,
            map_size,
            seed,
            reroll as u32,
            &homeworld_coords,
            &mut rng,
        );
        let metrics = score_map(map_size, &homeworld_coords, &neutral_worlds);
        if metrics.score > best_score {
            best_score = metrics.score;
            best_map = Some(GeneratedMap {
                seed,
                map_size,
                homeworld_coords,
                neutral_worlds,
                metrics,
            });
        }
    }

    let generated = best_map.expect("map generation should always produce a candidate");
    debug_assert!(all_systems_unique(
        &generated.homeworld_coords,
        &generated.neutral_worlds
    ));
    generated
}

fn seed_unowned_world(
    planet: &mut PlanetRecord,
    world: GeneratedWorld,
    world_index_1_based: usize,
) {
    *planet = PlanetRecord::new_zeroed();
    planet.set_coords_raw(world.coords);
    planet.set_potential_production_raw([world.potential_production, 0]);
    planet.set_economy_marker_raw(0);
    planet.set_planet_name(&format!("World {:02}", world_index_1_based));
}
