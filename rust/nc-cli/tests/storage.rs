mod common;

use nc_data::{
    CampaignStore, Coords, EmpireId, Order, Planet, Real48, is_homeworld_seed_ignoring_name,
    set_armies_aboard, set_troop_transports,
};
use std::fs;
use std::path::Path;

use common::{
    cleanup_dir, ecmaint_oracle_available, run_classic_ecgame_smoke_with_alias, run_ecmaint_oracle,
    run_nc_cli, unique_temp_dir, without_unread_bytes,
};
use nc_compat::{
    DatabaseDat, export_latest_snapshot_to_dir, load_classic_core_files, save_classic_core_files,
};

fn setup_classic_probe_players(target: &Path) {
    let player_specs = [
        ("1", "SYSOP", "Auroran Combine", "Foundation", "42"),
        ("2", "HECATE", "Red Horizon Pact", "Red Haven", "50"),
        ("3", "ORION", "Vela Syndicate", "Vela Prime", "36"),
        ("4", "TESS", "Helios Crown", "Crownfall", "58"),
    ];

    for (record, handle, empire, homeworld, tax_rate) in player_specs {
        if record == "1" {
            run_nc_cli(&[
                "player-join",
                target.to_str().unwrap(),
                record,
                handle,
                empire,
                homeworld,
            ]);
        } else {
            run_nc_cli(&[
                "player-name",
                target.to_str().unwrap(),
                record,
                handle,
                empire,
            ]);
        }
        run_nc_cli(&["player-tax", target.to_str().unwrap(), record, tax_rate]);
    }
}

fn setup_classic_probe_planets(target: &Path) {
    let planet_specs = [
        ("1", "1", "Foundation", "100", "0", "10", "4"),
        ("2", "2", "Red Haven", "100", "0", "10", "4"),
        ("3", "3", "Vela Prime", "100", "0", "10", "4"),
        ("4", "4", "Crownfall", "100", "0", "10", "4"),
        ("5", "4", "Helios Prime", "136", "35", "10", "6"),
        ("8", "3", "Outer Vela", "128", "26", "8", "5"),
        ("12", "3", "Vela Gate", "104", "18", "7", "4"),
        ("13", "2", "Red Bastion", "132", "28", "12", "7"),
        ("15", "2", "Crucible", "118", "24", "14", "8"),
        ("16", "1", "Aurora Prime", "144", "48", "12", "6"),
        ("17", "1", "Relay", "96", "20", "5", "3"),
        ("19", "1", "Outrider", "84", "14", "3", "2"),
    ];

    for (record, owner, name, potential, stored, armies, batteries) in planet_specs {
        run_nc_cli(&["planet-owner", target.to_str().unwrap(), record, owner]);
        run_nc_cli(&["planet-name", target.to_str().unwrap(), record, name]);
        run_nc_cli(&[
            "planet-potential",
            target.to_str().unwrap(),
            record,
            potential,
            "135",
        ]);
        run_nc_cli(&["planet-stored", target.to_str().unwrap(), record, stored]);
        run_nc_cli(&[
            "planet-stats",
            target.to_str().unwrap(),
            record,
            armies,
            batteries,
        ]);
    }

    let store = CampaignStore::open_default_in_dir(target).expect("probe store should open");
    let state = store
        .load_latest_runtime_state()
        .expect("probe runtime should load")
        .expect("probe runtime should exist");
    let mut game_data = state.game_data;
    // Keep the foreign-intel storage probe independent of mapgen drift: the
    // scout/view tests below intentionally target planet record 5 at (9,2).
    game_data.planets[4].sector = Coords::new(9, 2);
    // The probe worlds reuse the homeworld-seed marker (potential high byte
    // 135) after being named, so give them full production explicitly.
    for planet in &mut game_data.planets {
        if is_homeworld_seed_ignoring_name(planet) {
            let potential = u16::from(planet.potential_production);
            planet.production = Real48::from_f64(f64::from(potential));
        }
    }
    let planet_intel_by_viewer = (1..=game_data.conquest.player_count)
        .map(|viewer_empire_id| {
            store
                .latest_planet_intel_for_viewer(viewer_empire_id)
                .expect("probe intel should load")
                .into_iter()
                .map(|snapshot| (snapshot.planet_record_index_1_based, snapshot))
                .collect()
        })
        .collect::<Vec<_>>();
    store
        .save_runtime_state_structured_with_intel(
            game_data.conquest.game_year,
            &game_data,
            &state.planet_scorch_orders,
            &state.report_block_rows,
            &state.queued_mail,
            &planet_intel_by_viewer,
        )
        .expect("probe runtime should save");
    export_latest_snapshot_to_dir(&store, target).expect("probe snapshot should export");
}

fn setup_classic_probe_scout_order(target: &Path) {
    run_nc_cli(&[
        "fleet-ships",
        target.to_str().unwrap(),
        "2",
        "1",
        "0",
        "1",
        "2",
        "0",
        "0",
        "0",
    ]);

    run_nc_cli(&[
        "fleet-order",
        target.to_str().unwrap(),
        "2",
        "3",
        "11",
        "9",
        "2",
    ]);
}

fn setup_classic_probe_view_order(target: &Path) {
    run_nc_cli(&[
        "fleet-ships",
        target.to_str().unwrap(),
        "2",
        "1",
        "0",
        "1",
        "2",
        "0",
        "0",
        "0",
    ]);

    run_nc_cli(&[
        "fleet-order",
        target.to_str().unwrap(),
        "2",
        "3",
        "9",
        "9",
        "2",
    ]);
}

