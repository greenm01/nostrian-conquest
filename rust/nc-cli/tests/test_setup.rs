mod common;

use common::{cleanup_dir, export_campaign_db, run_nc_cli, unique_temp_dir};
use nc_compat::load_classic_core_files;
use nc_data::troop_transports;

#[test]
fn player_name_updates_handle_and_empire() {
    let target = unique_temp_dir("nc-cli-player-name");
    run_nc_cli(&[
        "sysop",
        "generate-gamestate",
        target.to_str().unwrap(),
        "4",
        "3010",
        "16:13",
        "30:6",
        "2:25",
        "26:26",
    ]);

    let stdout = run_nc_cli(&[
        "player-name",
        target.to_str().unwrap(),
        "1",
        "tester01",
        "Auroran_Combine",
    ]);
    assert!(stdout.contains("Player 1 renamed"));

    export_campaign_db(&target, &target);
    let data = load_classic_core_files(&target).unwrap();
    assert_eq!(data.players[0].handle.trim().to_string(), "tester01");
    assert_eq!(
        data.players[0].empire_name.trim().to_string(),
        "Auroran_Combine"
    );

    cleanup_dir(&target);
}

#[test]
fn classic_login_prepare_sets_matching_handle_without_renaming_empire_by_default() {
    let target = unique_temp_dir("nc-cli-classic-login-prepare");
    run_nc_cli(&[
        "sysop",
        "generate-gamestate",
        target.to_str().unwrap(),
        "4",
        "3010",
        "16:13",
        "30:6",
        "2:25",
        "26:26",
    ]);

    let rename_stdout = run_nc_cli(&[
        "player-name",
        target.to_str().unwrap(),
        "1",
        "tester01",
        "Auroran_Combine",
    ]);
    assert!(rename_stdout.contains("Player 1 renamed"));

    let prepare_stdout = run_nc_cli(&[
        "classic-login-prepare",
        target.to_str().unwrap(),
        "1",
        "SYSOP",
    ]);
    assert!(prepare_stdout.contains("Prepared classic login for player 1"));

    export_campaign_db(&target, &target);
    let data = load_classic_core_files(&target).unwrap();
    assert_eq!(data.players[0].handle.trim().to_string(), "SYSOP");
    assert_eq!(
        data.players[0].empire_name.trim().to_string(),
        "Auroran_Combine"
    );

    cleanup_dir(&target);
}

#[test]
fn player_join_seeds_last_run_year_for_returning_player_state() {
    let target = unique_temp_dir("nc-cli-player-join-last-run-year");
    run_nc_cli(&[
        "sysop",
        "new-game",
        target.to_str().unwrap(),
        "--players",
        "4",
        "--seed",
        "1515",
    ]);

    let stdout = run_nc_cli(&[
        "player-join",
        target.to_str().unwrap(),
        "1",
        "SYSOP",
        "Auroran_Combine",
        "Foundation",
    ]);
    assert!(stdout.contains("Joined player 1"));

    export_campaign_db(&target, &target);
    let data = load_classic_core_files(&target).unwrap();
    assert_eq!(data.players[0].last_year_on, 3000);
    assert_eq!(data.players[0].handle.trim().to_string(), "SYSOP");

    cleanup_dir(&target);
}

#[test]
fn economy_starbase_probe_init_creates_matched_plain_and_starbase_colonies() {
    let target = unique_temp_dir("nc-cli-economy-starbase-probe");
    run_nc_cli(&[
        "sysop",
        "new-game",
        target.to_str().unwrap(),
        "--players",
        "4",
        "--seed",
        "1515",
    ]);

    let stdout = run_nc_cli(&[
        "economy-starbase-probe-init",
        target.to_str().unwrap(),
        "1",
        "67",
    ]);
    assert!(stdout.contains("Initialized economy starbase tax-burden probe"));

    export_campaign_db(&target, &target);
    let data = load_classic_core_files(&target).unwrap();
    assert_eq!(data.players[0].bases_first.0, 1);

    let rows = data.empire_planet_economy_rows(1);
    let plain = rows
        .iter()
        .find(|row| row.planet_name == "Plain Colony")
        .expect("plain colony should exist");
    let starbase = rows
        .iter()
        .find(|row| row.planet_name == "Base Colony")
        .expect("starbase colony should exist");

    assert_eq!(plain.present_production, 50);
    assert_eq!(starbase.present_production, 50);
    assert!(!plain.has_friendly_starbase);
    assert!(starbase.has_friendly_starbase);
    assert_eq!(plain.build_capacity, 50);
    assert_eq!(starbase.build_capacity, 250);

    cleanup_dir(&target);
}

#[test]
fn fleet_ships_and_detach_create_varied_extra_fleet() {
    let target = unique_temp_dir("nc-cli-fleet-setup");
    run_nc_cli(&[
        "sysop",
        "generate-gamestate",
        target.to_str().unwrap(),
        "4",
        "3010",
        "16:13",
        "30:6",
        "2:25",
        "26:26",
    ]);

    let stdout = run_nc_cli(&[
        "fleet-ships",
        target.to_str().unwrap(),
        "1",
        "12",
        "8",
        "10",
        "14",
        "9",
        "4",
        "2",
    ]);
    // The feedback line writes the fleet in the ship notation: nine
    // transports, four of them loaded.
    assert!(stdout.contains("Fleet 1 ships set: SC12 BB8 CA10 DD14 TT*4 TT5 ETAC2"));

    let stdout = run_nc_cli(&[
        "fleet-detach",
        target.to_str().unwrap(),
        "1",
        "1",
        "1",
        "2",
        "3",
        "1",
        "1",
        "2",
        "1",
        "3",
        "6",
    ]);
    assert!(stdout.contains(
        "Detached fleet 1 -> new fleet 17 for player 1: SC2 BB1 CA2 DD3 TT*1 TT1 ETAC1 donor_speed=3 roe=6"
    ));

    export_campaign_db(&target, &target);
    let data = load_classic_core_files(&target).unwrap();
    assert_eq!(data.fleets.len(), 17);
    let donor = &data.fleets[0];
    let new_fleet = &data.fleets[16];
    assert_eq!(donor.ships.battleships, 7);
    assert_eq!(donor.ships.cruisers, 8);
    assert_eq!(donor.ships.destroyers, 11);
    assert_eq!(troop_transports(donor), 7);
    assert_eq!(donor.ships.loaded_transports, 3);
    assert_eq!(donor.ships.scouts, 10);
    assert_eq!(donor.ships.etacs, 1);
    assert_eq!(new_fleet.ships.battleships, 1);
    assert_eq!(new_fleet.ships.cruisers, 2);
    assert_eq!(new_fleet.ships.destroyers, 3);
    assert_eq!(troop_transports(new_fleet), 2);
    assert_eq!(new_fleet.ships.loaded_transports, 1);
    assert_eq!(new_fleet.ships.scouts, 2);
    assert_eq!(new_fleet.ships.etacs, 1);

    cleanup_dir(&target);
}
