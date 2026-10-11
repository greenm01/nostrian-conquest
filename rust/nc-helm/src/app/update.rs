use nc_client::hosted::store::{CachedHostedDraft, HostedDraftStatus};
use nc_client::password::validate_new_password;
use nc_data::TurnSubmission;
use nc_nostr::hosted::relay_url_to_invite_host;
use nostr_sdk::Keys;

use super::{
    DispatchOutcome, Effect, FirstJoinSetupField, HostedGameModel, LOBBY_TAB_ROW,
    LOCK_TIMEOUT_OPTIONS, LobbyTab, Model, Msg, NetworkState, Route, active_session_from_stored,
    append_text, bootstrap_route, field_string_mut, first_join_setup_from_snapshot,
    handle_help_click, is_printable_key, lobby_route, route_supports_session_lock,
    trim_first_join_setup_input,
};
use crate::ScreenGeometry;
use crate::dashboard;
use crate::dashboard::table_selection::{sync_scroll_to_cursor, wrap_next_index, wrap_prev_index};
use crate::input::{KeyCode, KeyModifiers, MouseButton, MouseEventKind};

const TABLE_DATA_ROW_START: usize = 7;

pub fn update(model: &mut Model, msg: Msg) -> DispatchOutcome {
    match msg {
        Msg::Resize(geometry) => {
            model.geometry = geometry;
            model
                .matrix_rain
                .reset_for_size(geometry.width(), geometry.height());
            if let Route::HostedGame(hosted) = &mut model.route {
                dashboard::resize_hosted_app(
                    &mut hosted.dashboard,
                    geometry.width(),
                    geometry.height(),
                );
            }
            DispatchOutcome::redraw(Vec::new())
        }
        Msg::FocusChanged(focused) => {
            model.window_focused = focused;
            DispatchOutcome::redraw(Vec::new())
        }
        Msg::MatrixFrame => {
            if matches!(model.route, Route::MatrixLocked) {
                model.matrix_rain.advance();
            }
            DispatchOutcome::redraw(Vec::new())
        }
        Msg::IdleLock => DispatchOutcome::redraw(handle_idle_lock(model)),
        Msg::BootLoaded(result) => DispatchOutcome::redraw(handle_boot_loaded(model, result)),
        Msg::IdentityCreated(result) => {
            DispatchOutcome::redraw(handle_identity_created(model, result))
        }
        Msg::Unlocked(result) => DispatchOutcome::redraw(handle_unlocked(model, result)),
        Msg::LobbyUpdated(result) => DispatchOutcome::redraw(handle_lobby_updated(model, result)),
        Msg::LobbyRefreshed(result) => {
            DispatchOutcome::redraw(handle_lobby_refreshed(model, result))
        }
        Msg::SandboxJoined(result) => DispatchOutcome::redraw(handle_sandbox_joined(model, result)),
        Msg::SandboxReleased(result) => {
            DispatchOutcome::redraw(handle_sandbox_released(model, result))
        }
        Msg::HostedGameOpened(result) => {
            DispatchOutcome::redraw(handle_hosted_game_opened(model, result))
        }
        Msg::FirstJoinSetupCompleted(result) => {
            DispatchOutcome::redraw(handle_first_join_setup_completed(model, result))
        }
        Msg::RelaySaved(result) => DispatchOutcome::redraw(handle_relay_saved(model, result)),
        Msg::Key(key) => DispatchOutcome::redraw(handle_key(model, key)),
        Msg::TextInput(text) => DispatchOutcome::redraw(handle_text_input(model, &text)),
        Msg::Mouse(mouse) => handle_mouse(model, mouse),
    }
}

fn handle_boot_loaded(
    model: &mut Model,
    result: Result<crate::storage::BootSnapshot, String>,
) -> Vec<Effect> {
    match result {
        Ok(snapshot) => {
            if !model.relay_overridden {
                model.relay_url = snapshot
                    .relay_url
                    .clone()
                    .unwrap_or_else(|| model.relay_url.clone());
            }
            model.lock_timeout_minutes = snapshot.lock_timeout_minutes;
            model.route = bootstrap_route(&snapshot, model.relay_url.clone());
            Vec::new()
        }
        Err(err) => {
            model.route = Route::FatalError(err);
            Vec::new()
        }
    }
}

fn handle_identity_created(
    model: &mut Model,
    result: Result<crate::storage::StoredSession, String>,
) -> Vec<Effect> {
    match result {
        Ok(stored) => {
            let relay_url = model.relay_url.clone();
            let cache = stored.cache.clone();
            let session = active_session_from_stored(stored, current_password(model));
            let nsec = session.active_nsec.clone();
            model.cache = cache.clone();
            model.session = Some(session);
            model.route = lobby_route(Some("Identity created.".to_string()), relay_url.clone());
            model.network = NetworkState::Connecting;
            vec![Effect::ConnectTransport {
                relay_url,
                nsec,
                cache,
            }]
        }
        Err(err) => {
            if let Route::FirstRun(first_run) = &mut model.route {
                first_run.status = Some(err);
            }
            Vec::new()
        }
    }
}

fn handle_unlocked(
    model: &mut Model,
    result: Result<crate::storage::StoredSession, String>,
) -> Vec<Effect> {
    match result {
        Ok(stored) => {
            let relay_url = model.relay_url.clone();
            let cache = stored.cache.clone();
            let session = active_session_from_stored(stored, current_password(model));
            let nsec = session.active_nsec.clone();
            model.cache = cache.clone();
            model.session = Some(session);
            model.route = restore_unlocked_route(model, relay_url.clone());
            model.network = NetworkState::Connecting;
            vec![Effect::ConnectTransport {
                relay_url,
                nsec,
                cache,
            }]
        }
        Err(err) => {
            if let Route::Locked(locked) = &mut model.route {
                locked.status = Some(err);
                locked.password_input.clear();
            }
            Vec::new()
        }
    }
}

fn restore_unlocked_route(model: &mut Model, relay_url: String) -> Route {
    match model.lock_resume_route.take() {
        Some(mut route) => {
            if let Route::Lobby(lobby) = &mut route {
                lobby.relay_draft = relay_url;
                lobby.status = Some("Keychain unlocked.".to_string());
            }
            route
        }
        None => lobby_route(Some("Keychain unlocked.".to_string()), relay_url),
    }
}

fn handle_lobby_updated(
    model: &mut Model,
    result: Result<crate::transport::LobbySnapshot, String>,
) -> Vec<Effect> {
    match result {
        Ok(snapshot) => {
            apply_lobby_snapshot(model, snapshot);
            if let Route::Lobby(lobby) = &mut model.route {
                lobby.status = None;
            }
            if let Some(session) = &model.session {
                return vec![Effect::SaveClientCache {
                    cache: model.cache.clone(),
                    password: session.password.clone(),
                }];
            }
        }
        Err(err) => {
            model.network = NetworkState::Error;
            if let Route::Lobby(lobby) = &mut model.route {
                lobby.status = Some(err);
            }
        }
    }
    Vec::new()
}

fn handle_lobby_refreshed(
    model: &mut Model,
    result: Result<crate::transport::LobbySnapshot, String>,
) -> Vec<Effect> {
    match result {
        Ok(snapshot) => {
            apply_lobby_snapshot(model, snapshot);
            if let Route::Lobby(lobby) = &mut model.route {
                lobby.status = Some("Hosted lobby refreshed.".to_string());
            }
            if let Some(session) = &model.session {
                return vec![Effect::SaveClientCache {
                    cache: model.cache.clone(),
                    password: session.password.clone(),
                }];
            }
        }
        Err(err) => {
            model.network = NetworkState::Error;
            if let Route::Lobby(lobby) = &mut model.route {
                lobby.status = Some(err);
            }
        }
    }
    Vec::new()
}

fn handle_sandbox_joined(
    model: &mut Model,
    result: Result<crate::transport::SandboxJoinResult, String>,
) -> Vec<Effect> {
    match result {
        Ok(crate::transport::SandboxJoinResult::Joined(success)) => {
            model.cache = success.cache.clone();
            upsert_joined_game(model, success.row.clone());
            open_snapshot_route(
                model,
                success.row,
                success.snapshot,
                None,
                Some("Sandbox joined.".to_string()),
            )
        }
        Ok(crate::transport::SandboxJoinResult::Full(message)) => {
            let row = match &model.route {
                Route::SandboxJoinConfirm(row) => row.clone(),
                _ => return Vec::new(),
            };
            model.route = Route::SandboxJoinUnavailable {
                row,
                notice: message,
            };
            Vec::new()
        }
        Err(err) => return_to_lobby_with_status(model, err),
    }
}

fn handle_sandbox_released(
    model: &mut Model,
    result: Result<crate::transport::SandboxReleaseSuccess, String>,
) -> Vec<Effect> {
    match result {
        Ok(success) => {
            model.cache = success.cache;
            model.my_games.retain(|row| row.game_id != success.game_id);
            let mut effects =
                return_to_lobby_with_status(model, "Sandbox removed from My Games.".to_string());
            if let Some(session) = &model.session {
                effects.push(Effect::SaveClientCache {
                    cache: model.cache.clone(),
                    password: session.password.clone(),
                });
            }
            effects
        }
        Err(err) => return_to_lobby_with_status(model, err),
    }
}

fn handle_hosted_game_opened(
    model: &mut Model,
    result: Result<crate::transport::HostedGameOpenResult, String>,
) -> Vec<Effect> {
    match result {
        Ok(crate::transport::HostedGameOpenResult::Opened(success)) => {
            model.cache = success.cache.clone();
            upsert_joined_game(model, success.row.clone());
            open_snapshot_route(
                model,
                success.row,
                success.snapshot,
                success.cached_draft,
                None,
            )
        }
        Ok(crate::transport::HostedGameOpenResult::Expired {
            row,
            cache,
            message,
        }) => {
            model.cache = cache;
            upsert_joined_game(model, row);
            return_to_lobby_with_status(model, message)
        }
        Err(err) => return_to_lobby_with_status(model, err),
    }
}

