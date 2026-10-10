mod common;

use common::{
    cleanup_dir, copy_fixture_dir, export_campaign_db, repo_root, run_nc_cli, run_nc_cli_in_dir,
    unique_temp_dir, without_unread_bytes,
};
use nc_compat::load_classic_core_files;
use std::fs;

#[test]
fn planet_build_recreates_known_valid_build_pre_fixture() {
    let target = unique_temp_dir("nc-cli-planet-build");
    copy_fixture_dir("fixtures/ecmaint-post/v1.5", &target);

    let stdout = run_nc_cli_in_dir(
        &[
            "planet-build",
            target.to_str().unwrap(),
            "15",
            "0x03",
            "0x01",
        ],
        common::rust_workspace(),
    );
    assert!(stdout.contains("Planet record 15 updated: build_slot=0x03 build_kind=0x01"));

    export_campaign_db(&target, &target);
    let expected = repo_root().join("fixtures/ecmaint-build-pre/v1.5/PLANETS.DAT");
    let actual = fs::read(target.join("PLANETS.DAT")).unwrap();
    assert_eq!(
        without_unread_bytes("PLANETS.DAT", &actual),
        without_unread_bytes("PLANETS.DAT", &fs::read(expected).unwrap())
    );

    cleanup_dir(&target);
}

#[test]
fn planet_build_report_prints_known_fixture_fields() {
    let stdout = run_nc_cli(&[
        "planet-build-report",
        "fixtures/ecmaint-build-pre/v1.5",
        "15",
    ]);
    assert!(stdout.contains("Planet Build Report"));
    assert!(stdout.contains("record=15"));
    assert!(stdout.contains("build_slot=0x03"));
    assert!(stdout.contains("build_kind=0x01"));
    assert!(stdout.contains("stardock_count=0x00"));
    assert!(stdout.contains("stardock_kind=0x00"));
}

#[test]
fn planet_build_init_materializes_parameterized_directory() {
    let target = unique_temp_dir("nc-cli-planet-build-init-params");

    let stdout = run_nc_cli_in_dir(
        &[
            "planet-build-init",
            target.to_str().unwrap(),
            "15",
            "0x03",
            "0x01",
        ],
        common::rust_workspace(),
    );
    assert!(stdout.contains("Planet-build directory initialized at"));

    let validate = run_nc_cli_in_dir(
        &["validate", target.to_str().unwrap(), "planet-build"],
        common::rust_workspace(),
    );
    assert!(validate.contains("Valid planet-build scenario"));

    cleanup_dir(&target);
}

#[test]
fn planet_build_batch_init_materializes_multiple_directories() {
    let target = unique_temp_dir("nc-cli-planet-build-batch");

    let stdout = run_nc_cli_in_dir(
        &[
            "planet-build-batch-init",
            target.to_str().unwrap(),
            "15:0x03:0x01",
            "12:0x02:0x04",
        ],
        common::rust_workspace(),
    );
    assert!(stdout.contains("Initialized 2 planet-build directories under"));

    let manifest = fs::read_to_string(target.join("PLANET_BUILDS.txt")).unwrap();
    assert!(manifest.contains("p15-s03-k01"));
    assert!(manifest.contains("p12-s02-k04"));

    let validate = run_nc_cli_in_dir(
        &[
            "validate",
            target.join("p15-s03-k01").to_str().unwrap(),
            "planet-build",
        ],
        common::rust_workspace(),
    );
    assert!(validate.contains("Valid planet-build scenario"));

    cleanup_dir(&target);
}

#[test]
fn scenario_planet_build_recreates_known_valid_build_pre_fixture() {
    let target = unique_temp_dir("nc-cli-scenario-planet-build");
    copy_fixture_dir("fixtures/ecmaint-post/v1.5", &target);

    let stdout = run_nc_cli_in_dir(
        &["scenario", target.to_str().unwrap(), "planet-build"],
        common::rust_workspace(),
    );
    assert!(stdout.contains("Applied scenario: planet-build"));

    export_campaign_db(&target, &target);
    let expected = repo_root().join("fixtures/ecmaint-build-pre/v1.5/PLANETS.DAT");
    let actual = fs::read(target.join("PLANETS.DAT")).unwrap();
    assert_eq!(
        without_unread_bytes("PLANETS.DAT", &actual),
        without_unread_bytes("PLANETS.DAT", &fs::read(expected).unwrap())
    );

    cleanup_dir(&target);
}

#[test]
fn validate_planet_build_accepts_known_valid_fixture() {
    let stdout = run_nc_cli(&[
        "validate",
        "fixtures/ecmaint-build-pre/v1.5",
        "planet-build",
    ]);
    assert!(stdout.contains("Valid planet-build scenario"));
    assert!(stdout.contains("PLANET[15].build_kind = 0x01"));
}

