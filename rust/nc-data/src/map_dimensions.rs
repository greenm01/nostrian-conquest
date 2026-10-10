pub fn map_size_for_player_count(player_count: u8) -> u8 {
    match player_count {
        1..=4 => 18,
        5..=9 => 27,
        10..=16 => 36,
        _ => 45,
    }
}

/// Map sectors are 1-based: an `N x N` map spans `1..=N` on both axes.
pub fn sector_within_map(coords: [u8; 2], map_size: u8) -> bool {
    coords.iter().all(|&coord| (1..=map_size).contains(&coord))
}
