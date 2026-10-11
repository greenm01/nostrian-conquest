use crate::support::*;
use nc_game::domains::messaging::state::OutboxMode;
use nc_game::reports::InboxItemType;

fn load_player_one(fixture_dir: &Path) -> App {
    App::load(AppConfig {
        game_dir: fixture_dir.to_path_buf(),
        player_record_index_1_based: 1,
        export_root: None,
        queue_dir: None,
        session_timeout_secs: None,
        game_config: Default::default(),
    })
    .expect("app should load")
}

fn press(app: &mut App, code: KeyCode) {
    let action = app.handle_key(key(code));
    assert_eq!(apply_action(app, action), AppOutcome::Continue);
}

fn act(app: &mut App, action: MessagingAction) {
    assert_eq!(
        apply_action(app, Action::Messaging(action)),
        AppOutcome::Continue
    );
}

fn render(app: &mut App) -> CaptureTerminal {
    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal).expect("screen should render");
    terminal
}

fn own_mail(recipient: u8, year: u16, subject: &str) -> QueuedPlayerMail {
    incoming_mail(1, recipient, year, subject, &format!("{subject} body"))
}

fn queue_own_messages(fixture_dir: &Path, subjects: &[&str]) {
    let mut runtime = latest_runtime_state(fixture_dir);
    let year = runtime.game_data.conquest.game_year;
    for subject in subjects {
        runtime.queued_mail.push(own_mail(2, year, subject));
    }
    save_runtime_state(fixture_dir, &runtime);
}

fn open_outbox(app: &mut App) {
    act(app, MessagingAction::OpenComposeRecipient);
    press(app, KeyCode::Char('D'));
    assert_eq!(app.current_screen(), ScreenId::ComposeMessageOutbox);
    assert_eq!(app.messaging.compose_outbox_mode, OutboxMode::List);
}

fn saved_subjects(fixture_dir: &Path) -> Vec<String> {
    latest_runtime_state(fixture_dir)
        .queued_mail
        .into_iter()
        .filter(|mail| !mail.cancelled)
        .map(|mail| mail.subject)
        .collect()
}

fn assert_fits(terminal: &CaptureTerminal, height: usize) {
    assert_eq!(terminal.lines.len(), height);
    for line in &terminal.lines {
        assert!(line.chars().count() <= 80, "{line:?}");
    }
}

fn send_message(app: &mut App, subject: &str, body: &str) {
    act(app, MessagingAction::OpenComposeRecipient);
    act(app, MessagingAction::AppendComposeRecipientChar('2'));
    act(app, MessagingAction::SubmitComposeRecipient);
    for ch in subject.chars() {
        act(app, MessagingAction::AppendComposeSubjectChar(ch));
    }
    act(app, MessagingAction::SubmitComposeSubject);
    for ch in body.chars() {
        act(app, MessagingAction::AppendComposeBodyChar(ch));
    }
    act(app, MessagingAction::OpenComposeSendConfirm);
    act(app, MessagingAction::ConfirmSendComposedMessage);
    assert_eq!(app.current_screen(), ScreenId::ComposeMessageSent);
}

#[test]
fn outbox_enter_reads_the_queued_message_without_deleting_it() {
    let fixture_dir = temp_game_copy();
    queue_own_messages(&fixture_dir, &["Alpha", "Beta"]);
    let mut app = load_player_one(&fixture_dir);
    open_outbox(&mut app);

    press(&mut app, KeyCode::Down);
    press(&mut app, KeyCode::Enter);
    assert_eq!(app.messaging.compose_outbox_mode, OutboxMode::Read);
    let read = render(&mut app);
    assert!(
        line_containing(&read, "To ").contains("(Empire #2)"),
        "{:?}",
        read.lines
    );
    assert!(read.lines.iter().any(|line| line.contains("Subject: Beta")));
    assert!(read.lines.iter().any(|line| line.contains("Beta body")));
    assert!(
        read.lines
            .iter()
            .any(|line| line.contains("<end of message>"))
    );
    assert_eq!(read.line(24).trim_end(), " (slap a key)");

    press(&mut app, KeyCode::Char('x'));
    assert_eq!(app.current_screen(), ScreenId::ComposeMessageOutbox);
    assert_eq!(app.messaging.compose_outbox_mode, OutboxMode::List);

    // A typed number reads that message instead of the highlighted one.
    press(&mut app, KeyCode::Char('1'));
    press(&mut app, KeyCode::Enter);
    let read = render(&mut app);
    assert!(
        read.lines
            .iter()
            .any(|line| line.contains("Subject: Alpha"))
    );
    press(&mut app, KeyCode::Enter);
    assert_eq!(app.messaging.compose_outbox_mode, OutboxMode::List);

    assert_eq!(app.queued_mail.len(), 2);
    assert_eq!(saved_subjects(&fixture_dir), vec!["Alpha", "Beta"]);
}

