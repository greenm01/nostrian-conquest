use crate::support::*;

#[test]
fn delete_reviewables_stays_on_general_menu_with_notice_when_nothing_is_reviewable() {
    let fixture_dir = temp_game_copy();
    let mut state = latest_runtime_state(&fixture_dir);
    clear_runtime_report_blocks(&mut state);
    save_runtime_state(&fixture_dir, &state);

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
        apply_action(&mut app, Action::OpenGeneralMenu),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(
            &mut app,
            Action::Messaging(MessagingAction::OpenDeleteReviewables)
        ),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::GeneralMenu);

    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("general menu should render empty-reviewables notice");
    assert!(
        terminal
            .lines
            .iter()
            .any(|line| line.contains("No reports to delete."))
    );
}

#[test]
fn delete_reports_ignores_classic_message_pending_flags() {
    let fixture_dir = temp_game_copy();
    let mut state = latest_runtime_state(&fixture_dir);
    clear_runtime_report_blocks(&mut state);
    state.game_data.players[0].messages_first =
        (state.game_data.players[0].messages_first & !0xff) | 1;
    state.game_data.players[0].messages_last =
        (state.game_data.players[0].messages_last & !0xff) | 1;
    save_runtime_state(&fixture_dir, &state);

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
        apply_action(&mut app, Action::OpenGeneralMenu),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(
            &mut app,
            Action::Messaging(MessagingAction::OpenDeleteReviewables)
        ),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::GeneralMenu);
    assert!(!app.messaging.delete_reviewables_prompt_active);
    let runtime = latest_runtime_state(&fixture_dir);
    assert_eq!(runtime.game_data.players[0].messages_first as u8, 1);
    assert_eq!(runtime.game_data.players[0].messages_last as u8, 1);
}