fn handle_first_join_setup_completed(
    model: &mut Model,
    result: Result<crate::transport::HostedGameOpenSuccess, String>,
) -> Vec<Effect> {
    match result {
        Ok(success) => {
            model.cache = success.cache.clone();
            upsert_joined_game(model, success.row.clone());
            open_snapshot_route(model, success.row, success.snapshot, None, None)
        }
        Err(err) => {
            if let Route::FirstJoinSetup(setup) = &mut model.route {
                setup.status = Some(err);
            }
            Vec::new()
        }
    }
}

fn handle_relay_saved(model: &mut Model, result: Result<String, String>) -> Vec<Effect> {
    match result {
        Ok(relay_url) => {
            model.relay_url = relay_url.clone();
            if let Route::Lobby(lobby) = &mut model.route {
                lobby.relay_draft = relay_url;
                lobby.status = Some("Relay setting saved.".to_string());
            }
        }
        Err(err) => {
            if let Route::Lobby(lobby) = &mut model.route {
                lobby.status = Some(err);
            }
        }
    }
    Vec::new()
}

fn handle_key(model: &mut Model, key: crate::input::KeyEvent) -> Vec<Effect> {
    if is_quit_shortcut(key)
        && !matches!(model.route, Route::HostedGame(_) | Route::Lobby(_))
        && !route_blocks_quit_shortcut(&model.route)
    {
        model.should_quit = true;
        return vec![Effect::Quit];
    }
    if is_lock_shortcut(key) && model.session.is_some() && route_supports_session_lock(&model.route)
    {
        return lock_session(model);
    }
    match model.route {
        Route::Boot(_) => Vec::new(),
        Route::FatalError(_) => match key.code {
            KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('Q') => {
                model.should_quit = true;
                vec![Effect::Quit]
            }
            _ => Vec::new(),
        },
        Route::FirstRun(_) => handle_first_run_key(model, key),
        Route::MatrixLocked => handle_matrix_locked_key(model, key),
        Route::Locked(_) => handle_locked_key(model, key),
        Route::Lobby(_) => handle_lobby_key(model, key),
        Route::SandboxJoinConfirm(_) => handle_sandbox_join_confirm_key(model, key),
        Route::SandboxJoinUnavailable { .. } => handle_sandbox_join_unavailable_key(model),
        Route::SandboxDeleteConfirm(_) => handle_sandbox_delete_confirm_key(model, key),
        Route::FirstJoinSetup(_) => handle_first_join_setup_key(model, key),
        Route::HostedGame(_) => handle_hosted_game_key(model, key),
    }
}

fn handle_mouse(model: &mut Model, mouse: crate::input::MouseEvent) -> DispatchOutcome {
    if mouse.kind == MouseEventKind::Down(MouseButton::Left)
        && handle_help_click(model, mouse.position)
    {
        return DispatchOutcome::redraw(Vec::new());
    }

    let hosted_password = current_password(model);
    let hosted_player_pubkey = current_player_pubkey_hex(model);

    match &mut model.route {
        Route::Lobby(lobby) => {
            if mouse.kind != MouseEventKind::Down(MouseButton::Left) {
                return DispatchOutcome::no_redraw(Vec::new());
            }

            let row = mouse.position.row.as_usize();
            let column = mouse.position.column.as_usize();
            if row == LOBBY_TAB_ROW {
                for (tab, start, end) in super::lobby_tab_bounds(model.geometry) {
                    if column >= start && column < end {
                        let changed = lobby.active_tab != tab;
                        lobby.active_tab = tab;
                        return DispatchOutcome::new(Vec::new(), changed);
                    }
                }
            }

            if row < TABLE_DATA_ROW_START || row >= model.geometry.height().saturating_sub(4) {
                return DispatchOutcome::no_redraw(Vec::new());
            }

            let visible_rows = lobby_table_visible_rows(model.geometry, lobby.status.is_some());
            if row >= TABLE_DATA_ROW_START.saturating_add(visible_rows) {
                return DispatchOutcome::no_redraw(Vec::new());
            }

            let mut changed = false;
            match lobby.active_tab {
                LobbyTab::MyGames if !model.my_games.is_empty() => {
                    let previous_selected = lobby.selected_my_game;
                    let previous_scroll = lobby.my_games_scroll;
                    let index = lobby.my_games_scroll + row.saturating_sub(TABLE_DATA_ROW_START);
                    lobby.selected_my_game = index.min(model.my_games.len().saturating_sub(1));
                    sync_scroll_to_cursor(
                        &mut lobby.my_games_scroll,
                        lobby.selected_my_game,
                        visible_rows,
                    );
                    changed = lobby.selected_my_game != previous_selected
                        || lobby.my_games_scroll != previous_scroll;
                }
                LobbyTab::OpenGames if !model.open_games.is_empty() => {
                    let previous_selected = lobby.selected_open_game;
                    let previous_scroll = lobby.open_games_scroll;
                    let index = lobby.open_games_scroll + row.saturating_sub(TABLE_DATA_ROW_START);
                    lobby.selected_open_game = index.min(model.open_games.len().saturating_sub(1));
                    sync_scroll_to_cursor(
                        &mut lobby.open_games_scroll,
                        lobby.selected_open_game,
                        visible_rows,
                    );
                    changed = lobby.selected_open_game != previous_selected
                        || lobby.open_games_scroll != previous_scroll;
                }
                _ => {}
            }
            DispatchOutcome::new(Vec::new(), changed)
        }
        Route::HostedGame(hosted) => {
            let Some(mapped) = dashboard_mouse_event(mouse) else {
                return DispatchOutcome::no_redraw(Vec::new());
            };
            let before = hosted.dashboard.hosted_turn_text();
            let row = hosted.row.clone();
            let password = hosted_password;
            let player_pubkey = hosted_player_pubkey;
            let changed = dashboard::dispatch_hosted_mouse(&mut hosted.dashboard, mapped);
            let after = hosted_turn_draft_for_save(&hosted.dashboard);
            let after_text = after.as_ref().map(TurnSubmission::to_kdl_string);
            let effects =
                hosted_draft_save_effect(row, password, player_pubkey, before, after_text, after)
                    .into_iter()
                    .collect();
            DispatchOutcome::new(effects, changed)
        }
        _ => DispatchOutcome::no_redraw(Vec::new()),
    }
}

fn handle_text_input(model: &mut Model, text: &str) -> Vec<Effect> {
    match &mut model.route {
        Route::FirstRun(first_run) => {
            append_text(field_string_mut(first_run), text);
            Vec::new()
        }
        Route::Locked(locked) => {
            append_text(&mut locked.password_input, text);
            Vec::new()
        }
        Route::Lobby(lobby) if lobby.active_tab == LobbyTab::Settings && lobby.editing_relay => {
            append_text(&mut lobby.relay_draft, text);
            Vec::new()
        }
        Route::FirstJoinSetup(setup) => {
            let input = match setup.active_field {
                FirstJoinSetupField::Empire => &mut setup.empire_input,
                FirstJoinSetupField::Homeworld => &mut setup.homeworld_input,
            };
            append_text(input, text);
            trim_first_join_setup_input(input);
            Vec::new()
        }
        _ => Vec::new(),
    }
}

fn handle_first_run_key(model: &mut Model, key: crate::input::KeyEvent) -> Vec<Effect> {
    let Route::FirstRun(first_run) = &mut model.route else {
        return Vec::new();
    };
    match key.code {
        KeyCode::Tab => {
            first_run.active_field = first_run.active_field.next();
            Vec::new()
        }
        KeyCode::BackTab => {
            first_run.active_field = first_run.active_field.previous();
            Vec::new()
        }
        KeyCode::Backspace => {
            field_string_mut(first_run).pop();
            Vec::new()
        }
        KeyCode::Enter => {
            let handle = first_run.handle_input.trim().to_string();
            let password = first_run.password_input.clone();
            let confirm = first_run.confirm_input.clone();
            let relay = first_run.relay_input.trim().to_string();
            if handle.is_empty() {
                first_run.status = Some("Handle cannot be empty.".to_string());
                return Vec::new();
            }
            if let Err(err) = validate_new_password(&password, &confirm) {
                first_run.status = Some(err);
                return Vec::new();
            }
            if let Err(err) = relay_url_to_invite_host(&relay) {
                first_run.status = Some(err);
                return Vec::new();
            }
            model.relay_url = relay.clone();
            first_run.status = Some("Creating local identity...".to_string());
            vec![Effect::CreateIdentity {
                handle,
                password,
                relay_url: relay,
            }]
        }
        KeyCode::Esc => {
            model.should_quit = true;
            vec![Effect::Quit]
        }
        _ => {
            if let Some(ch) = is_printable_key(key) {
                field_string_mut(first_run).push(ch);
            }
            Vec::new()
        }
    }
}

fn handle_matrix_locked_key(model: &mut Model, key: crate::input::KeyEvent) -> Vec<Effect> {
    if is_quit_shortcut(key) {
        return Vec::new();
    }
    match key.code {
        _ => {
            model.route = Route::Locked(super::LockedModel {
                password_input: String::new(),
                status: None,
                resume_session: true,
            });
            Vec::new()
        }
    }
}

fn handle_locked_key(model: &mut Model, key: crate::input::KeyEvent) -> Vec<Effect> {
    let Route::Locked(locked) = &mut model.route else {
        return Vec::new();
    };
    if locked.resume_session && is_quit_shortcut(key) {
        return Vec::new();
    }
    match key.code {
        KeyCode::Backspace => {
            locked.password_input.pop();
            Vec::new()
        }
        KeyCode::Enter => {
            let password = locked.password_input.clone();
            if password.is_empty() {
                locked.status = Some("Password cannot be empty.".to_string());
                return Vec::new();
            }
            locked.status = None;
            vec![Effect::Unlock { password }]
        }
        KeyCode::Esc => {
            if locked.resume_session {
                model.route = Route::MatrixLocked;
                Vec::new()
            } else {
                model.should_quit = true;
                vec![Effect::Quit]
            }
        }
        _ => {
            if let Some(ch) = is_printable_key(key) {
                locked.password_input.push(ch);
            }
            Vec::new()
        }
    }
}

