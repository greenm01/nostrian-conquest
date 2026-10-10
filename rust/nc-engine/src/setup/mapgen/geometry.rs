use super::{
    FRONTIER_WORLD_EDGE_CLEARANCE_WEIGHT, FRONTIER_WORLD_EDGE_RING_PENALTY, GeneratedWorld,
    HOMEWORLD_EDGE_MARGIN, LOCAL_WORLD_EDGE_CLEARANCE_WEIGHT, NEUTRAL_EDGE_RING_THRESHOLD,
    NEUTRAL_MIN_SPACING,
};

pub(super) fn candidate_allowed(candidate: [u8; 2], used: &[[u8; 2]]) -> bool {
    !used.contains(&candidate) && nearest_used_distance(candidate, used) >= NEUTRAL_MIN_SPACING
}

pub(super) fn nearest_homeworld(candidate: [u8; 2], homeworlds: &[[u8; 2]]) -> usize {
    homeworlds
        .iter()
        .enumerate()
        .min_by(|(_, a), (_, b)| {
            distance_sq(candidate, **a)
                .partial_cmp(&distance_sq(candidate, **b))
                .unwrap()
        })
        .map(|(idx, _)| idx)
        .unwrap_or(0)
}

pub(super) fn sorted_homeworld_distances(candidate: [u8; 2], homeworlds: &[[u8; 2]]) -> Vec<f32> {
    let mut distances = homeworlds
        .iter()
        .map(|coords| distance(candidate, *coords))
        .collect::<Vec<_>>();
    distances.sort_by(|a, b| a.partial_cmp(b).unwrap());
    distances
}

pub(super) fn minimum_pair_distance(coords: &[[u8; 2]]) -> f32 {
    let mut min = f32::MAX;
    for left in 0..coords.len() {
        for right in left + 1..coords.len() {
            min = min.min(distance(coords[left], coords[right]));
        }
    }
    if min.is_finite() { min } else { 0.0 }
}

pub(super) fn nearest_used_distance(candidate: [u8; 2], used: &[[u8; 2]]) -> f32 {
    used.iter()
        .map(|other| distance(candidate, *other))
        .fold(f32::MAX, f32::min)
}

pub(super) fn local_world_edge_bias(candidate: [u8; 2], map_size: u8) -> f32 {
    neutral_world_edge_bias(candidate, map_size, LOCAL_WORLD_EDGE_CLEARANCE_WEIGHT, 0.0)
}

pub(super) fn frontier_world_edge_bias(candidate: [u8; 2], map_size: u8) -> f32 {
    neutral_world_edge_bias(
        candidate,
        map_size,
        FRONTIER_WORLD_EDGE_CLEARANCE_WEIGHT,
        FRONTIER_WORLD_EDGE_RING_PENALTY,
    )
}

fn neutral_world_edge_bias(
    candidate: [u8; 2],
    map_size: u8,
    clearance_weight: f32,
    edge_ring_penalty_weight: f32,
) -> f32 {
    let clearance = edge_clearance(candidate, map_size);
    clearance.min(3.0) * clearance_weight
        - edge_ring_shortfall(candidate, map_size) * edge_ring_penalty_weight
}

pub(super) fn edge_hugging_world_penalty(map_size: u8, neutral_worlds: &[GeneratedWorld]) -> f32 {
    neutral_worlds
        .iter()
        .map(|world| edge_ring_shortfall(world.coords, map_size))
        .sum()
}

fn edge_ring_shortfall(candidate: [u8; 2], map_size: u8) -> f32 {
    (NEUTRAL_EDGE_RING_THRESHOLD - edge_clearance(candidate, map_size)).max(0.0)
}

/// Sectors between `candidate` and the nearest map edge; coordinates are
/// 1-based, so the edges are sector 1 and sector `map_size`.
pub(super) fn edge_clearance(candidate: [u8; 2], map_size: u8) -> f32 {
    f32::min(
        f32::min(
            candidate[0].saturating_sub(1) as f32,
            candidate[1].saturating_sub(1) as f32,
        ),
        f32::min(
            map_size.saturating_sub(candidate[0]) as f32,
            map_size.saturating_sub(candidate[1]) as f32,
        ),
    )
}

pub(super) fn distance(a: [u8; 2], b: [u8; 2]) -> f32 {
    distance_sq(a, b).sqrt()
}

