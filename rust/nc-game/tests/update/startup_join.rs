use crate::support::*;
use nc_data::SeatBindSource;

#[test]
fn first_time_menu_branch_opens_help_intro_and_empire_list() {
    let fixture_dir = temp_first_time_game_copy();
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

    advance_to_first_time_menu(&mut app);
    assert_eq!(app.current_screen(), ScreenId::FirstTimeMenu);

    assert_eq!(
        apply_action(&mut app, Action::OpenPopupHelp),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FirstTimeMenu);
    assert!(app.popup_help.is_some());
    assert_eq!(
        apply_action(&mut app, Action::DismissPopupHelp),
        AppOutcome::Continue
    );
    assert!(app.popup_help.is_none());

    assert_eq!(
        apply_action(
            &mut app,
            Action::Startup(StartupAction::OpenFirstTimeEmpires)
        ),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FirstTimeEmpires);

    assert_eq!(
        apply_action(&mut app, Action::Startup(StartupAction::OpenFirstTimeIntro)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FirstTimeIntro);
}

#[test]
fn first_time_startup_skips_joined_player_login_summary() {
    let fixture_dir = temp_first_time_game_copy();
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

    apply_action(&mut app, Action::Startup(StartupAction::SkipIntro));
    assert_eq!(app.current_screen(), ScreenId::FirstTimeMenu);
    assert_eq!(app.classic_login_state(), ClassicLoginState::FirstTimeMenu);
}

#[test]
fn joined_player_with_unnamed_homeworld_is_routed_to_homeworld_naming() {
    let fixture_dir = temp_joined_needs_homeworld_copy();
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
        app.classic_login_state(),
        ClassicLoginState::MatchedPreloadedFirstLogin
    );

    assert_eq!(
        app.current_screen(),
        ScreenId::Startup(StartupPhase::Splash)
    );

    for _ in 0..16 {
        if app.current_screen() == ScreenId::FirstTimePreloadedRenamePrompt {
            break;
        }
        app.advance_startup();
    }
    assert_eq!(
        app.current_screen(),
        ScreenId::FirstTimePreloadedRenamePrompt
    );
}

/// Declining the first-login rename keeps the stored name on the join
/// summary (a sysop saw `Commander, you are ""`).
#[test]
fn declining_the_first_login_rename_shows_the_stored_name() {
    let fixture_dir = temp_joined_needs_homeworld_copy();
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
        if app.current_screen() == ScreenId::FirstTimePreloadedRenamePrompt {
            break;
        }
        app.advance_startup();
    }
    assert_eq!(
        app.current_screen(),
        ScreenId::FirstTimePreloadedRenamePrompt
    );

    apply_action(
        &mut app,
        Action::Startup(StartupAction::RejectFirstTimePrompt),
    );
    assert_eq!(app.current_screen(), ScreenId::FirstTimeJoinSummary);
    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("the join summary should render");
    assert!(
        line_containing(&terminal, "Commander, you are").contains("\"Empire 1\""),
        "{:?}",
        line_containing(&terminal, "Commander, you are")
    );
}

#[test]
fn player_two_joined_with_unnamed_homeworld_is_not_retreated_as_first_time() {
    let fixture_dir = temp_joined_needs_homeworld_copy_for_player(2);
    let mut app = App::load(AppConfig {
        game_dir: fixture_dir.clone(),
        player_record_index_1_based: 2,
        export_root: None,
        queue_dir: None,
        session_timeout_secs: None,
        game_config: Default::default(),
    })
    .expect("app should load");

    assert_eq!(
        app.classic_login_state(),
        ClassicLoginState::MatchedPreloadedFirstLogin
    );

    for _ in 0..16 {
        if app.current_screen() == ScreenId::FirstTimePreloadedRenamePrompt {
            break;
        }
        app.advance_startup();
    }

    assert_eq!(
        app.current_screen(),
        ScreenId::FirstTimePreloadedRenamePrompt
    );
}

#[test]
fn reserved_first_time_player_skips_menu_and_sees_reserved_prompt() {
    let fixture_dir = temp_first_time_game_copy();
    let mut app = App::load(AppConfig {
        game_dir: fixture_dir.clone(),
        player_record_index_1_based: 1,
        export_root: None,
        queue_dir: None,
        session_timeout_secs: None,
        game_config: reserved_game_config(1, "SYSOP"),
    })
    .expect("app should load");
    app.door_mode = true;

    for _ in 0..16 {
        if app.current_screen() == ScreenId::FirstTimeReservedPrompt {
            break;
        }
        app.advance_startup();
    }

    assert_eq!(app.current_screen(), ScreenId::FirstTimeReservedPrompt);
    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("reserved prompt should render");
    assert!(
        terminal
            .line(2)
            .contains("This player seat is reserved for you.")
    );
    assert!(terminal.line(6).contains("You may name your empire now"));
    assert!(
        terminal
            .line(8)
            .contains("Continue with reserved setup? [Y]/N ->")
    );
}

#[test]
fn reserved_local_first_time_player_without_door_mode_still_sees_first_time_menu() {
    let fixture_dir = temp_first_time_game_copy();
    let mut app = App::load(AppConfig {
        game_dir: fixture_dir.clone(),
        player_record_index_1_based: 1,
        export_root: None,
        queue_dir: None,
        session_timeout_secs: None,
        game_config: reserved_game_config(1, "SYSOP"),
    })
    .expect("app should load");

    for _ in 0..16 {
        if app.current_screen() == ScreenId::FirstTimeMenu {
            break;
        }
        app.advance_startup();
    }

    assert_eq!(app.current_screen(), ScreenId::FirstTimeMenu);
}

#[test]
fn preloaded_first_login_routes_through_login_summary_before_rename_prompt() {
    let fixture_dir = temp_joined_needs_homeworld_copy();
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
    let mut saw_summary_year_text = false;
    for _ in 0..16 {
        if app.current_screen() == ScreenId::Startup(StartupPhase::LoginSummary) {
            saw_login_summary = true;
            let mut terminal = CaptureTerminal::new();
            app.render(&mut terminal)
                .expect("login summary should render");
            saw_summary_year_text = terminal
                .lines
                .iter()
                .any(|line| line.contains("The year is:"));
        }
        if app.current_screen() == ScreenId::FirstTimePreloadedRenamePrompt {
            break;
        }
        app.advance_startup();
    }

    assert!(saw_login_summary);
    assert!(saw_summary_year_text);
    assert_eq!(
        app.current_screen(),
        ScreenId::FirstTimePreloadedRenamePrompt
    );

    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("rename prompt should render");
    assert!(
        terminal
            .line(2)
            .contains("This empire is already joined, and this is your first login.")
    );
    assert!(terminal.line(6).contains("Rename your empire? Y/[N] ->"));
}

