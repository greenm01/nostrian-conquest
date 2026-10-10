//! Tests for maintenance logic (Milestone 4 Phase 2)

use nc_compat::load_classic_core_files;
use nc_engine::run_maintenance_turn;
use std::path::Path;

#[test]
fn test_year_advancement_single_turn() {
    // Use the fleet pre fixture as baseline
    let fixture_dir =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/ecmaint-fleet-pre/v1.5");

    // Load initial state
    let mut game_data = load_classic_core_files(&fixture_dir).expect("Failed to load fixture");
    let initial_year = game_data.conquest.game_year();

    // Run one turn of maintenance
    run_maintenance_turn(&mut game_data).expect("Maintenance failed");

    // Verify year advanced by exactly 1
    let final_year = game_data.conquest.game_year();
    assert_eq!(
        final_year,
        initial_year + 1,
        "Year should advance by 1: {} -> {}",
        initial_year,
        final_year
    );
}

#[test]
fn test_year_advancement_multiple_turns() {
    let fixture_dir =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/ecmaint-fleet-pre/v1.5");

    let mut game_data = load_classic_core_files(&fixture_dir).expect("Failed to load fixture");
    let initial_year = game_data.conquest.game_year();

    // Run 3 turns
    for i in 0..3 {
        run_maintenance_turn(&mut game_data).expect(&format!("Maintenance turn {} failed", i));
    }

    let final_year = game_data.conquest.game_year();
    assert_eq!(
        final_year,
        initial_year + 3,
        "Year should advance by 3: {} -> {}",
        initial_year,
        final_year
    );
}

#[test]
fn test_fleet_fixture_year_matches_post() {
    // The fleet scenario pre fixture starts at year 3000
    // After running maint once, should match the post fixture at year 3001
    let pre_dir =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/ecmaint-fleet-pre/v1.5");
    let post_dir =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/ecmaint-fleet-post/v1.5");

    // Load and verify pre state
    let mut game_data = load_classic_core_files(&pre_dir).expect("Failed to load pre fixture");
    assert_eq!(
        game_data.conquest.game_year(),
        3000,
        "Pre fixture should be at year 3000"
    );

    // Run maintenance
    run_maintenance_turn(&mut game_data).expect("Maintenance failed");

    // Load post fixture and compare year
    let post_data = load_classic_core_files(&post_dir).expect("Failed to load post fixture");
    assert_eq!(
        game_data.conquest.game_year(),
        post_data.conquest.game_year(),
        "Year should match post fixture: {} vs {}",
        game_data.conquest.game_year(),
        post_data.conquest.game_year()
    );
}