#[test]
fn startup_results_only_pending_flags_do_not_route_into_messages() {
    let fixture_dir = temp_game_copy();
    let mut state = latest_runtime_state(&fixture_dir);
    clear_runtime_report_blocks(&mut state);
    state.game_data.players[0].messages_last =
        (state.game_data.players[0].messages_last & !0xff) | 1;
    save_runtime_state(&fixture_dir, &state);

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
        app.current_screen(),
        ScreenId::Startup(StartupPhase::Splash)
    );
    let mut splash_terminal = CaptureTerminal::new();
    app.render(&mut splash_terminal)
        .expect("startup splash should render");
    assert!(splash_terminal.line(24).starts_with(' '));
    assert!(
        splash_terminal
            .line(24)
            .contains("View the game introduction? Y/[N] ->")
    );

    assert_eq!(
        apply_action(&mut app, Action::Startup(StartupAction::Advance)),
        AppOutcome::Continue
    );
    assert_eq!(
        app.current_screen(),
        ScreenId::Startup(StartupPhase::Splash)
    );
    let mut splash_intro_terminal = CaptureTerminal::new();
    app.render(&mut splash_intro_terminal)
        .expect("startup splash intro should render");
    assert!(splash_intro_terminal.line(24).starts_with(' '));
    assert!(
        splash_intro_terminal
            .lines
            .iter()
            .any(|line| line.contains("Beyond the mapped frontiers"))
    );

    for _ in 0..16 {
        if app.current_screen() == ScreenId::Startup(StartupPhase::LoginSummary) {
            let mut terminal = CaptureTerminal::new();
            app.render(&mut terminal)
                .expect("login summary should render");
            assert!(terminal.line(24).starts_with(' '));
            assert!(
                terminal
                    .lines
                    .iter()
                    .any(|line| line.contains("The year is:"))
            );
            break;
        }
        app.advance_startup();
    }

    assert_eq!(
        apply_action(&mut app, Action::Startup(StartupAction::Advance)),
        AppOutcome::Continue
    );
    assert_eq!(
        app.current_screen(),
        ScreenId::Startup(StartupPhase::Results)
    );

    let mut results_terminal = CaptureTerminal::new();
    app.render(&mut results_terminal)
        .expect("startup results should render");
    assert!(results_terminal.line(24).starts_with(' '));
    assert!(results_terminal.lines.iter().any(|line| {
        line.contains("Reports are marked pending, but no review text is available yet.")
    }));

    assert_eq!(
        apply_action(&mut app, Action::Startup(StartupAction::Advance)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::MainMenu);

    advance_to_main_menu(&mut app);
    assert_eq!(app.current_screen(), ScreenId::MainMenu);

    assert_eq!(
        apply_action(&mut app, Action::Startup(StartupAction::OpenReports)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::Reports);

    let mut reports_terminal = CaptureTerminal::new();
    app.render(&mut reports_terminal)
        .expect("reports screen should render");
    assert!(reports_terminal.line(0).contains("INBOX:"));
    assert!(reports_terminal.line(1).starts_with('┌'));
    assert!(reports_terminal.line(2).contains("ID"));
    assert!(reports_terminal.line(2).contains("Stardate"));
    assert!(!reports_terminal.line(2).contains("Type"));
    assert!(reports_terminal.line(2).contains("Subject"));
    assert!(reports_terminal.line(5).starts_with('┌'));
    assert!(
        reports_terminal
            .lines
            .iter()
            .any(|line| line.contains("<no matching items>"))
    );
}
#[test]
fn startup_reviews_results_then_messages_then_enters_main_menu() {
    let fixture_dir = temp_game_copy();
    let mut state = latest_runtime_state(&fixture_dir);
    set_runtime_report_blocks(&mut state, b"Fleet battle report");
    state.queued_mail.push(incoming_mail(
        2,
        1,
        state.game_data.conquest.game_year.saturating_sub(1),
        "Diplomatic",
        "Diplomatic telegram",
    ));
    state.game_data.players[0].messages_first =
        (state.game_data.players[0].messages_first & !0xff) | 1;
    state.game_data.players[0].messages_last =
        (state.game_data.players[0].messages_last & !0xff) | 1;
    save_runtime_state(&fixture_dir, &state);

    let mut app = App::load(AppConfig {
        game_dir: fixture_dir.clone(),
        player_record_index_1_based: 1,
        export_root: None,
        queue_dir: None,
        session_timeout_secs: None,
        game_config: Default::default(),
    })
    .expect("app should load");

    let mut saw_login_summary = false;
    let mut saw_results = false;
    let mut saw_messages = false;

    for _ in 0..16 {
        match app.current_screen() {
            ScreenId::Startup(StartupPhase::LoginSummary) => saw_login_summary = true,
            ScreenId::Startup(StartupPhase::Results) => {
                assert!(saw_login_summary);
                saw_results = true;
            }
            ScreenId::Startup(StartupPhase::Messages) => {
                assert!(saw_results);
                saw_messages = true;
            }
            ScreenId::MainMenu => break,
            _ => {}
        }
        app.advance_startup();
    }

    assert!(saw_login_summary);
    assert!(saw_results);
    assert!(saw_messages);
    assert_eq!(app.current_screen(), ScreenId::MainMenu);
}

#[test]
fn startup_results_paginate_before_advancing_to_messages() {
    let fixture_dir = temp_game_copy();
    let mut state = latest_runtime_state(&fixture_dir);
    set_runtime_report_blocks(
        &mut state,
        (1..=24)
            .map(|idx| format!("Report line {idx:02} is long enough"))
            .collect::<Vec<_>>()
            .join("\n"),
    );
    state.queued_mail.push(incoming_mail(
        2,
        1,
        state.game_data.conquest.game_year.saturating_sub(1),
        "Message",
        "Message line 01 is long enough",
    ));
    state.game_data.players[0].messages_first =
        (state.game_data.players[0].messages_first & !0xff) | 1;
    state.game_data.players[0].messages_last =
        (state.game_data.players[0].messages_last & !0xff) | 1;
    save_runtime_state(&fixture_dir, &state);

    let mut app = App::load(AppConfig {
        game_dir: fixture_dir.clone(),
        player_record_index_1_based: 1,
        export_root: None,
        queue_dir: None,
        session_timeout_secs: None,
        game_config: Default::default(),
    })
    .expect("app should load");

    for _ in 0..16 {
        if app.current_screen() == ScreenId::Startup(StartupPhase::Results) {
            break;
        }
        app.advance_startup();
    }
    assert_eq!(
        app.current_screen(),
        ScreenId::Startup(StartupPhase::Results)
    );

    // Advance from ViewPrompt into ItemBody to start showing content.
    app.advance_startup();

    let mut first_page = CaptureTerminal::new();
    app.render(&mut first_page)
        .expect("first startup results page should render");
    assert!(
        first_page
            .lines
            .iter()
            .any(|line| line.contains(" -> Report line 01"))
    );
    assert!(
        !first_page
            .lines
            .iter()
            .any(|line| line.contains("Report line 28"))
    );
    assert!(
        first_page
            .lines
            .iter()
            .any(|line| line.contains("(Slap a key for more)"))
    );

    for _ in 0..18 {
        let mut screen = CaptureTerminal::new();
        app.render(&mut screen)
            .expect("scrolled startup results should render");
        if screen
            .lines
            .iter()
            .any(|line| line.contains("Delete this report [Y]/N ->"))
        {
            break;
        }
        app.advance_startup();
    }
    assert_eq!(
        app.current_screen(),
        ScreenId::Startup(StartupPhase::Results)
    );

    // Advance through DeletePrompt (keep) → EndStatus.
    app.advance_startup();

    let mut end_status = CaptureTerminal::new();
    app.render(&mut end_status)
        .expect("inline startup results completion should render");
    assert!(
        end_status
            .lines
            .iter()
            .any(|line| line.contains("All reports seen. (Slap a key)"))
    );
    assert!(
        end_status
            .lines
            .iter()
            .any(|line| line.contains("Delete this report [Y]/N ->"))
    );
    assert!(
        !end_status
            .lines
            .iter()
            .any(|line| line.contains("RESULTS REVIEW:"))
    );

    // EndStatus → phase exit → Messages.
    app.advance_startup();
    assert_eq!(
        app.current_screen(),
        ScreenId::Startup(StartupPhase::Messages)
    );
}

#[test]
fn startup_messages_allow_deleting_current_message_then_advancing() {
    let fixture_dir = temp_game_copy();
    let mut state = latest_runtime_state(&fixture_dir);
    clear_runtime_report_blocks(&mut state);
    state
        .queued_mail
        .push(incoming_mail(2, 1, 2999, "One", "Body one"));
    state
        .queued_mail
        .push(incoming_mail(3, 1, 2999, "Two", "Body two"));
    state.game_data.players[0].messages_first =
        (state.game_data.players[0].messages_first & !0xff) | 1;
    state.game_data.players[0].messages_last =
        (state.game_data.players[0].messages_last & !0xff) | 0;
    save_runtime_state(&fixture_dir, &state);

    let mut app = App::load(AppConfig {
        game_dir: fixture_dir.clone(),
        player_record_index_1_based: 1,
        export_root: None,
        queue_dir: None,
        session_timeout_secs: None,
        game_config: Default::default(),
    })
    .expect("app should load");

    for _ in 0..16 {
        if app.current_screen() == ScreenId::Startup(StartupPhase::Messages) {
            break;
        }
        app.advance_startup();
    }
    assert_eq!(
        app.current_screen(),
        ScreenId::Startup(StartupPhase::Messages)
    );

    // ViewPrompt → ItemBody (shows Alpha).
    app.advance_startup();

    let styled_buffer =
        nc_game::domains::startup::views::render(&mut app).expect("startup message buffer");
    let from_row = (0..25)
        .find(|row| styled_buffer.plain_line(*row).contains(" -> From"))
        .expect("from row");
    let subject_row = (0..25)
        .find(|row| styled_buffer.plain_line(*row).contains(" -> Subject:"))
        .expect("subject row");
    let from_col = styled_buffer
        .plain_line(from_row)
        .find(" -> From")
        .expect("from col");
    let subject_col = styled_buffer
        .plain_line(subject_row)
        .find(" -> Subject:")
        .expect("subject col");
    assert_eq!(
        styled_buffer.row(from_row)[from_col].style,
        nc_game::theme::classic::report_header_style()
    );
    assert_eq!(
        styled_buffer.row(subject_row)[subject_col].style,
        nc_game::theme::classic::status_label_style()
    );

    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("first startup message should render");
    assert!(
        terminal
            .lines
            .iter()
            .any(|line| line.contains(" -> From") && line.contains("Empire #2"))
    );
    assert!(
        terminal
            .lines
            .iter()
            .any(|line| line.contains("<end of message>"))
    );

    // Accept default at the end-of-block prompt → delete Alpha → ContinuePrompt (Beta still exists).
    assert_eq!(
        apply_action(&mut app, Action::Startup(StartupAction::AcceptDefault)),
        AppOutcome::Continue
    );
    assert_eq!(
        app.current_screen(),
        ScreenId::Startup(StartupPhase::Messages)
    );

    // ContinuePrompt → ItemBody (shows Beta).
    assert_eq!(
        apply_action(&mut app, Action::Startup(StartupAction::AcceptDefault)),
        AppOutcome::Continue
    );

    let mut after_delete = CaptureTerminal::new();
    app.render(&mut after_delete)
        .expect("next startup message should render");
    assert!(
        after_delete
            .lines
            .iter()
            .filter(|line| line.contains("Messages: Current game year is"))
            .count()
            <= 1
    );
    assert!(
        after_delete
            .lines
            .iter()
            .any(|line| line.contains("Empire #2"))
    );
    assert!(
        after_delete
            .lines
            .iter()
            .any(|line| line.contains(" -> From") && line.contains("Empire #3"))
    );

    let runtime = latest_runtime_state(&fixture_dir);
    let preview = nc_game::reports::ReportsPreview::from_block_rows(
        &runtime.game_data,
        1,
        &runtime.report_block_rows,
        &runtime.queued_mail,
    );
    assert_eq!(preview.message_blocks.len(), 1);
    assert!(
        preview
            .message_lines
            .iter()
            .any(|line| line.contains("From") && line.contains("Empire #3"))
    );
    assert!(
        preview
            .message_lines
            .iter()
            .any(|line| line.contains("<end of message>"))
    );
    assert!(runtime.queued_mail[0].recipient_deleted);
    assert!(!runtime.queued_mail[1].recipient_deleted);
}

#[test]
fn startup_message_review_shows_end_status_after_deleting_last_message() {
    let fixture_dir = temp_game_copy();
    let mut state = latest_runtime_state(&fixture_dir);
    clear_runtime_report_blocks(&mut state);
    state
        .queued_mail
        .push(incoming_mail(2, 1, 2999, "One", "Body one"));
    state.game_data.players[0].messages_first =
        (state.game_data.players[0].messages_first & !0xff) | 1;
    state.game_data.players[0].messages_last =
        (state.game_data.players[0].messages_last & !0xff) | 0;
    save_runtime_state(&fixture_dir, &state);

    let mut app = App::load(AppConfig {
        game_dir: fixture_dir.clone(),
        player_record_index_1_based: 1,
        export_root: None,
        queue_dir: None,
        session_timeout_secs: None,
        game_config: Default::default(),
    })
    .expect("app should load");

    for _ in 0..16 {
        if app.current_screen() == ScreenId::Startup(StartupPhase::Messages) {
            break;
        }
        app.advance_startup();
    }
    assert_eq!(
        app.current_screen(),
        ScreenId::Startup(StartupPhase::Messages)
    );

    // ViewPrompt → ItemBody.
    app.advance_startup();
    // Accept delete at the end-of-block prompt → EndStatus (only 1 block).
    assert_eq!(
        apply_action(&mut app, Action::Startup(StartupAction::AcceptDefault)),
        AppOutcome::Continue
    );
    assert_eq!(
        app.current_screen(),
        ScreenId::Startup(StartupPhase::Messages)
    );

    let mut end_status = CaptureTerminal::new();
    app.render(&mut end_status)
        .expect("end status should render");
    assert!(
        end_status
            .lines
            .iter()
            .any(|line| line.contains("Messages deleted."))
    );
    assert!(
        end_status
            .lines
            .iter()
            .any(|line| line.contains("All messages seen. (Slap a key)"))
    );
    assert!(
        !end_status
            .lines
            .iter()
            .any(|line| line.contains("MESSAGES REVIEW:"))
    );

    // Advance from EndStatus → phase exit → MainMenu.
    app.advance_startup();
    assert_eq!(app.current_screen(), ScreenId::MainMenu);

    let runtime = latest_runtime_state(&fixture_dir);
    assert_eq!(runtime.queued_mail.len(), 1);
    assert!(runtime.queued_mail[0].recipient_deleted);
    assert_eq!((runtime.game_data.players[0].messages_first as u8), 0);
}

#[test]
fn startup_results_wrap_long_lines_within_the_playfield() {
    let fixture_dir = temp_game_copy();
    let mut state = latest_runtime_state(&fixture_dir);
    set_runtime_report_blocks(
        &mut state,
        b"This is a deliberately long startup results line that should wrap cleanly within the eighty column playfield instead of overrunning a single row.",
    );
    state.game_data.players[0].messages_first =
        (state.game_data.players[0].messages_first & !0xff) | 0;
    state.game_data.players[0].messages_last =
        (state.game_data.players[0].messages_last & !0xff) | 1;
    save_runtime_state(&fixture_dir, &state);

    let mut app = App::load(AppConfig {
        game_dir: fixture_dir.clone(),
        player_record_index_1_based: 1,
        export_root: None,
        queue_dir: None,
        session_timeout_secs: None,
        game_config: Default::default(),
    })
    .expect("app should load");

    for _ in 0..16 {
        if app.current_screen() == ScreenId::Startup(StartupPhase::Results) {
            break;
        }
        app.advance_startup();
    }
    assert_eq!(
        app.current_screen(),
        ScreenId::Startup(StartupPhase::Results)
    );

    // Advance from ViewPrompt into ItemBody to start showing content.
    app.advance_startup();

    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("startup results should render");
    assert!(
        terminal
            .lines
            .iter()
            .any(|line| line.contains(" -> This is a deliberately long startup results line"))
    );
    assert!(terminal.lines.iter().any(|line| line.starts_with("  -> ")));
    assert!(
        terminal
            .lines
            .iter()
            .any(|line| line.contains("should wrap cleanly"))
    );
    assert!(
        terminal
            .lines
            .iter()
            .any(|line| line.contains("eighty column playfield"))
    );
    assert!(
        terminal
            .lines
            .iter()
            .any(|line| line.contains("single row."))
    );
}

#[test]
fn startup_results_preserve_blank_lines_as_classic_spacers() {
    let fixture_dir = temp_game_copy();
    let mut state = latest_runtime_state(&fixture_dir);
    set_runtime_report_blocks(
        &mut state,
        classic_chunked_report_bytes("Line one\n\nLine two"),
    );
    state.game_data.players[0].messages_first =
        (state.game_data.players[0].messages_first & !0xff) | 0;
    state.game_data.players[0].messages_last =
        (state.game_data.players[0].messages_last & !0xff) | 1;
    save_runtime_state(&fixture_dir, &state);

    let mut app = App::load(AppConfig {
        game_dir: fixture_dir.clone(),
        player_record_index_1_based: 1,
        export_root: None,
        queue_dir: None,
        session_timeout_secs: None,
        game_config: Default::default(),
    })
    .expect("app should load");

    for _ in 0..16 {
        if app.current_screen() == ScreenId::Startup(StartupPhase::Results) {
            break;
        }
        app.advance_startup();
    }
    assert_eq!(
        app.current_screen(),
        ScreenId::Startup(StartupPhase::Results)
    );

    // Advance from ViewPrompt into ItemBody to start showing content.
    app.advance_startup();

    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("startup results should render");
    assert!(
        terminal
            .lines
            .iter()
            .any(|line| line.contains(" -> Line one"))
    );
    assert!(terminal.lines.iter().any(|line| line.trim_end() == "  ->"));
    assert!(
        terminal
            .lines
            .iter()
            .any(|line| line.contains(" -> Line two"))
    );
}

#[test]
fn startup_results_preserve_leading_spaces_from_oracle_style_reports() {
    let fixture_dir = temp_game_copy();
    let mut state = latest_runtime_state(&fixture_dir);
    set_runtime_report_blocks(
        &mut state,
        classic_chunked_report_bytes("  Stardate 11 / 3003\n    Fleet 7 arrived"),
    );
    state.game_data.players[0].messages_first =
        (state.game_data.players[0].messages_first & !0xff) | 0;
    state.game_data.players[0].messages_last =
        (state.game_data.players[0].messages_last & !0xff) | 1;
    save_runtime_state(&fixture_dir, &state);

    let mut app = App::load(AppConfig {
        game_dir: fixture_dir.clone(),
        player_record_index_1_based: 1,
        export_root: None,
        queue_dir: None,
        session_timeout_secs: None,
        game_config: Default::default(),
    })
    .expect("app should load");

    for _ in 0..16 {
        if app.current_screen() == ScreenId::Startup(StartupPhase::Results) {
            break;
        }
        app.advance_startup();
    }
    assert_eq!(
        app.current_screen(),
        ScreenId::Startup(StartupPhase::Results)
    );

    app.advance_startup();

    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("startup results should render");
    assert!(
        terminal
            .lines
            .iter()
            .any(|line| line.starts_with("  ->   Stardate 11 / 3003"))
    );
    assert!(
        terminal
            .lines
            .iter()
            .any(|line| line.starts_with("  ->     Fleet 7 arrived"))
    );
}

#[test]
fn startup_results_use_the_full_intro_review_page_height() {
    let fixture_dir = temp_game_copy();
    let mut state = latest_runtime_state(&fixture_dir);
    set_runtime_report_blocks(
        &mut state,
        (1..=15)
            .map(|idx| format!("Report {idx:02}"))
            .collect::<Vec<_>>()
            .join("\n"),
    );
    state.game_data.players[0].messages_first =
        (state.game_data.players[0].messages_first & !0xff) | 0;
    state.game_data.players[0].messages_last =
        (state.game_data.players[0].messages_last & !0xff) | 1;
    save_runtime_state(&fixture_dir, &state);

    let mut app = App::load(AppConfig {
        game_dir: fixture_dir.clone(),
        player_record_index_1_based: 1,
        export_root: None,
        queue_dir: None,
        session_timeout_secs: None,
        game_config: Default::default(),
    })
    .expect("app should load");

    for _ in 0..16 {
        if app.current_screen() == ScreenId::Startup(StartupPhase::Results) {
            break;
        }
        app.advance_startup();
    }
    assert_eq!(
        app.current_screen(),
        ScreenId::Startup(StartupPhase::Results)
    );

    app.advance_startup();

    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("startup results should render");
    assert!(
        terminal
            .lines
            .iter()
            .any(|line| line.contains(" -> Report 15"))
    );
    assert!(!terminal.line(19).contains("for more"));
}

#[test]
fn startup_results_decode_length_prefixed_lines_as_separate_classic_rows() {
    let fixture_dir = temp_game_copy();
    let mut state = latest_runtime_state(&fixture_dir);
    set_runtime_report_blocks(
        &mut state,
        length_prefixed_report_block(&[
            "From your 12th Fleet, located in System(9,14):          Stardate: 2/3003",
            "We were attacked by \"Nadir Compact\", (Empire #4) in System(9,14). Our",
            "force contained 1 destroyer and 1 ETAC ship. Alien force contained 1",
            "<end of transmission>",
        ]),
    );
    state.game_data.players[0].messages_first =
        (state.game_data.players[0].messages_first & !0xff) | 0;
    state.game_data.players[0].messages_last =
        (state.game_data.players[0].messages_last & !0xff) | 1;
    save_runtime_state(&fixture_dir, &state);

    let mut app = App::load(AppConfig {
        game_dir: fixture_dir.clone(),
        player_record_index_1_based: 1,
        export_root: None,
        queue_dir: None,
        session_timeout_secs: None,
        game_config: Default::default(),
    })
    .expect("app should load");

    for _ in 0..16 {
        if app.current_screen() == ScreenId::Startup(StartupPhase::Results) {
            break;
        }
        app.advance_startup();
    }
    assert_eq!(
        app.current_screen(),
        ScreenId::Startup(StartupPhase::Results)
    );

    app.advance_startup();

    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("startup results should render");
    assert!(
        terminal
            .lines
            .iter()
            .any(|line| { line.contains("System(9,14):          Stardate: 02/3003") })
    );
    assert!(
        terminal
            .lines
            .iter()
            .any(|line| { line.contains("We were attacked by \"Nadir Compact\", (Empire #4)") })
    );
    assert!(
        terminal
            .lines
            .iter()
            .any(|line| line.contains("<end of transmission>"))
    );
    assert!(
        terminal
            .lines
            .iter()
            .any(|line| line.contains("Delete this report [Y]/N ->"))
    );
    assert!(terminal.line(COMMAND_LINE_ROW - 1).trim().is_empty());
    assert!(!terminal.lines.iter().any(|line| line.contains("----")));
}

#[test]
fn startup_results_continue_prompt_preserves_blank_spacing_without_rule() {
    let fixture_dir = temp_game_copy();
    let mut state = latest_runtime_state(&fixture_dir);
    set_runtime_report_blocks(
        &mut state,
        classic_chunked_report_blocks(&["From Alpha\nBody one", "From Beta\nBody two"]),
    );
    state.game_data.players[0].messages_first =
        (state.game_data.players[0].messages_first & !0xff) | 0;
    state.game_data.players[0].messages_last =
        (state.game_data.players[0].messages_last & !0xff) | 1;
    save_runtime_state(&fixture_dir, &state);

    let mut app = App::load(AppConfig {
        game_dir: fixture_dir.clone(),
        player_record_index_1_based: 1,
        export_root: None,
        queue_dir: None,
        session_timeout_secs: None,
        game_config: Default::default(),
    })
    .expect("app should load");

    for _ in 0..16 {
        if app.current_screen() == ScreenId::Startup(StartupPhase::Results) {
            break;
        }
        app.advance_startup();
    }
    assert_eq!(
        app.current_screen(),
        ScreenId::Startup(StartupPhase::Results)
    );

    app.advance_startup();
    app.advance_startup();

    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("startup continue prompt should render");
    assert!(
        terminal
            .line(19)
            .contains("There are more reports. Continue?")
    );
    assert!(terminal.line(COMMAND_LINE_ROW - 1).trim().is_empty());
    assert!(
        terminal
            .lines
            .iter()
            .any(|line| line.contains("Delete this report [Y]/N ->"))
    );
    assert_eq!(
        terminal
            .lines
            .iter()
            .filter(|line| line.contains("There are more reports. Continue?"))
            .count(),
        1
    );
    assert!(!terminal.lines.iter().any(|line| line.contains("----")));
}

#[test]
fn startup_results_second_block_does_not_reinsert_roll_header() {
    let fixture_dir = temp_game_copy();
    let mut state = latest_runtime_state(&fixture_dir);
    set_runtime_report_blocks(
        &mut state,
        classic_chunked_report_blocks(&["From Alpha\nBody one", "From Beta\nBody two"]),
    );
    state.game_data.players[0].messages_first =
        (state.game_data.players[0].messages_first & !0xff) | 0;
    state.game_data.players[0].messages_last =
        (state.game_data.players[0].messages_last & !0xff) | 1;
    save_runtime_state(&fixture_dir, &state);

    let mut app = App::load(AppConfig {
        game_dir: fixture_dir.clone(),
        player_record_index_1_based: 1,
        export_root: None,
        queue_dir: None,
        session_timeout_secs: None,
        game_config: Default::default(),
    })
    .expect("app should load");

    for _ in 0..16 {
        if app.current_screen() == ScreenId::Startup(StartupPhase::Results) {
            break;
        }
        app.advance_startup();
    }
    assert_eq!(
        app.current_screen(),
        ScreenId::Startup(StartupPhase::Results)
    );

    app.advance_startup();
    assert_eq!(
        apply_action(&mut app, Action::Startup(StartupAction::AcceptDefault)),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Startup(StartupAction::AcceptDefault)),
        AppOutcome::Continue
    );

    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("second startup report should render");
    assert!(
        terminal
            .lines
            .iter()
            .filter(|line| line.contains("Reports: Current game year is"))
            .count()
            <= 1
    );
    assert!(
        terminal
            .lines
            .iter()
            .any(|line| line.contains("From Alpha"))
    );
    assert!(terminal.lines.iter().any(|line| line.contains("From Beta")));
}

