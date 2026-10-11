use crate::support::*;

#[test]
fn apply_action_queues_composed_message() {
    let fixture_dir = temp_game_copy();
    let mut app = App::load(AppConfig {
        game_dir: fixture_dir.clone(),
        player_record_index_1_based: 1,
        export_root: None,
        queue_dir: None,
        session_timeout_secs: None,
        game_config: Default::default(),
    })
    .expect("app should load");

    assert_eq!(
        apply_action(
            &mut app,
            Action::Messaging(MessagingAction::OpenComposeRecipient)
        ),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::ComposeMessageRecipient);
    assert_eq!(
        apply_action(
            &mut app,
            Action::Messaging(MessagingAction::AppendComposeRecipientChar('2'))
        ),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(
            &mut app,
            Action::Messaging(MessagingAction::SubmitComposeRecipient)
        ),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::ComposeMessageSubject);
    assert_eq!(
        apply_action(
            &mut app,
            Action::Messaging(MessagingAction::AppendComposeSubjectChar('H'))
        ),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(
            &mut app,
            Action::Messaging(MessagingAction::AppendComposeSubjectChar('i'))
        ),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(
            &mut app,
            Action::Messaging(MessagingAction::SubmitComposeSubject)
        ),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::ComposeMessageBody);
    assert_eq!(
        apply_action(
            &mut app,
            Action::Messaging(MessagingAction::AppendComposeBodyChar('H'))
        ),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(
            &mut app,
            Action::Messaging(MessagingAction::AppendComposeBodyChar('i'))
        ),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(
            &mut app,
            Action::Messaging(MessagingAction::OpenComposeSendConfirm)
        ),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::ComposeMessageSendConfirm);
    assert_eq!(
        apply_action(
            &mut app,
            Action::Messaging(MessagingAction::ConfirmSendComposedMessage)
        ),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::ComposeMessageSent);
    let queue = latest_runtime_state(&fixture_dir).queued_mail;
    assert_eq!(queue.len(), 1);
    assert_eq!(queue[0].recipient_empire_id, 2);
    assert_eq!(queue[0].subject, "Hi");
    assert_eq!(queue[0].body, "Hi");
}
#[test]
fn compose_subject_treats_q_as_text_and_esc_as_return() {
    let fixture_dir = temp_game_copy();
    let mut app = App::load(AppConfig {
        game_dir: fixture_dir.clone(),
        player_record_index_1_based: 1,
        export_root: None,
        queue_dir: None,
        session_timeout_secs: None,
        game_config: Default::default(),
    })
    .expect("app should load");

    assert_eq!(
        apply_action(
            &mut app,
            Action::Messaging(MessagingAction::OpenComposeRecipient)
        ),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(
            &mut app,
            Action::Messaging(MessagingAction::AppendComposeRecipientChar('2'))
        ),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(
            &mut app,
            Action::Messaging(MessagingAction::SubmitComposeRecipient)
        ),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::ComposeMessageSubject);

    assert_eq!(
        app.handle_key(key(KeyCode::Char('Q'))),
        Action::Messaging(MessagingAction::AppendComposeSubjectChar('Q'))
    );
    assert_eq!(
        apply_action(
            &mut app,
            Action::Messaging(MessagingAction::AppendComposeSubjectChar('Q'))
        ),
        AppOutcome::Continue
    );
    assert_eq!(app.messaging.compose_subject, "Q");
    assert_eq!(app.current_screen(), ScreenId::ComposeMessageSubject);

    assert_eq!(
        app.handle_key(key(KeyCode::Esc)),
        Action::Messaging(MessagingAction::OpenComposeRecipient)
    );
    assert_eq!(
        apply_action(
            &mut app,
            Action::Messaging(MessagingAction::OpenComposeRecipient)
        ),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::ComposeMessageRecipient);
}
#[test]
fn compose_subject_prompt_uses_esc_cancel_markup() {
    let fixture_dir = temp_game_copy();
    let mut app = App::load(AppConfig {
        game_dir: fixture_dir.clone(),
        player_record_index_1_based: 1,
        export_root: None,
        queue_dir: None,
        session_timeout_secs: None,
        game_config: Default::default(),
    })
    .expect("app should load");
    let mut terminal = CaptureTerminal::new();

    assert_eq!(
        apply_action(
            &mut app,
            Action::Messaging(MessagingAction::OpenComposeRecipient)
        ),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(
            &mut app,
            Action::Messaging(MessagingAction::AppendComposeRecipientChar('2'))
        ),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(
            &mut app,
            Action::Messaging(MessagingAction::SubmitComposeRecipient)
        ),
        AppOutcome::Continue
    );

    app.render(&mut terminal)
        .expect("compose subject prompt should render");
    let prompt = line_containing(&terminal, "COMMAND <- Message subject");
    assert!(prompt.contains("<ESC> ->"), "{prompt}");
    assert!(!prompt.contains("<Q> ->"), "{prompt}");
}