fn handle_lobby_key(model: &mut Model, key: crate::input::KeyEvent) -> Vec<Effect> {
    let settings_row_count = lobby_settings_rows(model).len();
    let Route::Lobby(lobby) = &mut model.route else {
        return Vec::new();
    };
    let visible_rows = lobby_table_visible_rows(model.geometry, lobby.status.is_some());
    if is_quit_shortcut(key) {
        lobby.quit_confirm_open = true;
        return Vec::new();
    }
    if lobby.quit_confirm_open {
        match key.code {
            KeyCode::Char('y') | KeyCode::Char('Y') => {
                model.should_quit = true;
                vec![Effect::Quit]
            }
            KeyCode::Enter | KeyCode::Esc | KeyCode::Char('n') | KeyCode::Char('N') => {
                lobby.quit_confirm_open = false;
                Vec::new()
            }
            _ => Vec::new(),
        }
    } else if lobby.help_open {
        match key.code {
            KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('Q') => lobby.help_open = false,
            _ => {}
        }
        Vec::new()
    } else {
        if lobby.active_tab == LobbyTab::Settings && lobby.editing_relay {
            match key.code {
                KeyCode::Enter => {
                    let relay_url = lobby.relay_draft.trim().to_string();
                    if let Err(err) = relay_url_to_invite_host(&relay_url) {
                        lobby.status = Some(err);
                        return Vec::new();
                    }
                    lobby.editing_relay = false;
                    model.relay_url = relay_url.clone();
                    model.network = if model.session.is_some() {
                        NetworkState::Connecting
                    } else {
                        NetworkState::Idle
                    };
                    let mut effects = vec![Effect::SaveRelayUrl {
                        relay_url: relay_url.clone(),
                    }];
                    if let Some(session) = &model.session {
                        effects.push(Effect::DisconnectTransport);
                        effects.push(Effect::ConnectTransport {
                            relay_url,
                            nsec: session.active_nsec.clone(),
                            cache: model.cache.clone(),
                        });
                    }
                    return effects;
                }
                KeyCode::Esc => {
                    lobby.editing_relay = false;
                    lobby.relay_draft = model.relay_url.clone();
                    lobby.status = Some("Relay edit cancelled.".to_string());
                    return Vec::new();
                }
                KeyCode::Backspace => {
                    lobby.relay_draft.pop();
                    return Vec::new();
                }
                _ => {
                    if let Some(ch) = is_printable_key(key) {
                        lobby.relay_draft.push(ch);
                    }
                    return Vec::new();
                }
            }
        }

        match key.code {
            KeyCode::Esc => {
                lobby.quit_confirm_open = true;
                Vec::new()
            }
            KeyCode::Tab => {
                lobby.active_tab = lobby.active_tab.next();
                Vec::new()
            }
            KeyCode::Up => {
                match lobby.active_tab {
                    LobbyTab::MyGames if !model.my_games.is_empty() => {
                        lobby.selected_my_game =
                            wrap_prev_index(lobby.selected_my_game, model.my_games.len());
                        sync_scroll_to_cursor(
                            &mut lobby.my_games_scroll,
                            lobby.selected_my_game,
                            visible_rows,
                        );
                    }
                    LobbyTab::OpenGames if !model.open_games.is_empty() => {
                        lobby.selected_open_game =
                            wrap_prev_index(lobby.selected_open_game, model.open_games.len());
                        sync_scroll_to_cursor(
                            &mut lobby.open_games_scroll,
                            lobby.selected_open_game,
                            visible_rows,
                        );
                    }
                    LobbyTab::Settings => {
                        lobby.settings_scroll = lobby.settings_scroll.saturating_sub(1);
                    }
                    LobbyTab::MyGames | LobbyTab::OpenGames => {}
                    LobbyTab::Comms => {}
                }
                Vec::new()
            }
            KeyCode::Down => {
                match lobby.active_tab {
                    LobbyTab::MyGames if !model.my_games.is_empty() => {
                        lobby.selected_my_game =
                            wrap_next_index(lobby.selected_my_game, model.my_games.len());
                        sync_scroll_to_cursor(
                            &mut lobby.my_games_scroll,
                            lobby.selected_my_game,
                            visible_rows,
                        );
                    }
                    LobbyTab::OpenGames if !model.open_games.is_empty() => {
                        lobby.selected_open_game =
                            wrap_next_index(lobby.selected_open_game, model.open_games.len());
                        sync_scroll_to_cursor(
                            &mut lobby.open_games_scroll,
                            lobby.selected_open_game,
                            visible_rows,
                        );
                    }
                    LobbyTab::Settings => {
                        let max_scroll = settings_row_count.saturating_sub(visible_rows);
                        lobby.settings_scroll =
                            lobby.settings_scroll.saturating_add(1).min(max_scroll);
                    }
                    LobbyTab::MyGames | LobbyTab::OpenGames => {}
                    LobbyTab::Comms => {}
                }
                Vec::new()
            }
            KeyCode::PageUp => {
                match lobby.active_tab {
                    LobbyTab::MyGames if !model.my_games.is_empty() => {
                        lobby.selected_my_game =
                            lobby.selected_my_game.saturating_sub(visible_rows);
                        sync_scroll_to_cursor(
                            &mut lobby.my_games_scroll,
                            lobby.selected_my_game,
                            visible_rows,
                        );
                    }
                    LobbyTab::OpenGames if !model.open_games.is_empty() => {
                        lobby.selected_open_game =
                            lobby.selected_open_game.saturating_sub(visible_rows);
                        sync_scroll_to_cursor(
                            &mut lobby.open_games_scroll,
                            lobby.selected_open_game,
                            visible_rows,
                        );
                    }
                    LobbyTab::Settings => {
                        lobby.settings_scroll = lobby.settings_scroll.saturating_sub(visible_rows);
                    }
                    LobbyTab::MyGames | LobbyTab::OpenGames => {}
                    LobbyTab::Comms => {}
                }
                Vec::new()
            }
            KeyCode::PageDown => {
                match lobby.active_tab {
                    LobbyTab::MyGames if !model.my_games.is_empty() => {
                        lobby.selected_my_game = lobby
                            .selected_my_game
                            .saturating_add(visible_rows)
                            .min(model.my_games.len().saturating_sub(1));
                        sync_scroll_to_cursor(
                            &mut lobby.my_games_scroll,
                            lobby.selected_my_game,
                            visible_rows,
                        );
                    }
                    LobbyTab::OpenGames if !model.open_games.is_empty() => {
                        lobby.selected_open_game = lobby
                            .selected_open_game
                            .saturating_add(visible_rows)
                            .min(model.open_games.len().saturating_sub(1));
                        sync_scroll_to_cursor(
                            &mut lobby.open_games_scroll,
                            lobby.selected_open_game,
                            visible_rows,
                        );
                    }
                    LobbyTab::Settings => {
                        let max_scroll = settings_row_count.saturating_sub(visible_rows);
                        lobby.settings_scroll = lobby
                            .settings_scroll
                            .saturating_add(visible_rows)
                            .min(max_scroll);
                    }
                    LobbyTab::MyGames | LobbyTab::OpenGames => {}
                    LobbyTab::Comms => {}
                }
                Vec::new()
            }
            KeyCode::Home => {
                match lobby.active_tab {
                    LobbyTab::MyGames => {
                        lobby.selected_my_game = 0;
                        lobby.my_games_scroll = 0;
                    }
                    LobbyTab::OpenGames => {
                        lobby.selected_open_game = 0;
                        lobby.open_games_scroll = 0;
                    }
                    LobbyTab::Settings => {
                        lobby.settings_scroll = 0;
                    }
                    LobbyTab::Comms => {}
                }
                Vec::new()
            }
            KeyCode::End => {
                match lobby.active_tab {
                    LobbyTab::MyGames if !model.my_games.is_empty() => {
                        lobby.selected_my_game = model.my_games.len().saturating_sub(1);
                        sync_scroll_to_cursor(
                            &mut lobby.my_games_scroll,
                            lobby.selected_my_game,
                            visible_rows,
                        );
                    }
                    LobbyTab::OpenGames if !model.open_games.is_empty() => {
                        lobby.selected_open_game = model.open_games.len().saturating_sub(1);
                        sync_scroll_to_cursor(
                            &mut lobby.open_games_scroll,
                            lobby.selected_open_game,
                            visible_rows,
                        );
                    }
                    LobbyTab::Settings => {
                        lobby.settings_scroll = settings_row_count.saturating_sub(visible_rows);
                    }
                    LobbyTab::MyGames | LobbyTab::OpenGames => {}
                    LobbyTab::Comms => {}
                }
                Vec::new()
            }
            KeyCode::Enter => activate_selected_row(model),
            KeyCode::Char('?') | KeyCode::Char('h') | KeyCode::Char('H') => {
                lobby.help_open = true;
                Vec::new()
            }
            KeyCode::Char('r') | KeyCode::Char('R')
                if key.modifiers.contains(KeyModifiers::ALT) =>
            {
                lobby.status = Some("Refreshing hosted lobby...".to_string());
                model.network = NetworkState::Connecting;
                vec![Effect::RefreshLobby]
            }
            KeyCode::Char('d') | KeyCode::Char('D')
                if key.modifiers.contains(KeyModifiers::ALT) =>
            {
                open_sandbox_delete_confirm(model)
            }
            KeyCode::Char('m') | KeyCode::Char('M') => {
                lobby.active_tab = LobbyTab::MyGames;
                Vec::new()
            }
            KeyCode::Char('o') | KeyCode::Char('O') => {
                lobby.active_tab = LobbyTab::OpenGames;
                Vec::new()
            }
            KeyCode::Char('c') | KeyCode::Char('C') => {
                lobby.active_tab = LobbyTab::Comms;
                Vec::new()
            }
            KeyCode::Char('s') | KeyCode::Char('S') => {
                lobby.active_tab = LobbyTab::Settings;
                Vec::new()
            }
            KeyCode::Char('r') | KeyCode::Char('R') if lobby.active_tab == LobbyTab::Settings => {
                lobby.editing_relay = true;
                lobby.settings_scroll = 0;
                lobby.relay_draft = model.relay_url.clone();
                lobby.status = Some("Editing relay URL. Enter saves, Esc cancels.".to_string());
                Vec::new()
            }
            KeyCode::Char('i') | KeyCode::Char('I') if lobby.active_tab == LobbyTab::Settings => {
                let next = cycle_lock_timeout(model.lock_timeout_minutes);
                model.lock_timeout_minutes = next;
                lobby.status = Some(format!(
                    "Idle lock timeout set to {}.",
                    lock_timeout_label(next)
                ));
                vec![Effect::SaveLockTimeout {
                    lock_timeout_minutes: next,
                }]
            }
            KeyCode::Char('j') | KeyCode::Char('J') if lobby.active_tab == LobbyTab::OpenGames => {
                activate_selected_row(model)
            }
            _ => Vec::new(),
        }
    }
}