pub(super) fn distance_sq(a: [u8; 2], b: [u8; 2]) -> f32 {
    let dx = a[0] as f32 - b[0] as f32;
    let dy = a[1] as f32 - b[1] as f32;
    dx * dx + dy * dy
}

pub(super) fn density_noise(candidate: [u8; 2], map_size: u8, seed: u64, reroll: u32) -> f32 {
    let x = candidate[0] as f32 / map_size.max(1) as f32;
    let y = candidate[1] as f32 / map_size.max(1) as f32;
    let phase_a = (seed as f32 / 97.0) + reroll as f32 * 0.19;
    let phase_b = (seed as f32 / 211.0) + reroll as f32 * 0.11;
    let coarse = ((x * 5.7 + phase_a).sin() + (y * 6.3 + phase_b).cos()) * 0.5;
    let fine = ((x * 13.0 + phase_b * 0.7).sin() * (y * 11.0 + phase_a * 0.4).cos()) * 0.5;
    (coarse * 0.65 + fine * 0.35).clamp(-1.0, 1.0)
}

pub(super) fn hashed_range(seed: u64, reroll: u32, salt: u32, min: u8, max: u8) -> u8 {
    if min >= max {
        return min;
    }
    let mut value = seed ^ ((reroll as u64) << 16) ^ ((salt as u64) << 32) ^ 0x9E37_79B9_7F4A_7C15;
    value ^= value >> 33;
    value = value.wrapping_mul(0xFF51_AFD7_ED55_8CCD);
    value ^= value >> 33;
    value = value.wrapping_mul(0xC4CE_B9FE_1A85_EC53);
    value ^= value >> 33;
    let span = max - min + 1;
    min + (value as u8 % span)
}

/// Round into the 1-based grid, keeping `margin` sectors from each edge.
pub(super) fn clamp_coord(value: f32, map_size: u8, margin: f32) -> u8 {
    value.round().clamp(1.0 + margin, map_size as f32 - margin) as u8
}

pub(super) fn fallback_local_world(home: [u8; 2], map_size: u8, used: &[[u8; 2]]) -> [u8; 2] {
    for dx in -4..=4 {
        for dy in -4..=4 {
            let x = (home[0] as i16 + dx).clamp(2, map_size as i16 - 1) as u8;
            let y = (home[1] as i16 + dy).clamp(2, map_size as i16 - 1) as u8;
            let candidate = [x, y];
            if candidate != home && candidate_allowed(candidate, used) {
                return candidate;
            }
        }
    }
    first_free_sector(map_size, used)
}

pub(super) fn fallback_frontier_world(map_size: u8, used: &[[u8; 2]]) -> [u8; 2] {
    let center = map_center_sector(map_size);
    for ring in 0..map_size {
        for dx in -(ring as i16)..=(ring as i16) {
            for dy in -(ring as i16)..=(ring as i16) {
                let x = (center as i16 + dx).clamp(2, map_size as i16 - 1) as u8;
                let y = (center as i16 + dy).clamp(2, map_size as i16 - 1) as u8;
                let candidate = [x, y];
                if candidate_allowed(candidate, used) {
                    return candidate;
                }
            }
        }
    }
    first_free_sector(map_size, used)
}

/// Last resort when no spaced candidate exists: any sector not yet used.
fn first_free_sector(map_size: u8, used: &[[u8; 2]]) -> [u8; 2] {
    (1..=map_size)
        .flat_map(|x| (1..=map_size).map(move |y| [x, y]))
        .find(|candidate| !used.contains(candidate))
        .expect("maps have more sectors than systems")
}

/// Middle sector of the 1-based grid (the upper middle on even sizes).
fn map_center_sector(map_size: u8) -> u8 {
    map_size / 2 + 1
}

#[derive(Debug, Clone, Copy)]
pub(super) struct HomeworldRegion {
    pub min: [u8; 2],
    pub max: [u8; 2],
    pub anchor: [u8; 2],
}

