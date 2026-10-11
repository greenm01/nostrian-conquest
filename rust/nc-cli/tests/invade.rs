mod common;

use common::{
    cleanup_dir, export_campaign_db, repo_root, run_nc_cli_in_dir, unique_temp_dir,
    without_unread_bytes,
};
use nc_data::classic::parse_fleets_dat;
use std::fs;

#[test]
fn scenario_invade_recreates_known_valid_pre_fixture() {
    let target = unique_temp_dir("nc-cli-scenario-invade");
    common::copy_fixture_dir("fixtures/ecmaint-post/v1.5", &target);

    let stdout = run_nc_cli_in_dir(
        &["scenario", target.to_str().unwrap(), "invade"],
        common::rust_workspace(),
    );
    assert!(stdout.contains("Applied scenario: invade"));

    export_campaign_db(&target, &target);
    let fleets = parse_fleets_dat(&fs::read(target.join("FLEETS.DAT")).unwrap()).unwrap();
    assert_eq!(fleets[2].order.to_raw(), 0x07);

    let fixture_pre = repo_root().join("fixtures/ecmaint-invade-pre/v1.5");
    let expected_planets = fs::read(fixture_pre.join("PLANETS.DAT")).unwrap();
    let actual_planets = fs::read(target.join("PLANETS.DAT")).unwrap();
    assert_eq!(
        without_unread_bytes("PLANETS.DAT", &actual_planets),
        without_unread_bytes("PLANETS.DAT", &expected_planets),
        "PLANETS.DAT does not match preserved invade pre-fixture"
    );

    cleanup_dir(&target);
}

#[test]
fn validate_invade_accepts_known_valid_fixture() {
    let stderr = common::run_nc_cli_failure_in_dir(
        &["validate", "fixtures/ecmaint-invade-pre/v1.5", "invade"],
        common::rust_workspace(),
    );
    assert!(stderr.contains("FLEET[3].order expected 0x07 (InvadeWorld), got 0x0a"));
}

#[test]
fn scenario_init_replayable_invade_uses_documented_order_code() {
    let target = unique_temp_dir("nc-cli-invade-init-replayable");

    let stdout = run_nc_cli_in_dir(
        &[
            "scenario-init-replayable",
            "fixtures/ecmaint-post/v1.5",
            target.to_str().unwrap(),
            "invade",
        ],
        common::rust_workspace(),
    );
    assert!(stdout.contains("Replayable scenario directory initialized at"));

    let fleets = parse_fleets_dat(&fs::read(target.join("FLEETS.DAT")).unwrap()).unwrap();
    assert_eq!(fleets[2].order.to_raw(), 0x07);

    let fixture_pre = repo_root().join("fixtures/ecmaint-invade-pre/v1.5");
    for name in ["PLANETS.DAT", "SETUP.DAT"] {
        let expected = fs::read(fixture_pre.join(name)).unwrap();
        let actual = fs::read(target.join(name)).unwrap();
        // The typed model does not carry the bytes no classic program reads.
        assert_eq!(
            without_unread_bytes(name, &actual),
            without_unread_bytes(name, &expected),
            "{name} does not match preserved invade pre-fixture"
        );
    }

    cleanup_dir(&target);
}

#[test]
fn invade_init_uses_documented_order_code() {
    let target = unique_temp_dir("nc-cli-invade-init-param");

    // Recreate the fixture parameters: (15,13) target, SC=100, BB=100, CA=50, DD=50, TT=50, armies=100
    let stdout = run_nc_cli_in_dir(
        &[
            "invade-init",
            "fixtures/ecmaint-post/v1.5",
            target.to_str().unwrap(),
            "15",
            "13",
            "100",
            "100",
            "50",
            "50",
            "50",
            "100",
        ],
        common::rust_workspace(),
    );
    assert!(stdout.contains("Invade directory initialized at"));

    export_campaign_db(&target, &target);
    let fleets = parse_fleets_dat(&fs::read(target.join("FLEETS.DAT")).unwrap()).unwrap();
    assert_eq!(fleets[2].order.to_raw(), 0x07);

    let fixture_pre = repo_root().join("fixtures/ecmaint-invade-pre/v1.5");
    let expected_planets = fs::read(fixture_pre.join("PLANETS.DAT")).unwrap();
    let actual_planets = fs::read(target.join("PLANETS.DAT")).unwrap();
    assert_eq!(
        without_unread_bytes("PLANETS.DAT", &actual_planets),
        without_unread_bytes("PLANETS.DAT", &expected_planets),
        "PLANETS.DAT does not match preserved invade pre-fixture via invade-init"
    );

    cleanup_dir(&target);
}