fn handle_sandbox_join_confirm_key(model: &mut Model, key: crate::input::KeyEvent) -> Vec<Effect> {
    let Route::SandboxJoinConfirm(row) = &model.route else {
        return Vec::new();
    };
    if !matches!(key.code, KeyCode::Char('y') | KeyCode::Char('Y')) {
        return_to_lobby_with_status(model, String::new())
    } else {
        vec![Effect::JoinSandboxGame {
            row: row.clone(),
            password: current_password(model),
            handle: model
                .session
                .as_ref()
                .and_then(|session| session.active_handle.clone()),
        }]
    }
}

fn handle_sandbox_join_unavailable_key(model: &mut Model) -> Vec<Effect> {
    return_to_lobby_with_status(model, String::new())
}

fn handle_sandbox_delete_confirm_key(
    model: &mut Model,
    key: crate::input::KeyEvent,
) -> Vec<Effect> {
    let Route::SandboxDeleteConfirm(row) = &model.route else {
        return Vec::new();
    };
    if matches!(key.code, KeyCode::Char('y') | KeyCode::Char('Y')) {
        let row = row.clone();
        model.route = lobby_route(
            Some("Releasing sandbox seat...".to_string()),
            model.relay_url.clone(),
        );
        vec![Effect::ReleaseSandboxGame { row }]
    } else {
        return_to_lobby_with_status(model, String::new())
    }
}

fn handle_first_join_setup_key(model: &mut Model, key: crate::input::KeyEvent) -> Vec<Effect> {
    let Route::FirstJoinSetup(setup) = &mut model.route else {
        return Vec::new();
    };
    match key.code {
        KeyCode::Tab | KeyCode::BackTab => {
            setup.active_field = setup.active_field.next();
            Vec::new()
        }
        KeyCode::Backspace => {
            match setup.active_field {
                FirstJoinSetupField::Empire => {
                    setup.empire_input.pop();
                }
                FirstJoinSetupField::Homeworld => {
                    setup.homeworld_input.pop();
                }
            }
            Vec::new()
        }
        KeyCode::Esc => {
            return_to_lobby_with_status(model, "First-join setup cancelled.".to_string())
        }
        KeyCode::Enter => {
            let empire_name = setup.empire_input.trim().to_string();
            let homeworld_name = setup.homeworld_input.trim().to_string();
            if empire_name.is_empty() {
                setup.status =
                    Some("Empire names need at least one visible character.".to_string());
                setup.active_field = FirstJoinSetupField::Empire;
                return Vec::new();
            }
            if setup.active_field == FirstJoinSetupField::Empire && homeworld_name.is_empty() {
                setup.active_field = FirstJoinSetupField::Homeworld;
                return Vec::new();
            }
            if homeworld_name.is_empty() {
                setup.status =
                    Some("Homeworld names need at least one visible character.".to_string());
                setup.active_field = FirstJoinSetupField::Homeworld;
                return Vec::new();
            }
            vec![Effect::CompleteFirstJoinSetup {
                row: setup.row.clone(),
                empire_name,
                homeworld_name,
                password: current_password(model),
            }]
        }
        _ => {
            if let Some(ch) = is_printable_key(key) {
                let input = match setup.active_field {
                    FirstJoinSetupField::Empire => &mut setup.empire_input,
                    FirstJoinSetupField::Homeworld => &mut setup.homeworld_input,
                };
                input.push(ch);
                trim_first_join_setup_input(input);
                setup.status = None;
            }
            Vec::new()
        }
    }
}

fn handle_hosted_game_key(model: &mut Model, key: crate::input::KeyEvent) -> Vec<Effect> {
    let password = current_password(model);
    let player_pubkey = current_player_pubkey_hex(model);
    let Route::HostedGame(hosted) = &mut model.route else {
        return Vec::new();
    };
    if let Some(mapped) = dashboard_key_event(key) {
        let before = hosted.dashboard.hosted_turn_text();
        let row = hosted.row.clone();
        dashboard::dispatch_hosted_key(&mut hosted.dashboard, mapped);
        let after = hosted_turn_draft_for_save(&hosted.dashboard);
        let after_text = after.as_ref().map(TurnSubmission::to_kdl_string);
        let save_effect =
            hosted_draft_save_effect(row, password, player_pubkey, before, after_text, after);
        if let Some(request) = dashboard::hosted_take_exit_request(&mut hosted.dashboard) {
            let mut effects = match request {
                dashboard::DashboardExitRequest::QuitClient => {
                    model.should_quit = true;
                    vec![Effect::Quit]
                }
                dashboard::DashboardExitRequest::ReturnToLobby => {
                    return_to_lobby_with_status(model, String::new())
                }
            };
            if let Some(effect) = save_effect {
                effects.push(effect);
            }
            return effects;
        }
        if let Some(effect) = save_effect {
            return vec![effect];
        }
    }
    Vec::new()
}

fn hosted_turn_draft_for_save(dashboard: &dashboard::DashApp) -> Option<TurnSubmission> {
    dashboard
        .hosted_turn_draft
        .as_ref()
        .and_then(|draft| dashboard.hosted_turn_text().map(|_| draft.clone()))
}

fn hosted_draft_save_effect(
    row: super::MyGameRow,
    password: String,
    player_pubkey: Option<String>,
    before: Option<String>,
    after_text: Option<String>,
    draft: Option<TurnSubmission>,
) -> Option<Effect> {
    if before == after_text {
        return None;
    }
    Some(Effect::SaveHostedTurnDraft {
        game_id: row.game_id,
        player_pubkey: player_pubkey?,
        password,
        base_hash: row.last_hash.unwrap_or_default(),
        draft,
    })
}

fn current_player_pubkey_hex(model: &Model) -> Option<String> {
    model
        .session
        .as_ref()
        .and_then(|session| Keys::parse(&session.active_nsec).ok())
        .map(|keys| keys.public_key().to_hex())
}

fn activate_selected_row(model: &mut Model) -> Vec<Effect> {
    let Some(lobby) = (match &model.route {
        Route::Lobby(lobby) => Some(lobby.clone()),
        _ => None,
    }) else {
        return Vec::new();
    };
    match lobby.active_tab {
        LobbyTab::OpenGames => activate_selected_open_game(model, &lobby),
        LobbyTab::MyGames => open_or_claim_selected_game(model, &lobby),
        _ => Vec::new(),
    }
}

fn activate_selected_open_game(model: &mut Model, lobby: &super::LobbyModel) -> Vec<Effect> {
    let Some(row) = model.open_games.get(lobby.selected_open_game).cloned() else {
        return return_to_lobby_with_status(model, "no hosted game selected".to_string());
    };
    if row.game_tier.eq_ignore_ascii_case("sandbox") {
        if let Some(joined_row) = model
            .my_games
            .iter()
            .find(|joined| joined.game_id == row.game_id && joined.status == "joined")
            .cloned()
        {
            return open_joined_game(model, joined_row);
        }
        model.route = Route::SandboxJoinConfirm(row);
        return Vec::new();
    }
    return_to_lobby_with_status(
        model,
        "League join requests are not wired in nc-helm yet.".to_string(),
    )
}

fn open_or_claim_selected_game(model: &mut Model, lobby: &super::LobbyModel) -> Vec<Effect> {
    let Some(row) = model.my_games.get(lobby.selected_my_game).cloned() else {
        return return_to_lobby_with_status(model, "no hosted game selected".to_string());
    };
    if row.status != "joined" {
        let message = match row.status.as_str() {
            "requested" => "Join request is still waiting for nc-host approval.",
            "rejected" => {
                "Join request was rejected. Select the game in Open Games to request again."
            }
            "expired" => "Your sandbox seat is no longer active. Rejoin from Open Games.",
            _ => "This game is not ready to open from the lobby.",
        };
        return return_to_lobby_with_status(model, message.to_string());
    }
    open_joined_game(model, row)
}

fn open_joined_game(model: &mut Model, row: super::MyGameRow) -> Vec<Effect> {
    vec![Effect::OpenHostedGame {
        row,
        password: current_password(model),
        handle: model
            .session
            .as_ref()
            .and_then(|session| session.active_handle.clone()),
    }]
}

fn open_sandbox_delete_confirm(model: &mut Model) -> Vec<Effect> {
    let Some(lobby) = (match &model.route {
        Route::Lobby(lobby) => Some(lobby.clone()),
        _ => None,
    }) else {
        return Vec::new();
    };
    if lobby.active_tab != LobbyTab::MyGames {
        return Vec::new();
    }
    let Some(row) = model.my_games.get(lobby.selected_my_game).cloned() else {
        return Vec::new();
    };
    if !row.game_tier.eq_ignore_ascii_case("sandbox") {
        return Vec::new();
    }
    model.route = Route::SandboxDeleteConfirm(row);
    Vec::new()
}