#[test]
fn outbox_d_prompt_defaults_to_yes_and_n_keeps_the_message() {
    let fixture_dir = temp_game_copy();
    queue_own_messages(&fixture_dir, &["Alpha", "Beta", "Gamma"]);
    let mut app = load_player_one(&fixture_dir);
    open_outbox(&mut app);

    press(&mut app, KeyCode::Char('D'));
    assert_eq!(app.messaging.compose_outbox_mode, OutboxMode::DeleteConfirm);
    let prompt = render(&mut app);
    assert!(
        line_containing(&prompt, "COMMAND <-").contains("COMMAND <- Delete item 01? [Y]/N ->"),
        "{:?}",
        prompt.lines
    );
    press(&mut app, KeyCode::Char('N'));
    assert_eq!(app.messaging.compose_outbox_mode, OutboxMode::List);
    assert_eq!(saved_subjects(&fixture_dir), vec!["Alpha", "Beta", "Gamma"]);

    press(&mut app, KeyCode::Char('d'));
    press(&mut app, KeyCode::Char('y'));
    assert_eq!(saved_subjects(&fixture_dir), vec!["Beta", "Gamma"]);
    assert_eq!(app.messaging.compose_outbox_cursor, 0);

    press(&mut app, KeyCode::Down);
    press(&mut app, KeyCode::Char('D'));
    press(&mut app, KeyCode::Enter);
    assert_eq!(saved_subjects(&fixture_dir), vec!["Beta"]);
    assert_eq!(app.messaging.compose_outbox_cursor, 0);
    assert_eq!(app.messaging.compose_outbox_mode, OutboxMode::List);

    let list = render(&mut app);
    assert!(list.lines.iter().any(|line| line.contains("Beta")));
    assert!(!list.lines.iter().any(|line| line.contains("Gamma")));
    assert!(line_containing(&list, "COMMAND <-").contains("[01]"));
}

#[test]
fn outbox_lists_only_this_turns_messages_so_delivered_ones_cannot_be_deleted() {
    let fixture_dir = temp_game_copy();
    let mut runtime = latest_runtime_state(&fixture_dir);
    let year = runtime.game_data.conquest.game_year;
    runtime.queued_mail.push(own_mail(2, year - 1, "Delivered"));
    runtime.queued_mail.push(own_mail(3, year, "Queued"));
    save_runtime_state(&fixture_dir, &runtime);
    let mut app = load_player_one(&fixture_dir);
    open_outbox(&mut app);

    let list = render(&mut app);
    assert!(line_containing(&list, "│01│").contains("Queued"));
    assert!(!list.lines.iter().any(|line| line.contains("Delivered")));
    assert!(!list.lines.iter().any(|line| line.contains("│02│")));

    // The delivered message has no number to type.
    press(&mut app, KeyCode::Char('2'));
    press(&mut app, KeyCode::Char('D'));
    assert_eq!(app.messaging.compose_outbox_mode, OutboxMode::List);
    assert_eq!(
        app.messaging.compose_outbox_status.as_deref(),
        Some("Enter a queued message number in 1..=1.")
    );
    press(&mut app, KeyCode::Backspace);

    press(&mut app, KeyCode::Char('D'));
    press(&mut app, KeyCode::Char('Y'));
    assert_eq!(saved_subjects(&fixture_dir), vec!["Delivered"]);
    let list = render(&mut app);
    assert!(!list.lines.iter().any(|line| line.contains("Delivered")));
}

