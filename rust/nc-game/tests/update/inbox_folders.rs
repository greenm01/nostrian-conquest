use crate::support::*;
use nc_game::domains::messaging::state::InboxTypeFilter;
use nc_game::screen::ScreenGeometry;

fn press(app: &mut App, code: KeyCode) {
    let action = app.handle_key(key(code));
    assert_eq!(apply_action(app, action), AppOutcome::Continue);
}

#[test]
fn folders_fit_both_door_heights_and_sent_deletion_keeps_recipient_copy() {
    for height in [24, 25] {
        let fixture_dir = temp_game_copy();
        let mut state = latest_runtime_state(&fixture_dir);
        let year = state.game_data.conquest.game_year;
        set_runtime_report_blocks(&mut state, b"Fleet battle report");
        state.queued_mail = vec![
            incoming_mail(2, 1, year - 1, "Received", "Received body"),
            incoming_mail(1, 2, year - 1, "Delivered", "Delivered body"),
            incoming_mail(1, 2, year, "Pending", "Pending body"),
        ];
        save_runtime_state(&fixture_dir, &state);
        let mut app = App::load(AppConfig {
            game_dir: fixture_dir.clone(),
            player_record_index_1_based: 1,
            export_root: None,
            queue_dir: None,
            session_timeout_secs: None,
            game_config: Default::default(),
        })
        .unwrap();
        advance_to_main_menu(&mut app);
        app.screen_geometry = ScreenGeometry::new(80, height);
        apply_action(&mut app, Action::Startup(StartupAction::OpenReports));
        for (key, folder, title, column) in [
            ('R', InboxTypeFilter::Reports, "[Reports (1)]", "Stardate"),
            ('M', InboxTypeFilter::Messages, "[Messages (1)]", "From"),
            ('S', InboxTypeFilter::Sent, "[Sent (2)]", "To"),
        ] {
            press(&mut app, KeyCode::Char(key));
            assert_eq!(app.messaging.inbox_type_filter, folder);
            let mut screen = CaptureTerminal::new();
            app.render(&mut screen).unwrap();
            assert_eq!(screen.lines.len(), height);
            assert!(screen.lines.iter().all(|line| line.chars().count() <= 80));
            assert!(screen.line(0).contains(title));
            assert!(screen.line(2).contains(column));
            assert!(!screen.line(2).contains("Type"));
            assert!(screen.line(height - 1).contains("COMMAND <- ? R M S Y D"));
            if folder == InboxTypeFilter::Sent {
                assert!(
                    screen
                        .lines
                        .iter()
                        .any(|line| line.contains("Queued") && line.contains("Pending"))
                );
                assert!(
                    screen
                        .lines
                        .iter()
                        .any(|line| line.contains(&year.to_string()) && line.contains("Delivered"))
                );
            }
        }
        press(&mut app, KeyCode::Down); // delivered copy, pending comes first
        press(&mut app, KeyCode::Char('D'));
        let mut confirm = CaptureTerminal::new();
        app.render(&mut confirm).unwrap();
        assert!(confirm.line(height - 1).contains("Delete sent copy"));
        press(&mut app, KeyCode::Char('Y'));
        let stored = latest_runtime_state(&fixture_dir);
        assert_eq!(stored.queued_mail.len(), 3);
        assert_eq!(stored.queued_mail[1].body, "Delivered body");
        assert!(stored.queued_mail[1].sender_deleted);
        assert!(stored.queued_mail[1].is_visible_to_recipient(2, year));
        assert!(!stored.queued_mail[2].cancelled);
    }
}
