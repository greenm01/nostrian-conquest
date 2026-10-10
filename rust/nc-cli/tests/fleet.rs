mod common;

use common::{
    cleanup_dir, copy_fixture_dir, export_campaign_db, repo_root, run_nc_cli, run_nc_cli_in_dir,
    unique_temp_dir, without_unread_bytes,
};
use std::fs;

#[test]
fn fleet_order_recreates_known_valid_fleet_pre_fixture() {
    let target = unique_temp_dir("nc-cli-fleet-order");
    copy_fixture_dir("fixtures/ecmaint-post/v1.5", &target);

    let stdout = run_nc_cli_in_dir(
        &[
            "fleet-order",
            target.to_str().unwrap(),
            "1",
            "3",
            "12",
            "15",
            "13",
        ],
        common::rust_workspace(),
    );
    assert!(stdout.contains("Fleet record 1 updated: speed=3 order=0x0c target=(15, 13)"));

    export_campaign_db(&target, &target);
    let expected = repo_root().join("fixtures/ecmaint-fleet-pre/v1.5/FLEETS.DAT");
    let actual = fs::read(target.join("FLEETS.DAT")).unwrap();
    assert_eq!(
        without_unread_bytes("FLEETS.DAT", &actual),
        without_unread_bytes("FLEETS.DAT", &fs::read(expected).unwrap())
    );

    cleanup_dir(&target);
}

#[test]
fn fleet_order_report_prints_known_fixture_fields() {
    let stdout = run_nc_cli(&["fleet-order-report", "fixtures/ecmaint-fleet-pre/v1.5", "1"]);
    assert!(stdout.contains("Fleet Order Report"));
    assert!(stdout.contains("record=1"));
    assert!(stdout.contains("current_speed=3"));
    assert!(stdout.contains("order=0x0c"));
    assert!(stdout.contains("target=[15, 13]"));
}

#[test]
fn fleet_order_init_materializes_parameterized_directory() {
    let target = unique_temp_dir("nc-cli-fleet-order-init-params");

    let stdout = run_nc_cli_in_dir(
        &[
            "fleet-order-init",
            target.to_str().unwrap(),
            "1",
            "3",
            "0x0c",
            "15",
            "13",
        ],
        common::rust_workspace(),
    );
    assert!(stdout.contains("Fleet-order directory initialized at"));

    let validate = run_nc_cli_in_dir(
        &["validate", target.to_str().unwrap(), "fleet-order"],
        common::rust_workspace(),
    );
    assert!(validate.contains("Valid fleet-order scenario"));

    cleanup_dir(&target);
}

#[test]
fn fleet_order_batch_init_materializes_multiple_directories() {
    let target = unique_temp_dir("nc-cli-fleet-order-batch");

    let stdout = run_nc_cli_in_dir(
        &[
            "fleet-order-batch-init",
            target.to_str().unwrap(),
            "1:3:0x0c:15:13",
            "1:2:0x0c:10:9:0x01:0x00",
        ],
        common::rust_workspace(),
    );
    assert!(stdout.contains("Initialized 2 fleet-order directories under"));

    let manifest = fs::read_to_string(target.join("FLEET_ORDERS.txt")).unwrap();
    assert!(manifest.contains("r01-s03-o0c-x15-y13"));
    assert!(manifest.contains("r01-s02-o0c-x10-y09"));

    let validate = run_nc_cli_in_dir(
        &[
            "validate",
            target.join("r01-s03-o0c-x15-y13").to_str().unwrap(),
            "fleet-order",
        ],
        common::rust_workspace(),
    );
    assert!(validate.contains("Valid fleet-order scenario"));

    cleanup_dir(&target);
}