fn open_snapshot_route(
    model: &mut Model,
    row: super::MyGameRow,
    snapshot: nc_nostr::state_sync::GameState,
    cached_draft: Option<CachedHostedDraft>,
    lobby_status: Option<String>,
) -> Vec<Effect> {
    if let Some(setup) = first_join_setup_from_snapshot(row.clone(), &snapshot) {
        model.route = Route::FirstJoinSetup(setup);
        return Vec::new();
    }
    match dashboard::build_hosted_dash_app(
        &snapshot,
        dashboard::ScreenGeometry::new(model.geometry.width(), model.geometry.height()),
    ) {
        Ok(mut dashboard) => {
            if let Some(cached_draft) = replayable_cached_draft(&snapshot, cached_draft) {
                if let Err(err) =
                    dashboard::replay_hosted_draft(&mut dashboard, &cached_draft.draft)
                {
                    return return_to_lobby_with_status(
                        model,
                        lobby_status
                            .map(|status| format!("{status} Unable to replay saved orders: {err}"))
                            .unwrap_or_else(|| format!("Unable to replay saved orders: {err}")),
                    );
                }
            }
            model.route = Route::HostedGame(HostedGameModel {
                row,
                dashboard,
                status: None,
            });
            Vec::new()
        }
        Err(err) => return_to_lobby_with_status(
            model,
            lobby_status
                .map(|status| format!("{status} Unable to build hosted dashboard: {err}"))
                .unwrap_or_else(|| format!("Unable to build hosted dashboard: {err}")),
        ),
    }
}

fn replayable_cached_draft(
    snapshot: &nc_nostr::state_sync::GameState,
    cached_draft: Option<CachedHostedDraft>,
) -> Option<CachedHostedDraft> {
    cached_draft.filter(|cached| {
        cached.status == HostedDraftStatus::Local
            && cached.turn == snapshot.turn
            && cached.base_hash == snapshot.state_hash
            && cached.draft.player_record_index_1_based == snapshot.player_seat as usize
            && u32::from(cached.draft.year.saturating_sub(3000)) == snapshot.turn
    })
}

fn return_to_lobby_with_status(model: &mut Model, status: String) -> Vec<Effect> {
    let mut route = super::lobby_route(
        if status.is_empty() {
            None
        } else {
            Some(status)
        },
        model.relay_url.clone(),
    );
    if let Route::Lobby(lobby) = &mut route {
        lobby.selected_my_game = 0;
        lobby.selected_open_game = 0;
    }
    model.route = route;
    Vec::new()
}

fn upsert_joined_game(model: &mut Model, row: super::MyGameRow) {
    if let Some(index) = model
        .my_games
        .iter()
        .position(|existing| existing.game_id == row.game_id)
    {
        model.my_games[index] = row;
    } else {
        model.my_games.push(row);
    }
    model
        .my_games
        .sort_by(|left, right| left.game_id.cmp(&right.game_id));
}

fn apply_lobby_snapshot(model: &mut Model, snapshot: crate::transport::LobbySnapshot) {
    model.network = NetworkState::Synced;
    model.cache = snapshot.cache;
    model.my_games = snapshot.my_games;
    model.open_games = snapshot.open_games;
    model.notices = snapshot.notices;
    let settings_row_count = lobby_settings_rows(model).len();
    if let Route::Lobby(lobby) = &mut model.route {
        let visible_rows = lobby_table_visible_rows(model.geometry, lobby.status.is_some());
        lobby.selected_my_game = lobby
            .selected_my_game
            .min(model.my_games.len().saturating_sub(1));
        lobby.selected_open_game = lobby
            .selected_open_game
            .min(model.open_games.len().saturating_sub(1));
        sync_scroll_to_cursor(
            &mut lobby.my_games_scroll,
            lobby.selected_my_game,
            visible_rows,
        );
        sync_scroll_to_cursor(
            &mut lobby.open_games_scroll,
            lobby.selected_open_game,
            visible_rows,
        );
        lobby.my_games_scroll = lobby
            .my_games_scroll
            .min(model.my_games.len().saturating_sub(visible_rows));
        lobby.open_games_scroll = lobby
            .open_games_scroll
            .min(model.open_games.len().saturating_sub(visible_rows));
        lobby.settings_scroll = lobby
            .settings_scroll
            .min(settings_row_count.saturating_sub(visible_rows));
    }
}

fn route_blocks_quit_shortcut(route: &Route) -> bool {
    matches!(route, Route::MatrixLocked)
        || matches!(route, Route::Locked(locked) if locked.resume_session)
}

fn lobby_table_visible_rows(geometry: ScreenGeometry, reserve_status_row: bool) -> usize {
    let command_panel_top = geometry.height().saturating_sub(3);
    let content_bottom_row = if reserve_status_row {
        command_panel_top.saturating_sub(2)
    } else {
        command_panel_top.saturating_sub(1)
    };
    content_bottom_row.saturating_sub(TABLE_DATA_ROW_START)
}

fn lobby_settings_rows(model: &Model) -> Vec<String> {
    let mut rows = vec![
        String::from("Relay URL"),
        format!(
            "Window Focus : {}",
            if model.window_focused { "yes" } else { "no" }
        ),
        format!(
            "Text Input   : {}",
            if model.wants_text_input() {
                "armed"
            } else {
                "off"
            }
        ),
        format!(
            "Idle Lock    : {}",
            if model.lock_timeout_minutes == 0 {
                String::from("Off")
            } else {
                format!("{} min", model.lock_timeout_minutes)
            }
        ),
    ];
    if let Some(session) = &model.session {
        rows.push(format!(
            "Handle       : {}",
            session.active_handle.as_deref().unwrap_or("unset")
        ));
        rows.push(format!("Identity     : {}", session.active_npub));
    }
    rows.push(String::from(
        "R : Edit relay URL   Enter : Save relay   Esc : Cancel edit",
    ));
    rows.push(String::from(
        "L : Lock the local session and stop background sync",
    ));
    rows.push(String::from("I : Cycle idle lock timeout"));
    rows.push(String::from("Alt+Q : Quit nc-helm"));
    rows
}

fn dashboard_key_event(key: crate::input::KeyEvent) -> Option<crate::dashboard::input::KeyEvent> {
    Some(crate::dashboard::input::KeyEvent::new(
        dashboard_key_code(key.code)?,
        dashboard_modifiers(key.modifiers),
    ))
}

fn dashboard_mouse_event(
    mouse: crate::input::MouseEvent,
) -> Option<crate::dashboard::input::MouseEvent> {
    Some(crate::dashboard::input::MouseEvent {
        kind: match mouse.kind {
            MouseEventKind::Down(MouseButton::Left) => {
                crate::dashboard::input::MouseEventKind::Down(
                    crate::dashboard::input::MouseButton::Left,
                )
            }
            MouseEventKind::Down(MouseButton::Right) => {
                crate::dashboard::input::MouseEventKind::Down(
                    crate::dashboard::input::MouseButton::Right,
                )
            }
            MouseEventKind::Down(MouseButton::Middle) => {
                crate::dashboard::input::MouseEventKind::Down(
                    crate::dashboard::input::MouseButton::Middle,
                )
            }
            MouseEventKind::Up(MouseButton::Left) => crate::dashboard::input::MouseEventKind::Up(
                crate::dashboard::input::MouseButton::Left,
            ),
            MouseEventKind::Up(MouseButton::Right) => crate::dashboard::input::MouseEventKind::Up(
                crate::dashboard::input::MouseButton::Right,
            ),
            MouseEventKind::Up(MouseButton::Middle) => crate::dashboard::input::MouseEventKind::Up(
                crate::dashboard::input::MouseButton::Middle,
            ),
            MouseEventKind::Drag(MouseButton::Left) => {
                crate::dashboard::input::MouseEventKind::Drag(
                    crate::dashboard::input::MouseButton::Left,
                )
            }
            MouseEventKind::Drag(MouseButton::Right) => {
                crate::dashboard::input::MouseEventKind::Drag(
                    crate::dashboard::input::MouseButton::Right,
                )
            }
            MouseEventKind::Drag(MouseButton::Middle) => {
                crate::dashboard::input::MouseEventKind::Drag(
                    crate::dashboard::input::MouseButton::Middle,
                )
            }
            MouseEventKind::Scroll { lines } => {
                crate::dashboard::input::MouseEventKind::Scroll { lines }
            }
            MouseEventKind::Moved => crate::dashboard::input::MouseEventKind::Moved,
        },
        column: mouse.position.column.as_usize().min(u16::MAX as usize) as u16,
        row: mouse.position.row.as_usize().min(u16::MAX as usize) as u16,
        modifiers: dashboard_modifiers(mouse.modifiers),
    })
}

fn dashboard_key_code(code: KeyCode) -> Option<crate::dashboard::input::KeyCode> {
    Some(match code {
        KeyCode::Backspace => crate::dashboard::input::KeyCode::Backspace,
        KeyCode::Enter => crate::dashboard::input::KeyCode::Enter,
        KeyCode::Left => crate::dashboard::input::KeyCode::Left,
        KeyCode::Right => crate::dashboard::input::KeyCode::Right,
        KeyCode::Up => crate::dashboard::input::KeyCode::Up,
        KeyCode::Down => crate::dashboard::input::KeyCode::Down,
        KeyCode::Home => crate::dashboard::input::KeyCode::Home,
        KeyCode::End => crate::dashboard::input::KeyCode::End,
        KeyCode::PageUp => crate::dashboard::input::KeyCode::PageUp,
        KeyCode::PageDown => crate::dashboard::input::KeyCode::PageDown,
        KeyCode::Tab => crate::dashboard::input::KeyCode::Tab,
        KeyCode::BackTab => crate::dashboard::input::KeyCode::BackTab,
        KeyCode::Delete => crate::dashboard::input::KeyCode::Delete,
        KeyCode::Esc => crate::dashboard::input::KeyCode::Esc,
        KeyCode::F(n) => crate::dashboard::input::KeyCode::F(n),
        KeyCode::Char(ch) => crate::dashboard::input::KeyCode::Char(ch),
    })
}

fn dashboard_modifiers(modifiers: KeyModifiers) -> crate::dashboard::input::KeyModifiers {
    let mut mapped = crate::dashboard::input::KeyModifiers::empty();
    if modifiers.contains(KeyModifiers::SHIFT) {
        mapped.insert(crate::dashboard::input::KeyModifiers::SHIFT);
    }
    if modifiers.contains(KeyModifiers::CONTROL) {
        mapped.insert(crate::dashboard::input::KeyModifiers::CONTROL);
    }
    if modifiers.contains(KeyModifiers::ALT) {
        mapped.insert(crate::dashboard::input::KeyModifiers::ALT);
    }
    mapped
}

