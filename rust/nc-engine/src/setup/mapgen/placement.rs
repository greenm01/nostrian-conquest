use super::{
    CONTESTED_GAP_LIMIT, GeneratedWorld, HOMEWORLD_MIN_DISTANCE, LOCAL_WORLD_COUNT_PER_PLAYER,
};
use crate::setup::mapgen::geometry::{
    HomeworldRegion, candidate_allowed, clamp_coord, density_noise, distance, distance_sq,
    edge_clearance, fallback_frontier_world, fallback_local_world, frontier_world_edge_bias,
    hashed_range, homeworld_regions, local_world_edge_bias, nearest_homeworld,
    nearest_used_distance, sorted_homeworld_distances,
};
use crate::setup::mapgen::scoring::frontier_balance_bonus;
use nc_data::GameRng;

pub(super) fn generate_homeworlds(
    player_count: u8,
    map_size: u8,
    rng: &mut GameRng,
) -> Vec<[u8; 2]> {
    let min_distance_sq = HOMEWORLD_MIN_DISTANCE.powi(2);
    let regions = homeworld_regions(player_count, map_size);
    let mut homeworlds = Vec::with_capacity(player_count as usize);

    for region in &regions {
        let mut best = None;
        let mut best_score = f32::MIN;
        for _ in 0..96 {
            let x = rng.range_u8(region.min[0], region.max[0]);
            let y = rng.range_u8(region.min[1], region.max[1]);
            let candidate = [x, y];
            if homeworlds.contains(&candidate) {
                continue;
            }

            let mut min_dist_sq = f32::MAX;
            for other in &homeworlds {
                let dist_sq = distance_sq(candidate, *other);
                min_dist_sq = min_dist_sq.min(dist_sq);
            }
            if !homeworlds.is_empty() && min_dist_sq < min_distance_sq {
                continue;
            }

            let edge_clearance = edge_clearance(candidate, map_size);
            let region_distance = distance(candidate, region.anchor);
            let score = edge_clearance * 1.5 + min_dist_sq.sqrt() * 1.8 - region_distance * 1.2;
            if score > best_score {
                best_score = score;
                best = Some(candidate);
            }
        }

        let homeworld = best.unwrap_or_else(|| most_isolated_sector(region, &homeworlds));
        homeworlds.push(homeworld);
    }

    homeworlds
}

/// Fallback when no sampled sector meets the spacing rule: the free sector
/// of the region farthest from every homeworld placed so far.
fn most_isolated_sector(region: &HomeworldRegion, homeworlds: &[[u8; 2]]) -> [u8; 2] {
    (region.min[0]..=region.max[0])
        .flat_map(|x| (region.min[1]..=region.max[1]).map(move |y| [x, y]))
        .filter(|candidate| !homeworlds.contains(candidate))
        .map(|candidate| (candidate, nearest_used_distance(candidate, homeworlds)))
        .max_by(|(_, left), (_, right)| left.total_cmp(right))
        .map(|(candidate, _)| candidate)
        .expect("homeworld regions have a free sector")
}

pub(super) fn generate_neutral_worlds(
    player_count: u8,
    map_size: u8,
    seed: u64,
    reroll: u32,
    homeworlds: &[[u8; 2]],
    rng: &mut GameRng,
) -> Vec<GeneratedWorld> {
    let total_neutrals = player_count as usize * 4;
    let local_count = player_count as usize * LOCAL_WORLD_COUNT_PER_PLAYER;
    let frontier_count = total_neutrals.saturating_sub(local_count);
    let mut worlds = Vec::with_capacity(total_neutrals);
    let mut used = homeworlds.to_vec();

    for (home_idx, &home) in homeworlds.iter().enumerate() {
        for slot in 0..LOCAL_WORLD_COUNT_PER_PLAYER {
            let coords = choose_local_world(
                home_idx, home, map_size, seed, reroll, homeworlds, &used, rng,
            );
            let potential = local_potential(seed, reroll, home_idx, slot);
            used.push(coords);
            worlds.push(GeneratedWorld {
                coords,
                potential_production: potential,
            });
        }
    }

    let frontier_potentials = frontier_potentials(frontier_count, rng);
    for (idx, potential) in frontier_potentials.into_iter().enumerate() {
        let coords = choose_frontier_world(
            idx, potential, map_size, seed, reroll, homeworlds, &worlds, &used, rng,
        );
        used.push(coords);
        worlds.push(GeneratedWorld {
            coords,
            potential_production: potential,
        });
    }

    worlds
}

