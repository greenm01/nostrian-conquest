mod common;

use common::{
    cleanup_dir, copy_fixture_dir, export_campaign_db, repo_root, run_nc_cli, run_nc_cli_in_dir,
    unique_temp_dir,
};
use std::fs;

#[test]
fn guard_starbase_scenario_recreates_known_valid_starbase_pre_fixture() {
    let target = unique_temp_dir("nc-cli-guard-starbase");
    copy_fixture_dir("fixtures/ecmaint-post/v1.5", &target);

    let stdout = run_nc_cli_in_dir(
        &["scenario", target.to_str().unwrap(), "guard-starbase"],
        common::rust_workspace(),
    );
    assert!(stdout.contains("Applied scenario: guard-starbase"));

    export_campaign_db(&target, &target);
    let expected_player = repo_root().join("fixtures/ecmaint-starbase-pre/v1.5/PLAYER.DAT");
    let expected_fleets = repo_root().join("fixtures/ecmaint-starbase-pre/v1.5/FLEETS.DAT");
    let expected_bases = repo_root().join("fixtures/ecmaint-starbase-pre/v1.5/BASES.DAT");

    assert_eq!(
        fs::read(target.join("PLAYER.DAT")).unwrap(),
        fs::read(expected_player).unwrap()
    );
    assert_eq!(
        fs::read(target.join("FLEETS.DAT")).unwrap(),
        fs::read(expected_fleets).unwrap()
    );
    assert_eq!(
        fs::read(target.join("BASES.DAT")).unwrap(),
        fs::read(expected_bases).unwrap()
    );

    cleanup_dir(&target);
}

#[test]
fn guard_starbase_onebase_recreates_known_valid_starbase_pre_fixture() {
    let target = unique_temp_dir("nc-cli-guard-starbase-onebase");
    copy_fixture_dir("fixtures/ecmaint-post/v1.5", &target);

    let stdout = run_nc_cli_in_dir(
        &[
            "guard-starbase-onebase",
            target.to_str().unwrap(),
            "0x10",
            "0x0d",
        ],
        common::rust_workspace(),
    );
    assert!(stdout.contains("PLAYER[1].starbase_count_raw = 1"));
    assert!(stdout.contains("structured single-base record at (16, 13)"));

    export_campaign_db(&target, &target);
    let expected_player = repo_root().join("fixtures/ecmaint-starbase-pre/v1.5/PLAYER.DAT");
    let expected_fleets = repo_root().join("fixtures/ecmaint-starbase-pre/v1.5/FLEETS.DAT");
    let expected_bases = repo_root().join("fixtures/ecmaint-starbase-pre/v1.5/BASES.DAT");

    assert_eq!(
        fs::read(target.join("PLAYER.DAT")).unwrap(),
        fs::read(expected_player).unwrap()
    );
    assert_eq!(
        fs::read(target.join("FLEETS.DAT")).unwrap(),
        fs::read(expected_fleets).unwrap()
    );
    assert_eq!(
        fs::read(target.join("BASES.DAT")).unwrap(),
        fs::read(expected_bases).unwrap()
    );

    cleanup_dir(&target);
}

#[test]
fn guard_starbase_onebase_allows_coordinate_variation() {
    let target = unique_temp_dir("nc-cli-guard-starbase-shifted");
    copy_fixture_dir("fixtures/ecmaint-post/v1.5", &target);

    let stdout = run_nc_cli_in_dir(
        &[
            "guard-starbase-onebase",
            target.to_str().unwrap(),
            "12",
            "9",
        ],
        common::rust_workspace(),
    );
    assert!(stdout.contains("structured single-base record at (12, 9)"));

    let validate = run_nc_cli_in_dir(
        &["validate", target.to_str().unwrap(), "guard-starbase"],
        common::rust_workspace(),
    );
    assert!(validate.contains("Valid guard-starbase scenario"));
    assert!(validate.contains("one-base guard-starbase linkage holds at coords [12, 9]"));

    cleanup_dir(&target);
}