#[test]
fn reports_screen_preserves_blank_separator_lines() {
    let fixture_dir = temp_game_copy();
    let mut state = latest_runtime_state(&fixture_dir);
    set_runtime_report_blocks(
        &mut state,
        classic_chunked_report_bytes("Line one\n\nLine two"),
    );
    state.game_data.players[0].messages_last =
        (state.game_data.players[0].messages_last & !0xff) | 1;
    save_runtime_state(&fixture_dir, &state);

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

    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("reports screen should render");
    let line_one_idx = terminal
        .lines
        .iter()
        .position(|line| line.contains("Line one"))
        .expect("reports screen should contain first line");
    let blank_line =
        terminal.lines[line_one_idx + 1].trim_matches(|ch: char| ch == '│' || ch == ' ');
    assert!(blank_line.is_empty());
    assert!(terminal.lines[line_one_idx + 2].contains("Line two"));
}

#[test]
fn reports_screen_wraps_long_lines_within_the_playfield() {
    let fixture_dir = temp_game_copy();
    let mut state = latest_runtime_state(&fixture_dir);
    set_runtime_report_blocks(
        &mut state,
        b"This is a deliberately long reports review line that should wrap cleanly within the eighty column playfield instead of overrunning a single row.",
    );
    state.game_data.players[0].messages_last =
        (state.game_data.players[0].messages_last & !0xff) | 1;
    save_runtime_state(&fixture_dir, &state);

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

    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("reports screen should render");
    let first_line_idx = terminal
        .lines
        .iter()
        .position(|line| line.contains("This is a deliberately long reports review line"))
        .expect("reports screen should contain wrapped first line");
    assert!(
        terminal.lines[first_line_idx].contains("should wrap cleanly")
            || terminal.lines[first_line_idx + 1].contains("should wrap cleanly")
    );
    assert!(
        terminal.lines[first_line_idx + 1].contains("eighty column playfield")
            || terminal.lines[first_line_idx + 2].contains("eighty column playfield")
    );
}

