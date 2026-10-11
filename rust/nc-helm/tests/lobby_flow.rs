mod support;

use nc_client::cache::ClientCache;
use nc_helm::{App, Effect, LobbySnapshot, Msg, Route, SandboxReleaseSuccess};

use crate::support::{
    alt_key, dummy_session, key, league_my_game_row, left_click, my_game_row, sandbox_open_game_row,
};

fn find_line(buffer: &nc_helm::PlayfieldBuffer, needle: &str) -> usize {
    (0..buffer.height())
        .find(|&row| buffer.plain_line(row).contains(needle))
        .unwrap_or_else(|| panic!("expected line containing {needle:?}"))
}

#[test]
fn lobby_help_closes_on_q_or_escape_and_reopens_on_question_mark() {
    let (mut app, _) = App::new(None);
    let _ = app.dispatch(Msg::Unlocked(Ok(dummy_session("captain"))));
    match &app.model().route {
        Route::Lobby(lobby) => assert!(!lobby.help_open),
        other => panic!("expected lobby route, got {other:?}"),
    }
    let _ = app.dispatch(Msg::Key(key(nc_helm::KeyCode::Char('?'))));
    let _ = app.dispatch(Msg::Key(key(nc_helm::KeyCode::Enter)));
    match &app.model().route {
        Route::Lobby(lobby) => assert!(lobby.help_open),
        other => panic!("expected lobby route, got {other:?}"),
    }
    let _ = app.dispatch(Msg::Key(key(nc_helm::KeyCode::Char('q'))));
    match &app.model().route {
        Route::Lobby(lobby) => assert!(!lobby.help_open),
        other => panic!("expected lobby route, got {other:?}"),
    }
    let _ = app.dispatch(Msg::Key(key(nc_helm::KeyCode::Char('?'))));
    match &app.model().route {
        Route::Lobby(lobby) => assert!(lobby.help_open),
        other => panic!("expected lobby route, got {other:?}"),
    }
}

#[test]
fn lobby_help_clicking_close_tag_closes_popup() {
    let (mut app, _) = App::new(None);
    let _ = app.dispatch(Msg::Unlocked(Ok(dummy_session("captain"))));
    let _ = app.dispatch(Msg::Key(key(nc_helm::KeyCode::Char('?'))));

    let row = 11usize;
    let line = app.view().plain_line(row);
    let tag_offset = line.find("┐[X]┌").expect("close tag should render");
    let tag_col = line[..tag_offset].chars().count();
    let _ = app.dispatch(Msg::Mouse(left_click(tag_col + 3, row)));

    match &app.model().route {
        Route::Lobby(lobby) => assert!(!lobby.help_open),
        other => panic!("expected lobby route, got {other:?}"),
    }
}

#[test]
fn lobby_help_clicking_body_does_not_close_popup() {
    let (mut app, _) = App::new(None);
    let _ = app.dispatch(Msg::Unlocked(Ok(dummy_session("captain"))));
    let _ = app.dispatch(Msg::Key(key(nc_helm::KeyCode::Char('?'))));
    let _ = app.dispatch(Msg::Mouse(left_click(24, 14)));

    match &app.model().route {
        Route::Lobby(lobby) => assert!(lobby.help_open),
        other => panic!("expected lobby route, got {other:?}"),
    }
}

#[test]
fn lobby_update_populates_games_and_notices() {
    let (mut app, _) = App::new(None);
    let _ = app.dispatch(Msg::Unlocked(Ok(dummy_session("captain"))));
    let _ = app.dispatch(Msg::LobbyUpdated(Ok(LobbySnapshot {
        cache: ClientCache::empty(),
        my_games: Vec::new(),
        open_games: vec![sandbox_open_game_row()],
        notices: vec!["sysop: sandbox reset tonight".to_string()],
    })));
    assert_eq!(app.model().open_games.len(), 1);
    assert_eq!(app.model().notices.len(), 1);
    match &app.model().route {
        Route::Lobby(lobby) => assert!(lobby.status.is_none()),
        other => panic!("expected lobby route, got {other:?}"),
    }
}

