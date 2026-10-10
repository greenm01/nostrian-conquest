mod common;

use common::{
    cleanup_dir, copy_fixture_dir, export_campaign_db, run_nc_cli, run_nc_cli_in_dir,
    unique_temp_dir,
};
use std::fs;

#[test]
fn ipbm_report_prints_known_empty_post_fixture_state() {
    let stdout = run_nc_cli(&["ipbm-report", "fixtures/ecmaint-post/v1.5"]);
    assert!(stdout.contains("IPBM Report"));
    assert!(stdout.contains("player[1].ipbm_count_raw=0"));
    assert!(stdout.contains("file_record_count=0"));
    assert!(stdout.contains("expected_size_from_player1=0"));
}

#[test]
fn ipbm_zero_sets_player_count_and_file_size() {
    let target = unique_temp_dir("nc-cli-ipbm-zero");
    copy_fixture_dir("fixtures/ecmaint-post/v1.5", &target);

    let stdout = run_nc_cli_in_dir(
        &["ipbm-zero", target.to_str().unwrap(), "3"],
        common::rust_workspace(),
    );
    assert!(stdout.contains("player[1].ipbm_count_raw = 3"));
    assert!(stdout.contains("IPBM.DAT size = 96"));

    let report = run_nc_cli_in_dir(
        &["ipbm-report", target.to_str().unwrap()],
        common::rust_workspace(),
    );
    assert!(report.contains("player[1].ipbm_count_raw=3"));
    assert!(report.contains("file_record_count=3"));
    assert!(report.contains("expected_size_from_player1=96"));

    cleanup_dir(&target);
}

#[test]
fn ipbm_record_set_updates_known_structural_prefix_fields() {
    let target = unique_temp_dir("nc-cli-ipbm-record-set");
    copy_fixture_dir("fixtures/ecmaint-post/v1.5", &target);

    run_nc_cli_in_dir(
        &["ipbm-zero", target.to_str().unwrap(), "1"],
        common::rust_workspace(),
    );
    let stdout = run_nc_cli_in_dir(
        &[
            "ipbm-record-set",
            target.to_str().unwrap(),
            "1",
            "0x1234",
            "2",
            "0x4567",
            "0x89ab",
        ],
        common::rust_workspace(),
    );
    assert!(stdout.contains("IPBM record 1 updated"));
    assert!(stdout.contains("primary=0x1234 owner=2 gate=0x4567 follow_on=0x89ab"));

    let report = run_nc_cli_in_dir(
        &["ipbm-report", target.to_str().unwrap()],
        common::rust_workspace(),
    );
    assert!(report.contains("record 1: primary=0x1234 owner=2 gate=0x4567 follow_on=0x89ab"));
    assert!(report.contains("x_fraction=[00, 00, 00, 00, 00, 00]"));

    cleanup_dir(&target);
}

#[test]
fn ipbm_validate_accepts_known_empty_post_fixture_state() {
    let stdout = run_nc_cli(&["ipbm-validate", "fixtures/ecmaint-post/v1.5"]);
    assert!(stdout.contains("Valid IPBM count/length state"));
    assert!(stdout.contains("player[1].ipbm_count_raw = 0"));
}

#[test]
fn ipbm_validate_rejects_count_length_mismatch() {
    let target = unique_temp_dir("nc-cli-ipbm-invalid");
    copy_fixture_dir("fixtures/ecmaint-post/v1.5", &target);

    run_nc_cli_in_dir(
        &["ipbm-zero", target.to_str().unwrap(), "2"],
        common::rust_workspace(),
    );
    export_campaign_db(&target, &target);
    fs::write(target.join("IPBM.DAT"), vec![0u8; 32]).unwrap();

    let stderr = common::run_nc_cli_failure_in_dir(
        &["ipbm-validate", target.to_str().unwrap()],
        common::rust_workspace(),
    );
    assert!(stderr.contains("IPBM record count expected 2, got 1"));

    cleanup_dir(&target);
}

#[test]
fn ipbm_init_materializes_valid_zero_filled_directory() {
    let target = unique_temp_dir("nc-cli-ipbm-init");

    let stdout = run_nc_cli_in_dir(
        &["ipbm-init", target.to_str().unwrap(), "2"],
        common::rust_workspace(),
    );
    assert!(stdout.contains("IPBM directory initialized at"));
    assert!(stdout.contains("player[1].ipbm_count_raw = 2"));

    let validate = run_nc_cli_in_dir(
        &["ipbm-validate", target.to_str().unwrap()],
        common::rust_workspace(),
    );
    assert!(validate.contains("Valid IPBM count/length state"));
    assert!(validate.contains("record_count = 2"));

    cleanup_dir(&target);
}

#[test]
fn ipbm_batch_init_materializes_multiple_valid_directories() {
    let target = unique_temp_dir("nc-cli-ipbm-batch");

    let stdout = run_nc_cli_in_dir(
        &["ipbm-batch-init", target.to_str().unwrap(), "0", "2", "4"],
        common::rust_workspace(),
    );
    assert!(stdout.contains("Initialized 3 IPBM directories under"));

    let manifest = fs::read_to_string(target.join("IPBM_BATCH.txt")).unwrap();
    assert!(manifest.contains("count-00"));
    assert!(manifest.contains("count-02"));
    assert!(manifest.contains("count-04"));

    let validate = run_nc_cli_in_dir(
        &["ipbm-validate", target.join("count-02").to_str().unwrap()],
        common::rust_workspace(),
    );
    assert!(validate.contains("Valid IPBM count/length state"));
    assert!(validate.contains("record_count = 2"));

    cleanup_dir(&target);
}