#[test]
fn preloaded_first_login_becomes_returning_player_after_homeworld_naming() {
    let fixture_dir = temp_joined_needs_homeworld_copy();
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
        if app.current_screen() == ScreenId::FirstTimePreloadedRenamePrompt {
            break;
        }
        app.advance_startup();
    }
    assert_eq!(
        app.current_screen(),
        ScreenId::FirstTimePreloadedRenamePrompt
    );
    assert_eq!(
        apply_action(
            &mut app,
            Action::Startup(StartupAction::RejectFirstTimePrompt)
        ),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FirstTimeJoinSummary);
    assert_eq!(
        apply_action(
            &mut app,
            Action::Startup(StartupAction::AcceptFirstTimePrompt)
        ),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FirstTimeJoinNoPending);
    assert_eq!(
        apply_action(
            &mut app,
            Action::Startup(StartupAction::AcceptFirstTimePrompt)
        ),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FirstTimeHomeworldName);

    for ch in "Codex Prime".chars() {
        assert_eq!(
            apply_action(
                &mut app,
                Action::Startup(StartupAction::AppendFirstTimeInputChar(ch))
            ),
            AppOutcome::Continue
        );
    }
    assert_eq!(
        apply_action(
            &mut app,
            Action::Startup(StartupAction::SubmitFirstTimeInput)
        ),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FirstTimeHomeworldConfirm);
    assert_eq!(
        apply_action(
            &mut app,
            Action::Startup(StartupAction::AcceptFirstTimePrompt)
        ),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::MainMenu);

    let reloaded = App::load(AppConfig {
        game_dir: fixture_dir.clone(),
        player_record_index_1_based: 1,
        export_root: None,
        queue_dir: None,
        session_timeout_secs: None,
        game_config: Default::default(),
    })
    .expect("reloaded app should load");

    assert_eq!(
        reloaded.classic_login_state(),
        ClassicLoginState::ReturningPlayer
    );
    assert_eq!(
        reloaded.current_screen(),
        ScreenId::Startup(StartupPhase::Splash)
    );
}

#[test]
fn first_time_join_summary_and_no_pending_accept_any_key_dismissal() {
    let fixture_dir = temp_joined_needs_homeworld_copy();
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
        if app.current_screen() == ScreenId::FirstTimePreloadedRenamePrompt {
            break;
        }
        app.advance_startup();
    }
    assert_eq!(
        app.current_screen(),
        ScreenId::FirstTimePreloadedRenamePrompt
    );

    assert_eq!(
        apply_action(
            &mut app,
            Action::Startup(StartupAction::RejectFirstTimePrompt)
        ),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FirstTimeJoinSummary);
    assert_eq!(
        app.handle_key(key(KeyCode::Char(' '))),
        Action::Startup(StartupAction::AcceptFirstTimePrompt)
    );
    assert_eq!(
        apply_action(
            &mut app,
            Action::Startup(StartupAction::AcceptFirstTimePrompt)
        ),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FirstTimeJoinNoPending);
    assert_eq!(
        app.handle_key(key(KeyCode::Char('x'))),
        Action::Startup(StartupAction::AcceptFirstTimePrompt)
    );
    assert_eq!(
        apply_action(
            &mut app,
            Action::Startup(StartupAction::AcceptFirstTimePrompt)
        ),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FirstTimeHomeworldName);
}

#[test]
fn preloaded_first_login_can_rename_empire_before_homeworld_naming() {
    let fixture_dir = temp_joined_needs_homeworld_copy();
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
        if app.current_screen() == ScreenId::FirstTimePreloadedRenamePrompt {
            break;
        }
        app.advance_startup();
    }
    assert_eq!(
        app.current_screen(),
        ScreenId::FirstTimePreloadedRenamePrompt
    );

    assert_eq!(
        apply_action(
            &mut app,
            Action::Startup(StartupAction::AcceptFirstTimePrompt)
        ),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FirstTimeJoinEmpireName);

    let mut rename_terminal = CaptureTerminal::new();
    app.render(&mut rename_terminal)
        .expect("rename input should render");
    assert!(
        rename_terminal
            .line(2)
            .contains("This empire is already joined, and this is your first login.")
    );

    for _ in 0..24 {
        assert_eq!(
            apply_action(
                &mut app,
                Action::Startup(StartupAction::BackspaceFirstTimeInput)
            ),
            AppOutcome::Continue
        );
    }
    for ch in "Codex Dominion".chars() {
        assert_eq!(
            apply_action(
                &mut app,
                Action::Startup(StartupAction::AppendFirstTimeInputChar(ch))
            ),
            AppOutcome::Continue
        );
    }
    assert_eq!(
        apply_action(
            &mut app,
            Action::Startup(StartupAction::SubmitFirstTimeInput)
        ),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FirstTimeJoinEmpireConfirm);
    assert_eq!(
        apply_action(
            &mut app,
            Action::Startup(StartupAction::AcceptFirstTimePrompt)
        ),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FirstTimeJoinSummary);
    assert_eq!(
        apply_action(
            &mut app,
            Action::Startup(StartupAction::AcceptFirstTimePrompt)
        ),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FirstTimeJoinNoPending);
    assert_eq!(
        apply_action(
            &mut app,
            Action::Startup(StartupAction::AcceptFirstTimePrompt)
        ),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FirstTimeHomeworldName);

    let runtime = latest_runtime_state(&fixture_dir);
    assert_eq!(
        runtime.game_data.players[0].empire_name.trim().to_string(),
        "Codex Dominion"
    );
}

#[test]
fn first_time_join_empire_name_prompt_shows_esc_without_redundant_instruction() {
    let fixture_dir = temp_first_time_game_copy();
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
            Action::Startup(StartupAction::OpenFirstTimeJoinName)
        ),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FirstTimeJoinEmpireName);

    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("first-time empire naming prompt should render");

    assert!(line_containing(&terminal, "EMPIRE NAME <-").contains("<ESC> ->"));
    assert!(!terminal.lines.iter().any(|line| line.contains("Press Esc")));
}

#[test]
fn preloaded_first_login_empire_name_prompt_shows_esc_without_redundant_instruction() {
    let fixture_dir = temp_joined_needs_homeworld_copy();
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
        if app.current_screen() == ScreenId::FirstTimePreloadedRenamePrompt {
            break;
        }
        app.advance_startup();
    }
    assert_eq!(
        app.current_screen(),
        ScreenId::FirstTimePreloadedRenamePrompt
    );
    assert_eq!(
        apply_action(
            &mut app,
            Action::Startup(StartupAction::AcceptFirstTimePrompt)
        ),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FirstTimeJoinEmpireName);

    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("preloaded empire naming prompt should render");

    assert!(line_containing(&terminal, "EMPIRE NAME <-").contains("<ESC> ->"));
    assert!(!terminal.lines.iter().any(|line| line.contains("Press Esc")));
}

