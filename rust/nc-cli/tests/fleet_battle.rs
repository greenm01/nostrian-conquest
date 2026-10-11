mod common;

use common::{
    cleanup_dir, export_campaign_db, repo_root, run_nc_cli, run_nc_cli_in_dir, unique_temp_dir,
    without_unread_bytes,
};
use std::fs;

#[test]
fn scenario_fleet_battle_recreates_known_valid_pre_fixture() {
    let target = unique_temp_dir("nc-cli-scenario-fleet-battle");
    common::copy_fixture_dir("fixtures/ecmaint-post/v1.5", &target);

    let stdout = run_nc_cli_in_dir(
        &["scenario", target.to_str().unwrap(), "fleet-battle"],
        common::rust_workspace(),
    );
    assert!(stdout.contains("Applied scenario: fleet-battle"));

    export_campaign_db(&target, &target);
    let fixture_pre = repo_root().join("fixtures/ecmaint-fleet-battle-pre/v1.5");
    for name in ["FLEETS.DAT", "PLANETS.DAT"] {
        let expected = fs::read(fixture_pre.join(name)).unwrap();
        let actual = fs::read(target.join(name)).unwrap();
        // The typed model does not carry the bytes no classic program reads.
        assert_eq!(
            without_unread_bytes(name, &actual),
            without_unread_bytes(name, &expected),
            "{name} does not match preserved fleet-battle pre-fixture"
        );
    }

    cleanup_dir(&target);
}

#[test]
fn validate_fleet_battle_accepts_known_valid_fixture() {
    let stdout = run_nc_cli(&[
        "validate",
        "fixtures/ecmaint-fleet-battle-pre/v1.5",
        "fleet-battle",
    ]);
    assert!(stdout.contains("Valid fleet-battle scenario"));
    assert!(stdout.contains("FLEET[1]: loc=(10,10) tgt=(10,10) RoE=100 BB50 CA50 DD50"));
    assert!(stdout.contains("FLEET[5]: order=0x03 (PatrolSector) loc=(10,10) speed=0/6 BB100"));
    assert!(
        stdout.contains("FLEET[9]: order=0x01 (MoveOnly) loc=(9,10) tgt=(10,10) speed=3/6 BB1")
    );
    assert!(stdout.contains("PLANET[14]: (15,13) empire=2 armies=142 batteries=15"));
}

#[test]
fn scenario_init_replayable_fleet_battle_matches_exact_preserved_pre_fixture() {
    let target = unique_temp_dir("nc-cli-fleet-battle-init-replayable");

    let stdout = run_nc_cli_in_dir(
        &[
            "scenario-init-replayable",
            "fixtures/ecmaint-post/v1.5",
            target.to_str().unwrap(),
            "fleet-battle",
        ],
        common::rust_workspace(),
    );
    assert!(stdout.contains("Replayable scenario directory initialized at"));

    let fixture_pre = repo_root().join("fixtures/ecmaint-fleet-battle-pre/v1.5");
    for name in ["FLEETS.DAT", "PLANETS.DAT", "SETUP.DAT"] {
        let expected = fs::read(fixture_pre.join(name)).unwrap();
        let actual = fs::read(target.join(name)).unwrap();
        // The typed model does not carry the bytes no classic program reads.
        assert_eq!(
            without_unread_bytes(name, &actual),
            without_unread_bytes(name, &expected),
            "{name} does not match preserved fleet-battle pre-fixture"
        );
    }

    cleanup_dir(&target);
}

#[test]
fn fleet_battle_init_recreates_known_valid_pre_fixture() {
    let target = unique_temp_dir("nc-cli-fleet-battle-init-param");

    // Recreate the fixture parameters: battle=(10,10), plus all fleet and planet specs
    let stdout = run_nc_cli_in_dir(
        &[
            "fleet-battle-init",
            "fixtures/ecmaint-post/v1.5",
            target.to_str().unwrap(),
            "10",
            "10",  // battle_x, battle_y
            "100", // f0_roe
            "50",
            "50",
            "50", // f0_bb, f0_ca, f0_dd
            "50",
            "50", // f2_ca, f2_dd
            "0",
            "100",
            "0", // f4_sc, f4_bb, f4_ca
            "9",
            "10",
            "0",
            "1",
            "0", // f8_loc_x, f8_loc_y, f8_sc, f8_bb, f8_ca
            "15",
            "13",
            "142",
            "15", // p14_x, p14_y, p14_armies, p14_batteries
        ],
        common::rust_workspace(),
    );
    assert!(stdout.contains("Fleet-battle directory initialized at"));

    export_campaign_db(&target, &target);
    let fixture_pre = repo_root().join("fixtures/ecmaint-fleet-battle-pre/v1.5");
    for name in ["FLEETS.DAT", "PLANETS.DAT"] {
        let expected = fs::read(fixture_pre.join(name)).unwrap();
        let actual = fs::read(target.join(name)).unwrap();
        // The typed model does not carry the bytes no classic program reads.
        assert_eq!(
            without_unread_bytes(name, &actual),
            without_unread_bytes(name, &expected),
            "{name} does not match preserved fleet-battle pre-fixture via fleet-battle-init"
        );
    }

    cleanup_dir(&target);
}
