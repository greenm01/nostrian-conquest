mod common;

use std::fs;

use common::{
    cleanup_dir, copy_fixture_dir, import_campaign_db, run_nc_cli_failure_in_dir,
    run_nc_cli_in_dir, run_nc_cli_output_in_dir, unique_temp_dir,
};
use nc_data::{
    CampaignStore, InboxItemType, has_visible_runtime_messages, runtime_inbox_items,
    runtime_outgoing_inbox_items,
};

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
fn submitted_message_reaches_its_recipient_only_after_maintenance() {
    let target = unique_temp_dir("nc-cli-submit-turn-mail-delivery");
    copy_fixture_dir("fixtures/ecutil-init/v1.5", &target);
    import_campaign_db(&target);
    let turn_path = target.join("turn.kdl");
    fs::write(
        &turn_path,
        r#"
turn player=1 year=3000
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

    // The game's first year: the sender lists the message, the recipient
    // has nothing yet.
    let queued = CampaignStore::open_default_in_dir(&target)
        .unwrap()
        .load_latest_runtime_state()
        .unwrap()
        .unwrap();
    let year = queued.game_data.conquest.game_year();
    assert_eq!(year, 3000);
    assert_eq!(
        runtime_outgoing_inbox_items(&queued.game_data, 1, &queued.queued_mail).len(),
        1
    );
    assert!(!has_visible_runtime_messages(2, &queued.queued_mail, year));
    assert!(runtime_inbox_items(&queued.game_data, 2, &[], &queued.queued_mail).is_empty());

    let stdout = run_nc_cli_in_dir(
        &["maint-rust", target.to_str().unwrap(), "1"],
        common::rust_workspace(),
    );
    assert!(stdout.contains("Rust maintenance complete."));

    // Maintenance advanced the year and kept the queue, which delivers it.
    let delivered = CampaignStore::open_default_in_dir(&target)
        .unwrap()
        .load_latest_runtime_state()
        .unwrap()
        .unwrap();
    assert_eq!(delivered.game_data.conquest.game_year(), year + 1);
    assert_eq!(delivered.queued_mail, queued.queued_mail);
    assert!(
        runtime_outgoing_inbox_items(&delivered.game_data, 1, &delivered.queued_mail).is_empty()
    );
    assert!(has_visible_runtime_messages(
        2,
        &delivered.queued_mail,
        year + 1
    ));
    let messages = runtime_inbox_items(
        &delivered.game_data,
        2,
        &delivered.report_block_rows,
        &delivered.queued_mail,
    )
    .into_iter()
    .filter(|item| item.item_type == InboxItemType::Message)
    .map(|item| item.subject)
    .collect::<Vec<_>>();
    assert_eq!(messages, vec!["Scout"]);

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