#[test]
fn scenario_fleet_order_recreates_known_valid_fleet_pre_fixture() {
    let target = unique_temp_dir("nc-cli-scenario-fleet-order");
    copy_fixture_dir("fixtures/ecmaint-post/v1.5", &target);

    let stdout = run_nc_cli_in_dir(
        &["scenario", target.to_str().unwrap(), "fleet-order"],
        common::rust_workspace(),
    );
    assert!(stdout.contains("Applied scenario: fleet-order"));

    export_campaign_db(&target, &target);
    let expected = repo_root().join("fixtures/ecmaint-fleet-pre/v1.5/FLEETS.DAT");
    let actual = fs::read(target.join("FLEETS.DAT")).unwrap();
    assert_eq!(
        without_unread_bytes("FLEETS.DAT", &actual),
        without_unread_bytes("FLEETS.DAT", &fs::read(expected).unwrap())
    );

    cleanup_dir(&target);
}

#[test]
fn validate_fleet_order_accepts_known_valid_fixture() {
    let stdout = run_nc_cli(&["validate", "fixtures/ecmaint-fleet-pre/v1.5", "fleet-order"]);
    assert!(stdout.contains("Valid fleet-order scenario"));
    assert!(stdout.contains("FLEET[1].order = 0x0c"));
}

#[test]
fn validate_fleet_order_rejects_post_maint_fixture() {
    let stderr = common::run_nc_cli_failure_in_dir(
        &["validate", "fixtures/ecmaint-post/v1.5", "fleet-order"],
        common::rust_workspace(),
    );
    assert!(stderr.contains("FLEET[1].order expected 0x0c, got 0x05"));
    assert!(stderr.contains("FLEET[1].target expected (15, 13), got [16, 13]"));
}

#[test]
fn compare_preserved_reports_zero_diff_for_known_fleet_fixture() {
    let stdout = run_nc_cli(&[
        "compare-preserved",
        "fixtures/ecmaint-fleet-pre/v1.5",
        "fleet-order",
    ]);
    assert!(stdout.contains("Scenario: fleet-order"));
    assert!(stdout.contains("FLEETS.DAT: size 864 vs 864, differing bytes 0"));
}

#[test]
fn scenario_init_fleet_order_materializes_runnable_directory() {
    let target = unique_temp_dir("nc-cli-fleet-order-init");

    let stdout = run_nc_cli_in_dir(
        &[
            "scenario-init",
            "fixtures/ecmaint-post/v1.5",
            target.to_str().unwrap(),
            "fleet-order",
        ],
        common::rust_workspace(),
    );
    assert!(stdout.contains("Applied scenario: fleet-order"));
    assert!(stdout.contains("Scenario directory initialized at"));

    export_campaign_db(&target, &target);
    let expected_fleets = repo_root().join("fixtures/ecmaint-fleet-pre/v1.5/FLEETS.DAT");
    let expected_planets = repo_root().join("fixtures/ecmaint-post/v1.5/PLANETS.DAT");

    assert_eq!(
        without_unread_bytes("FLEETS.DAT", &fs::read(target.join("FLEETS.DAT")).unwrap()),
        without_unread_bytes("FLEETS.DAT", &fs::read(expected_fleets).unwrap())
    );
    assert_eq!(
        without_unread_bytes(
            "PLANETS.DAT",
            &fs::read(target.join("PLANETS.DAT")).unwrap()
        ),
        without_unread_bytes("PLANETS.DAT", &fs::read(expected_planets).unwrap())
    );

    cleanup_dir(&target);
}

#[test]
fn scenario_init_replayable_fleet_order_matches_exact_preserved_pre_fixture() {
    let target = unique_temp_dir("nc-cli-fleet-order-init-replayable");

    let stdout = run_nc_cli_in_dir(
        &[
            "scenario-init-replayable",
            "fixtures/ecmaint-post/v1.5",
            target.to_str().unwrap(),
            "fleet-order",
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
            .join("fixtures/ecmaint-fleet-pre/v1.5")
            .join(name);
        // The typed model does not carry the bytes no classic program reads.
        assert_eq!(
            without_unread_bytes(name, &fs::read(target.join(name)).unwrap()),
            without_unread_bytes(name, &fs::read(expected).unwrap())
        );
    }

    cleanup_dir(&target);
}
