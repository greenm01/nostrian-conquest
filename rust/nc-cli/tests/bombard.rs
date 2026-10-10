mod common;

use common::{
    cleanup_dir, export_campaign_db, repo_root, run_nc_cli, run_nc_cli_in_dir, unique_temp_dir,
    without_unread_bytes,
};
use std::fs;

#[test]
fn scenario_bombard_recreates_known_valid_bombard_pre_fixture() {
    let target = unique_temp_dir("nc-cli-scenario-bombard");
    common::copy_fixture_dir("fixtures/ecmaint-post/v1.5", &target);

    let stdout = run_nc_cli_in_dir(
        &["scenario", target.to_str().unwrap(), "bombard"],
        common::rust_workspace(),
    );
    assert!(stdout.contains("Applied scenario: bombard"));

    export_campaign_db(&target, &target);
    let fixture_pre = repo_root().join("fixtures/ecmaint-bombard-pre/v1.5");
    for name in ["FLEETS.DAT", "PLANETS.DAT"] {
        let expected = fs::read(fixture_pre.join(name)).unwrap();
        let actual = fs::read(target.join(name)).unwrap();
        // The typed model does not carry the bytes no classic program reads.
        assert_eq!(
            without_unread_bytes(name, &actual),
            without_unread_bytes(name, &expected),
            "{name} does not match preserved bombard pre-fixture"
        );
    }

    cleanup_dir(&target);
}

#[test]
fn validate_bombard_accepts_known_valid_fixture() {
    let stdout = run_nc_cli(&["validate", "fixtures/ecmaint-bombard-pre/v1.5", "bombard"]);
    assert!(stdout.contains("Valid bombard scenario"));
    assert!(stdout.contains("FLEET[3].order = 0x06 (BombardWorld)"));
    assert!(stdout.contains("PLANET[14]: homeworld seed empire=2 armies=10 batteries=4"));
}

#[test]
fn scenario_init_replayable_bombard_matches_exact_preserved_pre_fixture() {
    let target = unique_temp_dir("nc-cli-bombard-init-replayable");

    let stdout = run_nc_cli_in_dir(
        &[
            "scenario-init-replayable",
            "fixtures/ecmaint-post/v1.5",
            target.to_str().unwrap(),
            "bombard",
        ],
        common::rust_workspace(),
    );
    assert!(stdout.contains("Replayable scenario directory initialized at"));

    let fixture_pre = repo_root().join("fixtures/ecmaint-bombard-pre/v1.5");
    for name in [
        "PLAYER.DAT",
        "PLANETS.DAT",
        "FLEETS.DAT",
        "SETUP.DAT",
        "CONQUEST.DAT",
        "DATABASE.DAT",
    ] {
        let expected = fs::read(fixture_pre.join(name)).unwrap();
        let actual = fs::read(target.join(name)).unwrap();
        // The typed model does not carry the bytes no classic program reads.
        assert_eq!(
            without_unread_bytes(name, &actual),
            without_unread_bytes(name, &expected),
            "{name} does not match preserved bombard pre-fixture"
        );
    }

    cleanup_dir(&target);
}

#[test]
fn bombard_onefleet_allows_coordinate_variation() {
    let target = unique_temp_dir("nc-cli-bombard-onefleet-varied");
    common::copy_fixture_dir("fixtures/ecmaint-post/v1.5", &target);

    let stdout = run_nc_cli_in_dir(
        &[
            "bombard-onefleet",
            target.to_str().unwrap(),
            "10",
            "8",
            "4",
            "6",
        ],
        common::rust_workspace(),
    );
    assert!(stdout.contains("FLEET[3].order = 0x06 (BombardWorld), target = (10, 8), CA=4, DD=6"));
    assert!(stdout.contains("PLANET[14]: homeworld clone empire=2, coords=(10, 8)"));

    cleanup_dir(&target);
}

#[test]
fn bombard_init_materializes_parameterized_directory() {
    let target = unique_temp_dir("nc-cli-bombard-init");

    let stdout = run_nc_cli_in_dir(
        &[
            "bombard-init",
            target.to_str().unwrap(),
            "12",
            "9",
            "3",
            "5",
        ],
        common::rust_workspace(),
    );
    assert!(stdout.contains("Bombard directory initialized at"));
    assert!(stdout.contains("FLEET[3].order = 0x06 (BombardWorld), target = (12, 9), CA=3, DD=5"));
    assert!(stdout.contains("PLANET[14]: homeworld clone empire=2, coords=(12, 9)"));

    cleanup_dir(&target);
}

#[test]
fn bombard_init_with_fixture_coords_matches_exact_preserved_pre_fixture() {
    let target = unique_temp_dir("nc-cli-bombard-init-exact");

    let stdout = run_nc_cli_in_dir(
        &[
            "bombard-init",
            "fixtures/ecmaint-post/v1.5",
            target.to_str().unwrap(),
            "15",
            "13",
            "3",
            "5",
        ],
        common::rust_workspace(),
    );
    assert!(stdout.contains("Bombard directory initialized at"));

    export_campaign_db(&target, &target);
    let fixture_pre = repo_root().join("fixtures/ecmaint-bombard-pre/v1.5");
    for name in ["FLEETS.DAT", "PLANETS.DAT"] {
        let expected = fs::read(fixture_pre.join(name)).unwrap();
        let actual = fs::read(target.join(name)).unwrap();
        // The typed model does not carry the bytes no classic program reads.
        assert_eq!(
            without_unread_bytes(name, &actual),
            without_unread_bytes(name, &expected),
            "{name} does not match preserved bombard pre-fixture when initialized via bombard-init"
        );
    }

    cleanup_dir(&target);
}

#[test]
fn bombard_batch_init_materializes_multiple_parameterized_directories() {
    let target = unique_temp_dir("nc-cli-bombard-batch");

    let stdout = run_nc_cli_in_dir(
        &[
            "bombard-batch-init",
            target.to_str().unwrap(),
            "12:9:3:5",
            "14:7:2:4",
        ],
        common::rust_workspace(),
    );
    assert!(stdout.contains("Initialized 2 Bombard directories under"));

    let manifest = fs::read_to_string(target.join("BOMBARD_BATCH.txt")).unwrap();
    assert!(manifest.contains("x12-y09-ca3-dd5"));
    assert!(manifest.contains("x14-y07-ca2-dd4"));
    assert!(manifest.contains("target=[12, 9]"));
    assert!(manifest.contains("target=[14, 7]"));

    cleanup_dir(&target);
}
