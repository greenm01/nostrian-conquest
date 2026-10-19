use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use nc_compat::load_classic_core_files;
use nc_data::{
    CampaignStore, InboxItemType, QueuedPlayerMail, ReportBlockRow, append_mail_queue,
    carry_unread_report_blocks, delete_report_blocks_for_viewer, load_mail_queue,
    runtime_sent_inbox_items,
};

fn mail(subject: &str, year: u16) -> QueuedPlayerMail {
    QueuedPlayerMail {
        sender_empire_id: 1,
        recipient_empire_id: 2,
        year,
        subject: subject.into(),
        body: format!("{subject} body"),
        recipient_deleted: false,
        sender_deleted: false,
        cancelled: false,
    }
}

/// Tests run in parallel and the clock can repeat, so each directory also
/// takes a sequence number.
static DIRECTORY_SEQ: AtomicU64 = AtomicU64::new(0);

fn directory() -> PathBuf {
    let path = std::env::temp_dir().join(format!(
        "nc-mail-retention-{}-{}-{}",
        std::process::id(),
        DIRECTORY_SEQ.fetch_add(1, Ordering::Relaxed),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&path).unwrap();
    path
}

#[test]
fn hidden_and_cancelled_mail_survives_database_reload_and_year_advance() {
    let dir = directory();
    let mut game = load_classic_core_files(
        &PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/ecutil-init/v1.5"),
    )
    .unwrap();
    game.conquest.game_year = 3001;
    let mut messages = vec![
        mail("Sender hidden", 3000),
        mail("Recipient hidden", 3000),
        mail("Cancelled", 3001),
    ];
    messages[0].sender_deleted = true;
    messages[1].recipient_deleted = true;
    messages[2].cancel();
    let mut reports = vec![ReportBlockRow {
        viewer_empire_id: 1,
        block_index: 0,
        decoded_text: "Retained report body".into(),
        raw_bytes: None,
        recipient_deleted: false,
    }];
    delete_report_blocks_for_viewer(&mut reports, &game.players, 1);
    let store = CampaignStore::open(dir.join("game.db")).unwrap();
    store.save_year(3001, &game, &reports, &messages).unwrap();
    let reopened = CampaignStore::open(dir.join("game.db")).unwrap();
    let state = reopened.load_latest_runtime_state().unwrap().unwrap();
    assert_eq!(state.queued_mail, messages);
    assert_eq!(state.report_block_rows, reports);
    reopened
        .save_year(
            3001,
            &state.game_data,
            &state.report_block_rows,
            &state.queued_mail,
        )
        .unwrap();
    assert_eq!(
        reopened
            .load_latest_runtime_state()
            .unwrap()
            .unwrap()
            .report_block_rows,
        reports
    );
    assert!(state.queued_mail[0].is_visible_to_recipient(2, 3001));
    assert!(state.queued_mail[1].is_visible_to_sender(1));
    assert!(!state.queued_mail[1].is_visible_to_recipient(2, 3001));
    for year in [3001, 3002, 3010] {
        assert!(!state.queued_mail[2].is_queued_by(1, year));
        assert!(!state.queued_mail[2].is_delivered(year));
        assert!(!state.queued_mail[2].is_visible_to_recipient(2, year));
    }
    game.conquest.game_year = 3002;
    let next_reports = carry_unread_report_blocks(&reports, vec![]);
    reopened
        .save_year(3001, &game, &next_reports, &messages)
        .unwrap();
    assert_eq!(
        reopened
            .load_latest_runtime_state()
            .unwrap()
            .unwrap()
            .queued_mail,
        messages
    );
    let db = rusqlite::Connection::open(dir.join("game.db")).unwrap();
    let archived: i64 = db
        .query_row(
            "SELECT count(*) FROM report_blocks WHERE decoded_text = 'Retained report body'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(archived, 1, "hidden report stays in its year's snapshot");
    // Windows cannot remove a file that is still open.
    drop(db);
    drop(store);
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn sender_flags_round_trip_in_queue_file() {
    let dir = directory();
    let kept = mail("Kept", 3000);
    append_mail_queue(&dir, &kept).unwrap();
    let mut cancelled = mail("Cancelled", 3001);
    cancelled.cancel();
    append_mail_queue(&dir, &cancelled).unwrap();
    let loaded = load_mail_queue(&dir).unwrap();
    assert_eq!(loaded, vec![kept, cancelled]);
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn sent_lists_pending_then_delivery_year_and_hides_only_the_senders_removed_copies() {
    let mut game = load_classic_core_files(
        &PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/ecutil-init/v1.5"),
    )
    .unwrap();
    game.conquest.game_year = 3003;
    let mut messages = vec![
        mail("Old", 3000),
        mail("Recent", 3002),
        mail("Queued", 3003),
        mail("Cancelled", 3003),
        mail("Hidden", 3001),
    ];
    messages[1].recipient_deleted = true;
    messages[3].cancel();
    messages[4].sender_deleted = true;
    let rows = runtime_sent_inbox_items(&game, 1, &messages);
    let labels: Vec<_> = rows
        .iter()
        .map(|item| (item.subject.as_str(), item.year, item.item_type))
        .collect();
    assert_eq!(
        labels,
        vec![
            ("Queued", 3003, InboxItemType::Outgoing),
            ("Recent", 3003, InboxItemType::Sent),
            ("Old", 3001, InboxItemType::Sent)
        ]
    );
    assert!(runtime_sent_inbox_items(&game, 2, &messages).is_empty());
}