#[test]
fn first_time_homeworld_name_prompt_shows_esc_without_redundant_instruction() {
    let fixture_dir = temp_joined_needs_homeworld_copy();
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
        if app.current_screen() == ScreenId::FirstTimePreloadedRenamePrompt {
            break;
        }
        app.advance_startup();
    }
    assert_eq!(
        app.current_screen(),
        ScreenId::FirstTimePreloadedRenamePrompt
    );
    assert_eq!(
        apply_action(
            &mut app,
            Action::Startup(StartupAction::RejectFirstTimePrompt)
        ),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(
            &mut app,
            Action::Startup(StartupAction::AcceptFirstTimePrompt)
        ),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(
            &mut app,
            Action::Startup(StartupAction::AcceptFirstTimePrompt)
        ),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FirstTimeHomeworldName);

    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("homeworld naming prompt should render");

    assert!(line_containing(&terminal, "HOMEWORLD <-").contains("<ESC> ->"));
    assert!(!terminal.lines.iter().any(|line| line.contains("Press Esc")));
}

#[test]
fn preloaded_empire_rename_failure_returns_to_name_entry_with_status() {
    let fixture_dir = temp_joined_needs_homeworld_copy();
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
        if app.current_screen() == ScreenId::FirstTimePreloadedRenamePrompt {
            break;
        }
        app.advance_startup();
    }
    assert_eq!(
        app.current_screen(),
        ScreenId::FirstTimePreloadedRenamePrompt
    );

    assert_eq!(
        apply_action(
            &mut app,
            Action::Startup(StartupAction::AcceptFirstTimePrompt)
        ),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FirstTimeJoinEmpireName);

    let attempted_name = app.startup_state.first_time_input.clone();
    assert_eq!(
        apply_action(
            &mut app,
            Action::Startup(StartupAction::SubmitFirstTimeInput)
        ),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FirstTimeJoinEmpireConfirm);

    make_runtime_db_read_only(&fixture_dir);

    assert_eq!(
        apply_action(
            &mut app,
            Action::Startup(StartupAction::AcceptFirstTimePrompt)
        ),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FirstTimeJoinEmpireName);
    assert!(app.startup_state.first_time_rename_preloaded_empire);
    assert_eq!(app.startup_state.first_time_input, attempted_name);
    assert_eq!(
        app.startup_state.first_time_status.as_deref(),
        Some("Unable to save your empire name right now. Please try again.")
    );
}

#[test]
fn returning_player_with_owned_unnamed_colony_is_routed_to_colony_naming() {
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
        if app.current_screen() == ScreenId::ColonyWorldName {
            break;
        }
        app.advance_startup();
    }
    assert_eq!(app.current_screen(), ScreenId::ColonyWorldName);
}

#[test]
fn colony_world_naming_updates_planet_and_enters_main_menu() {
    let fixture_dir = temp_game_copy();
    let mut state = latest_runtime_state(&fixture_dir);
    let homeworld_index = state.game_data.players[0].homeworld as usize;
    let colony_index = state
        .game_data
        .planets
        .iter_mut()
        .enumerate()
        .find(|(idx, planet)| *idx + 1 != homeworld_index && planet.owner.0 != 1)
        .map(|(idx, planet)| {
            planet.owner.0 = 1;
            nc_data::set_planet_name(planet, "Not Named Yet");
            idx + 1
        })
        .expect("need a non-homeworld planet for colony naming test");
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
        if app.current_screen() == ScreenId::ColonyWorldName {
            break;
        }
        app.advance_startup();
    }
    assert_eq!(app.current_screen(), ScreenId::ColonyWorldName);

    for ch in "New Horizon".chars() {
        assert_eq!(
            apply_action(
                &mut app,
                Action::Startup(StartupAction::AppendFirstTimeInputChar(ch))
            ),
            AppOutcome::Continue
        );
    }
    assert_eq!(
        apply_action(
            &mut app,
            Action::Startup(StartupAction::SubmitFirstTimeInput)
        ),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::ColonyWorldConfirm);
    assert_eq!(
        apply_action(
            &mut app,
            Action::Startup(StartupAction::AcceptFirstTimePrompt)
        ),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::MainMenu);

    let runtime = latest_runtime_state(&fixture_dir);
    assert_eq!(
        runtime.game_data.planets[colony_index - 1].name.clone(),
        "New Horizon"
    );
}

#[test]
fn colony_world_name_failure_returns_to_name_entry_with_status() {
    let fixture_dir = temp_game_copy();
    let mut state = latest_runtime_state(&fixture_dir);
    let homeworld_index = state.game_data.players[0].homeworld as usize;
    state
        .game_data
        .planets
        .iter_mut()
        .enumerate()
        .find(|(idx, planet)| *idx + 1 != homeworld_index && planet.owner.0 != 1)
        .map(|(_, planet)| {
            planet.owner.0 = 1;
            nc_data::set_planet_name(planet, "Not Named Yet");
        })
        .expect("need a non-homeworld planet for colony naming test");
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
        if app.current_screen() == ScreenId::ColonyWorldName {
            break;
        }
        app.advance_startup();
    }
    assert_eq!(app.current_screen(), ScreenId::ColonyWorldName);

    for ch in "New Horizon".chars() {
        assert_eq!(
            apply_action(
                &mut app,
                Action::Startup(StartupAction::AppendFirstTimeInputChar(ch))
            ),
            AppOutcome::Continue
        );
    }
    assert_eq!(
        apply_action(
            &mut app,
            Action::Startup(StartupAction::SubmitFirstTimeInput)
        ),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::ColonyWorldConfirm);

    app.startup_state.colony_world_planet_record_index_1_based = Some(999);
    make_runtime_db_read_only(&fixture_dir);

    assert_eq!(
        apply_action(
            &mut app,
            Action::Startup(StartupAction::AcceptFirstTimePrompt)
        ),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::ColonyWorldName);
    assert_eq!(app.startup_state.first_time_input, "New Horizon");
    assert_eq!(
        app.startup_state.first_time_status.as_deref(),
        Some("Unable to save the world name right now. Please try again.")
    );
}