#[test]
fn compose_message_rejects_fourth_message_to_same_recipient_in_same_year() {
    let fixture_dir = temp_game_copy();
    let mut runtime = latest_runtime_state(&fixture_dir);
    let current_year = runtime.game_data.conquest.game_year;
    for subject in ["One", "Two", "Three"] {
        runtime.queued_mail.push(QueuedPlayerMail {
            sender_empire_id: 1,
            recipient_empire_id: 2,
            year: current_year,
            subject: subject.to_string(),
            body: "Queued".to_string(),
            recipient_deleted: false,
            sender_deleted: false,
            cancelled: false,
        });
    }
    save_runtime_state(&fixture_dir, &runtime);

    let mut app = App::load(AppConfig {
        game_dir: fixture_dir.clone(),
        player_record_index_1_based: 1,
        export_root: None,
        queue_dir: None,
        session_timeout_secs: None,
        game_config: Default::default(),
    })
    .expect("app should load");

    app.messaging.compose_recipient_empire = Some(2);
    app.messaging.compose_subject = "Four".to_string();
    app.messaging.compose_body = "Blocked".to_string();
    app.current_screen = ScreenId::ComposeMessageSendConfirm;

    assert_eq!(
        apply_action(
            &mut app,
            Action::Messaging(MessagingAction::ConfirmSendComposedMessage)
        ),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::ComposeMessageBody);
    assert_eq!(
        app.messaging.compose_body_status.as_deref(),
        Some("You may only queue 3 messages to Empire 2 this turn.")
    );
    assert_eq!(app.queued_mail.len(), 3);
}

#[test]
fn apply_action_deletes_queued_message_from_outbox() {
    let fixture_dir = temp_game_copy();
    let mut runtime = latest_runtime_state(&fixture_dir);
    runtime.queued_mail.push(QueuedPlayerMail {
        sender_empire_id: 1,
        recipient_empire_id: 2,
        year: 3000,
        subject: "Test".to_string(),
        body: "Queued".to_string(),
        recipient_deleted: false,
        sender_deleted: false,
        cancelled: false,
    });
    save_runtime_state(&fixture_dir, &runtime);

    let mut app = App::load(AppConfig {
        game_dir: fixture_dir.clone(),
        player_record_index_1_based: 1,
        export_root: None,
        queue_dir: None,
        session_timeout_secs: None,
        game_config: Default::default(),
    })
    .expect("app should load");

    assert_eq!(
        apply_action(
            &mut app,
            Action::Messaging(MessagingAction::OpenComposeOutbox)
        ),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::ComposeMessageOutbox);
    assert_eq!(
        apply_action(
            &mut app,
            Action::Messaging(MessagingAction::AppendComposeOutboxChar('1'))
        ),
        AppOutcome::Continue
    );
    // t067: deleting now goes through the default-yes D prompt; Enter only reads.
    assert_eq!(
        apply_action(
            &mut app,
            Action::Messaging(MessagingAction::OpenComposeOutboxDeleteConfirm)
        ),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(
            &mut app,
            Action::Messaging(MessagingAction::DeleteQueuedComposeMessage)
        ),
        AppOutcome::Continue
    );

    let queue = latest_runtime_state(&fixture_dir).queued_mail;
    assert_eq!(queue.len(), 1);
    assert!(queue[0].cancelled);
}

#[test]
fn reports_inbox_stacks_type_and_year_filters_and_deletes_selected_item() {
    let fixture_dir = temp_game_copy();
    let mut runtime = latest_runtime_state(&fixture_dir);
    // Maintenance delivers mail, so received mail carries an earlier year.
    let mail_year = runtime.game_data.conquest.game_year - 1;
    set_runtime_report_blocks(
        &mut runtime,
        classic_chunked_report_blocks(&[
            "Stardate: 03/3003\nFleet contact report",
            "Stardate: 02/3002\nOlder report",
        ]),
    );
    runtime.queued_mail.push(incoming_mail(
        2,
        1,
        mail_year,
        "Current message",
        "Newest body",
    ));
    runtime.queued_mail.push(incoming_mail(
        3,
        1,
        mail_year - 1,
        "Older message",
        "Older body",
    ));
    save_runtime_state(&fixture_dir, &runtime);

    let mut app = App::load(AppConfig {
        game_dir: fixture_dir.clone(),
        player_record_index_1_based: 1,
        export_root: None,
        queue_dir: None,
        session_timeout_secs: None,
        game_config: Default::default(),
    })
    .expect("app should load");
    advance_to_main_menu(&mut app);

    assert_eq!(
        apply_action(&mut app, Action::Startup(StartupAction::OpenReports)),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(
            &mut app,
            Action::Messaging(MessagingAction::SetInboxTypeFilterMessages)
        ),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(
            &mut app,
            Action::Messaging(MessagingAction::OpenInboxYearPrompt)
        ),
        AppOutcome::Continue
    );
    for ch in mail_year.to_string().chars() {
        assert_eq!(
            apply_action(
                &mut app,
                Action::Messaging(MessagingAction::AppendInboxYearChar(ch))
            ),
            AppOutcome::Continue
        );
    }
    assert_eq!(
        apply_action(
            &mut app,
            Action::Messaging(MessagingAction::SubmitInboxYearInput)
        ),
        AppOutcome::Continue
    );

    let mut filtered = CaptureTerminal::new();
    app.render(&mut filtered)
        .expect("filtered inbox should render");
    assert!(
        filtered
            .lines
            .iter()
            .any(|line| line.contains("[Messages ("))
    );
    assert!(
        filtered
            .lines
            .iter()
            .any(|line| line.contains(&format!("Year: {mail_year}")))
    );
    assert!(
        filtered
            .lines
            .iter()
            .any(|line| line.contains("Current message"))
    );
    assert!(
        !filtered
            .lines
            .iter()
            .any(|line| line.contains("Older message"))
    );
    assert!(!filtered.lines.iter().any(|line| line.contains("Scout")));

    assert_eq!(
        apply_action(
            &mut app,
            Action::Messaging(MessagingAction::OpenInboxDeleteConfirm)
        ),
        AppOutcome::Continue
    );
    let mut confirm = CaptureTerminal::new();
    app.render(&mut confirm)
        .expect("delete confirm should render");
    assert!(confirm.line(24).contains("Delete item 03? [Y]/N ->"));

    assert_eq!(
        apply_action(
            &mut app,
            Action::Messaging(MessagingAction::ConfirmDeleteInboxItem)
        ),
        AppOutcome::Continue
    );

    let reloaded = latest_runtime_state(&fixture_dir);
    assert!(reloaded.queued_mail[0].recipient_deleted);
}

