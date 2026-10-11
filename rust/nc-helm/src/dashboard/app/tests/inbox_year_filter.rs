use super::*;
use crate::dashboard::app::state::InboxFilter;
use crate::dashboard::inbox::{
    DashInboxItem, DashInboxItemSource, DashInboxItemType, matches_filter,
};

fn item(item_type: DashInboxItemType, year: u16) -> DashInboxItem {
    DashInboxItem {
        correspondent: String::new(),
        source: DashInboxItemSource::QueuedMail(0),
        item_type,
        year,
        week: None,
        subject: "Subject".to_string(),
        body_lines: Vec::new(),
        report_bucket: None,
    }
}

#[test]
fn current_year_filter_keeps_the_latest_delivered_year() {
    let year = 3005;
    for item_type in [DashInboxItemType::Message, DashInboxItemType::Report] {
        assert!(matches_filter(
            &item(item_type, year - 1),
            InboxFilter::All,
            true,
            year
        ));
        assert!(!matches_filter(
            &item(item_type, year - 2),
            InboxFilter::All,
            true,
            year
        ));
    }
}

#[test]
fn current_year_filter_shows_the_senders_own_queued_rows() {
    let year = 3005;
    assert!(matches_filter(
        &item(DashInboxItemType::Outgoing, year),
        InboxFilter::All,
        true,
        year
    ));
    assert!(!matches_filter(
        &item(DashInboxItemType::Outgoing, year - 1),
        InboxFilter::All,
        true,
        year
    ));
}

#[test]
fn all_years_keeps_every_year() {
    let year = 3005;
    for item_year in [year - 3, year - 1, year] {
        assert!(matches_filter(
            &item(DashInboxItemType::Report, item_year),
            InboxFilter::All,
            false,
            year
        ));
    }
}

#[test]
fn current_year_overlay_lists_delivered_mail_and_reports() {
    let mut app = dash_app();
    let year = app.game_data.conquest.game_year;
    let mail = |year: u16, subject: &str| QueuedPlayerMail {
        sender_empire_id: 2,
        recipient_empire_id: 1,
        year,
        subject: subject.to_string(),
        body: "Body".to_string(),
        recipient_deleted: false,
        sender_deleted: false,
        cancelled: false,
    };
    let report = |index: usize, report_year: u16| ReportBlockRow {
        viewer_empire_id: 1,
        block_index: index,
        decoded_text: format!("Stardate: 05/{report_year}\nFleet contact report"),
        raw_bytes: None,
        recipient_deleted: false,
    };
    app.queued_mail = vec![
        mail(year - 1, "Latest"),
        mail(year - 2, "Older"),
        mail(year, "Not yet delivered"),
    ];
    app.report_block_rows = vec![report(0, year - 1), report(1, year - 2)];
    app.overlay = ActiveOverlay::Inbox;

    assert_eq!(
        crate::dashboard::overlays::inbox::list::inbox_items(&app).len(),
        4
    );

    app.inbox_overlay.current_year_only = true;
    let kept = crate::dashboard::overlays::inbox::list::inbox_items(&app);
    assert_eq!(kept.len(), 2);
    assert!(kept.iter().all(|item| item.year == year - 1));
    assert!(kept.iter().any(|item| item.subject == "Latest"));
}