fn handle_idle_lock(model: &mut Model) -> Vec<Effect> {
    if !route_supports_session_lock(&model.route) || model.lock_timeout_minutes == 0 {
        return Vec::new();
    }
    lock_session(model)
}

fn lock_session(model: &mut Model) -> Vec<Effect> {
    model.lock_resume_route = Some(model.route.clone());
    model.session = None;
    model.network = NetworkState::Idle;
    model.my_games.clear();
    model.open_games.clear();
    model.notices.clear();
    model.matrix_rain.reset();
    model.route = Route::MatrixLocked;
    vec![Effect::DisconnectTransport]
}

fn cycle_lock_timeout(current: u16) -> u16 {
    let index = LOCK_TIMEOUT_OPTIONS
        .iter()
        .position(|value| *value == current)
        .unwrap_or(0);
    LOCK_TIMEOUT_OPTIONS[(index + 1) % LOCK_TIMEOUT_OPTIONS.len()]
}

fn lock_timeout_label(value: u16) -> String {
    if value == 0 {
        "Off".to_string()
    } else {
        format!("{value} min")
    }
}

fn current_password(model: &Model) -> String {
    match &model.route {
        Route::FirstRun(first_run) => first_run.password_input.clone(),
        Route::Locked(locked) => locked.password_input.clone(),
        _ => model
            .session
            .as_ref()
            .map(|session| session.password.clone())
            .unwrap_or_default(),
    }
}

fn is_quit_shortcut(key: crate::input::KeyEvent) -> bool {
    matches!(key.code, KeyCode::Char('q') | KeyCode::Char('Q'))
        && key.modifiers.contains(KeyModifiers::ALT)
}

fn is_lock_shortcut(key: crate::input::KeyEvent) -> bool {
    matches!(key.code, KeyCode::Char('l') | KeyCode::Char('L'))
        && key.modifiers.contains(KeyModifiers::ALT)
}

#[cfg(test)]
mod tests {
    use super::{
        dashboard_mouse_event, handle_idle_lock, handle_key, handle_mouse, handle_unlocked,
        open_snapshot_route, update,
    };
    use crate::Point;
    use crate::app::{
        App, Effect, HostedGameModel, LobbyModel, LobbyTab, LockedModel, Model, Msg, MyGameRow,
        NetworkState, Route, active_session_from_stored, help_close_tag_bounds,
    };
    use crate::dashboard;
    use crate::dashboard::app::state::{ActiveOverlay, ActivePopup, FleetOverlayRowKey};
    use crate::dashboard::overlays::fleet_list;
    use crate::input::{KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
    use crate::storage::StoredSession;
    use nc_client::cache::ClientCache;
    use nc_client::hosted::store::{CachedHostedDraft, HostedDraftStatus};
    use nc_client::keychain::{Keychain, active_identity_npub, now_iso8601, push_new_identity};
    use nc_data::{
        DiplomaticRelation, FleetTurnAction, PlanetTurnAction, PlanetTurnBlock, TurnSubmission,
    };
    use nc_nostr::state_sync::{
        GameState, HostedDiplomacyState, HostedFleetShips, HostedOwnedFleet, HostedOwnedPlanet,
        HostedPlayerRosterEntry, HostedPlayerState, HostedQueuedMail, HostedReportBlock,
        HostedStardockSlot, HostedStarmapState, HostedStatePayload, HostedWorldState,
    };

    #[test]
    fn alt_l_locks_from_hosted_game_route() {
        let mut model = hosted_game_model();

        let effects = handle_key(
            &mut model,
            KeyEvent::new(KeyCode::Char('l'), KeyModifiers::ALT),
        );

        assert!(matches!(model.route, Route::MatrixLocked));
        assert!(model.session.is_none());
        assert_eq!(model.network, NetworkState::Idle);
        assert_eq!(effects.len(), 1);
        assert!(matches!(effects[0], Effect::DisconnectTransport));
    }

    #[test]
    fn idle_lock_locks_from_hosted_game_route() {
        let mut model = hosted_game_model();
        model.lock_timeout_minutes = 10;

        let effects = handle_idle_lock(&mut model);

        assert!(matches!(model.route, Route::MatrixLocked));
        assert!(model.session.is_none());
        assert_eq!(model.network, NetworkState::Idle);
        assert_eq!(effects.len(), 1);
        assert!(matches!(effects[0], Effect::DisconnectTransport));
    }

    #[test]
    fn unlock_restores_previous_hosted_screen_after_lock() {
        let mut model = hosted_game_model();
        let previous_route = model.route.clone();
        if let Route::HostedGame(hosted) = &mut model.route {
            hosted.dashboard.overlay = ActiveOverlay::PlanetList;
        }

        let effects = handle_key(
            &mut model,
            KeyEvent::new(KeyCode::Char('l'), KeyModifiers::ALT),
        );
        assert!(matches!(model.route, Route::MatrixLocked));
        assert_eq!(effects.len(), 1);

        model.route = Route::Locked(LockedModel {
            password_input: "hunter2".to_string(),
            status: None,
            resume_session: true,
        });
        let effects = handle_unlocked(&mut model, Ok(dummy_session("captain")));

        assert_eq!(effects.len(), 1);
        assert!(matches!(effects[0], Effect::ConnectTransport { .. }));
        assert_eq!(model.network, NetworkState::Connecting);
        assert!(matches!(model.route, Route::HostedGame(_)));
        assert!(model.lock_resume_route.is_none());
        if let Route::HostedGame(hosted) = &model.route {
            assert_eq!(hosted.dashboard.overlay, ActiveOverlay::PlanetList);
        }
        assert!(!matches!(previous_route, Route::Lobby(_)));
    }

    #[test]
    fn unlock_without_saved_route_returns_to_lobby() {
        let (app, _) = App::new(None);
        let mut model = app.model;
        model.route = Route::Locked(LockedModel {
            password_input: "hunter2".to_string(),
            status: None,
            resume_session: false,
        });

        let effects = handle_unlocked(&mut model, Ok(dummy_session("captain")));

        assert_eq!(effects.len(), 1);
        assert!(matches!(effects[0], Effect::ConnectTransport { .. }));
        match &model.route {
            Route::Lobby(lobby) => {
                assert_eq!(lobby.active_tab, LobbyTab::MyGames);
                assert_eq!(lobby.status.as_deref(), Some("Keychain unlocked."));
            }
            other => panic!("expected lobby after unlock, got {other:?}"),
        }
    }

    #[test]
    fn dashboard_mouse_event_maps_move_and_drag_variants() {
        let moved = dashboard_mouse_event(MouseEvent {
            kind: MouseEventKind::Moved,
            position: Point::from_usize(12, 7),
            modifiers: KeyModifiers::SHIFT,
        })
        .expect("moved event");
        assert!(matches!(
            moved.kind,
            crate::dashboard::input::MouseEventKind::Moved
        ));
        assert_eq!(moved.column, 12);
        assert_eq!(moved.row, 7);
        assert!(
            moved
                .modifiers
                .contains(crate::dashboard::input::KeyModifiers::SHIFT)
        );

        let drag = dashboard_mouse_event(MouseEvent {
            kind: MouseEventKind::Drag(MouseButton::Left),
            position: Point::from_usize(5, 9),
            modifiers: KeyModifiers::CONTROL,
        })
        .expect("drag event");
        assert!(matches!(
            drag.kind,
            crate::dashboard::input::MouseEventKind::Drag(
                crate::dashboard::input::MouseButton::Left
            )
        ));
        assert_eq!(drag.column, 5);
        assert_eq!(drag.row, 9);
        assert!(
            drag.modifiers
                .contains(crate::dashboard::input::KeyModifiers::CONTROL)
        );
    }

    #[test]
    fn hosted_game_escape_opens_quit_to_lobby_confirm() {
        let mut model = hosted_game_model();

        let effects = handle_key(&mut model, KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));

        assert!(effects.is_empty());
        assert!(matches!(model.route, Route::HostedGame(_)));
        assert!(!model.should_quit);
        if let Route::HostedGame(hosted) = &model.route {
            assert_eq!(hosted.dashboard.popup, ActivePopup::QuitConfirm);
        }
    }

    #[test]
    fn hosted_game_build_command_emits_draft_autosave_effect() {
        let mut model = hosted_game_model();
        let Route::HostedGame(hosted) = &mut model.route else {
            panic!("expected hosted route");
        };
        hosted.dashboard.overlay = ActiveOverlay::PlanetList;
        hosted.dashboard.game_data.planets[0].stored_goods =
            nc_data::Real48::from_f64(f64::from(80));
        hosted.dashboard.open_planet_build_specify();

        let effects = handle_key(
            &mut model,
            KeyEvent::new(KeyCode::Char('='), KeyModifiers::NONE),
        );

        assert_eq!(effects.len(), 1);
        let Effect::SaveHostedTurnDraft {
            game_id,
            base_hash,
            draft: Some(draft),
            ..
        } = &effects[0]
        else {
            panic!("expected hosted draft save effect, got {:?}", effects[0]);
        };
        assert_eq!(game_id, "friday-night");
        assert_eq!(base_hash, "abc123");
        let Route::HostedGame(hosted) = &model.route else {
            panic!("expected hosted route");
        };
        let orders = nc_engine::planet_build_orders(&hosted.dashboard.game_data.planets[0]);
        assert_eq!(orders.len(), 1);
        assert_eq!(orders[0].points_remaining, 5);
        assert_eq!(draft.planets.len(), 1);
        assert!(matches!(
            draft.planets[0].actions.as_slice(),
            [PlanetTurnAction::Build {
                points_remaining_raw: 5,
                kind_raw: 1
            }]
        ));
    }

    #[test]
    fn hosted_game_tax_command_emits_draft_autosave_effect() {
        let mut model = hosted_game_model();

        assert!(
            handle_key(
                &mut model,
                KeyEvent::new(KeyCode::Char('x'), KeyModifiers::NONE)
            )
            .is_empty()
        );
        assert!(
            handle_key(
                &mut model,
                KeyEvent::new(KeyCode::Char('4'), KeyModifiers::NONE)
            )
            .is_empty()
        );
        assert!(
            handle_key(
                &mut model,
                KeyEvent::new(KeyCode::Char('2'), KeyModifiers::NONE)
            )
            .is_empty()
        );
        let effects = handle_key(
            &mut model,
            KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE),
        );