#[test]
fn validate_planet_build_rejects_post_maint_fixture() {
    let stderr = common::run_nc_cli_failure_in_dir(
        &["validate", "fixtures/ecmaint-post/v1.5", "planet-build"],
        common::rust_workspace(),
    );
    assert!(stderr.contains("PLANET[15].build_slot expected 0x03, got 0x00"));
    assert!(stderr.contains("PLANET[15].build_kind expected 0x01, got 0x00"));
}

#[test]
fn scenario_init_planet_build_materializes_runnable_directory() {
    let target = unique_temp_dir("nc-cli-planet-build-init");

    let stdout = run_nc_cli_in_dir(
        &[
            "scenario-init",
            "fixtures/ecmaint-post/v1.5",
            target.to_str().unwrap(),
            "planet-build",
        ],
        common::rust_workspace(),
    );
    assert!(stdout.contains("Applied scenario: planet-build"));
    assert!(stdout.contains("Scenario directory initialized at"));

    export_campaign_db(&target, &target);
    let expected_planets = repo_root().join("fixtures/ecmaint-build-pre/v1.5/PLANETS.DAT");
    let expected_fleets = repo_root().join("fixtures/ecmaint-post/v1.5/FLEETS.DAT");

    assert_eq!(
        without_unread_bytes(
            "PLANETS.DAT",
            &fs::read(target.join("PLANETS.DAT")).unwrap()
        ),
        without_unread_bytes("PLANETS.DAT", &fs::read(expected_planets).unwrap())
    );
    assert_eq!(
        without_unread_bytes("FLEETS.DAT", &fs::read(target.join("FLEETS.DAT")).unwrap()),
        without_unread_bytes("FLEETS.DAT", &fs::read(expected_fleets).unwrap())
    );

    cleanup_dir(&target);
}

#[test]
fn scenario_init_replayable_planet_build_matches_exact_preserved_pre_fixture() {
    let target = unique_temp_dir("nc-cli-planet-build-init-replayable");

    let stdout = run_nc_cli_in_dir(
        &[
            "scenario-init-replayable",
            "fixtures/ecmaint-post/v1.5",
            target.to_str().unwrap(),
            "planet-build",
        ],
        common::rust_workspace(),
    );
    assert!(stdout.contains("Replayable scenario directory initialized at"));

    for name in [
        "PLAYER.DAT",
        "PLANETS.DAT",
        "FLEETS.DAT",
        "SETUP.DAT",
        "CONQUEST.DAT",
        "DATABASE.DAT",
    ] {
        let expected = repo_root()
            .join("fixtures/ecmaint-build-pre/v1.5")
            .join(name);
        // The typed model does not carry the bytes no classic program reads.
        assert_eq!(
            without_unread_bytes(name, &fs::read(target.join(name)).unwrap()),
            without_unread_bytes(name, &fs::read(expected).unwrap())
        );
    }

    cleanup_dir(&target);
}

#[test]
fn planet_init_original_updates_runtime_store_for_db_export() {
    let source = unique_temp_dir("nc-cli-planet-init-original-source");
    let exported = unique_temp_dir("nc-cli-planet-init-original-exported");
    copy_fixture_dir("fixtures/ecmaint-post/v1.5", &source);

    let stdout = run_nc_cli_in_dir(
        &["planet-init-original", source.to_str().unwrap()],
        common::rust_workspace(),
    );
    assert!(stdout.contains("Planet topology initialized to original sample state"));

    run_nc_cli_in_dir(
        &[
            "db-export",
            source.to_str().unwrap(),
            exported.to_str().unwrap(),
        ],
        common::rust_workspace(),
    );

    assert_ne!(
        fs::read(source.join("PLANETS.DAT")).unwrap(),
        fs::read(exported.join("PLANETS.DAT")).unwrap()
    );
    let exported_data =
        load_classic_core_files(&exported).expect("exported original-sample topology should load");
    assert_eq!(exported_data.planets.records[0].coords_raw(), [0x06, 0x01]);
    assert_eq!(exported_data.planets.records[0].owner_empire_slot_raw(), 0);
    assert_eq!(exported_data.planets.records[4].coords_raw(), [0x07, 0x04]);
    assert_eq!(exported_data.planets.records[4].owner_empire_slot_raw(), 4);
    assert_eq!(
        exported_data.planets.records[4].planet_name(),
        "Not Named Yet"
    );
    assert_eq!(exported_data.planets.records[15].coords_raw(), [0x10, 0x0D]);
    assert_eq!(exported_data.planets.records[15].owner_empire_slot_raw(), 1);
    assert_eq!(exported_data.planets.records[15].planet_name(), "Dust Bowl");

    cleanup_dir(&source);
    cleanup_dir(&exported);
}