#[test]
fn outbox_read_view_scrolls_a_long_message_with_the_inbox_preview_keys() {
    let fixture_dir = temp_game_copy();
    let mut runtime = latest_runtime_state(&fixture_dir);
    let year = runtime.game_data.conquest.game_year;
    let body = (1..=40)
        .map(|line| format!("Line {line:02}"))
        .collect::<Vec<_>>()
        .join("\n");
    runtime
        .queued_mail
        .push(incoming_mail(1, 2, year, "Long", &body));
    save_runtime_state(&fixture_dir, &runtime);
    let mut app = load_player_one(&fixture_dir);
    open_outbox(&mut app);

    // The box holds 21 of the 43 lines: recipient, subject, and lines 01-19.
    press(&mut app, KeyCode::Enter);
    let read = render(&mut app);
    assert!(read.line(2).starts_with("│To "), "{:?}", read.lines);
    assert!(read.line(22).starts_with("│Line 19"), "{:?}", read.lines);

    press(&mut app, KeyCode::Down);
    assert_eq!(app.messaging.compose_outbox_mode, OutboxMode::Read);
    let read = render(&mut app);
    assert!(read.line(2).starts_with("│Subject: Long"));
    assert!(read.line(22).starts_with("│Line 20"));

    // PgDn stops at the end of the message.
    for _ in 0..5 {
        press(&mut app, KeyCode::PageDown);
    }
    let read = render(&mut app);
    assert!(read.line(22).starts_with("│<end of message>"));
    press(&mut app, KeyCode::Char('k'));
    assert!(render(&mut app).line(22).starts_with("│Line 40"));
    for _ in 0..5 {
        press(&mut app, KeyCode::PageUp);
    }
    assert!(render(&mut app).line(2).starts_with("│To "));

    // Any other key returns to the list; reading again starts at the top.
    press(&mut app, KeyCode::Char('j'));
    press(&mut app, KeyCode::Esc);
    assert_eq!(app.messaging.compose_outbox_mode, OutboxMode::List);
    press(&mut app, KeyCode::Enter);
    assert!(render(&mut app).line(2).starts_with("│To "));
}

#[test]
fn outbox_footer_and_help_list_read_and_delete_keys() {
    let fixture_dir = temp_game_copy();
    queue_own_messages(&fixture_dir, &["Alpha"]);
    let mut app = load_player_one(&fixture_dir);
    open_outbox(&mut app);

    let list = render(&mut app);
    let footer = line_containing(&list, "COMMAND <-");
    assert!(footer.contains("? D <Q> [01] ->"), "{footer}");

    press(&mut app, KeyCode::Char('?'));
    let popup = app.popup_help.as_ref().expect("outbox help should open");
    assert_eq!(popup.title, "MESSAGE COMMANDS");
    assert!(
        popup
            .lines
            .iter()
            .any(|line| line.contains("Enter") && line.contains("read the highlighted"))
    );
    assert!(
        popup
            .lines
            .iter()
            .any(|line| line.contains("D") && line.contains("delete (cancel)"))
    );
    press(&mut app, KeyCode::Esc);
    assert!(app.popup_help.is_none());

    press(&mut app, KeyCode::Char('Q'));
    assert_eq!(app.current_screen(), ScreenId::ComposeMessageRecipient);
    let recipients = render(&mut app);
    assert!(line_containing(&recipients, "COMMAND <-").contains("? D <Q>"));
    press(&mut app, KeyCode::Char('?'));
    let popup = app.popup_help.as_ref().expect("recipient help should open");
    assert!(
        popup
            .lines
            .iter()
            .any(|line| line.contains("D") && line.contains("open queued outgoing messages"))
    );
    assert!(
        !popup
            .lines
            .iter()
            .any(|line| line.contains("read the highlighted"))
    );
}