fn assert_database_row_survives_db_import_export(
    source: &Path,
    exported: &Path,
    planet_idx: usize,
    player_idx: usize,
) {
    let source_data = load_classic_core_files(source).expect("source game should load");
    let source_database_bytes =
        fs::read(source.join("DATABASE.DAT")).expect("source DATABASE.DAT should exist");
    let source_database =
        DatabaseDat::parse(&source_database_bytes).expect("source DATABASE.DAT should parse");
    let expected = source_database
        .record(planet_idx, player_idx, source_data.planets.len())
        .raw;

    fs::remove_file(source.join("ncgame.db")).expect("source ncgame.db should remove cleanly");
    let import_stdout = run_nc_cli(&["db-import", source.to_str().unwrap()]);
    assert!(import_stdout.contains("Imported"));

    let export_stdout = run_nc_cli(&[
        "db-export",
        source.to_str().unwrap(),
        exported.to_str().unwrap(),
    ]);
    assert!(export_stdout.contains("Exported year"));

    let exported_data = load_classic_core_files(exported).expect("exported game should load");
    let exported_database_bytes =
        fs::read(exported.join("DATABASE.DAT")).expect("exported DATABASE.DAT should exist");
    let exported_database =
        DatabaseDat::parse(&exported_database_bytes).expect("exported DATABASE.DAT should parse");
    let actual = exported_database
        .record(planet_idx, player_idx, exported_data.planets.len())
        .raw;

    assert_eq!(actual, expected);
}

#[test]
fn db_import_and_export_round_trip_fixture() {
    let source = unique_temp_dir("nc-cli-db-import");
    let exported = unique_temp_dir("nc-cli-db-export");
    common::copy_fixture_dir("fixtures/ecutil-init/v1.5", &source);

    let import_stdout = run_nc_cli(&["db-import", source.to_str().unwrap()]);
    assert!(import_stdout.contains("Imported"));
    assert!(source.join("ncgame.db").exists());

    let export_stdout = run_nc_cli(&[
        "db-export",
        source.to_str().unwrap(),
        exported.to_str().unwrap(),
    ]);
    assert!(export_stdout.contains("Exported year 3000"));
    assert_eq!(
        without_unread_bytes("PLAYER.DAT", &fs::read(source.join("PLAYER.DAT")).unwrap()),
        without_unread_bytes(
            "PLAYER.DAT",
            &fs::read(exported.join("PLAYER.DAT")).unwrap()
        )
    );
    assert_eq!(
        without_unread_bytes(
            "DATABASE.DAT",
            &fs::read(source.join("DATABASE.DAT")).unwrap()
        ),
        without_unread_bytes(
            "DATABASE.DAT",
            &fs::read(exported.join("DATABASE.DAT")).unwrap()
        )
    );
    assert!(exported.join("ECGAME.EXE").exists());
    assert!(exported.join("ECMAINT.EXE").exists());
    assert!(exported.join("ECUTIL.EXE").exists());

    cleanup_dir(&source);
    cleanup_dir(&exported);
}

#[test]
fn sqlite_maint_exported_directory_is_accepted_by_ecmaint_oracle() {
    if !ecmaint_oracle_available() {
        eprintln!("skipping ecmaint oracle regression: external python3/dosbox-x unavailable");
        return;
    }

    let source = unique_temp_dir("nc-cli-db-maint-source");
    let exported = unique_temp_dir("nc-cli-db-maint-export");
    common::copy_fixture_dir("fixtures/ecmaint-post/v1.5", &source);

    let import_stdout = run_nc_cli(&["db-import", source.to_str().unwrap()]);
    assert!(import_stdout.contains("Imported"));

    let maint_stdout = run_nc_cli(&["maint-rust", source.to_str().unwrap(), "1"]);
    assert!(maint_stdout.contains("Rust maintenance complete."));

    let export_stdout = run_nc_cli(&[
        "db-export",
        source.to_str().unwrap(),
        exported.to_str().unwrap(),
    ]);
    assert!(export_stdout.contains("Exported year"));

    let oracle_stdout = run_ecmaint_oracle(&exported);
    assert!(!oracle_stdout.trim().is_empty());
    assert!(exported.join("ECGAME.EXE").exists());

    cleanup_dir(&source);
    cleanup_dir(&exported);
}

#[test]
fn db_export_preserves_classic_player_handle_identity() {
    let source = unique_temp_dir("nc-cli-db-export-player-handle-source");
    let exported = unique_temp_dir("nc-cli-db-export-player-handle-exported");

    let stdout = run_nc_cli(&["sysop", "new-game", source.to_str().unwrap()]);
    assert!(stdout.contains("Initialized new game"));

    let rename_stdout = run_nc_cli(&[
        "player-name",
        source.to_str().unwrap(),
        "1",
        "SYSOP",
        "Auroran Combine",
    ]);
    assert!(rename_stdout.contains("Player 1 renamed"));

    let export_stdout = run_nc_cli(&[
        "db-export",
        source.to_str().unwrap(),
        exported.to_str().unwrap(),
    ]);
    assert!(export_stdout.contains("Exported year 3000"));

    let exported_data = load_classic_core_files(&exported).expect("exported game should load");
    assert_eq!(exported_data.players[0].handle.trim().to_string(), "SYSOP");
    assert_eq!(
        exported_data.players[0].empire_name.trim().to_string(),
        "Auroran Combine"
    );

    cleanup_dir(&source);
    cleanup_dir(&exported);
}

