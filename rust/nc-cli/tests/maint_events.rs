//! `maint-rust` carries every kind of maintenance event into RESULTS.DAT.
//! A kind its run loop forgot to carry would vanish from the reports, so
//! each test below makes the engine emit one kind the report composer
//! reads and finds its report in the exported RESULTS.DAT.

mod common;

use common::{
    cleanup_dir, copy_fixture_dir, decode_results_text, export_campaign_db, import_campaign_db,
    run_maint_rust_with_export, run_nc_cli_in_dir, rust_workspace, unique_temp_dir,
};
use nc_data::{CampaignStore, CoreGameData, CrippledHulls, Order, PlayerMode, set_fleet_at_rest};
use std::fs;
use std::path::Path;

/// `maint-rust` over a store holding the directory's state after `plant`,
/// for state classic's files cannot hold.
fn run_maint_rust_with_planted_state(
    target: &Path,
    turns: u16,
    plant: impl FnOnce(&mut CoreGameData),
) -> String {
    import_campaign_db(target);
    let store = CampaignStore::open_default_in_dir(target).expect("open campaign store");
    let state = store
        .load_latest_runtime_state()
        .expect("load runtime state")
        .expect("imported runtime state");
    let mut game_data = state.game_data;
    plant(&mut game_data);
    store
        .save_year(
            game_data.conquest.game_year,
            &game_data,
            &state.report_block_rows,
            &state.queued_mail,
        )
        .expect("save planted state");
    drop(store);
    let stdout = run_nc_cli_in_dir(
        &["maint-rust", target.to_str().unwrap(), &turns.to_string()],
        rust_workspace(),
    );
    export_campaign_db(target, target);
    stdout
}

fn results_text(target: &Path) -> String {
    let results = fs::read(target.join("RESULTS.DAT")).expect("RESULTS.DAT should exist");
    decode_results_text(&results)
}

/// Empire 1's first planet and first fleet. The fleet is set to rest in
/// that planet's orbit on Hold, and the empire stays a human player, so
/// nothing but the planted state moves.
fn settle_empire_one(game_data: &mut CoreGameData) -> (usize, usize) {
    game_data.players[0].mode = PlayerMode::Human;
    game_data.players[0].autopilot = false;
    let planet_idx = game_data
        .planets
        .iter()
        .position(|planet| planet.owner.0 == 1)
        .expect("empire 1 owns a planet");
    let fleet_idx = game_data
        .fleets
        .iter()
        .position(|fleet| fleet.owner.0 == 1)
        .expect("empire 1 has a fleet");
    let sector = game_data.planets[planet_idx].sector;
    let fleet = &mut game_data.fleets[fleet_idx];
    fleet.sector = sector;
    fleet.order = Order::HoldPosition;
    fleet.target = sector;
    fleet.speed = 0;
    fleet.rules_of_engagement = 0;
    set_fleet_at_rest(fleet);
    (planet_idx, fleet_idx)
}

#[test]
fn maint_rust_carries_starbase_reports() {
    let target = unique_temp_dir("nc-cli-maint-rust-starbase-reports");
    copy_fixture_dir("fixtures/ecmaint-starbase-lift-pre/v1.5", &target);

    let stdout = run_maint_rust_with_export(&target, 1);
    assert!(stdout.contains("Rust maintenance complete."));

    // The world under the base changes hands, so the base reports it lost
    // and lifts off, then reports its arrival.
    let text = results_text(&target);
    assert!(text.contains("From Starbase 1"), "{text}");
    assert!(
        text.contains("Since we have lost control of planet"),
        "{text}"
    );
    assert!(
        text.contains("We have arrived at our destination"),
        "{text}"
    );

    cleanup_dir(&target);
}

#[test]
fn maint_rust_carries_a_scorched_worlds_report_from_every_year() {
    let target = unique_temp_dir("nc-cli-maint-rust-scorch-reports");
    copy_fixture_dir("fixtures/ecmaint-post/v1.5", &target);

    let stdout = run_maint_rust_with_planted_state(&target, 2, |game_data| {
        let (planet_idx, _) = settle_empire_one(game_data);
        game_data.planets[planet_idx].scorch_earth = true;
    });
    assert!(stdout.contains("Rust maintenance complete."));

    let text = results_text(&target);
    assert_eq!(
        text.matches("Scorch-Earth is in force.").count(),
        2,
        "{text}"
    );

    cleanup_dir(&target);
}

#[test]
fn maint_rust_carries_ship_recovery_reports() {
    let target = unique_temp_dir("nc-cli-maint-rust-recovery-reports");
    copy_fixture_dir("fixtures/ecmaint-post/v1.5", &target);

    let stdout = run_maint_rust_with_planted_state(&target, 1, |game_data| {
        let (_, fleet_idx) = settle_empire_one(game_data);
        let fleet = &mut game_data.fleets[fleet_idx];
        fleet.ships.cruisers = 3;
        // One cruiser has a pass left to be crippled, so it recovers in this one.
        fleet.crippled.cruisers = CrippledHulls {
            turns: [1, 0],
            fresh: 0,
        };
    });
    assert!(stdout.contains("Rust maintenance complete."));

    let text = results_text(&target);
    assert!(text.contains("1 ship recovered to full strength"), "{text}");

    cleanup_dir(&target);
}
