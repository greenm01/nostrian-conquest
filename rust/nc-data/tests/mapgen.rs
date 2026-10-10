use nc_engine::{GeneratedMap, build_seeded_initialized_game, build_seeded_new_game, generate_map};

fn edge_ring_world_count(generated: &GeneratedMap) -> usize {
    generated
        .neutral_worlds
        .iter()
        .filter(|world| {
            // Within two sectors of an edge of the 1-based 1..=map_size grid.
            let x = world.coords[0];
            let y = world.coords[1];
            let max = generated.map_size - 1;
            x <= 2 || y <= 2 || x >= max || y >= max
        })
        .count()
}

#[test]
fn generated_map_is_seed_reproducible() {
    let first = generate_map(4, 1515);
    let second = generate_map(4, 1515);
    assert_eq!(first, second);
}

#[test]
fn generated_map_produces_balanced_world_count_and_unique_coords() {
    let generated = generate_map(4, 424242);
    assert_eq!(generated.homeworld_coords.len(), 4);
    assert_eq!(generated.neutral_worlds.len(), 16);
    assert!(generated.metrics.contested_worlds >= 4);
    assert!(generated.metrics.early_count_range <= 1);
    assert!(generated.metrics.early_value_range <= 60);

    let mut coords = generated.homeworld_coords.clone();
    coords.extend(generated.neutral_worlds.iter().map(|world| world.coords));
    coords.sort_unstable();
    coords.dedup();
    assert_eq!(coords.len(), 20);

    for coords in generated.homeworld_coords {
        assert!(coords[0] < generated.map_size);
        assert!(coords[1] < generated.map_size);
    }
}

#[test]
fn generated_map_keeps_one_planet_per_system() {
    let generated = generate_map(4, 987654321);
    for (left_idx, left) in generated.neutral_worlds.iter().enumerate() {
        for right in generated.neutral_worlds.iter().skip(left_idx + 1) {
            assert_ne!(left.coords, right.coords);
        }
        for home in &generated.homeworld_coords {
            assert_ne!(left.coords, *home);
        }
    }
}

#[test]
fn seeded_new_game_populates_documented_planet_count_for_player_count() {
    let data = build_seeded_new_game(3, 3000, 99).expect("seeded game should build");
    let populated = data
        .planets
        .records
        .iter()
        .filter(|planet| planet.coords_raw() != [0, 0])
        .count();
    assert_eq!(populated, 15);

    for idx in 0..3 {
        assert_eq!(data.player.records[idx].owner_mode_raw(), 0);
        assert_eq!(data.player.records[idx].autopilot_flag(), 1);
        assert_eq!(
            data.planets.records[idx].owner_empire_slot_raw(),
            (idx + 1) as u8
        );
        assert_eq!(data.planets.records[idx].planet_name(), "Not Named Yet");
        assert_eq!(data.planets.records[idx].army_count_raw(), 10);
        assert_eq!(data.planets.records[idx].ground_batteries_raw(), 4);
        assert_eq!(data.planets.records[idx].potential_production_raw()[0], 100);
    }
}

#[test]
fn generated_map_keeps_rich_worlds_out_of_one_players_backyard() {
    let generated = generate_map(4, 1515);
    let rich_worlds = generated
        .neutral_worlds
        .iter()
        .filter(|world| world.potential_production >= 100)
        .collect::<Vec<_>>();
    assert!(!rich_worlds.is_empty());

    let mut per_owner = [0u8; 4];
    for world in rich_worlds {
        let owner = generated
            .homeworld_coords
            .iter()
            .enumerate()
            .min_by(|(_, left), (_, right)| {
                let left_dx = world.coords[0] as i16 - left[0] as i16;
                let left_dy = world.coords[1] as i16 - left[1] as i16;
                let right_dx = world.coords[0] as i16 - right[0] as i16;
                let right_dy = world.coords[1] as i16 - right[1] as i16;
                (left_dx * left_dx + left_dy * left_dy)
                    .cmp(&(right_dx * right_dx + right_dy * right_dy))
            })
            .map(|(idx, _)| idx)
            .unwrap();
        per_owner[owner] += 1;
    }

    let max = per_owner.into_iter().max().unwrap_or(0);
    assert!(max <= 1);
}

#[test]
fn four_player_maps_keep_edge_ring_neutral_worlds_uncommon() {
    let seeds = [1515_u64, 2025, 4242, 9999, 424242, 987654321, 1, 77];
    let generated = seeds
        .into_iter()
        .map(|seed| generate_map(4, seed))
        .collect::<Vec<_>>();
    let total_edge_ring = generated.iter().map(edge_ring_world_count).sum::<usize>();
    let total_neutrals = generated
        .iter()
        .map(|map| map.neutral_worlds.len())
        .sum::<usize>();

    assert!(total_edge_ring * 5 <= total_neutrals);
}

#[test]
fn nine_player_maps_keep_edge_ring_neutral_worlds_uncommon() {
    let seeds = [1515_u64, 2025, 4242, 9999, 424242, 987654321, 1, 77];
    let generated = seeds
        .into_iter()
        .map(|seed| generate_map(9, seed))
        .collect::<Vec<_>>();
    let total_edge_ring = generated.iter().map(edge_ring_world_count).sum::<usize>();
    let total_neutrals = generated
        .iter()
        .map(|map| map.neutral_worlds.len())
        .sum::<usize>();

    assert!(total_edge_ring * 5 <= total_neutrals);
}

#[test]
fn seeded_new_game_supports_nine_player_manual_tier() {
    let data = build_seeded_new_game(9, 3000, 2025).expect("9-player seeded game should build");
    assert_eq!(data.player.records.len(), 9);
    assert_eq!(data.planets.records.len(), 45);
    assert_eq!(data.fleets.records.len(), 36);
    assert!(data.ecmaint_preflight_errors().is_empty());
}

#[test]
fn seeded_initialized_game_retains_active_campaign_builder_semantics() {
    let data =
        build_seeded_initialized_game(4, 3000, 1515).expect("initialized seeded game should build");
    assert!(
        data.player
            .records
            .iter()
            .all(|player| player.owner_mode_raw() == 1)
    );
    assert_eq!(data.player.records[0].autopilot_flag(), 1);
    assert_eq!(data.planets.records[0].owner_empire_slot_raw(), 1);
    assert_eq!(data.planets.records[0].planet_name(), "Player 1 HW");
    assert!(data.ecmaint_preflight_errors().is_empty());
}

#[test]
fn player_count_validator_accepts_only_supported_counts() {
    assert!(!nc_data::is_valid_player_count(0));
    assert!(nc_data::is_valid_player_count(1));
    assert!(nc_data::is_valid_player_count(25));
    assert!(!nc_data::is_valid_player_count(26));
}

#[test]
fn seeded_games_reject_unsupported_player_counts() {
    for player_count in [0, 26, 255] {
        assert!(build_seeded_new_game(player_count, 3000, 1).is_err());
        assert!(build_seeded_initialized_game(player_count, 3000, 1).is_err());
    }
}