#[test]
fn db_export_preserves_classic_login_classification_for_prepared_slot() {
    let source = unique_temp_dir("nc-cli-db-export-classic-login-source");
    let exported = unique_temp_dir("nc-cli-db-export-classic-login-exported");

    let stdout = run_nc_cli(&["sysop", "new-game", source.to_str().unwrap()]);
    assert!(stdout.contains("Initialized new game"));

    let prepare_stdout = run_nc_cli(&[
        "classic-login-prepare",
        source.to_str().unwrap(),
        "2",
        "SYSOP",
        "foo",
    ]);
    assert!(prepare_stdout.contains("Prepared classic login for player 2"));

    let export_stdout = run_nc_cli(&[
        "db-export",
        source.to_str().unwrap(),
        exported.to_str().unwrap(),
    ]);
    assert!(export_stdout.contains("Exported year 3000"));

    let inspect_stdout =
        run_nc_cli(&["inspect-classic-login", exported.to_str().unwrap(), "SYSOP"]);
    assert!(inspect_stdout.contains("slot 2: classification=matched-preloaded-first-login"));
    assert!(inspect_stdout.contains("handle='SYSOP'"));
    assert!(inspect_stdout.contains("empire='foo'"));

    cleanup_dir(&source);
    cleanup_dir(&exported);
}

#[test]
fn db_export_preserves_returning_player_classification() {
    let source = unique_temp_dir("nc-cli-db-export-returning-player-source");
    let exported = unique_temp_dir("nc-cli-db-export-returning-player-exported");
    common::copy_fixture_dir("original/v1.5", &source);

    let import_stdout = run_nc_cli(&["db-import", source.to_str().unwrap()]);
    assert!(import_stdout.contains("Imported"));

    let export_stdout = run_nc_cli(&[
        "db-export",
        source.to_str().unwrap(),
        exported.to_str().unwrap(),
    ]);
    assert!(export_stdout.contains("Exported year"));

    let inspect_stdout = run_nc_cli(&[
        "inspect-classic-login",
        exported.to_str().unwrap(),
        "HANNIBAL",
    ]);
    assert!(inspect_stdout.contains("slot 1: classification=returning-player"));
    assert!(inspect_stdout.contains("homeworld='Dust Bowl'"));

    cleanup_dir(&source);
    cleanup_dir(&exported);
}

#[test]
fn db_export_preserves_owned_world_compat_word_family_from_imported_template() {
    let source = unique_temp_dir("nc-cli-db-export-owned-world-source");
    let exported = unique_temp_dir("nc-cli-db-export-owned-world-exported");
    common::copy_fixture_dir("original/v1.5", &source);

    let import_stdout = run_nc_cli(&["db-import", source.to_str().unwrap()]);
    assert!(import_stdout.contains("Imported"));

    let export_stdout = run_nc_cli(&[
        "db-export",
        source.to_str().unwrap(),
        exported.to_str().unwrap(),
    ]);
    assert!(export_stdout.contains("Exported year"));

    let exported_data = load_classic_core_files(&exported).expect("exported game should load");
    let database_bytes =
        fs::read(exported.join("DATABASE.DAT")).expect("DATABASE.DAT should exist");
    let database = DatabaseDat::parse(&database_bytes).expect("DATABASE.DAT should parse");
    let row = database.record(15, 0, exported_data.planets.len());
    assert_eq!(u16::from_le_bytes([row.raw[0x1e], row.raw[0x1f]]), 0x42);

    cleanup_dir(&source);
    cleanup_dir(&exported);
}

#[test]
fn db_export_keeps_unknown_rows_at_classic_sentinels() {
    let source = unique_temp_dir("nc-cli-db-export-unknown-sentinels-source");
    let exported = unique_temp_dir("nc-cli-db-export-unknown-sentinels-exported");

    let stdout = run_nc_cli(&["sysop", "new-game", source.to_str().unwrap()]);
    assert!(stdout.contains("Initialized new game"));

    let export_stdout = run_nc_cli(&[
        "db-export",
        source.to_str().unwrap(),
        exported.to_str().unwrap(),
    ]);
    assert!(export_stdout.contains("Exported year 3000"));

    let exported_data = load_classic_core_files(&exported).expect("exported game should load");
    let database_bytes =
        fs::read(exported.join("DATABASE.DAT")).expect("DATABASE.DAT should exist");
    let database = DatabaseDat::parse(&database_bytes).expect("DATABASE.DAT should parse");
    let row = database.record(0, 0, exported_data.planets.len());
    assert_eq!(row.planet_name_bytes(), b"UNKNOWN");
    for offset in [
        0x15usize, 0x1c, 0x1d, 0x1e, 0x1f, 0x20, 0x23, 0x24, 0x25, 0x26,
    ] {
        assert_eq!(
            row.raw[offset], 0xff,
            "offset {offset:#x} should be unknown"
        );
    }

    cleanup_dir(&source);
    cleanup_dir(&exported);
}

