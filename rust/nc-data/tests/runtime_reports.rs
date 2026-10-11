use std::path::PathBuf;

use nc_compat::load_classic_core_files;
use nc_data::{
    CoreGameData, InboxItemSource, InboxItemType, QueuedPlayerMail, ReportBlockRow,
    ReportSummaryBucket, ReportsPreview, has_visible_runtime_messages, runtime_inbox_items,
    runtime_inbox_preview_lines, runtime_outgoing_inbox_items,
};

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// The fixture in 3004, so the 3003 mail below has been delivered.
fn fixture_game_data() -> CoreGameData {
    let mut game_data = load_classic_core_files(&repo_root().join("fixtures/ecutil-init/v1.5"))
        .expect("load fixture");
    game_data.conquest.set_game_year(3004);
    game_data
}

fn visible_mail(
    sender: u8,
    recipient: u8,
    year: u16,
    subject: &str,
    body: &str,
) -> QueuedPlayerMail {
    QueuedPlayerMail {
        sender_empire_id: sender,
        recipient_empire_id: recipient,
        year,
        subject: subject.to_string(),
        body: body.to_string(),
        recipient_deleted: false,
    }
}

fn report_row(viewer_empire_id: u8, block_index: usize, decoded_text: &str) -> ReportBlockRow {
    ReportBlockRow {
        viewer_empire_id,
        block_index,
        decoded_text: decoded_text.to_string(),
        raw_bytes: None,
        recipient_deleted: false,
    }
}

#[test]
fn runtime_inbox_items_classify_subjects_and_report_buckets() {
    let game_data = fixture_game_data();
    let items = runtime_inbox_items(
        &game_data,
        1,
        &[
            report_row(0, 0, "Stardate: 03/3003\nFleet contact report"),
            report_row(0, 1, "Stardate: 11/3003\nBombardment complete."),
            report_row(0, 2, "Starbase 1 is moving to (08,12)."),
        ],
        &[],
    );

    assert!(
        items.iter().any(|item| item.subject == "Scout"
            && item.report_bucket == Some(ReportSummaryBucket::Intel))
    );
    assert!(
        items.iter().any(|item| item.subject == "Bombard"
            && item.report_bucket == Some(ReportSummaryBucket::Combat))
    );
    assert!(
        items.iter().any(|item| item.subject == "Starbase"
            && item.report_bucket == Some(ReportSummaryBucket::Ops))
    );
}

#[test]
fn runtime_inbox_items_sort_newest_to_oldest_by_stardate() {
    let game_data = fixture_game_data();
    let items = runtime_inbox_items(
        &game_data,
        1,
        &[
            report_row(0, 0, "Stardate: 01/3003\nPatrol mission update."),
            report_row(0, 0, "Stardate: 02/3003\nFleet contact report"),
            report_row(0, 0, "Stardate: 03/3003\nGeneral command advisory."),
            report_row(0, 1, "Stardate: 52/3002\nBombardment complete."),
        ],
        &[visible_mail(2, 1, 3003, "Diplomatic", "")],
    );

    let labels = items
        .iter()
        .map(|item| format!("{} {}", item.stardate_label(), item.subject))
        .collect::<Vec<_>>();
    assert_eq!(
        labels,
        vec![
            "03/3003 General".to_string(),
            "02/3003 Scout".to_string(),
            "01/3003 Patrol".to_string(),
            "00/3003 Diplomatic".to_string(),
            "52/3002 Bombard".to_string(),
        ]
    );
}

#[test]
fn reports_preview_collects_visible_report_and_message_blocks() {
    let game_data = fixture_game_data();
    let preview = ReportsPreview::from_block_rows(
        &game_data,
        1,
        &[
            report_row(0, 0, "Stardate: 03/3003\nFleet contact report"),
            report_row(2, 1, "Stardate: 04/3003\nPlayer 2 only report."),
        ],
        &[visible_mail(2, 1, 3003, "Diplomatic", "Need aid.")],
    );

    assert_eq!(preview.result_blocks.len(), 1);
    assert_eq!(preview.message_blocks.len(), 1);
    assert!(
        preview
            .results_lines
            .iter()
            .any(|line| line.contains("Fleet contact report"))
    );
    assert!(
        preview
            .message_lines
            .iter()
            .any(|line| line.contains("Subject: Diplomatic"))
    );
    assert!(
        preview
            .message_lines
            .iter()
            .any(|line| line.contains("<end of message>"))
    );
}