#[test]
fn reports_inbox_rejects_no_match_year_filter_without_blanking_the_table() {
    let fixture_dir = temp_game_copy();
    let mut runtime = latest_runtime_state(&fixture_dir);
    let mail_year = runtime.game_data.conquest.game_year - 1;
    clear_runtime_report_blocks(&mut runtime);
    runtime.queued_mail.clear();
    runtime
        .queued_mail
        .push(incoming_mail(2, 1, mail_year, "Visible", "Visible body"));
    save_runtime_state(&fixture_dir, &runtime);

    let mut app = App::load(AppConfig {
        game_dir: fixture_dir.clone(),
        player_record_index_1_based: 1,
        export_root: None,
        queue_dir: None,
        session_timeout_secs: None,
        game_config: Default::default(),
    })
    .expect("app should load");
    for _ in 0..64 {
        if matches!(
            app.current_screen(),
            ScreenId::MainMenu | ScreenId::Startup(StartupPhase::Messages)
        ) {
            break;
        }
        app.advance_startup();
    }
    assert!(matches!(
        app.current_screen(),
        ScreenId::MainMenu | ScreenId::Startup(StartupPhase::Messages)
    ));

    assert_eq!(
        apply_action(&mut app, Action::Startup(StartupAction::OpenReports)),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(
            &mut app,
            Action::Messaging(MessagingAction::OpenInboxYearPrompt)
        ),
        AppOutcome::Continue
    );
    for ch in (mail_year + 10).to_string().chars() {
        assert_eq!(
            apply_action(
                &mut app,
                Action::Messaging(MessagingAction::AppendInboxYearChar(ch))
            ),
            AppOutcome::Continue
        );
    }
    assert_eq!(
        apply_action(
            &mut app,
            Action::Messaging(MessagingAction::SubmitInboxYearInput)
        ),
        AppOutcome::Continue
    );

    assert_eq!(app.messaging.inbox_year_filter, None);
    assert!(app.messaging.inbox_feedback.is_none());
    assert_eq!(
        app.messaging.inbox_prompt_mode,
        nc_game::domains::messaging::state::InboxPromptMode::Normal
    );

    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("inbox should still render");
    assert!(terminal.line(0).contains("Year: All"));
    assert!(terminal.lines.iter().any(|line| line.contains("Visible")));
    assert!(
        !terminal
            .lines
            .iter()
            .any(|line| line.contains("<no matching items>"))
    );
}