#[test]
fn db_export_emits_ecgame_accepted_foreign_full_intel_row_shape() {
    let source = unique_temp_dir("nc-cli-db-export-foreign-full-intel-source");
    let exported = unique_temp_dir("nc-cli-db-export-foreign-full-intel-exported");

    let stdout = run_nc_cli(&[
        "sysop",
        "new-game",
        source.to_str().unwrap(),
        "--players",
        "4",
        "--seed",
        "1515",
    ]);
    assert!(stdout.contains("Initialized new game"));

    setup_classic_probe_players(&source);
    setup_classic_probe_planets(&source);
    setup_classic_probe_scout_order(&source);

    let maint_stdout = run_nc_cli(&["maint-rust", source.to_str().unwrap(), "4"]);
    assert!(maint_stdout.contains("Rust maintenance complete."));

    let export_stdout = run_nc_cli(&[
        "db-export",
        source.to_str().unwrap(),
        exported.to_str().unwrap(),
    ]);
    assert!(export_stdout.contains("Exported year 3004"));

    let exported_data = load_classic_core_files(&exported).expect("exported game should load");
    let database_bytes =
        fs::read(exported.join("DATABASE.DAT")).expect("DATABASE.DAT should exist");
    let database = DatabaseDat::parse(&database_bytes).expect("DATABASE.DAT should parse");
    let row = database.record(4, 0, exported_data.planets.len());
    assert_eq!(row.planet_name_bytes(), b"Helios Prime");
    assert_eq!(row.raw[0x15], 4);
    assert_eq!(row.raw[0x1c], 136);
    assert_eq!(row.raw[0x1d], 136);
    assert_eq!(u16::from_le_bytes([row.raw[0x1e], row.raw[0x1f]]), 0x23);
    assert_eq!(row.raw[0x23], 10);
    assert_eq!(row.raw[0x24], 0x00);
    assert_eq!(row.raw[0x25], 6);
    assert_eq!(row.raw[0x26], 0x00);
    assert_eq!(u16::from_le_bytes([row.raw[0x16], row.raw[0x17]]), 3003);
    assert_eq!(u16::from_le_bytes([row.raw[0x27], row.raw[0x28]]), 3003);

    cleanup_dir(&source);
    cleanup_dir(&exported);
}

#[test]
#[ignore = "launches classic ECGAME through dosbox-x"]
fn db_export_foreign_full_intel_directory_reopens_in_classic_ecgame_smoke() {
    let source = unique_temp_dir("nc-cli-db-export-foreign-full-intel-ecgame-source");
    let exported = unique_temp_dir("nc-cli-db-export-foreign-full-intel-ecgame-exported");

    let stdout = run_nc_cli(&[
        "sysop",
        "new-game",
        source.to_str().unwrap(),
        "--players",
        "4",
        "--seed",
        "1515",
    ]);
    assert!(stdout.contains("Initialized new game"));

    setup_classic_probe_players(&source);
    setup_classic_probe_planets(&source);
    setup_classic_probe_scout_order(&source);

    let maint_stdout = run_nc_cli(&["maint-rust", source.to_str().unwrap(), "4"]);
    assert!(maint_stdout.contains("Rust maintenance complete."));

    let export_stdout = run_nc_cli(&[
        "db-export",
        source.to_str().unwrap(),
        exported.to_str().unwrap(),
    ]);
    assert!(export_stdout.contains("Exported year 3004"));

    run_classic_ecgame_smoke_with_alias(&exported, 1, "SYSOP");

    cleanup_dir(&source);
    cleanup_dir(&exported);
}

#[test]
fn db_export_refreshes_stale_foreign_scout_row_visible_stats() {
    let source = unique_temp_dir("nc-cli-db-export-refresh-stale-scout-source");
    let exported = unique_temp_dir("nc-cli-db-export-refresh-stale-scout-exported");

    let stdout = run_nc_cli(&[
        "sysop",
        "new-game",
        source.to_str().unwrap(),
        "--players",
        "4",
        "--seed",
        "1515",
    ]);
    assert!(stdout.contains("Initialized new game"));

    setup_classic_probe_players(&source);
    setup_classic_probe_planets(&source);
    setup_classic_probe_scout_order(&source);

    let database_bytes = fs::read(source.join("DATABASE.DAT")).expect("DATABASE.DAT should exist");
    let mut database = DatabaseDat::parse(&database_bytes).expect("DATABASE.DAT should parse");
    let row = database.record_mut(4, 0, 20);
    row.set_planet_name("Helios Prime");
    row.raw[0x15] = 4;
    row.raw[0x1c] = 136;
    row.raw[0x1d] = 44;
    row.set_word_at(0x1e, 35);
    row.raw[0x23] = 17;
    row.raw[0x24] = 0x00;
    row.raw[0x25] = 9;
    row.raw[0x26] = 0x00;
    row.set_word_at(0x16, 2999);
    row.set_word_at(0x18, 2999);
    row.set_word_at(0x27, 2999);
    fs::write(source.join("DATABASE.DAT"), database.to_bytes()).expect("DATABASE.DAT should save");

    let maint_stdout = run_nc_cli(&["maint-rust", source.to_str().unwrap(), "4"]);
    assert!(maint_stdout.contains("Rust maintenance complete."));

    let export_stdout = run_nc_cli(&[
        "db-export",
        source.to_str().unwrap(),
        exported.to_str().unwrap(),
    ]);
    assert!(export_stdout.contains("Exported year 3004"));

    let exported_data = load_classic_core_files(&exported).expect("exported game should load");
    let database_bytes =
        fs::read(exported.join("DATABASE.DAT")).expect("DATABASE.DAT should exist");
    let database = DatabaseDat::parse(&database_bytes).expect("DATABASE.DAT should parse");
    let row = database.record(4, 0, exported_data.planets.len());
    assert_eq!(row.planet_name_bytes(), b"Helios Prime");
    assert_eq!(row.raw[0x15], 4);
    assert_eq!(row.raw[0x1c], 136);
    assert_eq!(row.raw[0x1d], 136);
    assert_eq!(u16::from_le_bytes([row.raw[0x1e], row.raw[0x1f]]), 35);
    assert_eq!(row.raw[0x23], 10);
    assert_eq!(row.raw[0x24], 0x00);
    assert_eq!(row.raw[0x25], 6);
    assert_eq!(row.raw[0x26], 0x00);
    assert_eq!(u16::from_le_bytes([row.raw[0x16], row.raw[0x17]]), 3003);
    assert_eq!(u16::from_le_bytes([row.raw[0x27], row.raw[0x28]]), 3003);

    cleanup_dir(&source);
    cleanup_dir(&exported);
}