#[test]
fn lock_action_disconnects_transport_and_returns_to_locked_route() {
    let (mut app, _) = App::new(None);
    let _ = app.dispatch(Msg::Unlocked(Ok(dummy_session("captain"))));
    let effects = app.dispatch(Msg::Key(alt_key(nc_helm::KeyCode::Char('l'))));
    assert!(matches!(effects.as_slice(), [Effect::DisconnectTransport]));
    match &app.model().route {
        Route::MatrixLocked => {}
        other => panic!("expected locked route, got {other:?}"),
    }
    assert!(app.model().session.is_none());
    assert!(app.model().open_games.is_empty());
    assert!(app.model().my_games.is_empty());
    assert!(app.model().notices.is_empty());
}

#[test]
fn any_key_from_matrix_lock_opens_unlock_gate() {
    let (mut app, _) = App::new(None);
    let _ = app.dispatch(Msg::Unlocked(Ok(dummy_session("captain"))));
    let _ = app.dispatch(Msg::Key(alt_key(nc_helm::KeyCode::Char('l'))));

    let _ = app.dispatch(Msg::Key(key(nc_helm::KeyCode::Enter)));
    match &app.model().route {
        Route::Locked(locked) => {
            assert!(locked.password_input.is_empty());
            assert!(locked.resume_session);
            assert!(locked.status.is_none());
        }
        other => panic!("expected locked route, got {other:?}"),
    }
}

#[test]
fn escape_from_resume_unlock_returns_to_matrix_lock() {
    let (mut app, _) = App::new(None);
    let _ = app.dispatch(Msg::Unlocked(Ok(dummy_session("captain"))));
    let _ = app.dispatch(Msg::Key(alt_key(nc_helm::KeyCode::Char('l'))));
    let _ = app.dispatch(Msg::Key(key(nc_helm::KeyCode::Enter)));

    let _ = app.dispatch(Msg::Key(key(nc_helm::KeyCode::Esc)));
    match &app.model().route {
        Route::MatrixLocked => {}
        other => panic!("expected matrix lock route, got {other:?}"),
    }
    assert!(!app.model().should_quit);
}

#[test]
fn alt_q_is_ignored_while_matrix_locked() {
    let (mut app, _) = App::new(None);
    let _ = app.dispatch(Msg::Unlocked(Ok(dummy_session("captain"))));
    let _ = app.dispatch(Msg::Key(alt_key(nc_helm::KeyCode::Char('l'))));

    let effects = app.dispatch(Msg::Key(alt_key(nc_helm::KeyCode::Char('q'))));

    assert!(effects.is_empty());
    assert!(matches!(app.model().route, Route::MatrixLocked));
    assert!(!app.model().should_quit);
}

#[test]
fn alt_q_is_ignored_on_resume_unlock_prompt() {
    let (mut app, _) = App::new(None);
    let _ = app.dispatch(Msg::Unlocked(Ok(dummy_session("captain"))));
    let _ = app.dispatch(Msg::Key(alt_key(nc_helm::KeyCode::Char('l'))));
    let _ = app.dispatch(Msg::Key(key(nc_helm::KeyCode::Enter)));

    let effects = app.dispatch(Msg::Key(alt_key(nc_helm::KeyCode::Char('q'))));

    assert!(effects.is_empty());
    match &app.model().route {
        Route::Locked(locked) => {
            assert!(locked.resume_session);
            assert!(locked.password_input.is_empty());
        }
        other => panic!("expected locked route, got {other:?}"),
    }
    assert!(!app.model().should_quit);
}