#[test]
fn colony_world_naming_cannot_be_escaped_to_main_menu() {
    let fixture_dir = temp_game_copy();
    let mut state = latest_runtime_state(&fixture_dir);
    let homeworld_index = state.game_data.players[0].homeworld as usize;
    state
        .game_data
        .planets
        .iter_mut()
        .enumerate()
        .find(|(idx, planet)| *idx + 1 != homeworld_index && planet.owner.0 != 1)
        .expect("need a non-homeworld planet for colony naming test")
        .1
        .owner
        .0 = 1;
    nc_data::set_planet_name(
        &mut state
            .game_data
            .planets
            .iter_mut()
            .enumerate()
            .find(|(idx, planet)| *idx + 1 != homeworld_index && planet.owner.0 == 1)
            .expect("need owned unnamed colony")
            .1,
        "Not Named Yet",
    );
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
        if app.current_screen() == ScreenId::ColonyWorldName {
            break;
        }
        app.advance_startup();
    }
    assert_eq!(app.current_screen(), ScreenId::ColonyWorldName);

    assert_eq!(
        apply_action(
            &mut app,
            Action::Startup(StartupAction::RejectFirstTimePrompt)
        ),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::ColonyWorldName);

    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("colony world naming screen should render");
    assert!(
        terminal
            .lines
            .iter()
            .any(|line| line.contains("must name this newly colonized world before continuing"))
    );
}

#[test]
fn colony_world_name_prompt_shows_esc_without_redundant_instruction() {
    let fixture_dir = temp_game_copy();
    let mut state = latest_runtime_state(&fixture_dir);
    let homeworld_index = state.game_data.players[0].homeworld as usize;
    state
        .game_data
        .planets
        .iter_mut()
        .enumerate()
        .find(|(idx, planet)| *idx + 1 != homeworld_index && planet.owner.0 != 1)
        .expect("need a non-homeworld planet for colony naming test")
        .1
        .owner
        .0 = 1;
    nc_data::set_planet_name(
        &mut state
            .game_data
            .planets
            .iter_mut()
            .enumerate()
            .find(|(idx, planet)| *idx + 1 != homeworld_index && planet.owner.0 == 1)
            .expect("need owned unnamed colony")
            .1,
        "Not Named Yet",
    );
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
        if app.current_screen() == ScreenId::ColonyWorldName {
            break;
        }
        app.advance_startup();
    }
    assert_eq!(app.current_screen(), ScreenId::ColonyWorldName);

    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("colony world naming prompt should render");

    assert!(line_containing(&terminal, "WORLD NAME <-").contains("<ESC> ->"));
    assert!(!terminal.lines.iter().any(|line| line.contains("Press Esc")));
}

#[test]
fn first_time_join_routes_from_homeworld_naming_to_colony_naming_when_needed() {
    let fixture_dir = temp_joined_needs_homeworld_copy();
    let mut state = latest_runtime_state(&fixture_dir);
    let homeworld_index = state.game_data.players[0].homeworld as usize;
    let colony_index = state
        .game_data
        .planets
        .iter_mut()
        .enumerate()
        .find(|(idx, planet)| *idx + 1 != homeworld_index && planet.owner.0 != 1)
        .map(|(idx, planet)| {
            planet.owner.0 = 1;
            nc_data::set_planet_name(planet, "Not Named Yet");
            idx + 1
        })
        .expect("need a non-homeworld planet for colony naming test");
    save_runtime_state(&fixture_dir, &state);

    let mut app = App::load(AppConfig {
        game_dir: fixture_dir.clone(),
        player_record_index_1_based: 1,
        export_root: None,
        queue_dir: None,
        session_timeout_secs: None,
        game_config: Default::default(),
    })
    .expect("app should reload");
    for _ in 0..16 {
        if app.current_screen() == ScreenId::FirstTimePreloadedRenamePrompt {
            break;
        }
        app.advance_startup();
    }
    assert_eq!(
        app.current_screen(),
        ScreenId::FirstTimePreloadedRenamePrompt
    );
    assert_eq!(
        apply_action(
            &mut app,
            Action::Startup(StartupAction::RejectFirstTimePrompt)
        ),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FirstTimeJoinSummary);
    assert_eq!(
        apply_action(
            &mut app,
            Action::Startup(StartupAction::AcceptFirstTimePrompt)
        ),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FirstTimeJoinNoPending);
    assert_eq!(
        apply_action(
            &mut app,
            Action::Startup(StartupAction::AcceptFirstTimePrompt)
        ),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FirstTimeHomeworldName);

    for ch in "Codex Prime".chars() {
        assert_eq!(
            apply_action(
                &mut app,
                Action::Startup(StartupAction::AppendFirstTimeInputChar(ch))
            ),
            AppOutcome::Continue
        );
    }
    assert_eq!(
        apply_action(
            &mut app,
            Action::Startup(StartupAction::SubmitFirstTimeInput)
        ),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FirstTimeHomeworldConfirm);
    assert_eq!(
        apply_action(
            &mut app,
            Action::Startup(StartupAction::AcceptFirstTimePrompt)
        ),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::ColonyWorldName);

    for ch in "New Horizon".chars() {
        assert_eq!(
            apply_action(
                &mut app,
                Action::Startup(StartupAction::AppendFirstTimeInputChar(ch))
            ),
            AppOutcome::Continue
        );
    }
    assert_eq!(
        apply_action(
            &mut app,
            Action::Startup(StartupAction::SubmitFirstTimeInput)
        ),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::ColonyWorldConfirm);
    assert_eq!(
        apply_action(
            &mut app,
            Action::Startup(StartupAction::AcceptFirstTimePrompt)
        ),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::MainMenu);

    let runtime = latest_runtime_state(&fixture_dir);
    assert_eq!(
        runtime.game_data.planets[colony_index - 1].name.clone(),
        "New Horizon"
    );
}