#[test]
#[ignore = "launches classic ECGAME through dosbox-x"]
fn db_export_refreshed_stale_foreign_scout_directory_reopens_in_classic_ecgame_smoke() {
    let source = unique_temp_dir("nc-cli-db-export-refresh-stale-scout-ecgame-source");
    let exported = unique_temp_dir("nc-cli-db-export-refresh-stale-scout-ecgame-exported");

    let stdout = run_nc_cli(&[
        "sysop",
        "new-game",
        source.to_str().unwrap(),
        "--players",
        "4",
        "--seed",
        "1515",
    ]);
    assert!(stdout.contains("Initialized new game"));

    setup_classic_probe_players(&source);
    setup_classic_probe_planets(&source);
    setup_classic_probe_scout_order(&source);

    let database_bytes = fs::read(source.join("DATABASE.DAT")).expect("DATABASE.DAT should exist");
    let mut database = DatabaseDat::parse(&database_bytes).expect("DATABASE.DAT should parse");
    let row = database.record_mut(4, 0, 20);
    row.set_planet_name("Helios Prime");
    row.raw[0x15] = 4;
    row.raw[0x1c] = 136;
    row.raw[0x1d] = 44;
    row.set_word_at(0x1e, 35);
    row.raw[0x23] = 17;
    row.raw[0x24] = 0x00;
    row.raw[0x25] = 9;
    row.raw[0x26] = 0x00;
    row.set_word_at(0x16, 2999);
    row.set_word_at(0x18, 2999);
    row.set_word_at(0x27, 2999);
    fs::write(source.join("DATABASE.DAT"), database.to_bytes()).expect("DATABASE.DAT should save");

    let maint_stdout = run_nc_cli(&["maint-rust", source.to_str().unwrap(), "4"]);
    assert!(maint_stdout.contains("Rust maintenance complete."));
    let export_stdout = run_nc_cli(&[
        "db-export",
        source.to_str().unwrap(),
        exported.to_str().unwrap(),
    ]);
    assert!(export_stdout.contains("Exported year 3004"));

    run_classic_ecgame_smoke_with_alias(&exported, 1, "SYSOP");

    cleanup_dir(&source);
    cleanup_dir(&exported);
}

#[test]
fn db_import_export_preserves_foreign_partial_intel_row_shape() {
    let source = unique_temp_dir("nc-cli-db-import-foreign-partial-intel-source");
    let exported = unique_temp_dir("nc-cli-db-import-foreign-partial-intel-exported");

    let stdout = run_nc_cli(&[
        "sysop",
        "new-game",
        source.to_str().unwrap(),
        "--players",
        "4",
        "--seed",
        "1515",
    ]);
    assert!(stdout.contains("Initialized new game"));

    setup_classic_probe_players(&source);
    setup_classic_probe_planets(&source);
    setup_classic_probe_scout_order(&source);

    let maint_stdout = run_nc_cli(&["maint-rust", source.to_str().unwrap(), "4"]);
    assert!(maint_stdout.contains("Rust maintenance complete."));
    let export_stdout = run_nc_cli(&[
        "db-export",
        source.to_str().unwrap(),
        source.to_str().unwrap(),
    ]);
    assert!(export_stdout.contains("Exported year 3004"));

    let database_bytes = fs::read(source.join("DATABASE.DAT")).expect("DATABASE.DAT should exist");
    let mut database = DatabaseDat::parse(&database_bytes).expect("DATABASE.DAT should parse");
    let row = database.record_mut(4, 0, 20);
    row.raw[0x23] = 0xff;
    row.raw[0x24] = 0xff;
    row.raw[0x25] = 0xff;
    row.raw[0x26] = 0xff;
    row.set_word_at(0x27, 0);
    fs::write(source.join("DATABASE.DAT"), database.to_bytes()).expect("DATABASE.DAT should save");

    assert_database_row_survives_db_import_export(&source, &exported, 4, 0);

    cleanup_dir(&source);
    cleanup_dir(&exported);
}