#[test]
fn guard_starbase_report_prints_linkage_fields_for_known_fixture() {
    let stdout = run_nc_cli(&[
        "guard-starbase-report",
        "fixtures/ecmaint-starbase-pre/v1.5",
    ]);
    assert!(stdout.contains("Guard Starbase Report"));
    assert!(stdout.contains("player[1].starbase_count_raw=1"));
    assert!(stdout.contains("fleet[1].local_slot_word_raw=1"));
    assert!(stdout.contains("fleet[1].fleet_id_word_raw=1"));
    assert!(stdout.contains("base_count=1"));
    assert!(stdout.contains("verdict=valid one-base guard-starbase linkage"));
}

#[test]
fn guard_starbase_init_materializes_parameterized_directory() {
    let target = unique_temp_dir("nc-cli-guard-starbase-init-xy");

    let stdout = run_nc_cli_in_dir(
        &["guard-starbase-init", target.to_str().unwrap(), "12", "9"],
        common::rust_workspace(),
    );
    assert!(stdout.contains("Guard Starbase directory initialized at"));
    assert!(stdout.contains("structured single-base record at (12, 9)"));

    let report = run_nc_cli_in_dir(
        &["guard-starbase-report", target.to_str().unwrap()],
        common::rust_workspace(),
    );
    assert!(report.contains("base_count=1"));
    assert!(report.contains("verdict=valid one-base guard-starbase linkage"));
    assert!(report.contains("target=[12, 9]"));

    cleanup_dir(&target);
}

#[test]
fn guard_starbase_batch_init_materializes_multiple_parameterized_directories() {
    let target = unique_temp_dir("nc-cli-guard-starbase-batch");

    let stdout = run_nc_cli_in_dir(
        &[
            "guard-starbase-batch-init",
            target.to_str().unwrap(),
            "12:9",
            "14:7",
        ],
        common::rust_workspace(),
    );
    assert!(stdout.contains("Initialized 2 Guard Starbase directories under"));

    let manifest = fs::read_to_string(target.join("GUARD_STARBASES.txt")).unwrap();
    assert!(manifest.contains("x12-y09"));
    assert!(manifest.contains("x14-y07"));

    let report = run_nc_cli_in_dir(
        &[
            "compliance-report",
            target.join("x12-y09").to_str().unwrap(),
        ],
        common::rust_workspace(),
    );
    assert!(report.contains("OK   guard-starbase-linkage"));
    assert!(report.contains("base1.escort=1 base1.id=1 base1.record=1 coords=[12, 9]"));

    cleanup_dir(&target);
}

#[test]
fn validate_guard_starbase_accepts_known_valid_fixture() {
    let stdout = run_nc_cli(&[
        "validate",
        "fixtures/ecmaint-starbase-pre/v1.5",
        "guard-starbase",
    ]);
    assert!(stdout.contains("Valid guard-starbase scenario"));
    assert!(stdout.contains("linkage keys: player[44]=1 fleet[00]=1 fleet[05]=1 base[07]=1"));
    assert!(stdout.contains("one-base guard-starbase linkage holds"));
}

#[test]
fn validate_preserved_guard_starbase_accepts_known_fixture() {
    let stdout = run_nc_cli(&[
        "validate-preserved",
        "fixtures/ecmaint-starbase-pre/v1.5",
        "guard-starbase",
    ]);
    assert!(stdout.contains("Exact preserved match: guard-starbase"));
    assert!(stdout.contains("fixtures/ecmaint-starbase-pre/v1.5"));
}

#[test]
fn validate_guard_starbase_rejects_post_maint_fixture() {
    let stderr = common::run_nc_cli_failure_in_dir(
        &["validate", "fixtures/ecmaint-post/v1.5", "guard-starbase"],
        common::rust_workspace(),
    );
    assert!(stderr.contains("PLAYER[1].starbase_count_raw expected 1, got 0"));
    assert!(stderr.contains("FLEET[1].order expected 0x04, got 0x05"));
    assert!(stderr.contains("BASES.DAT expected 1 record, got 0"));
}