#[test]
fn returning_player_with_multiple_unnamed_colonies_is_prompted_for_each_in_turn() {
    let fixture_dir = temp_game_copy();
    let mut state = latest_runtime_state(&fixture_dir);
    let homeworld_index = state.game_data.players[0].homeworld as usize;
    let mut renamed_targets = Vec::new();
    for (idx, planet) in state.game_data.planets.iter_mut().enumerate() {
        if idx + 1 == homeworld_index || planet.owner.0 == 1 {
            continue;
        }
        planet.owner.0 = 1;
        nc_data::set_planet_name(planet, "Not Named Yet");
        renamed_targets.push(idx + 1);
        if renamed_targets.len() == 2 {
            break;
        }
    }
    assert_eq!(
        renamed_targets.len(),
        2,
        "need two colony worlds for sequencing test"
    );
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
        if app.current_screen() == ScreenId::ColonyWorldName {
            break;
        }
        app.advance_startup();
    }
    assert_eq!(app.current_screen(), ScreenId::ColonyWorldName);

    for ch in "New Horizon".chars() {
        assert_eq!(
            apply_action(
                &mut app,
                Action::Startup(StartupAction::AppendFirstTimeInputChar(ch))
            ),
            AppOutcome::Continue
        );
    }
    assert_eq!(
        apply_action(
            &mut app,
            Action::Startup(StartupAction::SubmitFirstTimeInput)
        ),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::ColonyWorldConfirm);
    assert_eq!(
        apply_action(
            &mut app,
            Action::Startup(StartupAction::AcceptFirstTimePrompt)
        ),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::ColonyWorldName);

    for ch in "Second Dawn".chars() {
        assert_eq!(
            apply_action(
                &mut app,
                Action::Startup(StartupAction::AppendFirstTimeInputChar(ch))
            ),
            AppOutcome::Continue
        );
    }
    assert_eq!(
        apply_action(
            &mut app,
            Action::Startup(StartupAction::SubmitFirstTimeInput)
        ),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::ColonyWorldConfirm);
    assert_eq!(
        apply_action(
            &mut app,
            Action::Startup(StartupAction::AcceptFirstTimePrompt)
        ),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::MainMenu);

    let runtime = latest_runtime_state(&fixture_dir);
    assert_eq!(
        runtime.game_data.planets[renamed_targets[0] - 1]
            .name
            .clone(),
        "New Horizon"
    );
    assert_eq!(
        runtime.game_data.planets[renamed_targets[1] - 1]
            .name
            .clone(),
        "Second Dawn"
    );
}

#[test]
fn returning_player_routes_through_login_summary_before_main_menu() {
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
        app.classic_login_state(),
        ClassicLoginState::ReturningPlayer
    );

    let mut saw_login_summary = false;
    let mut saw_summary_year_text = false;
    for _ in 0..16 {
        if app.current_screen() == ScreenId::Startup(StartupPhase::LoginSummary) {
            saw_login_summary = true;
            let mut terminal = CaptureTerminal::new();
            app.render(&mut terminal)
                .expect("login summary should render");
            saw_summary_year_text = terminal
                .lines
                .iter()
                .any(|line| line.contains("The year is:"));
        }
        if app.current_screen() == ScreenId::MainMenu {
            break;
        }
        app.advance_startup();
    }

    assert!(saw_login_summary);
    assert!(saw_summary_year_text);
    assert_eq!(app.current_screen(), ScreenId::MainMenu);
}

#[test]
fn escaping_empire_name_does_not_partially_join_player() {
    let fixture_dir = temp_first_time_game_copy();
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
            Action::Startup(StartupAction::OpenFirstTimeJoinName)
        ),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FirstTimeJoinEmpireName);

    for ch in "Codex Dominion".chars() {
        assert_eq!(
            apply_action(
                &mut app,
                Action::Startup(StartupAction::AppendFirstTimeInputChar(ch))
            ),
            AppOutcome::Continue
        );
    }

    assert_eq!(
        apply_action(&mut app, Action::Startup(StartupAction::OpenFirstTimeMenu)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FirstTimeMenu);

    let reloaded = App::load(AppConfig {
        game_dir: fixture_dir.clone(),
        player_record_index_1_based: 1,
        export_root: None,
        queue_dir: None,
        session_timeout_secs: None,
        game_config: Default::default(),
    })
    .expect("app should reload");
    assert_eq!(
        reloaded.current_screen(),
        ScreenId::Startup(StartupPhase::Splash)
    );

    let game_data = latest_runtime_state(&fixture_dir).game_data;
    assert_eq!(game_data.players[0].mode.code(), 0);
}