        let Effect::SaveHostedTurnDraft {
            draft: Some(draft), ..
        } = &effects[0]
        else {
            panic!("expected hosted draft save effect, got {:?}", effects);
        };
        let Route::HostedGame(hosted) = &model.route else {
            panic!("expected hosted route");
        };
        assert_eq!(hosted.dashboard.game_data.players[0].tax_rate, 42);
        assert_eq!(draft.tax_rate, Some(42));
    }

    #[test]
    fn hosted_game_diplomacy_command_emits_draft_autosave_effect() {
        let mut model = hosted_game_model();
        let Route::HostedGame(hosted) = &mut model.route else {
            panic!("expected hosted route");
        };
        hosted.dashboard.overlay = ActiveOverlay::Diplomacy;
        while crate::dashboard::overlays::diplomacy::selected_empire_slot(&hosted.dashboard)
            == Some(hosted.dashboard.player_record_index_1_based as u8)
        {
            hosted.dashboard.diplomacy_overlay.selected += 1;
        }
        let target = crate::dashboard::overlays::diplomacy::selected_empire_slot(&hosted.dashboard)
            .expect("target");

        let effects = handle_key(
            &mut model,
            KeyEvent::new(KeyCode::Char('e'), KeyModifiers::NONE),
        );

        let Effect::SaveHostedTurnDraft {
            draft: Some(draft), ..
        } = &effects[0]
        else {
            panic!("expected hosted draft save effect, got {:?}", effects);
        };
        let Route::HostedGame(hosted) = &model.route else {
            panic!("expected hosted route");
        };
        assert_eq!(
            hosted.dashboard.game_data.stored_diplomatic_relation(
                hosted.dashboard.player_record_index_1_based as u8,
                target
            ),
            Some(DiplomaticRelation::Enemy)
        );
        assert_eq!(draft.diplomacy.len(), 1);
        assert_eq!(draft.diplomacy[0].to_empire_raw, target);
        assert_eq!(draft.diplomacy[0].relation, DiplomaticRelation::Enemy);
    }

    #[test]
    fn hosted_game_planet_commission_emits_draft_autosave_effect_after_preview_update() {
        let mut model = hosted_game_model();
        let Route::HostedGame(hosted) = &mut model.route else {
            panic!("expected hosted route");
        };
        hosted.dashboard.overlay = ActiveOverlay::PlanetList;

        assert!(
            handle_key(
                &mut model,
                KeyEvent::new(KeyCode::Char('c'), KeyModifiers::NONE)
            )
            .is_empty()
        );
        let effects = handle_key(
            &mut model,
            KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE),
        );

        let Effect::SaveHostedTurnDraft {
            draft: Some(draft), ..
        } = &effects[0]
        else {
            panic!("expected hosted draft save effect, got {:?}", effects);
        };
        let Route::HostedGame(hosted) = &model.route else {
            panic!("expected hosted route");
        };
        assert_eq!(
            (hosted.dashboard.game_data.planets[0].stardock[0].min(u32::from(u16::MAX)) as u16),
            0
        );
        assert_eq!(draft.planets.len(), 1);
        assert!(matches!(
            draft.planets[0].actions.as_slice(),
            [PlanetTurnAction::Commission { slot_0_based: 0 }]
        ));
    }

    #[test]
    fn hosted_game_fleet_change_emits_draft_autosave_effect_after_preview_update() {
        let mut model = hosted_game_model();
        let Route::HostedGame(hosted) = &mut model.route else {
            panic!("expected hosted route");
        };
        hosted.dashboard.overlay = ActiveOverlay::FleetList;
        hosted.dashboard.fleet_overlay.selected = fleet_list::table_rows(&hosted.dashboard)
            .iter()
            .position(|row| matches!(row.key, FleetOverlayRowKey::Fleet(_)))
            .expect("fleet row");

        for key in [KeyCode::Char('c'), KeyCode::Char('r'), KeyCode::Char('4')] {
            assert!(handle_key(&mut model, KeyEvent::new(key, KeyModifiers::NONE)).is_empty());
        }
        let effects = handle_key(
            &mut model,
            KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE),
        );

        let Effect::SaveHostedTurnDraft {
            draft: Some(draft), ..
        } = &effects[0]
        else {
            panic!("expected hosted draft save effect, got {:?}", effects);
        };
        let Route::HostedGame(hosted) = &model.route else {
            panic!("expected hosted route");
        };
        assert_eq!(hosted.dashboard.game_data.fleets[0].rules_of_engagement, 4);
        assert_eq!(draft.fleets.len(), 1);
        assert!(matches!(
            draft.fleets[0].actions.as_slice(),
            [FleetTurnAction::RulesOfEngagement { value: 4 }]
        ));
    }

    #[test]
    fn hosted_game_quick_message_emits_draft_autosave_effect() {
        let mut model = hosted_game_model();
        let Route::HostedGame(hosted) = &mut model.route else {
            panic!("expected hosted route");
        };
        hosted.dashboard.overlay = ActiveOverlay::Inbox;

        for key in [
            KeyCode::Char('c'),
            KeyCode::Char('2'),
            KeyCode::Enter,
            KeyCode::Char('H'),
            KeyCode::Char('i'),
            KeyCode::Enter,
            KeyCode::Char('M'),
            KeyCode::Char('o'),
            KeyCode::Char('v'),
            KeyCode::Char('e'),
        ] {
            assert!(handle_key(&mut model, KeyEvent::new(key, KeyModifiers::NONE)).is_empty());
        }
        assert!(
            handle_key(
                &mut model,
                KeyEvent::new(KeyCode::Char('s'), KeyModifiers::ALT)
            )
            .is_empty()
        );

        let effects = handle_key(
            &mut model,
            KeyEvent::new(KeyCode::Char('y'), KeyModifiers::NONE),
        );

        let Effect::SaveHostedTurnDraft {
            draft: Some(draft), ..
        } = &effects[0]
        else {
            panic!("expected hosted draft save effect, got {:?}", effects);
        };
        assert_eq!(draft.messages.len(), 1);
        assert_eq!(draft.messages[0].recipient_empire_raw, 2);
        assert_eq!(draft.messages[0].subject, "Hi");
        assert_eq!(draft.messages[0].body, "Move");
        assert!(
            draft
                .to_kdl_string()
                .contains("message to=2 subject=\"Hi\" body=\"Move\"")
        );
    }

    #[test]
    fn hosted_game_alt_q_confirm_yes_returns_to_lobby() {
        let mut model = hosted_game_model();

        let effects = handle_key(
            &mut model,
            KeyEvent::new(KeyCode::Char('q'), KeyModifiers::ALT),
        );
        assert!(effects.is_empty());

        let effects = handle_key(
            &mut model,
            KeyEvent::new(KeyCode::Char('y'), KeyModifiers::NONE),
        );

        assert!(effects.is_empty());
        assert!(!model.should_quit);
        match &model.route {
            Route::Lobby(lobby) => {
                assert_eq!(lobby.active_tab, LobbyTab::MyGames);
                assert_eq!(lobby.status, None);
            }
            other => panic!("expected lobby after confirm, got {other:?}"),
        }
    }

    #[test]
    fn opening_hosted_game_replays_matching_local_draft() {
        let (app, _) = App::new(None);
        let mut model = app.model;
        model.geometry = crate::ScreenGeometry::new(132, 44);
        let snapshot = sample_snapshot();

        let effects = open_snapshot_route(
            &mut model,
            sample_game_row(),
            snapshot,
            Some(cached_build_draft("abc123")),
            None,
        );

        assert!(effects.is_empty());
        let Route::HostedGame(hosted) = &model.route else {
            panic!("expected hosted route");
        };
        let orders = nc_engine::planet_build_orders(&hosted.dashboard.game_data.planets[0]);
        assert_eq!(orders.len(), 1);
        assert_eq!(orders[0].kind, nc_data::ProductionItemKind::Destroyer);
        assert_eq!(orders[0].points_remaining, 5);
        assert!(hosted.dashboard.hosted_turn_text().is_some());
    }

    #[test]
    fn opening_hosted_game_does_not_replay_stale_local_draft() {
        let (app, _) = App::new(None);
        let mut model = app.model;
        model.geometry = crate::ScreenGeometry::new(132, 44);
        let snapshot = sample_snapshot();

        let effects = open_snapshot_route(
            &mut model,
            sample_game_row(),
            snapshot,
            Some(cached_build_draft("stale-hash")),
            None,
        );

        assert!(effects.is_empty());
        let Route::HostedGame(hosted) = &model.route else {
            panic!("expected hosted route");
        };
        let orders = nc_engine::planet_build_orders(&hosted.dashboard.game_data.planets[0]);
        assert!(orders.is_empty());
        assert!(hosted.dashboard.hosted_turn_text().is_none());
    }

    #[test]
    fn hosted_game_quit_confirm_default_no_stays_in_hosted_game() {
        let mut model = hosted_game_model();

        let effects = handle_key(
            &mut model,
            KeyEvent::new(KeyCode::Char('q'), KeyModifiers::ALT),
        );
        assert!(effects.is_empty());

        let effects = handle_key(
            &mut model,
            KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE),
        );

        assert!(effects.is_empty());
        assert!(matches!(model.route, Route::HostedGame(_)));
        assert!(!model.should_quit);
        if let Route::HostedGame(hosted) = &model.route {
            assert_eq!(hosted.dashboard.popup, ActivePopup::None);
        }
    }

    #[test]
    fn hosted_game_control_c_still_quits_client() {
        let mut model = hosted_game_model();

        let effects = handle_key(
            &mut model,
            KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL),
        );

        assert_eq!(effects.len(), 1);
        assert!(matches!(effects[0], Effect::Quit));
        assert!(model.should_quit);
        assert!(matches!(model.route, Route::HostedGame(_)));
    }

    #[test]
    fn clicking_lobby_help_close_button_closes_help_popup() {
        let (app, _) = App::new(None);
        let mut model = app.model;
        model.route = Route::Lobby(LobbyModel {
            active_tab: LobbyTab::MyGames,
            help_open: true,
            quit_confirm_open: false,
            selected_my_game: 0,
            my_games_scroll: 0,
            selected_open_game: 0,
            open_games_scroll: 0,
            settings_scroll: 0,
            editing_relay: false,
            relay_draft: model.relay_url.clone(),
            status: None,
        });
        let (row, col, _) = help_close_tag_bounds(model.geometry).expect("help close bounds");

        let outcome = handle_mouse(
            &mut model,
            MouseEvent {
                kind: MouseEventKind::Down(MouseButton::Left),
                position: Point::from_usize(col, row),
                modifiers: KeyModifiers::NONE,
            },
        );

        assert!(outcome.effects.is_empty());
        assert!(outcome.needs_redraw);
        match &model.route {
            Route::Lobby(lobby) => assert!(!lobby.help_open),
            other => panic!("expected lobby after click, got {other:?}"),
        }
    }

    #[test]
    fn lobby_escape_opens_quit_confirm_when_no_popup_is_active() {
        let mut model = lobby_model(false, false);

        let effects = handle_key(&mut model, KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));

        assert!(effects.is_empty());
        match &model.route {
            Route::Lobby(lobby) => assert!(lobby.quit_confirm_open),
            other => panic!("expected lobby after Esc, got {other:?}"),
        }
    }

    #[test]
    fn lobby_escape_closes_help_without_opening_quit_confirm() {
        let mut model = lobby_model(true, false);

        let effects = handle_key(&mut model, KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));

        assert!(effects.is_empty());
        match &model.route {
            Route::Lobby(lobby) => {
                assert!(!lobby.help_open);
                assert!(!lobby.quit_confirm_open);
            }
            other => panic!("expected lobby after Esc, got {other:?}"),
        }
    }

    #[test]
    fn hosted_mouse_noop_skips_redraw() {
        let mut model = hosted_game_model();
        let Route::HostedGame(hosted) = &mut model.route else {
            panic!("expected hosted route");
        };
        let target = [hosted.dashboard.crosshair_x, hosted.dashboard.crosshair_y];
        let (column, row) = hosted
            .dashboard
            .screen_point_for_sector_for_repro(target)
            .expect("screen point for selected sector");

        let outcome = update(
            &mut model,
            Msg::Mouse(MouseEvent {
                kind: MouseEventKind::Moved,
                position: Point::from_usize(column as usize, row as usize),
                modifiers: KeyModifiers::NONE,
            }),
        );

        assert!(outcome.effects.is_empty());
        assert!(!outcome.needs_redraw);
    }

    #[test]
    fn hosted_mouse_crosshair_move_requests_redraw() {
        let mut model = hosted_game_model();
        let Route::HostedGame(hosted) = &mut model.route else {
            panic!("expected hosted route");
        };
        let target = [hosted.dashboard.crosshair_x, hosted.dashboard.crosshair_y];
        let (column, row) = hosted
            .dashboard
            .screen_point_for_sector_for_repro(target)
            .expect("screen point for selected sector");

        let outcome = update(
            &mut model,
            Msg::Mouse(MouseEvent {
                kind: MouseEventKind::Down(MouseButton::Right),
                position: Point::from_usize(column as usize, row as usize),
                modifiers: KeyModifiers::NONE,
            }),
        );

        assert!(outcome.effects.is_empty());
        assert!(outcome.needs_redraw);
    }

    fn hosted_game_model() -> Model {
        let (app, _) = App::new(None);
        let mut model = app.model;
        model.geometry = crate::ScreenGeometry::new(132, 44);
        model.session = Some(active_session_from_stored(
            dummy_session("captain"),
            "hunter2".to_string(),
        ));
        model.network = NetworkState::Synced;
        let snapshot = sample_snapshot();
        let row = sample_game_row();
        let dashboard = dashboard::build_hosted_dash_app(
            &snapshot,
            dashboard::ScreenGeometry::new(model.geometry.width(), model.geometry.height()),
        )
        .expect("hosted dashboard");
        let mut dashboard = dashboard;
        dashboard.overlay = ActiveOverlay::None;
        dashboard.popup = ActivePopup::None;
        model.route = Route::HostedGame(HostedGameModel {
            row,
            dashboard,
            status: None,
        });
        model
    }

    fn lobby_model(help_open: bool, quit_confirm_open: bool) -> Model {
        let (app, _) = App::new(None);
        let mut model = app.model;
        model.route = Route::Lobby(LobbyModel {
            active_tab: LobbyTab::MyGames,
            help_open,
            quit_confirm_open,
            selected_my_game: 0,
            my_games_scroll: 0,
            selected_open_game: 0,
            open_games_scroll: 0,
            settings_scroll: 0,
            editing_relay: false,
            relay_draft: model.relay_url.clone(),
            status: None,
        });
        model
    }

    fn cached_build_draft(base_hash: &str) -> CachedHostedDraft {
        CachedHostedDraft {
            game_id: "friday-night".to_string(),
            player_pubkey: "player".to_string(),
            turn: 4,
            base_hash: base_hash.to_string(),
            status: HostedDraftStatus::Local,
            submit_id: None,
            draft: TurnSubmission {
                player_record_index_1_based: 1,
                year: 3004,
                tax_rate: None,
                diplomacy: Vec::new(),
                planets: vec![PlanetTurnBlock {
                    planet_record_index_1_based: 1,
                    actions: vec![PlanetTurnAction::Build {
                        points_remaining_raw: 5,
                        kind_raw: 1,
                    }],
                }],
                fleets: Vec::new(),
                messages: Vec::new(),
            },
        }
    }

    fn sample_game_row() -> MyGameRow {
        MyGameRow {
            game_id: "friday-night".to_string(),
            status: "active".to_string(),
            game_tier: "sandbox".to_string(),
            game: "Friday Night".to_string(),
            host: "localhost".to_string(),
            host_contact_npub: None,
            relay_url: "ws://127.0.0.1:8080".to_string(),
            daemon_pubkey: "daemon".to_string(),
            seat: Some(1),
            turn_summary: "Turn 4".to_string(),
            last_turn: Some(4),
            last_hash: Some("abc123".to_string()),
        }
    }

    fn sample_snapshot() -> GameState {
        GameState {
            game_id: "friday-night".to_string(),
            turn: 4,
            year: 3004,
            player_seat: 1,
            player_name: "Terran Union".to_string(),
            state_hash: "abc123".to_string(),
            state: HostedStatePayload {
                player: HostedPlayerState {
                    seat: 1,
                    empire_name: "Terran Union".to_string(),
                    handle: Some("captain".to_string()),
                    mode: "active".to_string(),
                    tax_rate: 33,
                    planet_count: 1,
                    starbase_count: 1,
                    homeworld_planet_index: 1,
                    last_run_year: 3004,
                    diplomacy: vec![HostedDiplomacyState {
                        empire_id: 2,
                        relation: "enemy".to_string(),
                    }],
                },
                roster: vec![
                    HostedPlayerRosterEntry {
                        empire_id: 1,
                        empire_name: "Terran Union".to_string(),
                        is_self: true,
                    },
                    HostedPlayerRosterEntry {
                        empire_id: 2,
                        empire_name: "Rigel Empire".to_string(),
                        is_self: false,
                    },
                ],
                starmap: HostedStarmapState {
                    map_width: 18,
                    map_height: 18,
                    viewer_empire_id: 1,
                    year: 3004,
                    worlds: vec![HostedWorldState {
                        planet_index: 1,
                        coords: [8, 8],
                        intel_tier: "owned".to_string(),
                        known_name: Some("Sol".to_string()),
                        known_owner_empire_id: Some(1),
                        known_owner_empire_name: Some("Terran Union".to_string()),
                        known_potential_production: Some(100),
                        known_armies: Some(20),
                        known_ground_batteries: Some(5),
                        known_starbase_count: Some(1),
                        known_current_production: Some(40),
                        known_stored_points: Some(12),
                        known_docked_summary: None,
                        known_orbit_summary: None,
                    }],
                },
                owned_planets: vec![HostedOwnedPlanet {
                    planet_index: 1,
                    name: "Sol".to_string(),
                    coords: [8, 8],
                    potential_production: 100,
                    current_production: 40,
                    stored_points: 12,
                    armies: 20,
                    ground_batteries: 5,
                    starbase_count: 1,
                    stardock: vec![HostedStardockSlot {
                        slot: 1,
                        kind: "destroyer".to_string(),
                        count: 2,
                    }],
                }],
                owned_fleets: vec![HostedOwnedFleet {
                    fleet_id: 1,
                    local_slot: 1,
                    coords: [8, 8],
                    target_coords: [10, 10],
                    order: "move".to_string(),
                    order_summary: "Move fleet to Sector (10,10)".to_string(),
                    rules_of_engagement: 4,
                    current_speed: 5,
                    max_speed: 6,
                    ships: HostedFleetShips {
                        scout: 1,
                        battleship: 0,
                        cruiser: 2,
                        destroyer: 0,
                        transport: 0,
                        army: 0,
                        etac: 0,
                        total_starships: 3,
                        summary: "1 SC 2 CA".to_string(),
                    },
                }],
            },
            queued_mail: vec![HostedQueuedMail {
                sender_empire_id: 2,
                recipient_empire_id: 1,
                year: 3004,
                subject: "Scout".to_string(),
                body: "Hostiles near Rigel.".to_string(),
            }],
            report_blocks: vec![HostedReportBlock {
                viewer_empire_id: 1,
                block_index: 1,
                decoded_text: "Battle report".to_string(),
            }],
        }
    }

    fn dummy_session(handle: &str) -> StoredSession {
        let mut keychain = Keychain::empty();
        push_new_identity(&mut keychain, now_iso8601(), Some(handle.to_string()))
            .expect("new identity");
        let active_npub = active_identity_npub(&keychain).expect("npub");
        let active = keychain.active_identity().expect("active identity").clone();
        StoredSession {
            keychain,
            cache: ClientCache::empty(),
            active_npub,
            active_nsec: active.nsec.clone(),
            active_handle: active.handle.clone(),
        }
    }
}

#[allow(dead_code)]
fn _assert_screen_geometry_send(_: ScreenGeometry) {}
