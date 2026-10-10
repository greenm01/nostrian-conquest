use super::{CONTESTED_GAP_LIMIT, EARLY_RADIUS, GeneratedWorld, MAP_EDGE_RING_PENALTY, MapMetrics};
use crate::setup::mapgen::geometry::{
    distance, edge_hugging_world_penalty, minimum_pair_distance, nearest_homeworld,
    sorted_homeworld_distances,
};

pub(super) fn frontier_balance_bonus(
    candidate: [u8; 2],
    potential: u8,
    homeworlds: &[[u8; 2]],
    existing_worlds: &[GeneratedWorld],
) -> f32 {
    let mut counts = vec![0u8; homeworlds.len()];
    let mut values = vec![0u16; homeworlds.len()];

    for (home_idx, &home) in homeworlds.iter().enumerate() {
        for world in existing_worlds {
            if distance(world.coords, home) <= EARLY_RADIUS
                && nearest_homeworld(world.coords, homeworlds) == home_idx
            {
                counts[home_idx] += 1;
                values[home_idx] += world.potential_production as u16;
            }
        }
    }

    let before_count_range = range_u8(&counts);
    let before_value_range = range_u16(&values);
    let owner_idx = nearest_homeworld(candidate, homeworlds);
    if distance(candidate, homeworlds[owner_idx]) <= EARLY_RADIUS {
        counts[owner_idx] += 1;
        values[owner_idx] += potential as u16;
    }
    let after_count_range = range_u8(&counts);
    let after_value_range = range_u16(&values);

    (before_count_range as f32 - after_count_range as f32) * 18.0
        + (before_value_range as f32 - after_value_range as f32) * 0.22
}

pub(super) fn score_map(
    map_size: u8,
    homeworlds: &[[u8; 2]],
    neutral_worlds: &[GeneratedWorld],
) -> MapMetrics {
    let mut early_counts = Vec::with_capacity(homeworlds.len());
    let mut early_values = Vec::with_capacity(homeworlds.len());

    for (home_idx, &home) in homeworlds.iter().enumerate() {
        let mut count = 0u8;
        let mut value = 0u16;
        for world in neutral_worlds {
            if distance(world.coords, home) <= EARLY_RADIUS
                && nearest_homeworld(world.coords, homeworlds) == home_idx
            {
                count += 1;
                value += world.potential_production as u16;
            }
        }
        early_counts.push(count);
        early_values.push(value);
    }

    let early_count_range = range_u8(&early_counts);
    let early_value_range = range_u16(&early_values);
    let contested_worlds = neutral_worlds
        .iter()
        .filter(|world| {
            let dists = sorted_homeworld_distances(world.coords, homeworlds);
            let nearest = dists[0];
            let second = *dists.get(1).unwrap_or(&nearest);
            (nearest - second).abs() < CONTESTED_GAP_LIMIT
        })
        .count() as u8;
    let min_homeworld_spacing = minimum_pair_distance(homeworlds);
    let connected = neutral_worlds
        .iter()
        .all(|world| sorted_homeworld_distances(world.coords, homeworlds)[0] <= 9.0);

    let score = 120.0 - early_count_range as f32 * 42.0 - early_value_range as f32 * 0.70
        + contested_worlds as f32 * 2.5
        + min_homeworld_spacing * 3.2
        + if connected { 18.0 } else { -60.0 }
        - isolated_home_penalty(homeworlds, neutral_worlds) * 28.0
        - dominant_cluster_penalty(homeworlds, neutral_worlds) * 0.32
        + density_balance_bonus(map_size, neutral_worlds)
        - edge_hugging_world_penalty(map_size, neutral_worlds) * MAP_EDGE_RING_PENALTY
        - system_overlap_penalty(homeworlds, neutral_worlds) * 200.0;

    MapMetrics {
        score,
        early_count_range,
        early_value_range,
        contested_worlds,
        min_homeworld_spacing,
    }
}

fn isolated_home_penalty(homeworlds: &[[u8; 2]], neutral_worlds: &[GeneratedWorld]) -> f32 {
    homeworlds
        .iter()
        .map(|&home| {
            neutral_worlds
                .iter()
                .filter(|world| distance(world.coords, home) <= 6.5)
                .count()
        })
        .map(|count| if count >= 2 { 0.0 } else { (2 - count) as f32 })
        .sum()
}

fn dominant_cluster_penalty(homeworlds: &[[u8; 2]], neutral_worlds: &[GeneratedWorld]) -> f32 {
    let values = homeworlds
        .iter()
        .enumerate()
        .map(|(idx, &home)| {
            neutral_worlds
                .iter()
                .filter(|world| nearest_homeworld(world.coords, homeworlds) == idx)
                .map(|world| {
                    let dist = distance(world.coords, home).max(1.0);
                    (world.potential_production as f32) / dist
                })
                .sum::<f32>()
        })
        .collect::<Vec<_>>();

    let min = values.iter().copied().fold(f32::INFINITY, f32::min);
    let max = values.iter().copied().fold(f32::NEG_INFINITY, f32::max);
    max - min
}

fn density_balance_bonus(map_size: u8, neutral_worlds: &[GeneratedWorld]) -> f32 {
    let center = (map_size as f32 + 1.0) / 2.0;
    neutral_worlds
        .iter()
        .map(|world| {
            let center_distance = ((world.coords[0] as f32 - center).abs()
                + (world.coords[1] as f32 - center).abs())
                / map_size as f32;
            let richness = world.potential_production as f32 / 150.0;
            (1.0 - center_distance) * richness * 2.5
        })
        .sum()
}

fn system_overlap_penalty(homeworlds: &[[u8; 2]], neutral_worlds: &[GeneratedWorld]) -> f32 {
    let mut seen = homeworlds.to_vec();
    let mut duplicates = 0u32;
    for world in neutral_worlds {
        if seen.contains(&world.coords) {
            duplicates += 1;
        } else {
            seen.push(world.coords);
        }
    }
    duplicates as f32
}

pub(super) fn all_systems_unique(
    homeworlds: &[[u8; 2]],
    neutral_worlds: &[GeneratedWorld],
) -> bool {
    let mut seen = homeworlds.to_vec();
    for world in neutral_worlds {
        if seen.contains(&world.coords) {
            return false;
        }
        seen.push(world.coords);
    }
    true
}

fn range_u8(values: &[u8]) -> u8 {
    let min = values.iter().copied().min().unwrap_or(0);
    let max = values.iter().copied().max().unwrap_or(0);
    max.saturating_sub(min)
}

fn range_u16(values: &[u16]) -> u16 {
    let min = values.iter().copied().min().unwrap_or(0);
    let max = values.iter().copied().max().unwrap_or(0);
    max.saturating_sub(min)
}
