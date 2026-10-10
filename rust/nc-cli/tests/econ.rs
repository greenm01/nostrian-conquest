mod common;

use common::{
    cleanup_dir, export_campaign_db, repo_root, run_nc_cli, run_nc_cli_in_dir, unique_temp_dir,
    without_unread_bytes,
};
use std::fs;

#[test]
fn scenario_econ_recreates_known_valid_pre_fixture() {
    let target = unique_temp_dir("nc-cli-scenario-econ");
    common::copy_fixture_dir("fixtures/ecmaint-post/v1.5", &target);

    let stdout = run_nc_cli_in_dir(
        &["scenario", target.to_str().unwrap(), "econ"],
        common::rust_workspace(),
    );
    assert!(stdout.contains("Applied scenario: econ"));

    export_campaign_db(&target, &target);
    let fixture_pre = repo_root().join("fixtures/ecmaint-econ-pre/v1.5");
    for name in ["FLEETS.DAT", "PLANETS.DAT"] {
        let expected = fs::read(fixture_pre.join(name)).unwrap();
        let actual = fs::read(target.join(name)).unwrap();
        // The typed model does not carry the bytes no classic program reads.
        assert_eq!(
            without_unread_bytes(name, &actual),
            without_unread_bytes(name, &expected),
            "{name} does not match preserved econ pre-fixture"
        );
    }

    cleanup_dir(&target);
}

#[test]
fn validate_econ_accepts_known_valid_fixture() {
    let stdout = run_nc_cli(&["validate", "fixtures/ecmaint-econ-pre/v1.5", "econ"]);
    assert!(stdout.contains("Valid econ scenario"));
    assert!(stdout.contains("FLEET[3]: order=0x06 (BombardWorld)"));
    assert!(stdout.contains("PLANET[14]: (15,13) empire=2 armies=142 batteries=15"));
}

#[test]
fn scenario_init_replayable_econ_matches_exact_preserved_pre_fixture() {
    let target = unique_temp_dir("nc-cli-econ-init-replayable");

    let stdout = run_nc_cli_in_dir(
        &[
            "scenario-init-replayable",
            "fixtures/ecmaint-post/v1.5",
            target.to_str().unwrap(),
            "econ",
        ],
        common::rust_workspace(),
    );
    assert!(stdout.contains("Replayable scenario directory initialized at"));

    let fixture_pre = repo_root().join("fixtures/ecmaint-econ-pre/v1.5");
    for name in ["FLEETS.DAT", "PLANETS.DAT", "SETUP.DAT"] {
        let expected = fs::read(fixture_pre.join(name)).unwrap();
        let actual = fs::read(target.join(name)).unwrap();
        // The typed model does not carry the bytes no classic program reads.
        assert_eq!(
            without_unread_bytes(name, &actual),
            without_unread_bytes(name, &expected),
            "{name} does not match preserved econ pre-fixture"
        );
    }

    cleanup_dir(&target);
}

#[test]
fn econ_init_recreates_known_valid_pre_fixture() {
    let target = unique_temp_dir("nc-cli-econ-init-param");

    // Recreate the fixture parameters: target=(15,13), BB=0, CA=50, DD=50, planet14=(15,13), armies=142, batteries=15
    let stdout = run_nc_cli_in_dir(
        &[
            "econ-init",
            "fixtures/ecmaint-post/v1.5",
            target.to_str().unwrap(),
            "15",
            "13", // target_x, target_y
            "0",
            "50",
            "50", // bb, ca, dd
            "15",
            "13",
            "142",
            "15", // p14_x, p14_y, p14_armies, p14_batteries
        ],
        common::rust_workspace(),
    );
    assert!(stdout.contains("Econ directory initialized at"));

    export_campaign_db(&target, &target);
    let fixture_pre = repo_root().join("fixtures/ecmaint-econ-pre/v1.5");
    for name in ["FLEETS.DAT", "PLANETS.DAT"] {
        let expected = fs::read(fixture_pre.join(name)).unwrap();
        let actual = fs::read(target.join(name)).unwrap();
        // The typed model does not carry the bytes no classic program reads.
        assert_eq!(
            without_unread_bytes(name, &actual),
            without_unread_bytes(name, &expected),
            "{name} does not match preserved econ pre-fixture via econ-init"
        );
    }

    cleanup_dir(&target);
}
