use super::*;
use crate::dashboard::overlays::inbox::list::inbox_items;
use crate::dashboard::panels::comms;

fn type_text(app: &mut DashApp, text: &str) {
    for ch in text.chars() {
        app.handle_key(KeyEvent::new(KeyCode::Char(ch), KeyModifiers::NONE));
    }
}

/// Sends a message to empire 2 through the inbox's compose flow.
fn send_message(app: &mut DashApp, subject: &str, body: &str) {
    app.handle_key(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::NONE));
    type_text(app, "2");
    app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
    type_text(app, subject);
    app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
    type_text(app, body);
    app.handle_key(KeyEvent::new(KeyCode::Char('s'), KeyModifiers::ALT));
    app.handle_key(KeyEvent::new(KeyCode::Char('y'), KeyModifiers::NONE));
}

fn subjects(app: &DashApp) -> Vec<String> {
    inbox_items(app)
        .into_iter()
        .map(|item| item.subject)
        .collect()
}

fn mail(sender: u8, recipient: u8, year: u16, subject: &str) -> QueuedPlayerMail {
    QueuedPlayerMail {
        sender_empire_id: sender,
        recipient_empire_id: recipient,
        year,
        subject: subject.to_string(),
        body: "Body".to_string(),
        recipient_deleted: false,
    }
}

fn report(index: usize, year: u16, text: &str) -> ReportBlockRow {
    ReportBlockRow {
        viewer_empire_id: 1,
        block_index: index,
        decoded_text: format!("Stardate: 05/{year}\n{text}"),
        raw_bytes: None,
        recipient_deleted: false,
    }
}

#[test]
fn own_queued_messages_lead_the_inbox_newest_first() {
    let mut app = dash_app();
    let year = app.game_data.conquest.game_year;
    app.queued_mail = vec![
        mail(1, 2, year, "First out"),
        mail(2, 1, year - 1, "Delivered"),
        mail(1, 3, year, "Second out"),
        mail(3, 2, year, "Someone else's"),
        mail(1, 2, year - 1, "Already delivered"),
    ];
    app.report_block_rows = vec![report(0, year - 1, "Fleet contact report")];

    let items = inbox_items(&app);
    let types = items
        .iter()
        .map(|item| item.item_type.code())
        .collect::<Vec<_>>();
    assert_eq!(types, ["OUT", "OUT", "R", "M"]);
    assert_eq!(items[0].subject, "Second out");
    assert_eq!(items[1].subject, "First out");
}

#[test]
fn outgoing_rows_show_under_all_and_messages_not_reports() {
    let mut app = dash_app();
    let year = app.game_data.conquest.game_year;
    app.queued_mail = vec![mail(1, 2, year, "Out"), mail(2, 1, year - 1, "Delivered")];
    app.report_block_rows = vec![report(0, year - 1, "Fleet contact report")];
    app.overlay = ActiveOverlay::Inbox;

    assert_eq!(subjects(&app).len(), 3);

    app.handle_key(KeyEvent::new(KeyCode::Char('m'), KeyModifiers::NONE));
    assert_eq!(subjects(&app), ["Out", "Delivered"]);

    app.handle_key(KeyEvent::new(KeyCode::Char('r'), KeyModifiers::NONE));
    assert_eq!(inbox_items(&app).len(), 1);
    assert!(
        inbox_items(&app)
            .iter()
            .all(|item| item.item_type.code() == "R")
    );

    app.handle_key(KeyEvent::new(KeyCode::Char('a'), KeyModifiers::NONE));
    assert_eq!(subjects(&app).len(), 3);
}

#[test]
fn current_year_filter_keeps_outgoing_rows() {
    let mut app = dash_app();
    let year = app.game_data.conquest.game_year;
    app.queued_mail = vec![mail(1, 2, year, "Out"), mail(2, 1, year - 1, "Latest")];
    app.report_block_rows = vec![report(0, year - 2, "Fleet contact report")];
    app.overlay = ActiveOverlay::Inbox;

    app.handle_key(KeyEvent::new(KeyCode::Char('y'), KeyModifiers::NONE));

    assert!(app.inbox_overlay.current_year_only);
    assert_eq!(subjects(&app), ["Out", "Latest"]);
}

#[test]
fn outgoing_row_shows_recipient_subject_and_body() {
    let mut app = dash_app();
    let year = app.game_data.conquest.game_year;
    let mut out = mail(1, 2, year, "Border treaty");
    out.body = "Withdraw your fleets".to_string();
    app.queued_mail = vec![out];
    app.overlay = ActiveOverlay::Inbox;

    let lines = render_dashboard_lines(&app);
    let has = |needle: &str| lines.iter().any(|line| line.contains(needle));
    assert!(has("OUT"));
    assert!(has("(Empire #2)"));
    assert!(has("Subject: Border treaty"));
    assert!(has("Withdraw your fleets"));
}