#[test]
fn reports_screen_separates_messages_from_long_reports() {
    let fixture_dir = temp_game_copy();
    let mut state = latest_runtime_state(&fixture_dir);
    set_runtime_report_blocks(
        &mut state,
        (1..=8)
            .map(|idx| format!("This is long report line {idx:02} and it should wrap across rows"))
            .collect::<Vec<_>>()
            .join("\n"),
    );
    state.queued_mail.push(incoming_mail(
        2,
        1,
        state.game_data.conquest.game_year.saturating_sub(1),
        "Visible",
        "Message line 01 should still remain visible",
    ));
    state.game_data.players[0].messages_first =
        (state.game_data.players[0].messages_first & !0xff) | 1;
    state.game_data.players[0].messages_last =
        (state.game_data.players[0].messages_last & !0xff) | 1;
    save_runtime_state(&fixture_dir, &state);

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

    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("reports screen should render");
    assert!(!terminal.lines.iter().any(|line| line.contains("│Type│")));
    assert!(!terminal.lines.iter().any(|line| line.contains("Visible")));
    apply_action(
        &mut app,
        Action::Messaging(MessagingAction::SetInboxTypeFilterMessages),
    );
    let mut messages = CaptureTerminal::new();
    app.render(&mut messages).expect("messages folder");
    assert!(messages.lines.iter().any(|line| line.contains("Visible")));
}