#[test]
fn letter_shortcuts_switch_lobby_tabs() {
    let (mut app, _) = App::new(None);
    let _ = app.dispatch(Msg::Unlocked(Ok(dummy_session("captain"))));
    let buffer = app.view();
    assert!(
        buffer
            .plain_line(find_line(&buffer, "┐MY GAMES┌"))
            .contains("MY GAMES")
    );

    let _ = app.dispatch(Msg::Key(key(nc_helm::KeyCode::Char('o'))));
    let buffer = app.view();
    assert!(
        buffer
            .plain_line(find_line(&buffer, "┐OPEN GAMES AVAILABLE TO JOIN┌"))
            .contains("OPEN GAMES")
    );

    let _ = app.dispatch(Msg::Key(key(nc_helm::KeyCode::Char('c'))));
    let buffer = app.view();
    assert!(buffer.plain_line(4).contains("COMMS"));

    let _ = app.dispatch(Msg::Key(key(nc_helm::KeyCode::Char('s'))));
    let buffer = app.view();
    assert!(
        buffer
            .plain_line(find_line(&buffer, "┐SETTINGS┌"))
            .contains("SETTINGS")
    );
}

#[test]
fn left_clicking_tab_strip_switches_lobby_tabs() {
    let (mut app, _) = App::new(None);
    let _ = app.dispatch(Msg::Unlocked(Ok(dummy_session("captain"))));

    let row = 2usize;
    let line = app.view().plain_line(row);
    let tag_offset = line
        .find("[Open Games]")
        .expect("open games tab should render");
    let tag_col = line[..tag_offset].chars().count();
    let _ = app.dispatch(Msg::Mouse(left_click(tag_col + 1, row)));

    let buffer = app.view();
    assert!(
        buffer
            .plain_line(find_line(&buffer, "┐OPEN GAMES AVAILABLE TO JOIN┌"))
            .contains("OPEN GAMES")
    );
}

#[test]
fn settings_relay_edit_emits_save_and_reconnect_effects() {
    let (mut app, _) = App::new(None);
    let _ = app.dispatch(Msg::Unlocked(Ok(dummy_session("captain"))));
    let _ = app.dispatch(Msg::Key(key(nc_helm::KeyCode::Char('s'))));
    let _ = app.dispatch(Msg::Key(key(nc_helm::KeyCode::Char('r'))));
    for _ in 0.."ws://127.0.0.1:8080".chars().count() {
        let _ = app.dispatch(Msg::Key(key(nc_helm::KeyCode::Backspace)));
    }
    for ch in "ws://relay.example".chars() {
        let _ = app.dispatch(Msg::Key(key(nc_helm::KeyCode::Char(ch))));
    }
    let effects = app.dispatch(Msg::Key(key(nc_helm::KeyCode::Enter)));
    assert!(matches!(
        effects.as_slice(),
        [
            Effect::SaveRelayUrl { relay_url: saved },
            Effect::DisconnectTransport,
            Effect::ConnectTransport { relay_url: connected, .. }
        ] if saved == "ws://relay.example" && connected == "ws://relay.example"
    ));
}

#[test]
fn alt_q_opens_lobby_quit_confirm_and_y_quits() {
    let (mut app, _) = App::new(None);
    let _ = app.dispatch(Msg::Unlocked(Ok(dummy_session("captain"))));

    let effects = app.dispatch(Msg::Key(alt_key(nc_helm::KeyCode::Char('q'))));
    assert!(effects.is_empty());
    match &app.model().route {
        Route::Lobby(lobby) => assert!(lobby.quit_confirm_open),
        other => panic!("expected lobby route, got {other:?}"),
    }
    assert!(!app.model().should_quit);

    let effects = app.dispatch(Msg::Key(key(nc_helm::KeyCode::Char('y'))));
    assert!(matches!(effects.as_slice(), [Effect::Quit]));
    assert!(app.model().should_quit);
}