#[test]
fn inbox_cancel_of_an_outgoing_row_asks_then_removes_it_everywhere() {
    let mut app = dash_app_with_store();
    app.initialize_hosted_turn_draft();
    app.handle_key(KeyEvent::new(KeyCode::Char('i'), KeyModifiers::NONE));
    send_message(&mut app, "One", "First body");
    send_message(&mut app, "Two", "Second body");
    assert_eq!(subjects(&app), ["Two", "One"]);
    assert_eq!(app.hosted_turn_draft.as_ref().unwrap().messages.len(), 2);

    app.handle_key(KeyEvent::new(KeyCode::Char('d'), KeyModifiers::NONE));
    assert!(app.inbox_overlay.delete_confirm);
    assert!(
        render_dashboard_lines(&app)
            .iter()
            .any(|line| line.contains("Delete this item? [Y]/N"))
    );
    app.handle_key(KeyEvent::new(KeyCode::Char('n'), KeyModifiers::NONE));
    assert!(!app.inbox_overlay.delete_confirm);
    assert_eq!(subjects(&app), ["Two", "One"]);

    app.handle_key(KeyEvent::new(KeyCode::Char('d'), KeyModifiers::NONE));
    app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));

    assert_eq!(subjects(&app), ["One"]);
    let draft = app.hosted_turn_draft.as_ref().unwrap();
    assert_eq!(draft.messages.len(), 1);
    assert_eq!(draft.messages[0].subject, "One");
    assert_eq!(app.queued_mail.len(), 1);
    let stored = app
        .campaign_store
        .clone()
        .expect("campaign store")
        .load_latest_runtime_state()
        .expect("load runtime state")
        .expect("runtime state");
    assert_eq!(stored.queued_mail.len(), 1);
    assert_eq!(stored.queued_mail[0].subject, "One");
}

#[test]
fn outbox_cancel_asks_first_and_defaults_to_yes() {
    let mut app = dash_app_with_store();
    app.initialize_hosted_turn_draft();
    app.handle_key(KeyEvent::new(KeyCode::Char('i'), KeyModifiers::NONE));
    send_message(&mut app, "One", "First body");
    send_message(&mut app, "Two", "Second body");
    app.handle_key(KeyEvent::new(KeyCode::Char('o'), KeyModifiers::NONE));
    assert_eq!(app.inbox_overlay.prompt_mode, InboxPromptMode::Outbox);
    let staged = |app: &DashApp| app.hosted_turn_draft.as_ref().unwrap().messages.len();

    for decline in [KeyCode::Esc, KeyCode::Char('n')] {
        app.handle_key(KeyEvent::new(KeyCode::Char('d'), KeyModifiers::NONE));
        assert!(app.inbox_overlay.delete_confirm);
        let lines = render_dashboard_lines(&app);
        assert!(
            lines
                .iter()
                .any(|line| line.contains("Delete this item? [Y]/N"))
        );
        assert!(!lines.iter().any(|line| line.contains("Y/[N]")));
        assert_eq!(staged(&app), 2);
        assert_eq!(app.queued_mail.len(), 2);

        app.handle_key(KeyEvent::new(decline, KeyModifiers::NONE));
        assert!(!app.inbox_overlay.delete_confirm);
        assert_eq!(app.inbox_overlay.prompt_mode, InboxPromptMode::Outbox);
        assert_eq!(staged(&app), 2);
        assert_eq!(app.queued_mail.len(), 2);
    }

    app.handle_key(KeyEvent::new(KeyCode::Char('d'), KeyModifiers::NONE));
    app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
    assert!(!app.inbox_overlay.delete_confirm);
    assert_eq!(app.inbox_overlay.prompt_mode, InboxPromptMode::Outbox);
    assert_eq!(staged(&app), 1);
    assert_eq!(app.queued_mail.len(), 1);
    assert_eq!(
        app.hosted_turn_draft.as_ref().unwrap().messages[0].subject,
        "Two"
    );

    app.handle_key(KeyEvent::new(KeyCode::Char('d'), KeyModifiers::NONE));
    app.handle_key(KeyEvent::new(KeyCode::Char('y'), KeyModifiers::NONE));
    assert_eq!(staged(&app), 0);
    assert!(app.queued_mail.is_empty());
}

#[test]
fn comms_panel_counts_reports_of_the_latest_delivered_year() {
    let mut app = dash_app();
    let year = app.game_data.conquest.game_year;
    app.report_block_rows = vec![
        report(0, year - 1, "Bombard report"),
        report(1, year - 1, "Fleet contact report"),
        report(2, year - 2, "Bombard report"),
        report(3, year, "Bombard report"),
    ];
    app.queued_mail = vec![mail(1, 2, year, "Out"), mail(2, 1, year - 1, "Delivered")];

    let rows = comms::body_rows(&app)
        .into_iter()
        .map(|(text, _)| text)
        .collect::<Vec<_>>();
    let count = |label: &str| {
        rows.iter()
            .find(|row| row.starts_with(label))
            .and_then(|row| row.rsplit(' ').next())
            .map(str::to_string)
    };
    assert_eq!(count("Combat Rep").as_deref(), Some("1"));
    assert_eq!(count("Intel Rep").as_deref(), Some("1"));
    assert_eq!(count("Messages").as_deref(), Some("1"));
    assert_eq!(count("Total").as_deref(), Some("3"));
}