#[test]
fn send_confirmation_points_to_inbox_read_and_cancel_keys() {
    let fixture_dir = temp_game_copy();
    let mut app = load_player_one(&fixture_dir);
    send_message(&mut app, "Hi", "Hi");

    let sent = render(&mut app);
    assert!(sent.line(3).contains("Message queued for Empire 2."));
    assert_eq!(
        sent.line(4).trim_end(),
        "Inbox > Sent: Enter reads; D cancels before maintenance."
    );
    assert!(sent.line(5).trim().is_empty());
    assert_eq!(sent.line(6).trim_end(), "(slap a key)");
    assert_fits(&sent, 25);
}

#[test]
fn outbox_list_and_read_view_fit_80x25_and_24_row_doors() {
    let fixture_dir = temp_game_copy();
    let mut runtime = latest_runtime_state(&fixture_dir);
    let year = runtime.game_data.conquest.game_year;
    let long_body = format!("{} {}", "word ".repeat(150), "x".repeat(200));
    for idx in 0..12 {
        runtime.queued_mail.push(incoming_mail(
            1,
            2,
            year,
            &format!("{idx:02} {}", "s".repeat(57)),
            &long_body,
        ));
    }
    save_runtime_state(&fixture_dir, &runtime);
    let mut app = load_player_one(&fixture_dir);
    open_outbox(&mut app);

    // A 60-character subject used to push the list past 80 columns and panic;
    // the list now clips it and the read view shows all of it.
    let full_subject = format!("00 {}", "s".repeat(57));
    let list = render(&mut app);
    assert_fits(&list, 25);
    let row = line_containing(&list, "│01│");
    assert!(row.contains(&full_subject[..44]), "{row}");
    assert!(!row.contains(&full_subject), "{row}");
    press(&mut app, KeyCode::Enter);
    let read = render(&mut app);
    assert_fits(&read, 25);
    assert!(
        read.lines
            .iter()
            .any(|line| line.contains(&format!("Subject: {full_subject}")))
    );
    assert!(read.line(1).starts_with('┌'));
    assert!(read.line(23).starts_with('└'));
    assert_eq!(read.line(24).trim_end(), " (slap a key)");
    press(&mut app, KeyCode::Char(' '));

    enable_door_mode(&mut app);
    let list = render(&mut app);
    assert_fits(&list, 24);
    press(&mut app, KeyCode::Enter);
    let read = render(&mut app);
    assert_fits(&read, 24);
    assert!(read.line(22).starts_with('└'));
    assert_eq!(read.lines[23].trim_end(), " (slap a key)");
}

