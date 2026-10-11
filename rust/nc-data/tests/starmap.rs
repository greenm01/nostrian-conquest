use nc_compat::{DatabaseDat, merge_player_intel_from_compat};
use nc_data::{Coords, build_player_starmap_projection_from_snapshots, set_planet_name};
use nc_engine::GameStateBuilder;

#[test]
fn player_starmap_projection_shows_full_geometry_but_only_known_details() {
    let mut game_data = GameStateBuilder::new()
        .with_player_count(4)
        .with_year(3000)
        .build_initialized_baseline()
        .expect("baseline should build");
    game_data.planets[0].sector = Coords::new(5, 2);
    game_data.planets[1].sector = Coords::new(10, 15);

    let planet_count = game_data.planets.len();
    let mut database =
        DatabaseDat::new_zeroed((game_data.conquest.player_count as usize) * planet_count);
    let record = database.record_mut(0, 0, planet_count);
    record.set_planet_name("Home");
    record.raw[0x15] = 1;
    record.raw[0x1c] = 100;
    record.raw[0x23] = 10;
    record.raw[0x25] = 4;

    let snapshots = merge_player_intel_from_compat(
        &game_data,
        &database,
        1,
        game_data.conquest.game_year,
        None,
    );
    let projection = build_player_starmap_projection_from_snapshots(&game_data, &snapshots, 1);
    assert_eq!(projection.worlds.len(), planet_count);
    assert!(projection.worlds.iter().any(|world| world.coords == [5, 2]));
    assert!(
        projection
            .worlds
            .iter()
            .any(|world| world.coords == [10, 15])
    );

    let known = projection
        .worlds
        .iter()
        .find(|world| world.coords == [5, 2])
        .expect("known world should exist");
    assert_eq!(
        known.known_name.as_deref(),
        Some(game_data.planets[0].name.trim().to_string().as_str())
    );

    let unknown = projection
        .worlds
        .iter()
        .find(|world| world.coords == [10, 15])
        .expect("unknown world should exist");
    assert_eq!(unknown.known_name, None);
    assert_eq!(unknown.known_owner_empire_id, None);
}

#[test]
fn player_starmap_projection_always_marks_owned_worlds_as_owned() {
    let mut game_data = GameStateBuilder::new()
        .with_player_count(4)
        .with_year(3000)
        .build_initialized_baseline()
        .expect("baseline should build");
    game_data.planets[0].sector = Coords::new(5, 2);
    game_data.planets[0].owner.0 = 1;
    set_planet_name(&mut game_data.planets[0], "Home");

    let planet_count = game_data.planets.len();
    let database =
        DatabaseDat::new_zeroed((game_data.conquest.player_count as usize) * planet_count);

    let snapshots = merge_player_intel_from_compat(
        &game_data,
        &database,
        1,
        game_data.conquest.game_year,
        None,
    );
    let projection = build_player_starmap_projection_from_snapshots(&game_data, &snapshots, 1);
    let home = projection
        .worlds
        .iter()
        .find(|world| world.coords == [5, 2])
        .expect("owned world should exist");

    assert_eq!(home.known_owner_empire_id, Some(1));
    assert_eq!(home.known_name.as_deref(), Some("Home"));
    assert_eq!(
        home.known_potential_production,
        Some(u16::from(game_data.planets[0].potential_production))
    );
    assert_eq!(
        home.known_armies,
        Some(u8::try_from(game_data.planets[0].armies).unwrap_or(u8::MAX))
    );
    assert_eq!(
        home.known_ground_batteries,
        Some(u8::try_from(game_data.planets[0].ground_batteries).unwrap_or(u8::MAX))
    );
    assert_eq!(home.known_starbase_count, Some(0));
}

#[test]
fn ascii_map_right_side_row_labels_align_with_map_border() {
    let projection = nc_data::PlayerStarmapProjection {
        map_width: 18,
        map_height: 36,
        year: 3000,
        viewer_empire_id: 1,
        worlds: Vec::new(),
    };

    let rendered = projection.render_ascii_map();
    let lines = rendered.lines().collect::<Vec<_>>();
    let top_border = lines[1];
    let first_data_row = lines[2];
    let right_border_col = top_border.rfind('|').expect("top border should end with |");

    assert_eq!(top_border.chars().nth(right_border_col + 1), Some('-'));
    assert_eq!(first_data_row.chars().nth(right_border_col), Some('|'));
    assert_eq!(first_data_row.chars().nth(right_border_col + 1), Some(' '));
    assert_eq!(first_data_row.chars().nth(right_border_col + 2), Some('3'));
    assert_eq!(first_data_row.chars().nth(right_border_col + 3), Some('6'));
}

#[test]
fn player_starmap_projection_uses_database_intel_for_known_foreign_owner() {
    let mut game_data = GameStateBuilder::new()
        .with_player_count(4)
        .with_year(3000)
        .build_initialized_baseline()
        .expect("baseline should build");
    game_data.planets[1].sector = Coords::new(10, 15);
    game_data.planets[1].owner.0 = 2;

    let planet_count = game_data.planets.len();
    let mut database =
        DatabaseDat::new_zeroed((game_data.conquest.player_count as usize) * planet_count);
    let record = database.record_mut(1, 0, planet_count);
    record.raw[0x15] = 2;

    let snapshots = merge_player_intel_from_compat(
        &game_data,
        &database,
        1,
        game_data.conquest.game_year,
        None,
    );
    let projection = build_player_starmap_projection_from_snapshots(&game_data, &snapshots, 1);
    let foreign = projection
        .worlds
        .iter()
        .find(|world| world.coords == [10, 15])
        .expect("foreign world should exist");

    assert_eq!(foreign.known_owner_empire_id, Some(2));
}

#[test]
fn ascii_map_export_uses_printable_paged_grid() {
    for (width, height, expect_formfeed) in [
        (18, 18, false),
        (27, 27, true),
        (36, 36, true),
        (45, 45, true),
    ] {
        let projection = nc_data::PlayerStarmapProjection {
            map_width: width,
            map_height: height,
            year: 3000,
            viewer_empire_id: 1,
            worlds: vec![
                nc_data::PlayerStarmapWorld {
                    planet_record_index_1_based: 1,
                    coords: [1, 1],
                    intel_tier: nc_data::IntelTier::Unknown,
                    known_name: None,
                    known_owner_empire_id: None,
                    known_owner_empire_name: None,
                    known_potential_production: None,
                    known_armies: None,
                    known_ground_batteries: None,
                    known_starbase_count: None,
                    known_current_production: None,
                    known_stored_points: None,
                    known_docked_summary: None,
                    known_orbit_summary: None,
                },
                nc_data::PlayerStarmapWorld {
                    planet_record_index_1_based: 2,
                    coords: [width, height],
                    intel_tier: nc_data::IntelTier::Unknown,
                    known_name: None,
                    known_owner_empire_id: None,
                    known_owner_empire_name: None,
                    known_potential_production: None,
                    known_armies: None,
                    known_ground_batteries: None,
                    known_starbase_count: None,
                    known_current_production: None,
                    known_stored_points: None,
                    known_docked_summary: None,
                    known_orbit_summary: None,
                },
            ],
        };

        let rendered = projection.render_ascii_map();
        assert_eq!(rendered.contains('\u{0c}'), expect_formfeed);
        assert!(rendered.contains('*'));
        assert!(rendered.contains("  01"));
        assert!(rendered.contains(&format!("{:>4}", format!("{height:02}"))));
    }
}