#[test]
fn reports_inbox_question_mark_opens_popup_help_with_inbox_commands() {
    let fixture_dir = temp_game_copy();
    let mut app = App::load(AppConfig {
        game_dir: fixture_dir.clone(),
        player_record_index_1_based: 1,
        export_root: None,
        queue_dir: None,
        session_timeout_secs: None,
        game_config: Default::default(),
    })
    .expect("app should load");
    advance_to_main_menu(&mut app);

    assert_eq!(
        apply_action(&mut app, Action::Startup(StartupAction::OpenReports)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::Reports);

    let popup_action = app.handle_key(key(KeyCode::Char('?')));
    assert_eq!(popup_action, Action::OpenPopupHelp);
    assert_eq!(apply_action(&mut app, popup_action), AppOutcome::Continue);

    let popup = app.popup_help.as_ref().expect("popup help should open");
    assert_eq!(popup.title, "INBOX COMMANDS");
    assert!(
        popup
            .lines
            .iter()
            .any(|line| line.contains("M") && line.contains("Messages"))
    );
    assert!(
        popup
            .lines
            .iter()
            .any(|line| line.contains("R") && line.contains("Reports"))
    );
    assert!(
        popup
            .lines
            .iter()
            .any(|line| line.contains("S") && line.contains("Sent"))
    );
    assert!(popup.lines.iter().any(|line| line.contains("Tab")));
    assert!(popup.lines.iter().any(|line| line.contains("Digits")));
    assert!(popup.lines.iter().any(|line| line.contains("?")));
    assert!(!popup.lines.iter().any(|line| line.contains("M/R/A")));
    assert!(!popup.lines.iter().any(|line| line.contains("J/K")));
    assert!(!popup.lines.iter().any(|line| line.contains("^U/^D")));
    assert!(!popup.lines.iter().any(|line| line.contains("Backspace")));
}

#[test]
fn reports_inbox_enter_moves_focus_to_preview_when_id_input_is_empty() {
    let fixture_dir = temp_game_copy();
    let mut runtime = latest_runtime_state(&fixture_dir);
    let mail_year = runtime.game_data.conquest.game_year - 1;
    clear_runtime_report_blocks(&mut runtime);
    runtime.queued_mail.clear();
    runtime
        .queued_mail
        .push(incoming_mail(2, 1, mail_year, "Visible", "Visible body"));
    save_runtime_state(&fixture_dir, &runtime);

    let mut app = App::load(AppConfig {
        game_dir: fixture_dir.clone(),
        player_record_index_1_based: 1,
        export_root: None,
        queue_dir: None,
        session_timeout_secs: None,
        game_config: Default::default(),
    })
    .expect("app should load");
    for _ in 0..64 {
        if matches!(
            app.current_screen(),
            ScreenId::MainMenu | ScreenId::Startup(StartupPhase::Messages)
        ) {
            break;
        }
        app.advance_startup();
    }
    assert!(matches!(
        app.current_screen(),
        ScreenId::MainMenu | ScreenId::Startup(StartupPhase::Messages)
    ));

    assert_eq!(
        apply_action(&mut app, Action::Startup(StartupAction::OpenReports)),
        AppOutcome::Continue
    );
    assert_eq!(app.messaging.inbox_focus, InboxFocus::Inbox);
    assert!(app.messaging.inbox_id_input.is_empty());

    let enter = app.handle_key(key(KeyCode::Enter));
    assert_eq!(apply_action(&mut app, enter), AppOutcome::Continue);
    assert_eq!(app.messaging.inbox_focus, InboxFocus::Preview);
}

#[test]
fn reports_inbox_typed_id_jump_moves_selection_immediately() {
    let fixture_dir = temp_game_copy();
    let mut runtime = latest_runtime_state(&fixture_dir);
    let mail_year = runtime.game_data.conquest.game_year - 1;
    runtime
        .queued_mail
        .push(incoming_mail(2, 1, mail_year, "Alpha", "Body Alpha"));
    runtime
        .queued_mail
        .push(incoming_mail(3, 1, mail_year, "Beta", "Body Beta"));
    runtime
        .queued_mail
        .push(incoming_mail(4, 1, mail_year, "Gamma", "Body Gamma"));
    save_runtime_state(&fixture_dir, &runtime);

    let mut app = App::load(AppConfig {
        game_dir: fixture_dir.clone(),
        player_record_index_1_based: 1,
        export_root: None,
        queue_dir: None,
        session_timeout_secs: None,
        game_config: Default::default(),
    })
    .expect("app should load");
    advance_to_main_menu(&mut app);

    assert_eq!(
        apply_action(&mut app, Action::Startup(StartupAction::OpenReports)),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(
            &mut app,
            Action::Messaging(MessagingAction::AppendInboxIdChar('3'))
        ),
        AppOutcome::Continue
    );
    assert!(app.messaging.inbox_id_input.is_empty());

    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal).expect("inbox should render");
    assert!(terminal.line(0).contains("INBOX:"));
    assert!(terminal.line(1).starts_with('┌'));
    assert!(terminal.line(24).contains("<TAB> <Q> [03] ->"));
    assert!(!terminal.line(24).contains("-> 3"));
    assert!(terminal.lines.iter().any(|line| line.contains("Alpha")));
    assert!(
        terminal
            .lines
            .iter()
            .any(|line| line.contains("Body Alpha"))
    );
}

#[test]
fn reports_inbox_delete_keeps_sparse_session_ids_without_empty_rows() {
    let fixture_dir = temp_game_copy();
    let mut runtime = latest_runtime_state(&fixture_dir);
    let mail_year = runtime.game_data.conquest.game_year - 1;
    clear_runtime_report_blocks(&mut runtime);
    runtime.queued_mail.clear();
    for subject in ["Alpha", "Beta", "Gamma", "Delta"] {
        runtime
            .queued_mail
            .push(incoming_mail(2, 1, mail_year, subject, subject));
    }
    save_runtime_state(&fixture_dir, &runtime);

    let mut app = App::load(AppConfig {
        game_dir: fixture_dir.clone(),
        player_record_index_1_based: 1,
        export_root: None,
        queue_dir: None,
        session_timeout_secs: None,
        game_config: Default::default(),
    })
    .expect("app should load");
    for _ in 0..64 {
        if matches!(
            app.current_screen(),
            ScreenId::MainMenu | ScreenId::Startup(StartupPhase::Messages)
        ) {
            break;
        }
        app.advance_startup();
    }
    assert!(matches!(
        app.current_screen(),
        ScreenId::MainMenu | ScreenId::Startup(StartupPhase::Messages)
    ));

    assert_eq!(
        apply_action(&mut app, Action::Startup(StartupAction::OpenReports)),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(
            &mut app,
            Action::Messaging(MessagingAction::AppendInboxIdChar('3'))
        ),
        AppOutcome::Continue
    );
    assert_eq!(app.current_inbox_display_id(), "03");

    assert_eq!(
        apply_action(
            &mut app,
            Action::Messaging(MessagingAction::OpenInboxDeleteConfirm)
        ),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(
            &mut app,
            Action::Messaging(MessagingAction::ConfirmDeleteInboxItem)
        ),
        AppOutcome::Continue
    );

    let mut deleted = CaptureTerminal::new();
    app.render(&mut deleted)
        .expect("deleted inbox should render");
    assert!(
        deleted
            .lines
            .iter()
            .any(|line| line.contains("Deleted item 03."))
    );
    assert!(!deleted.lines.iter().any(|line| line.contains("│03│")));
    assert!(deleted.lines.iter().any(|line| line.contains("│04│")));
    assert_eq!(
        deleted
            .lines
            .iter()
            .filter(|line| {
                line.contains("│01│") || line.contains("│02│") || line.contains("│04│")
            })
            .count(),
        3
    );

    assert_eq!(
        apply_action(&mut app, Action::ReturnToCommandMenu),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Startup(StartupAction::OpenReports)),
        AppOutcome::Continue
    );

    let mut reopened = CaptureTerminal::new();
    app.render(&mut reopened)
        .expect("reopened inbox should render");
    assert!(!reopened.lines.iter().any(|line| line.contains("│03│")));
    assert!(reopened.lines.iter().any(|line| line.contains("│04│")));
}