#[test]
fn runtime_inbox_preview_lines_wrap_and_preserve_blank_lines() {
    let lines = vec![
        "This is a long line that should wrap cleanly.".to_string(),
        String::new(),
        "Tail".to_string(),
    ];

    let wrapped = runtime_inbox_preview_lines(&lines, 12);
    assert!(wrapped.len() >= 4);
    assert!(wrapped.iter().any(|line| line.is_empty()));
    assert_eq!(wrapped.last().expect("tail"), "Tail");
}

#[test]
fn message_rows_display_zero_week_stardate() {
    let game_data = fixture_game_data();
    let items = runtime_inbox_items(
        &game_data,
        1,
        &[],
        &[visible_mail(2, 1, 3003, "Diplomatic", "")],
    );
    let message = items
        .iter()
        .find(|item| item.item_type == InboxItemType::Message)
        .expect("message row");
    assert_eq!(message.stardate_label(), "00/3003");
}

#[test]
fn runtime_outgoing_inbox_items_list_this_turns_own_mail_newest_first() {
    let game_data = fixture_game_data();
    let year = game_data.conquest.game_year();
    let queue = [
        visible_mail(1, 2, year, "First", "Hello"),
        visible_mail(2, 1, year - 1, "Incoming", "Hi"),
        visible_mail(1, 3, year, "Second", "Line one\nLine two"),
        visible_mail(1, 2, year - 1, "Delivered", "Old"),
    ];

    let items = runtime_outgoing_inbox_items(&game_data, 1, &queue);
    let subjects = items
        .iter()
        .map(|item| item.subject.as_str())
        .collect::<Vec<_>>();
    assert_eq!(subjects, vec!["Second", "First"]);
    assert!(
        items
            .iter()
            .all(|item| item.item_type == InboxItemType::Outgoing)
    );
    assert_eq!(InboxItemType::Outgoing.code(), "OUT");
    assert_eq!(items[0].source, InboxItemSource::QueuedMail(2));
    assert_eq!(items[0].stardate_label(), format!("00/{year}"));
    let lines = &items[0].body_lines;
    assert!(lines[0].starts_with("To "), "{lines:?}");
    assert!(lines[0].ends_with("(Empire #3)"), "{lines:?}");
    assert_eq!(lines[1], "Subject: Second");
    assert_eq!(lines[2..], ["Line one", "Line two", "<end of message>"]);

    // The recipient-side inbox is unchanged: it never lists the viewer's own mail.
    assert!(
        runtime_inbox_items(&game_data, 1, &[], &queue)
            .iter()
            .all(|item| item.item_type == InboxItemType::Message && item.subject == "Incoming")
    );
}

#[test]
fn recipient_sees_mail_only_after_the_maintenance_that_delivers_it() {
    let mut game_data = fixture_game_data();
    let year = game_data.conquest.game_year();
    let queue = [visible_mail(2, 1, year, "Truce", "Hold fire")];

    // Queued this year: the sender lists it, the recipient has nothing yet.
    assert_eq!(runtime_outgoing_inbox_items(&game_data, 2, &queue).len(), 1);
    assert!(runtime_inbox_items(&game_data, 1, &[], &queue).is_empty());
    assert!(!has_visible_runtime_messages(1, &queue, year));
    let preview = ReportsPreview::from_block_rows(&game_data, 1, &[], &queue);
    assert!(preview.message_blocks.is_empty());

    // Maintenance advances the year and so delivers it.
    game_data.conquest.set_game_year(year + 1);
    assert!(runtime_outgoing_inbox_items(&game_data, 2, &queue).is_empty());
    let items = runtime_inbox_items(&game_data, 1, &[], &queue);
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].item_type, InboxItemType::Message);
    assert_eq!(items[0].subject, "Truce");
    assert!(has_visible_runtime_messages(1, &queue, year + 1));
    let preview = ReportsPreview::from_block_rows(&game_data, 1, &[], &queue);
    assert_eq!(preview.message_blocks.len(), 1);
}