#[test]
fn lobby_quit_confirm_uses_compact_shared_copy() {
    let (mut app, _) = App::new(None);
    let _ = app.dispatch(Msg::Unlocked(Ok(dummy_session("captain"))));
    let _ = app.dispatch(Msg::Key(key(nc_helm::KeyCode::Esc)));

    let buffer = app.view();
    let title_row = find_line(&buffer, "QUIT");
    let message_row = find_line(&buffer, "Quit NC? Y/[N]");

    assert_eq!(message_row, title_row + 2);
    assert_ne!(title_row, find_line(&buffer, "Quit NC? Y/[N]"));
    assert!((0..buffer.height()).all(|row| !buffer.plain_line(row).contains("QUIT NC-HELM")));
    assert!((0..buffer.height()).all(|row| !buffer.plain_line(row).contains("Quit nc-helm?")));
    assert!((0..buffer.height()).all(|row| {
        !buffer
            .plain_line(row)
            .contains("Enter, Esc, or N stays in the lobby.")
    }));
}

#[test]
fn lobby_quit_confirm_enter_defaults_to_no() {
    let (mut app, _) = App::new(None);
    let _ = app.dispatch(Msg::Unlocked(Ok(dummy_session("captain"))));
    let _ = app.dispatch(Msg::Key(key(nc_helm::KeyCode::Esc)));

    let effects = app.dispatch(Msg::Key(key(nc_helm::KeyCode::Enter)));

    assert!(effects.is_empty());
    assert!(!app.model().should_quit);
    match &app.model().route {
        Route::Lobby(lobby) => assert!(!lobby.quit_confirm_open),
        other => panic!("expected lobby route, got {other:?}"),
    }
}

#[test]
fn lobby_quit_confirm_escape_declines() {
    let (mut app, _) = App::new(None);
    let _ = app.dispatch(Msg::Unlocked(Ok(dummy_session("captain"))));
    let _ = app.dispatch(Msg::Key(key(nc_helm::KeyCode::Esc)));

    let effects = app.dispatch(Msg::Key(key(nc_helm::KeyCode::Esc)));

    assert!(effects.is_empty());
    assert!(!app.model().should_quit);
    match &app.model().route {
        Route::Lobby(lobby) => assert!(!lobby.quit_confirm_open),
        other => panic!("expected lobby route, got {other:?}"),
    }
}

#[test]
fn open_games_scrolls_when_selection_moves_past_visible_body() {
    let (mut app, _) = App::new(None);
    let _ = app.dispatch(Msg::Unlocked(Ok(dummy_session("captain"))));
    let open_games = (0..30)
        .map(|index| {
            let mut row = sandbox_open_game_row();
            row.game_id = format!("open-{index:02}");
            row.game = format!("Open Game {index:02}");
            row
        })
        .collect();
    let _ = app.dispatch(Msg::LobbyUpdated(Ok(LobbySnapshot {
        cache: ClientCache::empty(),
        my_games: Vec::new(),
        open_games,
        notices: Vec::new(),
    })));
    let _ = app.dispatch(Msg::Key(key(nc_helm::KeyCode::Char('o'))));
    for _ in 0..25 {
        let _ = app.dispatch(Msg::Key(key(nc_helm::KeyCode::Down)));
    }

    match &app.model().route {
        Route::Lobby(lobby) => {
            assert_eq!(lobby.selected_open_game, 25);
            assert!(lobby.open_games_scroll > 0);
        }
        other => panic!("expected lobby route, got {other:?}"),
    }
    let buffer = app.view();
    assert!(buffer.plain_line(7).contains("^"));
}

#[test]
fn enter_on_sandbox_open_game_opens_confirm_then_y_joins() {
    let (mut app, _) = App::new(None);
    let _ = app.dispatch(Msg::Unlocked(Ok(dummy_session("captain"))));
    let _ = app.dispatch(Msg::LobbyUpdated(Ok(LobbySnapshot {
        cache: ClientCache::empty(),
        my_games: Vec::new(),
        open_games: vec![sandbox_open_game_row()],
        notices: Vec::new(),
    })));
    let _ = app.dispatch(Msg::Key(key(nc_helm::KeyCode::Char('o'))));

    let effects = app.dispatch(Msg::Key(key(nc_helm::KeyCode::Enter)));
    assert!(effects.is_empty());
    match &app.model().route {
        Route::SandboxJoinConfirm(row) => assert_eq!(row.game_id, "phase-sapling-awful"),
        other => panic!("expected sandbox confirm route, got {other:?}"),
    }

    let effects = app.dispatch(Msg::Key(key(nc_helm::KeyCode::Char('y'))));
    assert!(matches!(
        effects.as_slice(),
        [Effect::JoinSandboxGame { row, .. }] if row.game_id == "phase-sapling-awful"
    ));
}