#[test]
#[ignore = "launches classic ECGAME through dosbox-x"]
fn db_import_export_foreign_partial_intel_directory_reopens_in_classic_ecgame_smoke() {
    let source = unique_temp_dir("nc-cli-db-import-foreign-partial-intel-ecgame-source");
    let exported = unique_temp_dir("nc-cli-db-import-foreign-partial-intel-ecgame-exported");

    let stdout = run_nc_cli(&[
        "sysop",
        "new-game",
        source.to_str().unwrap(),
        "--players",
        "4",
        "--seed",
        "1515",
    ]);
    assert!(stdout.contains("Initialized new game"));

    setup_classic_probe_players(&source);
    setup_classic_probe_planets(&source);
    setup_classic_probe_scout_order(&source);

    let maint_stdout = run_nc_cli(&["maint-rust", source.to_str().unwrap(), "4"]);
    assert!(maint_stdout.contains("Rust maintenance complete."));
    let export_stdout = run_nc_cli(&[
        "db-export",
        source.to_str().unwrap(),
        source.to_str().unwrap(),
    ]);
    assert!(export_stdout.contains("Exported year 3004"));

    let database_bytes = fs::read(source.join("DATABASE.DAT")).expect("DATABASE.DAT should exist");
    let mut database = DatabaseDat::parse(&database_bytes).expect("DATABASE.DAT should parse");
    let row = database.record_mut(4, 0, 20);
    row.raw[0x23] = 0xff;
    row.raw[0x24] = 0xff;
    row.raw[0x25] = 0xff;
    row.raw[0x26] = 0xff;
    row.set_word_at(0x27, 0);
    fs::write(source.join("DATABASE.DAT"), database.to_bytes()).expect("DATABASE.DAT should save");

    fs::remove_file(source.join("ncgame.db")).expect("source ncgame.db should remove cleanly");
    let import_stdout = run_nc_cli(&["db-import", source.to_str().unwrap()]);
    assert!(import_stdout.contains("Imported"));
    let export_stdout = run_nc_cli(&[
        "db-export",
        source.to_str().unwrap(),
        exported.to_str().unwrap(),
    ]);
    assert!(export_stdout.contains("Exported year 3004"));

    run_classic_ecgame_smoke_with_alias(&exported, 1, "SYSOP");

    cleanup_dir(&source);
    cleanup_dir(&exported);
}

#[test]
fn db_export_emits_ecgame_accepted_foreign_view_only_row_shape() {
    let source = unique_temp_dir("nc-cli-db-export-foreign-view-intel-source");
    let exported = unique_temp_dir("nc-cli-db-export-foreign-view-intel-exported");

    let stdout = run_nc_cli(&[
        "sysop",
        "new-game",
        source.to_str().unwrap(),
        "--players",
        "4",
        "--seed",
        "1515",
    ]);
    assert!(stdout.contains("Initialized new game"));

    setup_classic_probe_players(&source);
    setup_classic_probe_planets(&source);
    setup_classic_probe_view_order(&source);

    let maint_stdout = run_nc_cli(&["maint-rust", source.to_str().unwrap(), "4"]);
    assert!(maint_stdout.contains("Rust maintenance complete."));

    let export_stdout = run_nc_cli(&[
        "db-export",
        source.to_str().unwrap(),
        exported.to_str().unwrap(),
    ]);
    assert!(export_stdout.contains("Exported year 3004"));

    let exported_data = load_classic_core_files(&exported).expect("exported game should load");
    let database_bytes =
        fs::read(exported.join("DATABASE.DAT")).expect("DATABASE.DAT should exist");
    let database = DatabaseDat::parse(&database_bytes).expect("DATABASE.DAT should parse");
    let row = database.record(4, 0, exported_data.planets.len());
    assert_eq!(row.planet_name_bytes(), b"Helios Prime");
    assert_eq!(row.raw[0x15], 4);
    assert_eq!(row.raw[0x1c], 136);
    assert_eq!(row.raw[0x1d], 0xff);
    assert_eq!(u16::from_le_bytes([row.raw[0x1e], row.raw[0x1f]]), u16::MAX);
    assert_eq!(row.raw[0x23], 0xff);
    assert_eq!(row.raw[0x24], 0xff);
    assert_eq!(row.raw[0x25], 0xff);
    assert_eq!(row.raw[0x26], 0xff);
    assert_eq!(u16::from_le_bytes([row.raw[0x16], row.raw[0x17]]), 3003);
    assert_eq!(u16::from_le_bytes([row.raw[0x27], row.raw[0x28]]), 3003);

    cleanup_dir(&source);
    cleanup_dir(&exported);
}

#[test]
#[ignore = "launches classic ECGAME through dosbox-x"]
fn db_export_foreign_view_only_directory_reopens_in_classic_ecgame_smoke() {
    let source = unique_temp_dir("nc-cli-db-export-foreign-view-intel-ecgame-source");
    let exported = unique_temp_dir("nc-cli-db-export-foreign-view-intel-ecgame-exported");

    let stdout = run_nc_cli(&[
        "sysop",
        "new-game",
        source.to_str().unwrap(),
        "--players",
        "4",
        "--seed",
        "1515",
    ]);
    assert!(stdout.contains("Initialized new game"));

    setup_classic_probe_players(&source);
    setup_classic_probe_planets(&source);
    setup_classic_probe_view_order(&source);

    let maint_stdout = run_nc_cli(&["maint-rust", source.to_str().unwrap(), "4"]);
    assert!(maint_stdout.contains("Rust maintenance complete."));

    let export_stdout = run_nc_cli(&[
        "db-export",
        source.to_str().unwrap(),
        exported.to_str().unwrap(),
    ]);
    assert!(export_stdout.contains("Exported year 3004"));

    run_classic_ecgame_smoke_with_alias(&exported, 1, "SYSOP");

    cleanup_dir(&source);
    cleanup_dir(&exported);
}