#[test]
fn reports_inbox_leaves_scrollbar_gutter_when_many_items_exist() {
    let fixture_dir = temp_game_copy();
    let mut runtime = latest_runtime_state(&fixture_dir);
    let mail_year = runtime.game_data.conquest.game_year - 1;
    for idx in 0..12 {
        runtime.queued_mail.push(incoming_mail(
            2,
            1,
            mail_year,
            &format!("Message {}", idx + 1),
            "Body",
        ));
    }
    save_runtime_state(&fixture_dir, &runtime);

    let mut app = App::load(AppConfig {
        game_dir: fixture_dir.clone(),
        player_record_index_1_based: 1,
        export_root: None,
        queue_dir: None,
        session_timeout_secs: None,
        game_config: Default::default(),
    })
    .expect("app should load");
    for _ in 0..64 {
        if app.current_screen() == ScreenId::MainMenu {
            break;
        }
        app.advance_startup();
    }
    assert_eq!(app.current_screen(), ScreenId::MainMenu);

    assert_eq!(
        apply_action(&mut app, Action::Startup(StartupAction::OpenReports)),
        AppOutcome::Continue
    );

    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal).expect("inbox should render");
    assert!(terminal.lines.iter().any(|line| line.ends_with('^')));
    assert!(terminal.lines.iter().any(|line| line.ends_with('|')));
    assert!(terminal.lines.iter().any(|line| line.ends_with('v')));
}

#[test]
fn reports_inbox_long_preview_scrolls_and_clamps_without_panicking() {
    let fixture_dir = temp_game_copy();
    let mut runtime = latest_runtime_state(&fixture_dir);
    let mail_year = runtime.game_data.conquest.game_year - 1;
    runtime
        .queued_mail
        .push(incoming_mail(2, 1, mail_year, "Older", "Short body"));
    let long_body = (0..40)
        .map(|idx| format!("Line {idx:02} {}", "x".repeat(90)))
        .collect::<Vec<_>>()
        .join("\n");
    runtime
        .queued_mail
        .push(incoming_mail(3, 1, mail_year, "Long", &long_body));
    save_runtime_state(&fixture_dir, &runtime);

    let mut app = App::load(AppConfig {
        game_dir: fixture_dir.clone(),
        player_record_index_1_based: 1,
        export_root: None,
        queue_dir: None,
        session_timeout_secs: None,
        game_config: Default::default(),
    })
    .expect("app should load");
    for _ in 0..64 {
        if matches!(
            app.current_screen(),
            ScreenId::MainMenu | ScreenId::Startup(StartupPhase::Messages)
        ) {
            break;
        }
        app.advance_startup();
    }
    assert!(matches!(
        app.current_screen(),
        ScreenId::MainMenu | ScreenId::Startup(StartupPhase::Messages)
    ));

    assert_eq!(
        apply_action(&mut app, Action::Startup(StartupAction::OpenReports)),
        AppOutcome::Continue
    );
    let tab_to_preview = app.handle_key(key(KeyCode::Tab));
    assert_eq!(apply_action(&mut app, tab_to_preview), AppOutcome::Continue);
    assert_eq!(app.messaging.inbox_focus, InboxFocus::Preview);
    assert_eq!(app.messaging.inbox_preview_scroll, 0);

    let mut first = CaptureTerminal::new();
    app.render(&mut first).expect("initial preview render");
    assert!(first.lines.iter().any(|line| line.contains("Line 00")));

    let page_down = app.handle_key(key(KeyCode::PageDown));
    assert_eq!(apply_action(&mut app, page_down), AppOutcome::Continue);
    assert!(app.messaging.inbox_preview_scroll > 0);
    let after_first_page = app.messaging.inbox_preview_scroll;

    let mut paged = CaptureTerminal::new();
    app.render(&mut paged).expect("paged preview render");
    assert!(paged.lines.iter().any(|line| line.contains("Line 04")));

    for _ in 0..12 {
        let page_down = app.handle_key(key(KeyCode::PageDown));
        assert_eq!(apply_action(&mut app, page_down), AppOutcome::Continue);
    }
    let mut bottom_scroll = app.messaging.inbox_preview_scroll;
    assert!(bottom_scroll >= after_first_page);
    for _ in 0..32 {
        let page_down = app.handle_key(key(KeyCode::PageDown));
        assert_eq!(apply_action(&mut app, page_down), AppOutcome::Continue);
        if app.messaging.inbox_preview_scroll == bottom_scroll {
            break;
        }
        bottom_scroll = app.messaging.inbox_preview_scroll;
    }
    let page_down = app.handle_key(key(KeyCode::PageDown));
    assert_eq!(apply_action(&mut app, page_down), AppOutcome::Continue);
    assert_eq!(app.messaging.inbox_preview_scroll, bottom_scroll);

    let page_up = app.handle_key(key(KeyCode::PageUp));
    assert_eq!(apply_action(&mut app, page_up), AppOutcome::Continue);
    assert!(app.messaging.inbox_preview_scroll < bottom_scroll);

    let tab_to_inbox = app.handle_key(key(KeyCode::Tab));
    assert_eq!(apply_action(&mut app, tab_to_inbox), AppOutcome::Continue);
    assert_eq!(app.messaging.inbox_focus, InboxFocus::Inbox);
    let move_down = app.handle_key(key(KeyCode::Down));
    assert_eq!(apply_action(&mut app, move_down), AppOutcome::Continue);
    assert_eq!(app.messaging.inbox_preview_scroll, 0);

    let mut switched = CaptureTerminal::new();
    app.render(&mut switched).expect("switched preview render");
    assert!(
        switched
            .lines
            .iter()
            .any(|line| line.contains("Short body"))
    );
}

