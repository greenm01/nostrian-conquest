mod common;

use common::{cleanup_dir, export_campaign_db, run_nc_cli, unique_temp_dir};

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
    let data = nc_data::CoreGameData::load(&target).unwrap();
    assert_eq!(
        data.player.records[0].assigned_player_handle_summary(),
        "tester01"
    );
    assert_eq!(
        data.player.records[0].controlled_empire_name_summary(),
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
    let data = nc_data::CoreGameData::load(&target).unwrap();
    assert_eq!(
        data.player.records[0].assigned_player_handle_summary(),
        "SYSOP"
    );
    assert_eq!(
        data.player.records[0].controlled_empire_name_summary(),
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
    let data = nc_data::CoreGameData::load(&target).unwrap();
    assert_eq!(data.player.records[0].last_run_year_raw(), 3000);
    assert_eq!(
        data.player.records[0].assigned_player_handle_summary(),
        "SYSOP"
    );

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
    let data = nc_data::CoreGameData::load(&target).unwrap();
    assert_eq!(data.player.records[0].first_base_record_raw(), 1);

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

    run_nc_cli(&[
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
    assert!(stdout.contains("Detached fleet 1 -> new fleet 17"));

    export_campaign_db(&target, &target);
    let data = nc_data::CoreGameData::load(&target).unwrap();
    assert_eq!(data.fleets.records.len(), 17);
    let donor = &data.fleets.records[0];
    let new_fleet = &data.fleets.records[16];
    assert_eq!(donor.battleship_count(), 7);
    assert_eq!(donor.cruiser_count(), 8);
    assert_eq!(donor.destroyer_count(), 11);
    assert_eq!(donor.troop_transport_count(), 7);
    assert_eq!(donor.army_count(), 3);
    assert_eq!(donor.scout_count(), 10);
    assert_eq!(donor.etac_count(), 1);
    assert_eq!(new_fleet.battleship_count(), 1);
    assert_eq!(new_fleet.cruiser_count(), 2);
    assert_eq!(new_fleet.destroyer_count(), 3);
    assert_eq!(new_fleet.troop_transport_count(), 2);
    assert_eq!(new_fleet.army_count(), 1);
    assert_eq!(new_fleet.scout_count(), 2);
    assert_eq!(new_fleet.etac_count(), 1);

    cleanup_dir(&target);
}