#[test]
fn fixed_player_first_time_launch_skips_first_time_menu() {
    let fixture_dir = temp_first_time_game_copy();
    let mut app = App::load(AppConfig {
        game_dir: fixture_dir.clone(),
        player_record_index_1_based: 1,
        export_root: None,
        queue_dir: None,
        session_timeout_secs: None,
        game_config: Default::default(),
    })
    .expect("app should load");
    app.startup_state.fixed_player_launch = true;

    assert_eq!(
        apply_action(&mut app, Action::Startup(StartupAction::SkipIntro)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FirstTimeJoinEmpireName);
}

#[test]
fn fixed_player_join_name_screen_does_not_render_first_time_menu_rows() {
    let fixture_dir = temp_first_time_game_copy();
    let mut app = App::load(AppConfig {
        game_dir: fixture_dir.clone(),
        player_record_index_1_based: 1,
        export_root: None,
        queue_dir: None,
        session_timeout_secs: None,
        game_config: Default::default(),
    })
    .expect("app should load");
    app.startup_state.fixed_player_launch = true;

    assert_eq!(
        apply_action(&mut app, Action::Startup(StartupAction::SkipIntro)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FirstTimeJoinEmpireName);

    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("fixed player join name should render");
    assert!(terminal.line(0).contains("FIRST TIME SETUP:"));
    assert!(!(0..terminal.lines.len()).any(|row| terminal.line(row).contains("uit back to BBS")));
    assert!(!(0..terminal.lines.len()).any(|row| terminal.line(row).contains("oin this game")));
}

#[test]
fn fixed_player_join_confirm_screen_does_not_render_first_time_menu_rows() {
    let fixture_dir = temp_first_time_game_copy();
    let mut app = App::load(AppConfig {
        game_dir: fixture_dir.clone(),
        player_record_index_1_based: 1,
        export_root: None,
        queue_dir: None,
        session_timeout_secs: None,
        game_config: Default::default(),
    })
    .expect("app should load");
    app.startup_state.fixed_player_launch = true;

    assert_eq!(
        apply_action(&mut app, Action::Startup(StartupAction::SkipIntro)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FirstTimeJoinEmpireName);

    for ch in "Codex Dominion".chars() {
        assert_eq!(
            apply_action(
                &mut app,
                Action::Startup(StartupAction::AppendFirstTimeInputChar(ch))
            ),
            AppOutcome::Continue
        );
    }
    assert_eq!(
        apply_action(
            &mut app,
            Action::Startup(StartupAction::SubmitFirstTimeInput)
        ),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FirstTimeJoinEmpireConfirm);

    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("fixed player join confirm should render");
    assert!(terminal.line(0).contains("FIRST TIME SETUP:"));
    assert!(!(0..terminal.lines.len()).any(|row| terminal.line(row).contains("uit back to BBS")));
    assert!(!(0..terminal.lines.len()).any(|row| terminal.line(row).contains("oin this game")));
}

#[test]
fn first_time_join_flow_updates_player_and_homeworld_then_enters_main_menu() {
    let fixture_dir = temp_first_time_game_copy();
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
            Action::Startup(StartupAction::OpenFirstTimeJoinName)
        ),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FirstTimeJoinEmpireName);

    for ch in "Codex Dominion".chars() {
        assert_eq!(
            apply_action(
                &mut app,
                Action::Startup(StartupAction::AppendFirstTimeInputChar(ch))
            ),
            AppOutcome::Continue
        );
    }
    assert_eq!(
        apply_action(
            &mut app,
            Action::Startup(StartupAction::SubmitFirstTimeInput)
        ),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FirstTimeJoinEmpireConfirm);

    assert_eq!(
        apply_action(
            &mut app,
            Action::Startup(StartupAction::AcceptFirstTimePrompt)
        ),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FirstTimeJoinSummary);

    assert_eq!(
        apply_action(
            &mut app,
            Action::Startup(StartupAction::AcceptFirstTimePrompt)
        ),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FirstTimeJoinNoPending);

    assert_eq!(
        apply_action(
            &mut app,
            Action::Startup(StartupAction::AcceptFirstTimePrompt)
        ),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FirstTimeHomeworldName);

    for ch in "Codex Prime".chars() {
        assert_eq!(
            apply_action(
                &mut app,
                Action::Startup(StartupAction::AppendFirstTimeInputChar(ch))
            ),
            AppOutcome::Continue
        );
    }
    assert_eq!(
        apply_action(
            &mut app,
            Action::Startup(StartupAction::SubmitFirstTimeInput)
        ),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FirstTimeHomeworldConfirm);

    assert_eq!(
        apply_action(
            &mut app,
            Action::Startup(StartupAction::AcceptFirstTimePrompt)
        ),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::MainMenu);

    let game_data = latest_runtime_state(&fixture_dir).game_data;
    let player = &game_data.players[0];
    assert_eq!(player.mode.code(), 1);
    assert_eq!(player.empire_name.trim().to_string(), "Codex Dominion");
    assert_eq!(u8::from(player.autopilot), 0);
    let homeworld_index = player.homeworld as usize;
    let homeworld = &game_data.planets[homeworld_index - 1];
    assert_eq!(homeworld.name.clone(), "Codex Prime");
    assert_eq!(
        nc_data::stored_production_points(&homeworld),
        nc_data::tax_income(homeworld.production.to_f64(), player.tax_rate) as u32
    );
}

#[test]
fn unbound_bbs_first_time_join_claims_lowest_open_unreserved_seat() {
    let fixture_dir = temp_first_time_game_copy();
    let mut app = App::load(AppConfig {
        game_dir: fixture_dir.clone(),
        player_record_index_1_based: 1,
        export_root: None,
        queue_dir: None,
        session_timeout_secs: None,
        game_config: reserved_game_config(1, "SYSOP"),
    })
    .expect("app should load");
    app.door_mode = true;
    app.startup_state.caller_alias = Some("RIVAL".to_string());
    app.enter_unbound_bbs_first_time_mode();

    assert_eq!(app.current_screen(), ScreenId::FirstTimeMenu);
    assert_eq!(
        apply_action(
            &mut app,
            Action::Startup(StartupAction::OpenFirstTimeJoinName)
        ),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FirstTimeJoinEmpireName);

    for ch in "Codex Dominion".chars() {
        assert_eq!(
            apply_action(
                &mut app,
                Action::Startup(StartupAction::AppendFirstTimeInputChar(ch))
            ),
            AppOutcome::Continue
        );
    }
    assert_eq!(
        apply_action(
            &mut app,
            Action::Startup(StartupAction::SubmitFirstTimeInput)
        ),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FirstTimeJoinEmpireConfirm);

    assert_eq!(
        apply_action(
            &mut app,
            Action::Startup(StartupAction::AcceptFirstTimePrompt)
        ),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FirstTimeJoinSummary);
    assert_eq!(app.player.record_index_1_based, 2);

    let game_data = latest_runtime_state(&fixture_dir).game_data;
    assert_eq!(game_data.players[0].mode.code(), 0);
    assert_eq!(game_data.players[1].mode.code(), 1);
    assert_eq!(game_data.players[1].handle.trim().to_string(), "RIVAL");
    assert_eq!(
        game_data.players[1].empire_name.trim().to_string(),
        "Codex Dominion"
    );
}

fn join_unbound_bbs_caller(
    fixture_dir: &Path,
    caller_alias: &str,
    caller_alias_source: SeatBindSource,
) -> App {
    let mut app = App::load(AppConfig {
        game_dir: fixture_dir.to_path_buf(),
        player_record_index_1_based: 1,
        export_root: None,
        queue_dir: None,
        session_timeout_secs: None,
        game_config: Default::default(),
    })
    .expect("app should load");
    app.door_mode = true;
    app.startup_state.caller_alias = Some(caller_alias.to_string());
    app.startup_state.caller_alias_source = caller_alias_source;
    app.enter_unbound_bbs_first_time_mode();
    apply_action(
        &mut app,
        Action::Startup(StartupAction::OpenFirstTimeJoinName),
    );
    for ch in "Codex Dominion".chars() {
        apply_action(
            &mut app,
            Action::Startup(StartupAction::AppendFirstTimeInputChar(ch)),
        );
    }
    apply_action(
        &mut app,
        Action::Startup(StartupAction::SubmitFirstTimeInput),
    );
    apply_action(
        &mut app,
        Action::Startup(StartupAction::AcceptFirstTimePrompt),
    );
    assert_eq!(app.current_screen(), ScreenId::FirstTimeJoinSummary);
    app
}

#[test]
fn unbound_bbs_join_under_the_handle_records_the_seat_as_bound_by_handle() {
    let fixture_dir = temp_first_time_game_copy();
    let app = join_unbound_bbs_caller(&fixture_dir, "Starlord", SeatBindSource::Handle);

    let store = CampaignStore::open_default_in_dir(&fixture_dir).expect("open campaign store");
    assert_eq!(
        store.handle_bound_seats().expect("load handle-bound seats"),
        vec![app.player.record_index_1_based]
    );
}

#[test]
fn unbound_bbs_join_under_the_real_name_leaves_the_seat_bound_by_real_name() {
    let fixture_dir = temp_first_time_game_copy();
    join_unbound_bbs_caller(&fixture_dir, "Tachyon", SeatBindSource::RealName);

    let store = CampaignStore::open_default_in_dir(&fixture_dir).expect("open campaign store");
    assert_eq!(
        store.handle_bound_seats().expect("load handle-bound seats"),
        Vec::<usize>::new()
    );
}

#[test]
fn first_time_join_failure_returns_to_name_entry_with_status() {
    let fixture_dir = temp_first_time_game_copy();
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
            Action::Startup(StartupAction::OpenFirstTimeJoinName)
        ),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FirstTimeJoinEmpireName);

    for ch in "Codex Dominion".chars() {
        assert_eq!(
            apply_action(
                &mut app,
                Action::Startup(StartupAction::AppendFirstTimeInputChar(ch))
            ),
            AppOutcome::Continue
        );
    }
    assert_eq!(
        apply_action(
            &mut app,
            Action::Startup(StartupAction::SubmitFirstTimeInput)
        ),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FirstTimeJoinEmpireConfirm);

    make_runtime_db_read_only(&fixture_dir);

    assert_eq!(
        apply_action(
            &mut app,
            Action::Startup(StartupAction::AcceptFirstTimePrompt)
        ),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FirstTimeJoinEmpireName);
    assert_eq!(app.startup_state.first_time_input, "Codex Dominion");
    assert_eq!(
        app.startup_state.first_time_status.as_deref(),
        Some("Unable to join this empire right now. Please try again.")
    );

    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("failed join screen should render");
    assert!(
        line_containing(
            &terminal,
            "Unable to join this empire right now. Please try again."
        )
        .contains("Unable to join this empire right now. Please try again.")
    );
}