#[test]
fn apply_action_confirms_before_discarding_composed_message() {
    let fixture_dir = temp_game_copy();
    let mut app = App::load(AppConfig {
        game_dir: fixture_dir.clone(),
        player_record_index_1_based: 1,
        export_root: None,
        queue_dir: None,
        session_timeout_secs: None,
        game_config: Default::default(),
    })
    .expect("app should load");

    assert_eq!(
        apply_action(
            &mut app,
            Action::Messaging(MessagingAction::OpenComposeRecipient)
        ),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(
            &mut app,
            Action::Messaging(MessagingAction::AppendComposeRecipientChar('2'))
        ),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(
            &mut app,
            Action::Messaging(MessagingAction::SubmitComposeRecipient)
        ),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(
            &mut app,
            Action::Messaging(MessagingAction::SubmitComposeSubject)
        ),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::ComposeMessageBody);

    assert_eq!(
        apply_action(
            &mut app,
            Action::Messaging(MessagingAction::OpenComposeDiscardConfirm)
        ),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::ComposeMessageDiscardConfirm);

    assert_eq!(
        apply_action(
            &mut app,
            Action::Messaging(MessagingAction::OpenComposeBody)
        ),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::ComposeMessageBody);

    assert_eq!(
        apply_action(
            &mut app,
            Action::Messaging(MessagingAction::OpenComposeDiscardConfirm)
        ),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(
            &mut app,
            Action::Messaging(MessagingAction::ConfirmDiscardComposedMessage)
        ),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::ComposeMessageRecipient);
}

#[test]
fn compose_body_navigation_tracks_visual_wrapped_lines() {
    let fixture_dir = temp_game_copy();
    let mut app = App::load(AppConfig {
        game_dir: fixture_dir.clone(),
        player_record_index_1_based: 1,
        export_root: None,
        queue_dir: None,
        session_timeout_secs: None,
        game_config: Default::default(),
    })
    .expect("app should load");

    assert_eq!(
        apply_action(
            &mut app,
            Action::Messaging(MessagingAction::OpenComposeRecipient)
        ),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(
            &mut app,
            Action::Messaging(MessagingAction::AppendComposeRecipientChar('2'))
        ),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(
            &mut app,
            Action::Messaging(MessagingAction::SubmitComposeRecipient)
        ),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(
            &mut app,
            Action::Messaging(MessagingAction::SubmitComposeSubject)
        ),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::ComposeMessageBody);

    app.messaging.compose_body = format!("{} splitword", "a".repeat(78));
    app.messaging.compose_body_cursor_row = 1;
    app.messaging.compose_body_cursor_col = 4;

    assert_eq!(
        apply_action(
            &mut app,
            Action::Messaging(MessagingAction::MoveComposeBodyCursorHome)
        ),
        AppOutcome::Continue
    );
    assert_eq!(app.messaging.compose_body_cursor_row, 1);
    assert_eq!(app.messaging.compose_body_cursor_col, 0);

    assert_eq!(
        apply_action(
            &mut app,
            Action::Messaging(MessagingAction::MoveComposeBodyCursorUp)
        ),
        AppOutcome::Continue
    );
    assert_eq!(app.messaging.compose_body_cursor_row, 0);
    assert_eq!(app.messaging.compose_body_cursor_col, 0);

    app.messaging.compose_body_cursor_row = 0;
    app.messaging.compose_body_cursor_col = 4;
    assert_eq!(
        apply_action(
            &mut app,
            Action::Messaging(MessagingAction::MoveComposeBodyCursorDown)
        ),
        AppOutcome::Continue
    );
    assert_eq!(app.messaging.compose_body_cursor_row, 1);
    assert_eq!(app.messaging.compose_body_cursor_col, 4);

    assert_eq!(
        apply_action(
            &mut app,
            Action::Messaging(MessagingAction::MoveComposeBodyCursorEnd)
        ),
        AppOutcome::Continue
    );
    assert_eq!(app.messaging.compose_body_cursor_row, 1);
    assert_eq!(app.messaging.compose_body_cursor_col, 9);
}

#[test]
fn compose_body_allows_typing_hjkl_without_moving_cursor() {
    let fixture_dir = temp_game_copy();
    let mut app = App::load(AppConfig {
        game_dir: fixture_dir.clone(),
        player_record_index_1_based: 1,
        export_root: None,
        queue_dir: None,
        session_timeout_secs: None,
        game_config: Default::default(),
    })
    .expect("app should load");

    app.current_screen = ScreenId::ComposeMessageBody;
    app.messaging.compose_body.clear();
    app.messaging.compose_body_cursor_row = 0;
    app.messaging.compose_body_cursor_col = 0;

    for ch in ['h', 'j', 'k', 'l'] {
        let action = app.handle_key(key(KeyCode::Char(ch)));
        assert_eq!(
            apply_action(&mut app, action),
            AppOutcome::Continue,
            "keypress {ch} should apply as text entry"
        );
    }

    assert_eq!(app.messaging.compose_body, "hjkl");
    assert_eq!(app.messaging.compose_body_cursor_row, 0);
    assert_eq!(app.messaging.compose_body_cursor_col, 4);
}