#[test]
fn sandbox_join_and_delete_confirms_cancel_on_enter_and_escape() {
    let lobby_with = |my_games| {
        let (mut app, _) = App::new(None);
        let _ = app.dispatch(Msg::Unlocked(Ok(dummy_session("captain"))));
        let _ = app.dispatch(Msg::LobbyUpdated(Ok(LobbySnapshot {
            cache: ClientCache::empty(),
            my_games,
            open_games: vec![sandbox_open_game_row()],
            notices: Vec::new(),
        })));
        app
    };
    for cancel in [nc_helm::KeyCode::Enter, nc_helm::KeyCode::Esc] {
        let mut app = lobby_with(Vec::new());
        let _ = app.dispatch(Msg::Key(key(nc_helm::KeyCode::Char('o'))));
        let _ = app.dispatch(Msg::Key(key(nc_helm::KeyCode::Enter)));
        assert!(matches!(app.model().route, Route::SandboxJoinConfirm(_)));
        let effects = app.dispatch(Msg::Key(key(cancel)));
        assert!(effects.is_empty());
        assert!(matches!(app.model().route, Route::Lobby(_)));

        let mut app = lobby_with(vec![my_game_row("joined")]);
        let _ = app.dispatch(Msg::Key(alt_key(nc_helm::KeyCode::Char('d'))));
        assert!(matches!(app.model().route, Route::SandboxDeleteConfirm(_)));
        let effects = app.dispatch(Msg::Key(key(cancel)));
        assert!(effects.is_empty());
        assert!(matches!(app.model().route, Route::Lobby(_)));
    }
}

#[test]
fn alt_r_emits_refresh_effect_from_lobby() {
    let (mut app, _) = App::new(None);
    let _ = app.dispatch(Msg::Unlocked(Ok(dummy_session("captain"))));

    let effects = app.dispatch(Msg::Key(alt_key(nc_helm::KeyCode::Char('r'))));
    assert!(matches!(effects.as_slice(), [Effect::RefreshLobby]));
    match &app.model().route {
        Route::Lobby(lobby) => {
            assert_eq!(lobby.status.as_deref(), Some("Refreshing hosted lobby..."));
        }
        other => panic!("expected lobby route, got {other:?}"),
    }
}

#[test]
fn alt_d_on_sandbox_my_game_opens_confirm_then_y_releases() {
    let (mut app, _) = App::new(None);
    let _ = app.dispatch(Msg::Unlocked(Ok(dummy_session("captain"))));
    let _ = app.dispatch(Msg::LobbyUpdated(Ok(LobbySnapshot {
        cache: ClientCache::empty(),
        my_games: vec![my_game_row("joined")],
        open_games: vec![sandbox_open_game_row()],
        notices: Vec::new(),
    })));

    let effects = app.dispatch(Msg::Key(alt_key(nc_helm::KeyCode::Char('d'))));
    assert!(effects.is_empty());
    match &app.model().route {
        Route::SandboxDeleteConfirm(row) => assert_eq!(row.game_id, "phase-sapling-awful"),
        other => panic!("expected sandbox delete confirm route, got {other:?}"),
    }

    let effects = app.dispatch(Msg::Key(key(nc_helm::KeyCode::Char('y'))));
    assert!(matches!(
        effects.as_slice(),
        [Effect::ReleaseSandboxGame { row }] if row.game_id == "phase-sapling-awful"
    ));
    match &app.model().route {
        Route::Lobby(lobby) => {
            assert_eq!(lobby.status.as_deref(), Some("Releasing sandbox seat..."));
        }
        other => panic!("expected lobby route, got {other:?}"),
    }
}