#[test]
fn homeworld_name_failure_returns_to_name_entry_with_status() {
    let fixture_dir = temp_joined_needs_homeworld_copy();
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
        if app.current_screen() == ScreenId::FirstTimePreloadedRenamePrompt {
            break;
        }
        app.advance_startup();
    }
    assert_eq!(
        app.current_screen(),
        ScreenId::FirstTimePreloadedRenamePrompt
    );

    assert_eq!(
        apply_action(
            &mut app,
            Action::Startup(StartupAction::RejectFirstTimePrompt)
        ),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FirstTimeJoinSummary);

    assert_eq!(
        apply_action(
            &mut app,
            Action::Startup(StartupAction::AcceptFirstTimePrompt)
        ),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FirstTimeJoinNoPending);

    assert_eq!(
        apply_action(
            &mut app,
            Action::Startup(StartupAction::AcceptFirstTimePrompt)
        ),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FirstTimeHomeworldName);

    for ch in "Codex Prime".chars() {
        assert_eq!(
            apply_action(
                &mut app,
                Action::Startup(StartupAction::AppendFirstTimeInputChar(ch))
            ),
            AppOutcome::Continue
        );
    }
    assert_eq!(
        apply_action(
            &mut app,
            Action::Startup(StartupAction::SubmitFirstTimeInput)
        ),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FirstTimeHomeworldConfirm);

    make_runtime_db_read_only(&fixture_dir);

    assert_eq!(
        apply_action(
            &mut app,
            Action::Startup(StartupAction::AcceptFirstTimePrompt)
        ),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FirstTimeHomeworldName);
    assert_eq!(app.startup_state.first_time_input, "Codex Prime");
    assert_eq!(
        app.startup_state.first_time_status.as_deref(),
        Some("Unable to save the homeworld name right now. Please try again.")
    );
}

#[test]
fn reserved_first_time_join_flow_updates_player_and_homeworld_then_enters_main_menu() {
    let fixture_dir = temp_first_time_game_copy();
    let mut app = App::load(AppConfig {
        game_dir: fixture_dir.clone(),
        player_record_index_1_based: 1,
        export_root: None,
        queue_dir: None,
        session_timeout_secs: None,
        game_config: reserved_game_config(1, "SYSOP"),
    })
    .expect("app should load");
    app.door_mode = true;

    for _ in 0..16 {
        if app.current_screen() == ScreenId::FirstTimeReservedPrompt {
            break;
        }
        app.advance_startup();
    }
    assert_eq!(app.current_screen(), ScreenId::FirstTimeReservedPrompt);

    assert_eq!(
        apply_action(
            &mut app,
            Action::Startup(StartupAction::AcceptFirstTimePrompt)
        ),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FirstTimeJoinEmpireName);

    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("reserved join name should render");
    assert!(
        terminal
            .line(2)
            .contains("This player seat is reserved for you.")
    );

    for ch in "Codex Dominion".chars() {
        assert_eq!(
            apply_action(
                &mut app,
                Action::Startup(StartupAction::AppendFirstTimeInputChar(ch))
            ),
            AppOutcome::Continue
        );
    }
    assert_eq!(
        apply_action(
            &mut app,
            Action::Startup(StartupAction::SubmitFirstTimeInput)
        ),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FirstTimeJoinEmpireConfirm);

    assert_eq!(
        apply_action(
            &mut app,
            Action::Startup(StartupAction::AcceptFirstTimePrompt)
        ),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FirstTimeJoinSummary);

    assert_eq!(
        apply_action(
            &mut app,
            Action::Startup(StartupAction::AcceptFirstTimePrompt)
        ),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FirstTimeJoinNoPending);

    assert_eq!(
        apply_action(
            &mut app,
            Action::Startup(StartupAction::AcceptFirstTimePrompt)
        ),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FirstTimeHomeworldName);

    for ch in "Codex Prime".chars() {
        assert_eq!(
            apply_action(
                &mut app,
                Action::Startup(StartupAction::AppendFirstTimeInputChar(ch))
            ),
            AppOutcome::Continue
        );
    }
    assert_eq!(
        apply_action(
            &mut app,
            Action::Startup(StartupAction::SubmitFirstTimeInput)
        ),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FirstTimeHomeworldConfirm);

    assert_eq!(
        apply_action(
            &mut app,
            Action::Startup(StartupAction::AcceptFirstTimePrompt)
        ),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::MainMenu);

    let game_data = latest_runtime_state(&fixture_dir).game_data;
    let player = &game_data.players[0];
    assert_eq!(player.mode.code(), 1);
    assert_eq!(player.empire_name.trim().to_string(), "Codex Dominion");
    let homeworld_index = player.homeworld as usize;
    let homeworld = &game_data.planets[homeworld_index - 1];
    assert_eq!(homeworld.name.clone(), "Codex Prime");
}

#[test]
fn first_time_join_from_reserved_dropfile_persists_caller_alias() {
    let fixture_dir = temp_first_time_game_copy();
    let mut app = App::load(AppConfig {
        game_dir: fixture_dir.clone(),
        player_record_index_1_based: 1,
        export_root: None,
        queue_dir: None,
        session_timeout_secs: None,
        game_config: Default::default(),
    })
    .expect("app should load");
    app.startup_state.caller_alias = Some("SYSOP".to_string());

    assert_eq!(
        apply_action(
            &mut app,
            Action::Startup(StartupAction::OpenFirstTimeJoinName)
        ),
        AppOutcome::Continue
    );
    for ch in "Codex Dominion".chars() {
        assert_eq!(
            apply_action(
                &mut app,
                Action::Startup(StartupAction::AppendFirstTimeInputChar(ch))
            ),
            AppOutcome::Continue
        );
    }
    assert_eq!(
        apply_action(
            &mut app,
            Action::Startup(StartupAction::SubmitFirstTimeInput)
        ),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(
            &mut app,
            Action::Startup(StartupAction::AcceptFirstTimePrompt)
        ),
        AppOutcome::Continue
    );

    let player = &latest_runtime_state(&fixture_dir).game_data.players[0];
    assert_eq!(player.handle.trim().to_string(), "SYSOP");
}