#[test]
fn compare_preserved_reports_nonzero_diff_for_post_maint_fixture() {
    let stdout = run_nc_cli(&[
        "compare-preserved",
        "fixtures/ecmaint-post/v1.5",
        "guard-starbase",
    ]);
    assert!(stdout.contains("Scenario: guard-starbase"));
    assert!(stdout.contains("PLAYER.DAT: size 440 vs 440, differing bytes"));
    assert!(stdout.contains("FLEETS.DAT: size 864 vs 864, differing bytes"));
    assert!(stdout.contains("BASES.DAT: size 0 vs 35, differing bytes 35"));
}

#[test]
fn scenario_init_guard_starbase_materializes_runnable_directory() {
    let target = unique_temp_dir("nc-cli-guard-starbase-init");

    let stdout = run_nc_cli_in_dir(
        &[
            "scenario-init",
            "fixtures/ecmaint-post/v1.5",
            target.to_str().unwrap(),
            "guard-starbase",
        ],
        common::rust_workspace(),
    );
    assert!(stdout.contains("Applied scenario: guard-starbase"));
    assert!(stdout.contains("Scenario directory initialized at"));

    export_campaign_db(&target, &target);
    let expected_player = repo_root().join("fixtures/ecmaint-starbase-pre/v1.5/PLAYER.DAT");
    let expected_fleets = repo_root().join("fixtures/ecmaint-starbase-pre/v1.5/FLEETS.DAT");
    let expected_bases = repo_root().join("fixtures/ecmaint-starbase-pre/v1.5/BASES.DAT");
    let expected_setup = repo_root().join("fixtures/ecmaint-post/v1.5/SETUP.DAT");

    assert_eq!(
        fs::read(target.join("PLAYER.DAT")).unwrap(),
        fs::read(expected_player).unwrap()
    );
    assert_eq!(
        fs::read(target.join("FLEETS.DAT")).unwrap(),
        fs::read(expected_fleets).unwrap()
    );
    assert_eq!(
        fs::read(target.join("BASES.DAT")).unwrap(),
        fs::read(expected_bases).unwrap()
    );
    assert_eq!(
        fs::read(target.join("SETUP.DAT")).unwrap(),
        fs::read(expected_setup).unwrap()
    );

    cleanup_dir(&target);
}

#[test]
fn scenario_init_guard_starbase_accepts_omitted_source() {
    let target = unique_temp_dir("nc-cli-guard-starbase-default");

    let stdout = run_nc_cli_in_dir(
        &["scenario-init", target.to_str().unwrap(), "guard-starbase"],
        common::rust_workspace(),
    );
    assert!(stdout.contains("Applied scenario: guard-starbase"));
    assert!(stdout.contains("Scenario directory initialized at"));

    let validate = run_nc_cli_in_dir(
        &["validate", target.to_str().unwrap(), "guard-starbase"],
        common::rust_workspace(),
    );
    assert!(validate.contains("Valid guard-starbase scenario"));

    cleanup_dir(&target);
}

#[test]
fn scenario_init_replayable_guard_starbase_matches_exact_preserved_pre_fixture() {
    let target = unique_temp_dir("nc-cli-guard-starbase-init-replayable");

    let stdout = run_nc_cli_in_dir(
        &[
            "scenario-init-replayable",
            "fixtures/ecmaint-post/v1.5",
            target.to_str().unwrap(),
            "guard-starbase",
        ],
        common::rust_workspace(),
    );
    assert!(stdout.contains("Replayable scenario directory initialized at"));

    for name in [
        "PLAYER.DAT",
        "PLANETS.DAT",
        "FLEETS.DAT",
        "BASES.DAT",
        "SETUP.DAT",
        "CONQUEST.DAT",
        "DATABASE.DAT",
    ] {
        let expected = repo_root()
            .join("fixtures/ecmaint-starbase-pre/v1.5")
            .join(name);
        assert_eq!(
            fs::read(target.join(name)).unwrap(),
            fs::read(expected).unwrap()
        );
    }

    cleanup_dir(&target);
}
