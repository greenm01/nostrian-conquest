mod common;

use std::fs;

use common::{
    cleanup_dir, copy_fixture_dir, import_campaign_db, run_nc_cli_failure_in_dir,
    run_nc_cli_output_in_dir, unique_temp_dir,
};
use nc_data::CampaignStore;

#[test]
fn submit_turn_check_mode_leaves_runtime_state_unchanged() {
    let target = unique_temp_dir("nc-cli-submit-turn-check");
    copy_fixture_dir("fixtures/ecutil-init/v1.5", &target);
    import_campaign_db(&target);
    let turn_path = target.join("turn.kdl");
    fs::write(
        &turn_path,
        r#"
turn player=1 year=3000
tax rate=42
"#,
    )
    .unwrap();
    let store = CampaignStore::open_default_in_dir(&target).unwrap();
    let before = store.load_latest_runtime_state().unwrap().unwrap();

    let output = run_nc_cli_output_in_dir(
        &[
            "submit-turn",
            "--check",
            "--dir",
            target.to_str().unwrap(),
            "--player",
            "1",
            "--file",
            turn_path.to_str().unwrap(),
        ],
        common::rust_workspace(),
    );

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    let stderr = String::from_utf8(output.stderr).unwrap();

    assert!(stdout.contains("Validated turn submission"));
    assert!(stdout.contains("mode=check-only"));
    assert!(stderr.contains("deprecated"));
    let after = store.load_latest_runtime_state().unwrap().unwrap();
    assert_eq!(after.snapshot_id, before.snapshot_id);
    assert_eq!(after.game_data, before.game_data);

    cleanup_dir(&target);
}

#[test]
fn submit_turn_apply_updates_runtime_state_and_queued_mail() {
    let target = unique_temp_dir("nc-cli-submit-turn-apply");
    copy_fixture_dir("fixtures/ecutil-init/v1.5", &target);
    import_campaign_db(&target);
    let turn_path = target.join("turn.kdl");
    fs::write(
        &turn_path,
        r#"
turn player=1 year=3000
tax rate=37
message to=2 subject="Scout" body="Watch the lane."
"#,
    )
    .unwrap();

    let output = run_nc_cli_output_in_dir(
        &[
            "submit-turn",
            "--dir",
            target.to_str().unwrap(),
            "--player",
            "1",
            "--file",
            turn_path.to_str().unwrap(),
        ],
        common::rust_workspace(),
    );

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    let stderr = String::from_utf8(output.stderr).unwrap();

    assert!(stdout.contains("Applied turn submission"));
    assert!(stderr.contains("nc-game submit-turn"));

    let store = CampaignStore::open_default_in_dir(&target).unwrap();
    let state = store.load_latest_runtime_state().unwrap().unwrap();
    assert_eq!(state.game_data.player.records[0].tax_rate(), 37);
    assert_eq!(state.queued_mail.len(), 1);
    assert_eq!(state.queued_mail[0].sender_empire_id, 1);
    assert_eq!(state.queued_mail[0].recipient_empire_id, 2);
    assert_eq!(state.queued_mail[0].subject, "Scout");
    assert_eq!(state.queued_mail[0].body, "Watch the lane.");

    cleanup_dir(&target);
}

#[test]
fn submit_turn_rejects_cli_and_kdl_player_mismatch() {
    let target = unique_temp_dir("nc-cli-submit-turn-mismatch");
    copy_fixture_dir("fixtures/ecutil-init/v1.5", &target);
    let turn_path = target.join("turn.kdl");
    fs::write(
        &turn_path,
        r#"
turn player=2 year=3000
tax rate=20
"#,
    )
    .unwrap();

    let stderr = run_nc_cli_failure_in_dir(
        &[
            "submit-turn",
            "--check",
            "--dir",
            target.to_str().unwrap(),
            "--player",
            "1",
            "--file",
            turn_path.to_str().unwrap(),
        ],
        common::rust_workspace(),
    );

    assert!(stderr.contains("player mismatch"));

    cleanup_dir(&target);
}