#[test]
fn db_import_export_preserves_foreign_full_intel_row_shape() {
    let source = unique_temp_dir("nc-cli-db-import-foreign-full-intel-source");
    let exported = unique_temp_dir("nc-cli-db-import-foreign-full-intel-exported");

    let stdout = run_nc_cli(&[
        "sysop",
        "new-game",
        source.to_str().unwrap(),
        "--players",
        "4",
        "--seed",
        "1515",
    ]);
    assert!(stdout.contains("Initialized new game"));

    setup_classic_probe_players(&source);
    setup_classic_probe_planets(&source);
    setup_classic_probe_scout_order(&source);

    let maint_stdout = run_nc_cli(&["maint-rust", source.to_str().unwrap(), "4"]);
    assert!(maint_stdout.contains("Rust maintenance complete."));
    let export_stdout = run_nc_cli(&[
        "db-export",
        source.to_str().unwrap(),
        source.to_str().unwrap(),
    ]);
    assert!(export_stdout.contains("Exported year 3004"));
    assert_database_row_survives_db_import_export(&source, &exported, 4, 0);

    cleanup_dir(&source);
    cleanup_dir(&exported);
}

#[test]
fn db_import_export_preserves_foreign_view_only_row_shape() {
    let source = unique_temp_dir("nc-cli-db-import-foreign-view-intel-source");
    let exported = unique_temp_dir("nc-cli-db-import-foreign-view-intel-exported");

    let stdout = run_nc_cli(&[
        "sysop",
        "new-game",
        source.to_str().unwrap(),
        "--players",
        "4",
        "--seed",
        "1515",
    ]);
    assert!(stdout.contains("Initialized new game"));

    setup_classic_probe_players(&source);
    setup_classic_probe_planets(&source);
    setup_classic_probe_view_order(&source);

    let maint_stdout = run_nc_cli(&["maint-rust", source.to_str().unwrap(), "4"]);
    assert!(maint_stdout.contains("Rust maintenance complete."));
    let export_stdout = run_nc_cli(&[
        "db-export",
        source.to_str().unwrap(),
        source.to_str().unwrap(),
    ]);
    assert!(export_stdout.contains("Exported year 3004"));
    assert_database_row_survives_db_import_export(&source, &exported, 4, 0);

    cleanup_dir(&source);
    cleanup_dir(&exported);
}

#[test]
fn db_import_export_preserves_assault_failure_enemy_view_row_shape() {
    let source = unique_temp_dir("nc-cli-db-import-assault-failure-source");
    let exported = unique_temp_dir("nc-cli-db-import-assault-failure-exported");
    common::copy_fixture_dir("fixtures/ecmaint-post/v1.5", &source);

    let mut game_data = load_classic_core_files(&source).expect("fixture should load");
    let target_world = &mut game_data.planets[13];
    *target_world = Planet {
        sector: Coords::new(15, 13),
        potential_production: 100,
        production: Real48::from_int(100),
        stored_goods: Real48::from_int(66),
        name: "TargetPrime".to_string(),
        armies: 40,
        ground_batteries: 0,
        years_under_owner: 0,
        owner: EmpireId(2),
        ..Planet::default()
    };
    let attacker = &mut game_data.fleets[0];
    attacker.sector = Coords::new(15, 13);
    attacker.order = Order::InvadeWorld;
    attacker.target = Coords::new(15, 13);
    attacker.speed = 3;
    attacker.depth = Real48::HALF;
    attacker.rules_of_engagement = 10;
    attacker.ships.scouts = 0;
    attacker.ships.battleships = 0;
    attacker.ships.cruisers = 0;
    attacker.ships.destroyers = 1;
    set_troop_transports(attacker, 2);
    set_armies_aboard(attacker, 2);
    attacker.ships.etacs = 0;
    save_classic_core_files(&source, &game_data).expect("mutated fixture should save");

    let maint_stdout = run_nc_cli(&["maint-rust", source.to_str().unwrap(), "1"]);
    assert!(maint_stdout.contains("Rust maintenance complete."));
    let export_stdout = run_nc_cli(&[
        "db-export",
        source.to_str().unwrap(),
        source.to_str().unwrap(),
    ]);
    assert!(export_stdout.contains("Exported year"));

    assert_database_row_survives_db_import_export(&source, &exported, 13, 0);

    cleanup_dir(&source);
    cleanup_dir(&exported);
}