#[test]
fn reports_screen_shows_explicit_truncation_cue_when_wrapped_rows_overflow() {
    let fixture_dir = temp_game_copy();
    let mut state = latest_runtime_state(&fixture_dir);
    set_runtime_report_blocks(
        &mut state,
        (1..=16)
            .map(|idx| format!("This is long report line {idx:02} and it should wrap across rows"))
            .collect::<Vec<_>>()
            .join("\n"),
    );
    state.game_data.players[0].messages_last =
        (state.game_data.players[0].messages_last & !0xff) | 1;
    save_runtime_state(&fixture_dir, &state);

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

    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("reports screen should render");
    assert!(terminal.line(0).contains("INBOX:"));
    assert!(terminal.line(24).contains("COMMAND <- ? R"));
    assert!(terminal.line(24).contains("<TAB>"));
    assert!(terminal.line(24).contains("<Q> [01] ->"));
    assert!(
        !terminal
            .lines
            .iter()
            .any(|line| line.contains("full suspense"))
    );
}

#[test]
fn preloaded_first_login_reviews_reports_before_homeworld_naming() {
    let fixture_dir = temp_joined_needs_homeworld_copy();
    let mut state = latest_runtime_state(&fixture_dir);
    set_runtime_report_blocks(&mut state, b"Fleet battle report");
    state.queued_mail.push(incoming_mail(
        2,
        1,
        state.game_data.conquest.game_year.saturating_sub(1),
        "Diplomatic",
        "Diplomatic telegram",
    ));
    state.game_data.players[0].messages_first =
        (state.game_data.players[0].messages_first & !0xff) | 1;
    state.game_data.players[0].messages_last =
        (state.game_data.players[0].messages_last & !0xff) | 1;
    save_runtime_state(&fixture_dir, &state);

    let mut app = App::load(AppConfig {
        game_dir: fixture_dir.clone(),
        player_record_index_1_based: 1,
        export_root: None,
        queue_dir: None,
        session_timeout_secs: None,
        game_config: Default::default(),
    })
    .expect("app should load");

    let mut saw_results = false;
    let mut saw_messages = false;
    for _ in 0..16 {
        match app.current_screen() {
            ScreenId::Startup(StartupPhase::Results) => saw_results = true,
            ScreenId::Startup(StartupPhase::Messages) => {
                assert!(saw_results);
                saw_messages = true;
            }
            ScreenId::FirstTimePreloadedRenamePrompt => break,
            _ => {}
        }
        app.advance_startup();
    }

    assert!(saw_results);
    assert!(saw_messages);
    assert_eq!(
        app.current_screen(),
        ScreenId::FirstTimePreloadedRenamePrompt
    );
}

