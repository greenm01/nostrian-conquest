use crate::support::*;
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

fn render(app: &mut App) -> CaptureTerminal {
    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal).expect("screen should render");
    terminal
}

/// Walks the login sequence to the main menu and returns the text of the
/// message review it passed through, or `None` when it reviewed no messages.
fn login_message_review(app: &mut App) -> Option<String> {
    let mut review: Option<String> = None;
    for _ in 0..64 {
        match app.current_screen() {
            ScreenId::MainMenu => return review,
            ScreenId::Startup(StartupPhase::Messages) => {
                let text = render(app).lines.join("\n");
                review.get_or_insert_with(String::new).push_str(&text);
            }
            _ => {}
        }
        app.advance_startup();
    }
    panic!("startup did not reach main menu");
}

/// One maintenance on the stored campaign, as `maint-rust` runs it: the
/// engine advances the year and the queued mail is saved unchanged.
fn run_maintenance(fixture_dir: &Path) {
    let mut runtime = latest_runtime_state(fixture_dir);
    nc_engine::run_maintenance_turn(&mut runtime.game_data).expect("maintenance should run");
    save_runtime_state(fixture_dir, &runtime);
}

fn open_inbox(app: &mut App) {
    assert_eq!(
        apply_action(app, Action::Startup(StartupAction::OpenReports)),
        AppOutcome::Continue
    );
}

#[test]
fn recipient_sees_a_message_only_after_the_maintenance_that_delivers_it() {
    let fixture_dir = temp_game_copy();
    let mut runtime = latest_runtime_state(&fixture_dir);
    clear_runtime_report_blocks(&mut runtime);
    let year = runtime.game_data.conquest.game_year();
    runtime
        .queued_mail
        .push(incoming_mail(2, 1, year, "Truce", "Hold fire"));
    save_runtime_state(&fixture_dir, &runtime);

    // Queued this turn: no login review, nothing reviewable, an empty inbox.
    let mut app = load_player_one(&fixture_dir);
    assert_eq!(login_message_review(&mut app), None);
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
    assert!(!app.messaging.delete_reviewables_prompt_active);
    let menu = render(&mut app);
    assert!(
        line_containing(&menu, "currently reviewable")
            .contains("No messages or results are currently reviewable.")
    );
    open_inbox(&mut app);
    assert_eq!(app.current_screen(), ScreenId::GeneralMenu);
    let menu = render(&mut app);
    assert!(line_containing(&menu, "Inbox is empty.").contains("Inbox is empty."));

    run_maintenance(&fixture_dir);

    // Delivered: the login review shows it and the inbox lists it.
    let mut app = load_player_one(&fixture_dir);
    let review = login_message_review(&mut app).expect("login should review the message");
    assert!(review.contains("Subject: Truce"), "{review}");
    assert!(review.contains("Hold fire"), "{review}");
    open_inbox(&mut app);
    assert_eq!(app.current_screen(), ScreenId::Reports);
    let items = app
        .filtered_inbox_display_items()
        .into_iter()
        .map(|item| (item.item.subject, item.item.item_type))
        .collect::<Vec<_>>();
    assert_eq!(items, vec![("Truce".to_string(), InboxItemType::Message)]);
}

#[test]
fn delete_all_reviewables_leaves_undelivered_mail_for_its_recipient() {
    let fixture_dir = temp_game_copy();
    let mut runtime = latest_runtime_state(&fixture_dir);
    let year = runtime.game_data.conquest.game_year();
    set_runtime_report_blocks(&mut runtime, b"Fleet battle report");
    runtime
        .queued_mail
        .push(incoming_mail(2, 1, year - 1, "Delivered", "Old news"));
    runtime
        .queued_mail
        .push(incoming_mail(3, 1, year, "Queued", "Not yet"));
    save_runtime_state(&fixture_dir, &runtime);

    let mut app = load_player_one(&fixture_dir);
    advance_to_main_menu(&mut app);
    assert_eq!(
        apply_action(&mut app, Action::OpenGeneralMenu),
        AppOutcome::Continue
    );
    for action in [
        MessagingAction::OpenDeleteReviewables,
        MessagingAction::ConfirmDeleteReviewables,
    ] {
        assert_eq!(
            apply_action(&mut app, Action::Messaging(action)),
            AppOutcome::Continue
        );
    }

    let deleted = latest_runtime_state(&fixture_dir)
        .queued_mail
        .into_iter()
        .map(|mail| (mail.subject, mail.recipient_deleted))
        .collect::<Vec<_>>();
    assert_eq!(
        deleted,
        vec![
            ("Delivered".to_string(), true),
            ("Queued".to_string(), false)
        ]
    );
}