#[test]
#[ignore = "launches classic ECGAME through dosbox-x"]
fn db_import_export_assault_failure_enemy_view_directory_reopens_in_classic_ecgame_smoke() {
    let source = unique_temp_dir("nc-cli-db-import-assault-failure-ecgame-source");
    let exported = unique_temp_dir("nc-cli-db-import-assault-failure-ecgame-exported");
    common::copy_fixture_dir("fixtures/ecmaint-post/v1.5", &source);

    let mut game_data = load_classic_core_files(&source).expect("fixture should load");
    let target_world = &mut game_data.planets[13];
    *target_world = Planet {
        sector: Coords::new(15, 13),
        potential_production: 100,
        production: Real48::from_int(100),
        stored_goods: Real48::from_int(66),
        name: "TargetPrime".to_string(),
        armies: 40,
        ground_batteries: 0,
        years_under_owner: 0,
        owner: EmpireId(2),
        ..Planet::default()
    };
    let attacker = &mut game_data.fleets[0];
    attacker.sector = Coords::new(15, 13);
    attacker.order = Order::InvadeWorld;
    attacker.target = Coords::new(15, 13);
    attacker.speed = 3;
    attacker.depth = Real48::HALF;
    attacker.rules_of_engagement = 10;
    attacker.ships.scouts = 0;
    attacker.ships.battleships = 0;
    attacker.ships.cruisers = 0;
    attacker.ships.destroyers = 1;
    set_troop_transports(attacker, 2);
    set_armies_aboard(attacker, 2);
    attacker.ships.etacs = 0;
    save_classic_core_files(&source, &game_data).expect("mutated fixture should save");

    let maint_stdout = run_nc_cli(&["maint-rust", source.to_str().unwrap(), "1"]);
    assert!(maint_stdout.contains("Rust maintenance complete."));
    let export_stdout = run_nc_cli(&[
        "db-export",
        source.to_str().unwrap(),
        exported.to_str().unwrap(),
    ]);
    assert!(export_stdout.contains("Exported year"));

    run_classic_ecgame_smoke_with_alias(&exported, 1, "HANNIBAL");

    cleanup_dir(&source);
    cleanup_dir(&exported);
}

#[test]
fn db_import_export_preserves_assault_success_owned_row_shape() {
    let source = unique_temp_dir("nc-cli-db-import-assault-success-source");
    let exported = unique_temp_dir("nc-cli-db-import-assault-success-exported");
    common::copy_fixture_dir("fixtures/ecmaint-post/v1.5", &source);

    let mut game_data = load_classic_core_files(&source).expect("fixture should load");
    let target_world = &mut game_data.planets[13];
    *target_world = Planet {
        sector: Coords::new(15, 13),
        potential_production: 100,
        production: Real48::from_int(100),
        stored_goods: Real48::from_int(66),
        name: "TargetPrime".to_string(),
        armies: 142,
        ground_batteries: 15,
        years_under_owner: 0,
        owner: EmpireId(2),
        ..Planet::default()
    };
    let attacker = &mut game_data.fleets[0];
    attacker.sector = Coords::new(15, 13);
    attacker.order = Order::InvadeWorld;
    attacker.target = Coords::new(15, 13);
    attacker.speed = 3;
    attacker.depth = Real48::HALF;
    attacker.rules_of_engagement = 10;
    attacker.ships.scouts = 0;
    attacker.ships.battleships = 20;
    attacker.ships.cruisers = 20;
    attacker.ships.destroyers = 20;
    set_troop_transports(attacker, 2);
    set_armies_aboard(attacker, 2);
    attacker.ships.etacs = 0;
    save_classic_core_files(&source, &game_data).expect("mutated fixture should save");

    let maint_stdout = run_nc_cli(&["maint-rust", source.to_str().unwrap(), "1"]);
    assert!(maint_stdout.contains("Rust maintenance complete."));
    let export_stdout = run_nc_cli(&[
        "db-export",
        source.to_str().unwrap(),
        source.to_str().unwrap(),
    ]);
    assert!(export_stdout.contains("Exported year"));

    assert_database_row_survives_db_import_export(&source, &exported, 13, 0);

    cleanup_dir(&source);
    cleanup_dir(&exported);
}

#[test]
#[ignore = "launches classic ECGAME through dosbox-x"]
fn db_import_export_assault_success_owned_directory_reopens_in_classic_ecgame_smoke() {
    let source = unique_temp_dir("nc-cli-db-import-assault-success-ecgame-source");
    let exported = unique_temp_dir("nc-cli-db-import-assault-success-ecgame-exported");
    common::copy_fixture_dir("fixtures/ecmaint-post/v1.5", &source);

    let mut game_data = load_classic_core_files(&source).expect("fixture should load");
    let target_world = &mut game_data.planets[13];
    *target_world = Planet {
        sector: Coords::new(15, 13),
        potential_production: 100,
        production: Real48::from_int(100),
        stored_goods: Real48::from_int(66),
        name: "TargetPrime".to_string(),
        armies: 142,
        ground_batteries: 15,
        years_under_owner: 0,
        owner: EmpireId(2),
        ..Planet::default()
    };
    let attacker = &mut game_data.fleets[0];
    attacker.sector = Coords::new(15, 13);
    attacker.order = Order::InvadeWorld;
    attacker.target = Coords::new(15, 13);
    attacker.speed = 3;
    attacker.depth = Real48::HALF;
    attacker.rules_of_engagement = 10;
    attacker.ships.scouts = 0;
    attacker.ships.battleships = 20;
    attacker.ships.cruisers = 20;
    attacker.ships.destroyers = 20;
    set_troop_transports(attacker, 2);
    set_armies_aboard(attacker, 2);
    attacker.ships.etacs = 0;
    save_classic_core_files(&source, &game_data).expect("mutated fixture should save");

    let maint_stdout = run_nc_cli(&["maint-rust", source.to_str().unwrap(), "1"]);
    assert!(maint_stdout.contains("Rust maintenance complete."));
    let export_stdout = run_nc_cli(&[
        "db-export",
        source.to_str().unwrap(),
        exported.to_str().unwrap(),
    ]);
    assert!(export_stdout.contains("Exported year"));

    run_classic_ecgame_smoke_with_alias(&exported, 1, "HANNIBAL");

    cleanup_dir(&source);
    cleanup_dir(&exported);
}