#[test]
fn alt_d_on_non_sandbox_my_game_is_ignored() {
    let (mut app, _) = App::new(None);
    let _ = app.dispatch(Msg::Unlocked(Ok(dummy_session("captain"))));
    let _ = app.dispatch(Msg::LobbyUpdated(Ok(LobbySnapshot {
        cache: ClientCache::empty(),
        my_games: vec![league_my_game_row("joined")],
        open_games: vec![sandbox_open_game_row()],
        notices: Vec::new(),
    })));

    let effects = app.dispatch(Msg::Key(alt_key(nc_helm::KeyCode::Char('d'))));
    assert!(effects.is_empty());
    match &app.model().route {
        Route::Lobby(_) => {}
        other => panic!("expected lobby route, got {other:?}"),
    }
}

#[test]
fn sandbox_release_success_removes_row_and_returns_to_lobby() {
    let (mut app, _) = App::new(None);
    let _ = app.dispatch(Msg::Unlocked(Ok(dummy_session("captain"))));
    let _ = app.dispatch(Msg::LobbyUpdated(Ok(LobbySnapshot {
        cache: ClientCache::empty(),
        my_games: vec![my_game_row("joined")],
        open_games: vec![sandbox_open_game_row()],
        notices: Vec::new(),
    })));
    let _ = app.dispatch(Msg::Key(alt_key(nc_helm::KeyCode::Char('d'))));

    let effects = app.dispatch(Msg::SandboxReleased(Ok(SandboxReleaseSuccess {
        game_id: "phase-sapling-awful".to_string(),
        cache: ClientCache::empty(),
    })));
    assert!(matches!(
        effects.as_slice(),
        [Effect::SaveClientCache { .. }]
    ));
    assert!(app.model().my_games.is_empty());
    match &app.model().route {
        Route::Lobby(lobby) => {
            assert_eq!(
                lobby.status.as_deref(),
                Some("Sandbox removed from My Games.")
            );
        }
        other => panic!("expected lobby route, got {other:?}"),
    }
}

#[test]
fn enter_on_joined_my_game_emits_open_hosted_game_effect() {
    let (mut app, _) = App::new(None);
    let _ = app.dispatch(Msg::Unlocked(Ok(dummy_session("captain"))));
    let _ = app.dispatch(Msg::LobbyUpdated(Ok(LobbySnapshot {
        cache: ClientCache::empty(),
        my_games: vec![my_game_row("joined")],
        open_games: vec![sandbox_open_game_row()],
        notices: Vec::new(),
    })));

    let effects = app.dispatch(Msg::Key(key(nc_helm::KeyCode::Enter)));
    assert!(matches!(
        effects.as_slice(),
        [Effect::OpenHostedGame { row, .. }] if row.game_id == "phase-sapling-awful"
    ));
}

#[test]
fn enter_on_requested_my_game_stays_in_lobby_with_status() {
    let (mut app, _) = App::new(None);
    let _ = app.dispatch(Msg::Unlocked(Ok(dummy_session("captain"))));
    let _ = app.dispatch(Msg::LobbyUpdated(Ok(LobbySnapshot {
        cache: ClientCache::empty(),
        my_games: vec![my_game_row("requested")],
        open_games: vec![sandbox_open_game_row()],
        notices: Vec::new(),
    })));

    let effects = app.dispatch(Msg::Key(key(nc_helm::KeyCode::Enter)));
    assert!(effects.is_empty());
    match &app.model().route {
        Route::Lobby(lobby) => assert_eq!(
            lobby.status.as_deref(),
            Some("Join request is still waiting for nc-host approval.")
        ),
        other => panic!("expected lobby route, got {other:?}"),
    }
}