#[test]
fn unbound_bbs_first_time_menu_refuses_join_when_no_open_empires_remain() {
    let fixture_dir = temp_full_game_copy();
    let mut app = App::load(AppConfig {
        game_dir: fixture_dir.clone(),
        player_record_index_1_based: 1,
        export_root: None,
        queue_dir: None,
        session_timeout_secs: None,
        game_config: Default::default(),
    })
    .expect("app should load");
    app.door_mode = true;
    app.startup_state.caller_alias = Some("RIVAL".to_string());
    app.enter_unbound_bbs_first_time_mode();

    assert_eq!(
        apply_action(
            &mut app,
            Action::Startup(StartupAction::OpenFirstTimeJoinName)
        ),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FirstTimeMenu);
    assert_eq!(
        app.startup_state.first_time_status.as_deref(),
        Some("This game is already full. No open empires remain.")
    );
}

#[test]
fn first_time_join_from_menu_refuses_full_game_and_displays_notice() {
    let fixture_dir = temp_full_game_copy();
    let mut app = App::load(AppConfig {
        game_dir: fixture_dir.clone(),
        player_record_index_1_based: 1,
        export_root: None,
        queue_dir: None,
        session_timeout_secs: None,
        game_config: Default::default(),
    })
    .expect("app should load");

    app.open_first_time_menu();
    assert_eq!(
        apply_action(
            &mut app,
            Action::Startup(StartupAction::OpenFirstTimeJoinName)
        ),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FirstTimeMenu);
    assert_eq!(
        app.startup_state.first_time_status.as_deref(),
        Some("This game is already full. No open empires remain.")
    );

    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("first-time menu should render");
    assert!(terminal
        .lines
        .iter()
        .any(|line| line.contains("Notice: This game is already full. No open empires remain.")));
}

#[test]
fn defeated_player_with_pending_reports_uses_returning_review_flow() {
    let fixture_dir = temp_game_copy();
    let store = CampaignStore::open_default_in_dir(&fixture_dir).expect("open campaign store");
    let mut state = latest_runtime_state(&fixture_dir);
    state.game_data.players[0].mode = nc_data::PlayerMode::from_code(0);
    state.report_block_rows.push(nc_data::ReportBlockRow {
        viewer_empire_id: 1,
        block_index: 0,
        decoded_text:
            "From your Fleet Command Center:\nALERT: Empire defeated!\n<end of transmission>"
                .to_string(),
        raw_bytes: None,
        recipient_deleted: false,
    });
    let player_count = state.game_data.conquest.player_count;
    let planet_intel_by_viewer = (1..=player_count)
        .map(|viewer_empire_id| {
            store
                .latest_planet_intel_for_viewer(viewer_empire_id)
                .expect("load runtime intel")
                .into_iter()
                .map(|snapshot| (snapshot.planet_record_index_1_based, snapshot))
                .collect::<BTreeMap<_, _>>()
        })
        .collect::<Vec<_>>();
    let player_activity_states = store
        .latest_player_activity_states(player_count)
        .expect("load player activity");
    let mut player_lifecycle_states = store
        .latest_player_lifecycle_states(player_count)
        .expect("load player lifecycle");
    player_lifecycle_states[0].terminal_outcome = nc_data::TerminalOutcome::Defeated;
    player_lifecycle_states[0].terminal_review_consumed = false;
    store
        .save_year_with_intel_activity_and_lifecycle(
            state.game_data.conquest.game_year,
            &state.game_data,
            &state.report_block_rows,
            &state.queued_mail,
            &planet_intel_by_viewer,
            &player_activity_states,
            &player_lifecycle_states,
            state.winner_state,
        )
        .expect("save defeated runtime state");

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
        app.classic_login_state(),
        ClassicLoginState::ReturningPlayer
    );

    let mut saw_review = false;
    for _ in 0..32 {
        if app.current_screen() == ScreenId::Startup(StartupPhase::Results) {
            saw_review = true;
            break;
        }
        assert_ne!(app.current_screen(), ScreenId::FirstTimeMenu);
        app.advance_startup();
    }
    assert!(saw_review, "startup should reach results review");
}

#[test]
fn first_time_join_does_not_offer_spent_civil_disorder_seats() {
    let fixture_dir = temp_first_time_game_copy();
    let store = CampaignStore::open_default_in_dir(&fixture_dir).expect("open campaign store");
    let mut state = latest_runtime_state(&fixture_dir);
    for (idx, player) in state.game_data.players.iter_mut().enumerate() {
        player.mode = nc_data::PlayerMode::from_code(0);
        player.last_year_on = 3005 + idx as u16;
        player.planet_count = 0;
    }
    for planet in &mut state.game_data.planets {
        planet.owner.0 = 0;
        planet.years_under_owner = 0;
    }
    let player_count = state.game_data.conquest.player_count;
    let planet_intel_by_viewer = (1..=player_count)
        .map(|viewer_empire_id| {
            store
                .latest_planet_intel_for_viewer(viewer_empire_id)
                .expect("load runtime intel")
                .into_iter()
                .map(|snapshot| (snapshot.planet_record_index_1_based, snapshot))
                .collect::<BTreeMap<_, _>>()
        })
        .collect::<Vec<_>>();
    let player_activity_states = store
        .latest_player_activity_states(player_count)
        .expect("load player activity");
    store
        .save_year_with_intel_and_activity(
            state.game_data.conquest.game_year,
            &state.game_data,
            &state.report_block_rows,
            &state.queued_mail,
            &planet_intel_by_viewer,
            &player_activity_states,
        )
        .expect("save spent-seat runtime state");

    let mut app = App::load(AppConfig {
        game_dir: fixture_dir.clone(),
        player_record_index_1_based: 1,
        export_root: None,
        queue_dir: None,
        session_timeout_secs: None,
        game_config: Default::default(),
    })
    .expect("app should load");

    app.open_first_time_menu();
    assert_eq!(
        apply_action(
            &mut app,
            Action::Startup(StartupAction::OpenFirstTimeJoinName)
        ),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FirstTimeMenu);
    assert_eq!(
        app.startup_state.first_time_status.as_deref(),
        Some("This game is already full. No open empires remain.")
    );
}