#[test]
fn returning_player_reviews_reports_before_colony_naming() {
    let fixture_dir = temp_game_copy();
    let mut state = latest_runtime_state(&fixture_dir);
    let homeworld_index = state.game_data.players[0].homeworld as usize;
    let colony = state
        .game_data
        .planets
        .iter_mut()
        .enumerate()
        .find(|(idx, planet)| *idx + 1 != homeworld_index && planet.owner.0 != 1)
        .expect("need a non-homeworld planet for colony naming test");
    colony.1.owner.0 = 1;
    nc_data::set_planet_name(colony.1, "Not Named Yet");
    set_runtime_report_blocks(&mut state, b"Scout report");
    state.queued_mail.push(incoming_mail(
        2,
        1,
        state.game_data.conquest.game_year.saturating_sub(1),
        "Command",
        "Command mail",
    ));
    state.game_data.players[0].messages_first =
        (state.game_data.players[0].messages_first & !0xff) | 1;
    state.game_data.players[0].messages_last =
        (state.game_data.players[0].messages_last & !0xff) | 1;
    save_runtime_state(&fixture_dir, &state);

    let mut app = App::load(AppConfig {
        game_dir: fixture_dir.clone(),
        player_record_index_1_based: 1,
        export_root: None,
        queue_dir: None,
        session_timeout_secs: None,
        game_config: Default::default(),
    })
    .expect("app should load");

    let mut saw_results = false;
    let mut saw_messages = false;
    for _ in 0..16 {
        match app.current_screen() {
            ScreenId::Startup(StartupPhase::Results) => saw_results = true,
            ScreenId::Startup(StartupPhase::Messages) => {
                assert!(saw_results);
                saw_messages = true;
            }
            ScreenId::ColonyWorldName => break,
            _ => {}
        }
        app.advance_startup();
    }

    assert!(saw_results);
    assert!(saw_messages);
    assert_eq!(app.current_screen(), ScreenId::ColonyWorldName);
}