fn choose_local_world(
    home_idx: usize,
    home: [u8; 2],
    map_size: u8,
    seed: u64,
    reroll: u32,
    homeworlds: &[[u8; 2]],
    used: &[[u8; 2]],
    rng: &mut GameRng,
) -> [u8; 2] {
    let mut best = None;
    let mut best_score = f32::MIN;
    let base_angle = std::f32::consts::TAU * home_idx as f32 / homeworlds.len().max(1) as f32;

    for attempt in 0..96 {
        let orbit = 2.6 + rng.next_f32() * 2.4;
        let angle = base_angle + attempt as f32 * 0.31 + (rng.next_f32() - 0.5) * 1.0;
        let x = clamp_coord(home[0] as f32 + orbit * angle.cos(), map_size, 1.0);
        let y = clamp_coord(home[1] as f32 + orbit * angle.sin(), map_size, 1.0);
        let candidate = [x, y];
        if !candidate_allowed(candidate, used) {
            continue;
        }

        let nearest_idx = nearest_homeworld(candidate, homeworlds);
        if nearest_idx != home_idx {
            continue;
        }

        let nearest = distance(candidate, home);
        let spacing = nearest_used_distance(candidate, used);
        let noise = density_noise(candidate, map_size, seed, reroll);
        let score = 18.0 - (nearest - 3.8).abs() * 4.0
            + spacing * 1.8
            + noise * 3.0
            + local_world_edge_bias(candidate, map_size);
        if score > best_score {
            best_score = score;
            best = Some(candidate);
        }
    }

    best.unwrap_or_else(|| fallback_local_world(home, map_size, used))
}

fn choose_frontier_world(
    frontier_idx: usize,
    potential: u8,
    map_size: u8,
    seed: u64,
    reroll: u32,
    homeworlds: &[[u8; 2]],
    existing_worlds: &[GeneratedWorld],
    used: &[[u8; 2]],
    rng: &mut GameRng,
) -> [u8; 2] {
    let mut best = None;
    let mut best_score = f32::MIN;
    // 1-based grid: centre (map_size + 1) / 2, interior sectors 2..=map_size-1.
    let center = (map_size as f32 + 1.0) / 2.0;

    for attempt in 0..192 {
        let x = rng.range_u8(2, map_size.saturating_sub(1));
        let y = rng.range_u8(2, map_size.saturating_sub(1));
        let candidate = [x, y];
        if !candidate_allowed(candidate, used) {
            continue;
        }

        let dists = sorted_homeworld_distances(candidate, homeworlds);
        let nearest = dists[0];
        let second = *dists.get(1).unwrap_or(&nearest);
        let spacing = nearest_used_distance(candidate, used);
        let gap = (nearest - second).abs();
        let contest = if gap < CONTESTED_GAP_LIMIT { 12.0 } else { 0.0 };
        let center_bias =
            6.0 / (1.0 + ((x as f32 - center).abs() + (y as f32 - center).abs()) * 0.5);
        let noise = density_noise(candidate, map_size, seed, reroll);
        let void_penalty = if nearest < 2.5 { 8.0 } else { 0.0 };
        let frontier_ring = if frontier_idx % 3 == 0 { 1.5 } else { 0.0 };
        let balance_bonus =
            frontier_balance_bonus(candidate, potential, homeworlds, existing_worlds);
        let score = spacing * 1.7
            + contest
            + center_bias
            + noise * 6.0
            + frontier_ring
            + balance_bonus
            + frontier_world_edge_bias(candidate, map_size)
            - gap * 1.4
            - void_penalty;
        if score > best_score {
            best_score = score;
            best = Some(candidate);
        }

        if attempt > 64 && best_score > 18.0 {
            break;
        }
    }

    best.unwrap_or_else(|| fallback_frontier_world(map_size, used))
}

fn local_potential(seed: u64, reroll: u32, home_idx: usize, slot: usize) -> u8 {
    let base = if slot == 0 { 68 } else { 86 };
    let wobble = hashed_range(seed, reroll, home_idx as u32 * 4 + slot as u32, 0, 8);
    (base + wobble).min(99)
}

fn frontier_potentials(count: usize, rng: &mut GameRng) -> Vec<u8> {
    let mut values = Vec::with_capacity(count);
    if count == 0 {
        return values;
    }

    values.push(130 + (rng.next_u8() % 21));
    if count > 1 {
        values.push(108 + (rng.next_u8() % 18));
    }
    while values.len() < count {
        let next = match values.len() % 5 {
            0 => 34 + (rng.next_u8() % 18),
            1 => 48 + (rng.next_u8() % 22),
            2 => 62 + (rng.next_u8() % 24),
            3 => 78 + (rng.next_u8() % 18),
            _ => 90 + (rng.next_u8() % 12),
        };
        values.push(next.min(150));
    }

    for idx in 0..values.len() {
        let swap = (rng.next_u32() as usize) % values.len();
        values.swap(idx, swap);
    }
    values
}