pub(super) fn homeworld_regions(player_count: u8, map_size: u8) -> Vec<HomeworldRegion> {
    // Keep HOMEWORLD_EDGE_MARGIN clear sectors beyond both 1-based edges.
    let low = 1 + HOMEWORLD_EDGE_MARGIN as u8;
    let high = map_size.saturating_sub(HOMEWORLD_EDGE_MARGIN as u8);
    let mid = map_center_sector(map_size);
    match player_count {
        1 => vec![HomeworldRegion {
            min: [mid.saturating_sub(2), mid.saturating_sub(2)],
            max: [mid + 2, mid + 2],
            anchor: [mid, mid],
        }],
        2 => vec![
            HomeworldRegion {
                min: [low, low + 1],
                max: [mid.saturating_sub(2), high.saturating_sub(1)],
                anchor: [mid.saturating_sub(4), mid],
            },
            HomeworldRegion {
                min: [mid + 1, low + 1],
                max: [high, high.saturating_sub(1)],
                anchor: [mid + 4, mid],
            },
        ],
        3 => vec![
            HomeworldRegion {
                min: [low, low],
                max: [mid.saturating_sub(2), mid.saturating_sub(2)],
                anchor: [mid.saturating_sub(4), mid.saturating_sub(4)],
            },
            HomeworldRegion {
                min: [mid + 1, low],
                max: [high, mid.saturating_sub(2)],
                anchor: [mid + 4, mid.saturating_sub(4)],
            },
            HomeworldRegion {
                min: [mid.saturating_sub(2), mid + 1],
                max: [mid + 2, high],
                anchor: [mid, mid + 4],
            },
        ],
        4 => vec![
            HomeworldRegion {
                min: [low, low],
                max: [mid.saturating_sub(2), mid.saturating_sub(2)],
                anchor: [mid.saturating_sub(4), mid.saturating_sub(4)],
            },
            HomeworldRegion {
                min: [mid + 1, low],
                max: [high, mid.saturating_sub(2)],
                anchor: [mid + 4, mid.saturating_sub(4)],
            },
            HomeworldRegion {
                min: [low, mid + 1],
                max: [mid.saturating_sub(2), high],
                anchor: [mid.saturating_sub(4), mid + 4],
            },
            HomeworldRegion {
                min: [mid + 1, mid + 1],
                max: [high, high],
                anchor: [mid + 4, mid + 4],
            },
        ],
        _ => grid_homeworld_regions(player_count as usize, low, high),
    }
}

fn grid_homeworld_regions(player_count: usize, low: u8, high: u8) -> Vec<HomeworldRegion> {
    let cols = (player_count as f32).sqrt().ceil() as usize;
    let rows = player_count.div_ceil(cols);
    let usable_width = (high.saturating_sub(low) as usize).max(cols);
    let usable_height = (high.saturating_sub(low) as usize).max(rows);
    let cell_width = (usable_width / cols).max(1);
    let cell_height = (usable_height / rows).max(1);
    let mut regions = Vec::with_capacity(player_count);

    for idx in 0..player_count {
        let col = idx % cols;
        let row = idx / cols;
        let min_x = low as usize + col * cell_width;
        let max_x = if col + 1 == cols {
            high as usize
        } else {
            low as usize + ((col + 1) * cell_width).saturating_sub(1)
        };
        let min_y = low as usize + row * cell_height;
        let max_y = if row + 1 == rows {
            high as usize
        } else {
            low as usize + ((row + 1) * cell_height).saturating_sub(1)
        };
        let anchor = [((min_x + max_x) / 2) as u8, ((min_y + max_y) / 2) as u8];
        regions.push(HomeworldRegion {
            min: [min_x as u8, min_y as u8],
            max: [max_x as u8, max_y as u8],
            anchor,
        });
    }

    regions
}

#[cfg(test)]
mod tests {
    use super::{NEUTRAL_EDGE_RING_THRESHOLD, frontier_world_edge_bias, local_world_edge_bias};
    use nc_data::map_size_for_player_count;

    #[test]
    fn local_world_edge_bias_prefers_farther_clearance() {
        let map_size = map_size_for_player_count(4);
        let edge_adjacent = [1, 8];
        let interior = [3, 8];
        assert!(
            local_world_edge_bias(interior, map_size)
                > local_world_edge_bias(edge_adjacent, map_size)
        );
        assert_eq!(NEUTRAL_EDGE_RING_THRESHOLD, 2.0);
    }

    #[test]
    fn frontier_world_edge_bias_penalizes_edges_more_than_local_worlds() {
        let map_size = map_size_for_player_count(4);
        let edge_adjacent = [1, 8];
        let interior = [3, 8];
        let local_gap = local_world_edge_bias(interior, map_size)
            - local_world_edge_bias(edge_adjacent, map_size);
        let frontier_gap = frontier_world_edge_bias(interior, map_size)
            - frontier_world_edge_bias(edge_adjacent, map_size);
        assert!(frontier_gap > local_gap);
    }
}