#[test]
fn compose_body_popup_help_lists_send_and_cancel_shortcuts() {
    let fixture_dir = temp_game_copy();
    let mut app = App::load(AppConfig {
        game_dir: fixture_dir.clone(),
        player_record_index_1_based: 1,
        export_root: None,
        queue_dir: None,
        session_timeout_secs: None,
        game_config: Default::default(),
    })
    .expect("app should load");

    app.current_screen = ScreenId::ComposeMessageBody;

    let popup_action = app.handle_key(ctrl_key('g'));
    assert_eq!(popup_action, Action::OpenPopupHelp);
    assert_eq!(apply_action(&mut app, popup_action), AppOutcome::Continue);

    let popup = app.popup_help.as_ref().expect("popup help should open");
    assert_eq!(popup.title, "MESSAGE EDITOR HELP");
    assert!(popup.lines.iter().any(|line| line.contains("^E")));
    assert!(popup.lines.iter().any(|line| line.contains("^X")));
    assert!(!popup.lines.iter().any(|line| line.contains("Arrows")));
    assert!(!popup.lines.iter().any(|line| line.contains("Backspace")));
    assert!(!popup.lines.iter().any(|line| line.contains("Delete")));
}

#[test]
fn compose_body_cursor_can_move_down_from_empty_editor_without_mutating_body() {
    let fixture_dir = temp_game_copy();
    let mut app = App::load(AppConfig {
        game_dir: fixture_dir.clone(),
        player_record_index_1_based: 1,
        export_root: None,
        queue_dir: None,
        session_timeout_secs: None,
        game_config: Default::default(),
    })
    .expect("app should load");

    app.current_screen = ScreenId::ComposeMessageBody;
    app.messaging.compose_body.clear();
    app.messaging.compose_body_cursor_row = 0;
    app.messaging.compose_body_cursor_col = 0;

    assert_eq!(
        apply_action(
            &mut app,
            Action::Messaging(MessagingAction::MoveComposeBodyCursorDown)
        ),
        AppOutcome::Continue
    );
    assert_eq!(app.messaging.compose_body, "");
    assert_eq!(app.messaging.compose_body_cursor_row, 1);
    assert_eq!(app.messaging.compose_body_cursor_col, 0);
}

#[test]
fn compose_body_cursor_can_move_into_blank_lines_and_type_there() {
    let fixture_dir = temp_game_copy();
    let mut app = App::load(AppConfig {
        game_dir: fixture_dir.clone(),
        player_record_index_1_based: 1,
        export_root: None,
        queue_dir: None,
        session_timeout_secs: None,
        game_config: Default::default(),
    })
    .expect("app should load");

    app.current_screen = ScreenId::ComposeMessageBody;
    app.messaging.compose_body = "abc".to_string();
    app.messaging.compose_body_cursor_row = 0;
    app.messaging.compose_body_cursor_col = 3;

    assert_eq!(
        apply_action(
            &mut app,
            Action::Messaging(MessagingAction::MoveComposeBodyCursorDown)
        ),
        AppOutcome::Continue
    );
    assert_eq!(app.messaging.compose_body, "abc");
    assert_eq!(app.messaging.compose_body_cursor_row, 1);
    assert_eq!(app.messaging.compose_body_cursor_col, 3);

    assert_eq!(
        apply_action(
            &mut app,
            Action::Messaging(MessagingAction::AppendComposeBodyChar('Z'))
        ),
        AppOutcome::Continue
    );
    assert_eq!(app.messaging.compose_body, "abc\n   Z");
    assert_eq!(app.messaging.compose_body_cursor_row, 1);
    assert_eq!(app.messaging.compose_body_cursor_col, 4);
}

#[test]
fn compose_body_cursor_can_move_right_past_end_of_text_without_mutating_body() {
    let fixture_dir = temp_game_copy();
    let mut app = App::load(AppConfig {
        game_dir: fixture_dir.clone(),
        player_record_index_1_based: 1,
        export_root: None,
        queue_dir: None,
        session_timeout_secs: None,
        game_config: Default::default(),
    })
    .expect("app should load");

    app.current_screen = ScreenId::ComposeMessageBody;
    app.messaging.compose_body = "abc".to_string();
    app.messaging.compose_body_cursor_row = 0;
    app.messaging.compose_body_cursor_col = 3;

    assert_eq!(
        apply_action(
            &mut app,
            Action::Messaging(MessagingAction::MoveComposeBodyCursorRight)
        ),
        AppOutcome::Continue
    );
    assert_eq!(app.messaging.compose_body, "abc");
    assert_eq!(app.messaging.compose_body_cursor_row, 0);
    assert_eq!(app.messaging.compose_body_cursor_col, 4);
}

#[test]
fn compose_body_tab_inserts_four_spaces() {
    let fixture_dir = temp_game_copy();
    let mut app = App::load(AppConfig {
        game_dir: fixture_dir.clone(),
        player_record_index_1_based: 1,
        export_root: None,
        queue_dir: None,
        session_timeout_secs: None,
        game_config: Default::default(),
    })
    .expect("app should load");

    app.current_screen = ScreenId::ComposeMessageBody;
    app.messaging.compose_body = "abc".to_string();
    app.messaging.compose_body_cursor_row = 0;
    app.messaging.compose_body_cursor_col = 3;

    let tab = app.handle_key(key(KeyCode::Tab));
    assert_eq!(apply_action(&mut app, tab), AppOutcome::Continue);
    assert_eq!(app.messaging.compose_body, "abc    ");
    assert_eq!(app.messaging.compose_body_cursor_row, 0);
    assert_eq!(app.messaging.compose_body_cursor_col, 7);
}