#[test]
fn sent_message_appears_in_review_inbox_until_cancelled() {
    let fixture_dir = temp_game_copy();
    let mut app = load_player_one(&fixture_dir);
    advance_to_main_menu(&mut app);
    assert_eq!(
        apply_action(&mut app, Action::OpenGeneralMenu),
        AppOutcome::Continue
    );
    send_message(&mut app, "Truce", "Hold fire");
    press(&mut app, KeyCode::Enter);
    assert_eq!(app.current_screen(), ScreenId::GeneralMenu);

    assert_eq!(
        apply_action(&mut app, Action::Startup(StartupAction::OpenReports)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::Reports);
    let inbox = render(&mut app);
    let row = line_containing(&inbox, "Truce");
    assert!(row.contains("│01│Queued│"), "{row}");

    press(&mut app, KeyCode::Enter);
    assert_eq!(app.messaging.inbox_focus, InboxFocus::Preview);
    let preview = render(&mut app);
    assert!(
        preview
            .lines
            .iter()
            .any(|line| line.contains("To ") && line.contains("(Empire #2)"))
    );
    assert!(preview.lines.iter().any(|line| line.contains("Hold fire")));

    press(&mut app, KeyCode::Char('D'));
    let prompt = render(&mut app);
    assert!(
        prompt
            .line(24)
            .contains("Cancel queued message 01? [Y]/N ->")
    );
    press(&mut app, KeyCode::Enter);
    assert_eq!(app.queued_mail.len(), 1);
    assert!(app.queued_mail[0].cancelled);
    assert!(saved_subjects(&fixture_dir).is_empty());

    assert_eq!(
        apply_action(&mut app, Action::ReturnToCommandMenu),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Startup(StartupAction::OpenReports)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::GeneralMenu);
    let menu = render(&mut app);
    assert!(line_containing(&menu, "Inbox is empty.").contains("Inbox is empty."));
}

#[test]
fn sent_folder_lists_queued_before_delivered_and_keeps_ids_after_cancel() {
    let fixture_dir = temp_game_copy();
    let mut runtime = latest_runtime_state(&fixture_dir);
    let year = runtime.game_data.conquest.game_year;
    set_runtime_report_blocks(
        &mut runtime,
        classic_chunked_report_blocks(&["Stardate: 03/3003\nFleet contact report"]),
    );
    runtime.queued_mail.clear();
    runtime.queued_mail.push(own_mail(2, year, "First"));
    runtime
        .queued_mail
        .push(incoming_mail(2, 1, year - 1, "Incoming", "Incoming body"));
    runtime.queued_mail.push(own_mail(3, year, "Second"));
    runtime.queued_mail.push(own_mail(2, year - 1, "Last year"));
    save_runtime_state(&fixture_dir, &runtime);

    let mut app = load_player_one(&fixture_dir);
    advance_to_main_menu(&mut app);
    assert_eq!(
        apply_action(&mut app, Action::Startup(StartupAction::OpenReports)),
        AppOutcome::Continue
    );
    let ids = |app: &App| {
        app.filtered_inbox_display_items()
            .into_iter()
            .map(|item| (item.item.subject, item.display_id, item.item.item_type))
            .collect::<Vec<_>>()
    };

    act(&mut app, MessagingAction::SetInboxFolderSent);
    let before = ids(&app);
    assert_eq!(before.len(), 3);
    assert_eq!(
        (&*before[0].0, before[0].2),
        ("Second", InboxItemType::Outgoing)
    );
    assert_eq!(
        (&*before[1].0, before[1].2),
        ("First", InboxItemType::Outgoing)
    );
    assert_ne!(before[0].1, before[1].1);
    assert!(
        before[2..]
            .iter()
            .all(|(_, _, item_type)| *item_type != InboxItemType::Outgoing)
    );
    assert!(
        before
            .iter()
            .any(|(subject, _, kind)| subject == "Last year" && *kind == InboxItemType::Sent)
    );

    act(&mut app, MessagingAction::SetInboxTypeFilterMessages);
    let messages = ids(&app)
        .into_iter()
        .map(|(subject, _, _)| subject)
        .collect::<Vec<_>>();
    assert_eq!(messages, vec!["Incoming"]);
    act(&mut app, MessagingAction::SetInboxTypeFilterReports);
    assert!(
        ids(&app)
            .iter()
            .all(|(_, _, item_type)| *item_type == InboxItemType::Report)
    );
    act(&mut app, MessagingAction::SetInboxFolderSent);

    // Cancel "First", which sits ahead of the others in the queue.
    act(&mut app, MessagingAction::MoveInboxCursor(1));
    let cancelled_id = format!("{:02}", before[1].1);
    assert_eq!(app.current_inbox_display_id(), cancelled_id);
    press(&mut app, KeyCode::Char('D'));
    press(&mut app, KeyCode::Char('Y'));
    assert_eq!(
        saved_subjects(&fixture_dir),
        vec!["Incoming", "Second", "Last year"]
    );
    let after = ids(&app);
    let expected = before
        .into_iter()
        .filter(|(subject, _, _)| subject != "First")
        .collect::<Vec<_>>();
    assert_eq!(after, expected);
    let inbox = render(&mut app);
    assert!(
        inbox
            .lines
            .iter()
            .any(|line| line.contains(&format!("Deleted item {cancelled_id}.")))
    );
}