#[test]
fn compose_body_tab_pushes_existing_text_right() {
    let fixture_dir = temp_game_copy();
    let mut app = App::load(AppConfig {
        game_dir: fixture_dir.clone(),
        player_record_index_1_based: 1,
        export_root: None,
        queue_dir: None,
        session_timeout_secs: None,
        game_config: Default::default(),
    })
    .expect("app should load");

    app.current_screen = ScreenId::ComposeMessageBody;
    app.messaging.compose_body = "abcxyz".to_string();
    app.messaging.compose_body_cursor_row = 0;
    app.messaging.compose_body_cursor_col = 3;

    let tab = app.handle_key(key(KeyCode::Tab));
    assert_eq!(apply_action(&mut app, tab), AppOutcome::Continue);
    assert_eq!(app.messaging.compose_body, "abc    xyz");
    assert_eq!(app.messaging.compose_body_cursor_row, 0);
    assert_eq!(app.messaging.compose_body_cursor_col, 7);
}

#[test]
fn compose_body_cursor_left_and_right_do_not_page_jump_on_short_first_line() {
    let fixture_dir = temp_game_copy();
    let mut app = App::load(AppConfig {
        game_dir: fixture_dir.clone(),
        player_record_index_1_based: 1,
        export_root: None,
        queue_dir: None,
        session_timeout_secs: None,
        game_config: Default::default(),
    })
    .expect("app should load");

    app.current_screen = ScreenId::ComposeMessageBody;
    app.messaging.compose_body = "x".to_string();
    app.messaging.compose_body_cursor_row = 0;
    app.messaging.compose_body_cursor_col = 1;

    assert_eq!(
        apply_action(
            &mut app,
            Action::Messaging(MessagingAction::MoveComposeBodyCursorLeft)
        ),
        AppOutcome::Continue
    );
    assert_eq!(app.messaging.compose_body_cursor_row, 0);
    assert_eq!(app.messaging.compose_body_cursor_col, 0);

    assert_eq!(
        apply_action(
            &mut app,
            Action::Messaging(MessagingAction::MoveComposeBodyCursorRight)
        ),
        AppOutcome::Continue
    );
    assert_eq!(app.messaging.compose_body_cursor_row, 0);
    assert_eq!(app.messaging.compose_body_cursor_col, 1);

    assert_eq!(
        apply_action(
            &mut app,
            Action::Messaging(MessagingAction::MoveComposeBodyCursorRight)
        ),
        AppOutcome::Continue
    );
    assert_eq!(app.messaging.compose_body_cursor_row, 0);
    assert_eq!(app.messaging.compose_body_cursor_col, 2);
}

#[test]
fn compose_body_cursor_preserves_visual_column_in_blank_canvas_space() {
    let fixture_dir = temp_game_copy();
    let mut app = App::load(AppConfig {
        game_dir: fixture_dir.clone(),
        player_record_index_1_based: 1,
        export_root: None,
        queue_dir: None,
        session_timeout_secs: None,
        game_config: Default::default(),
    })
    .expect("app should load");

    app.current_screen = ScreenId::ComposeMessageBody;
    app.messaging.compose_body = "abc".to_string();
    app.messaging.compose_body_cursor_row = 2;
    app.messaging.compose_body_cursor_col = 8;

    assert_eq!(
        apply_action(
            &mut app,
            Action::Messaging(MessagingAction::MoveComposeBodyCursorUp)
        ),
        AppOutcome::Continue
    );
    assert_eq!(app.messaging.compose_body_cursor_row, 1);
    assert_eq!(app.messaging.compose_body_cursor_col, 8);

    assert_eq!(
        apply_action(
            &mut app,
            Action::Messaging(MessagingAction::MoveComposeBodyCursorDown)
        ),
        AppOutcome::Continue
    );
    assert_eq!(app.messaging.compose_body_cursor_row, 2);
    assert_eq!(app.messaging.compose_body_cursor_col, 8);
}

#[test]
fn question_mark_is_text_in_the_message_editor() {
    let fixture_dir = temp_game_copy();
    let mut app = App::load(AppConfig {
        game_dir: fixture_dir.clone(),
        player_record_index_1_based: 1,
        export_root: None,
        queue_dir: None,
        session_timeout_secs: None,
        game_config: Default::default(),
    })
    .expect("app should load");
    let question_mark = KeyEvent::new(KeyCode::Char('?'), KeyModifiers::SHIFT);

    apply_action(
        &mut app,
        Action::Messaging(MessagingAction::OpenComposeRecipient),
    );
    apply_action(
        &mut app,
        Action::Messaging(MessagingAction::AppendComposeRecipientChar('2')),
    );
    apply_action(
        &mut app,
        Action::Messaging(MessagingAction::SubmitComposeRecipient),
    );
    assert_eq!(app.current_screen(), ScreenId::ComposeMessageSubject);
    assert_eq!(
        app.handle_key(question_mark),
        Action::Messaging(MessagingAction::AppendComposeSubjectChar('?'))
    );

    apply_action(
        &mut app,
        Action::Messaging(MessagingAction::AppendComposeSubjectChar('Q')),
    );
    apply_action(
        &mut app,
        Action::Messaging(MessagingAction::SubmitComposeSubject),
    );
    assert_eq!(app.current_screen(), ScreenId::ComposeMessageBody);
    assert_eq!(
        app.handle_key(question_mark),
        Action::Messaging(MessagingAction::AppendComposeBodyChar('?'))
    );
}
