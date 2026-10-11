use crate::support::*;

#[test]
fn main_menu_matches_verified_v15_command_layout() {
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

    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal).expect("main menu should render");
    assert_eq!(terminal.line(1).trim_end(), " MAIN MENU:");
    assert_eq!(
        terminal.line(2).trim_end(),
        "  H>elp with commands   C>olor Theme               T>otal Planet Database"
    );
    assert_eq!(
        terminal.line(3).trim_end(),
        "  Q>uit back to BBS     G>ENERAL COMMAND MENU...   I>nfo about a Planet"
    );
    assert_eq!(
        terminal.line(4).trim_end(),
        "  X>pert mode ON/OFF    P>LANET COMMAND MENU...    B>rief Empire Report"
    );
    assert_eq!(
        terminal.line(5).trim_end(),
        "  V>iew Partial Map     F>LEET COMMAND MENU...     D>etailed Empire Report"
    );
    assert_eq!(terminal.line(6).trim_end(), "");
    assert_eq!(
        terminal.line(7).trim_end(),
        " MAIN COMMAND <- ? X V C G P F T I B D <Q> ->"
    );
    assert!(terminal.line(23).contains("-- "));
}
#[test]
fn general_menu_matches_verified_v15_command_layout() {
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
        apply_action(&mut app, Action::OpenGeneralMenu),
        AppOutcome::Continue
    );

    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("general menu should render");
    assert_eq!(
        terminal.line(1).trim_end(),
        " GENERAL COMMAND CENTER:  I>nfo about a Planet     C>ommunicate (send message)"
    );
    assert_eq!(
        terminal.line(2).trim_end(),
        "  H>elp with commands     A>utopilot [ON] [OFF]    R>eview Inbox"
    );
    assert_eq!(
        terminal.line(3).trim_end(),
        "  Q>uit to main menu      S>tatus, your            D>elete ALL reports"
    );
    assert_eq!(
        terminal.line(4).trim_end(),
        "  X>pert mode ON/OFF      P>rofile of your empire  O>ther empires (rankings)"
    );
    assert_eq!(
        terminal.line(5).trim_end(),
        "  V>iew Partial Starmap   M>ap of the galaxy       E>nemies, declare or list"
    );
    assert_eq!(
        line_containing(&terminal, "GENERAL COMMAND <-").trim_end(),
        " GENERAL COMMAND <- ? X V I A S P M C R D O E <Q> ->"
    );
}
#[test]
fn main_menu_notice_renders_below_fixed_command_row() {
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
    app.command_menu_notice = Some("No ships are waiting in stardock.".into());

    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal).expect("main menu should render");
    assert_eq!(
        terminal.lines[7].trim_end(),
        " MAIN COMMAND <- ? X V C G P F T I B D <Q> ->"
    );
    assert_eq!(terminal.lines[8].trim_end(), "");
    assert_eq!(terminal.lines[9].trim_end(), "");
    assert_eq!(terminal.lines[10].trim_end(), "");
    assert!(terminal.lines[11].contains("Notice: No ships are waiting in stardock."));
    assert!(!terminal.lines.iter().any(|line| line.contains("-- ")));
}

#[test]
fn main_menu_x_toggles_expert_mode_and_hides_menu_chrome() {
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
        app.handle_key(key(KeyCode::Char('x'))),
        Action::ToggleExpertMode
    );
    assert_eq!(
        apply_action(&mut app, Action::ToggleExpertMode),
        AppOutcome::Continue
    );
    assert!(app.expert_mode);

    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("expert main menu should render");
    assert_eq!(
        terminal.lines[0].trim_end(),
        " MAIN COMMAND <- ? X V C G P F T I B D <Q> ->"
    );
    assert_eq!(terminal.lines[1].trim_end(), "");
    assert_eq!(terminal.lines[23].trim_end(), "");

    assert_eq!(
        apply_action(&mut app, Action::ToggleExpertMode),
        AppOutcome::Continue
    );
    assert!(!app.expert_mode);
    app.render(&mut terminal)
        .expect("normal main menu should render");
    assert_eq!(terminal.lines[1].trim_end(), " MAIN MENU:");
}

#[test]
fn general_menu_x_toggles_expert_mode_and_hides_menu_chrome() {
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
        apply_action(&mut app, Action::OpenGeneralMenu),
        AppOutcome::Continue
    );

    assert_eq!(
        app.handle_key(key(KeyCode::Char('x'))),
        Action::ToggleExpertMode
    );
    assert_eq!(
        apply_action(&mut app, Action::ToggleExpertMode),
        AppOutcome::Continue
    );

    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("expert general menu should render");
    assert_eq!(
        terminal.lines[0].trim_end(),
        " GENERAL COMMAND <- ? X V I A S P M C R D O E <Q> ->"
    );
    assert_eq!(terminal.lines[1].trim_end(), "");
}

#[test]
fn main_menu_c_key_opens_theme_picker() {
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
        app.handle_key(key(KeyCode::Char('c'))),
        Action::Startup(StartupAction::OpenThemePicker)
    );
    assert_eq!(
        app.handle_key(key(KeyCode::Char('C'))),
        Action::Startup(StartupAction::OpenThemePicker)
    );
}

#[test]
fn main_menu_popup_help_describes_the_color_theme_picker() {
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
        app.handle_key(key(KeyCode::Char('h'))),
        Action::OpenPopupHelp
    );
    assert_eq!(
        apply_action(&mut app, Action::OpenPopupHelp),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::MainMenu);
    assert!(app.popup_help.is_some());

    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal).expect("main help should render");
    assert!(
        line_containing(&terminal, "open the color theme picker")
            .contains("open the color theme picker")
    );
}

#[test]
fn first_time_and_main_popup_help_share_the_same_color_theme_text() {
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
    advance_to_first_time_menu(&mut app);

    assert_eq!(
        apply_action(&mut app, Action::OpenPopupHelp),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FirstTimeMenu);
    assert!(app.popup_help.is_some());

    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("first-time help should render");
    assert!(
        line_containing(&terminal, "open the color theme picker")
            .contains("open the color theme picker")
    );
}

#[test]
fn door_mode_main_menu_uses_ansi_toggle_and_mag16_theme() {
    let fixture_dir = temp_game_copy();
    CampaignStore::open_default_in_dir(&fixture_dir)
        .expect("open store")
        .set_player_theme_preference(1, "gruvbox")
        .expect("save preference");

    let mut app = App::load(AppConfig {
        game_dir: fixture_dir.clone(),
        player_record_index_1_based: 1,
        export_root: None,
        queue_dir: None,
        session_timeout_secs: None,
        game_config: Default::default(),
    })
    .expect("app should load");
    assert_eq!(theme::current_theme_key().as_deref(), Some("gruvbox"));

    enable_door_mode(&mut app);
    advance_to_main_menu(&mut app);

    assert_eq!(
        theme::current_theme_key().as_deref(),
        Some(theme::door_theme_key())
    );
    assert_eq!(
        theme::classic::logo_style().fg,
        nc_game::screen::GameColor::BrightBlue
    );
    assert_eq!(
        theme::classic::notice_style().fg,
        nc_game::screen::GameColor::BrightRed
    );
    assert_eq!(
        app.handle_key(key(KeyCode::Char('a'))),
        Action::ToggleAnsiMode
    );
    assert_eq!(app.handle_key(key(KeyCode::Char('c'))), Action::Noop);

    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("door main menu should render");
    assert_eq!(
        terminal.line(2).trim_end(),
        "  H>elp with commands   A>nsi color ON/OFF         T>otal Planet Database"
    );
    assert_eq!(
        terminal.line(7).trim_end(),
        " MAIN COMMAND <- ? X V A G P F T I B D <Q> ->"
    );

    let toggle = app.handle_key(key(KeyCode::Char('a')));
    assert_eq!(apply_action(&mut app, toggle), AppOutcome::Continue);
    assert!(!theme::ansi_enabled());
    assert_eq!(
        theme::current_theme_key().as_deref(),
        Some(theme::door_theme_key())
    );
    assert_eq!(
        theme::classic::logo_style().fg,
        nc_game::screen::GameColor::White
    );
    assert_eq!(
        theme::classic::notice_style().fg,
        nc_game::screen::GameColor::White
    );
}

#[test]
fn door_mode_first_time_menu_and_popup_help_use_ansi_toggle_text() {
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
    enable_door_mode(&mut app);
    advance_to_first_time_menu(&mut app);

    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("door first-time menu should render");
    assert_eq!(
        terminal.line(2).trim_end(),
        "  H>elp with commands       L>ist current empires      A>nsi color ON/OFF"
    );
    assert_eq!(
        terminal.line(5).trim_end(),
        " FIRST TIME COMMAND <- ? L J A V <Q> ->"
    );

    assert_eq!(
        apply_action(&mut app, Action::OpenPopupHelp),
        AppOutcome::Continue
    );
    app.render(&mut terminal)
        .expect("door first-time help should render");
    assert!(
        line_containing(&terminal, "turn ANSI color on or off")
            .contains("turn ANSI color on or off")
    );
}

#[test]
fn theme_picker_opens_from_main_menu_applies_selection_and_stays_open() {
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

    open_theme_picker(&mut app);

    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("theme picker should render");
    let title_row = terminal
        .lines
        .iter()
        .position(|line| line.contains("COLOR THEMES:"))
        .expect("theme picker title row");
    let title_col = terminal.lines[title_row]
        .find("COLOR THEMES:")
        .expect("theme picker title col");
    let border_row = terminal
        .lines
        .iter()
        .position(|line| line.contains('┌'))
        .expect("theme picker table row");
    let border_col = terminal.lines[border_row]
        .find('┌')
        .expect("theme picker table col");
    assert_eq!(title_col, border_col + 1);
    let command_line = line_containing(&terminal, "COMMAND <- ? <Q>");
    assert!(command_line.contains("COMMAND <- ? <Q>"));
    let command_col = command_line
        .find("COMMAND")
        .expect("theme picker command col");
    assert_eq!(command_col, border_col + 1);

    theme_picker_select(&mut app, "tokyo_night");
    assert_eq!(
        apply_action(
            &mut app,
            Action::Startup(StartupAction::ApplyThemePickerSelection)
        ),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::ThemePicker);
    assert_eq!(theme::current_theme_key().as_deref(), Some("tokyo_night"));

    app.render(&mut terminal)
        .expect("theme picker should rerender");
    assert!(
        terminal
            .lines
            .iter()
            .all(|line| !line.contains("Applied theme: Tokyo Night."))
    );
}

#[test]
fn main_menu_question_mark_and_h_open_the_same_popup_help() {
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

    let popup_action = app.handle_key(key(KeyCode::Char('?')));
    assert_eq!(popup_action, Action::OpenPopupHelp);
    assert_eq!(apply_action(&mut app, popup_action), AppOutcome::Continue);
    assert_eq!(app.current_screen(), ScreenId::MainMenu);

    let popup = app.popup_help.as_ref().expect("popup help should open");
    let spec = menu_help_spec(MenuHelpTopic::Main, false);
    assert_eq!(popup.title, spec.title.trim_end_matches(':'));
    assert_eq!(popup.lines, help_lines(spec.lines));

    let dismiss_action = app.handle_key(key(KeyCode::Char('x')));
    assert_eq!(dismiss_action, Action::DismissPopupHelp);
    assert_eq!(apply_action(&mut app, dismiss_action), AppOutcome::Continue);
    assert!(app.popup_help.is_none());
    assert_eq!(app.current_screen(), ScreenId::MainMenu);

    let help_action = app.handle_key(key(KeyCode::Char('h')));
    assert_eq!(help_action, Action::OpenPopupHelp);
    assert_eq!(apply_action(&mut app, help_action), AppOutcome::Continue);
    assert_eq!(app.current_screen(), ScreenId::MainMenu);
    let popup = app
        .popup_help
        .as_ref()
        .expect("popup help should open from h");
    assert_eq!(popup.lines, help_lines(spec.lines));
}

#[test]
fn theme_picker_popup_help_consumes_the_dismiss_key() {
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
    open_theme_picker(&mut app);

    let starting_cursor = app.startup_state.theme_picker_cursor;
    let popup_action = app.handle_key(key(KeyCode::Char('?')));
    assert_eq!(popup_action, Action::OpenPopupHelp);
    assert_eq!(apply_action(&mut app, popup_action), AppOutcome::Continue);
    assert!(app.popup_help.is_some());

    let dismiss_action = app.handle_key(key(KeyCode::Char('j')));
    assert_eq!(dismiss_action, Action::DismissPopupHelp);
    assert_eq!(apply_action(&mut app, dismiss_action), AppOutcome::Continue);
    assert!(app.popup_help.is_none());
    assert_eq!(app.startup_state.theme_picker_cursor, starting_cursor);

    let move_action = app.handle_key(key(KeyCode::Char('j')));
    assert_eq!(
        move_action,
        Action::Startup(StartupAction::MoveThemePicker(1))
    );
    assert_eq!(apply_action(&mut app, move_action), AppOutcome::Continue);
    assert_eq!(app.startup_state.theme_picker_cursor, starting_cursor + 1);
}

#[test]
fn theme_picker_reopen_after_non_default_selection_keeps_cursor_on_active_theme() {
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

    // First open: pick a non-default theme (nord).
    open_theme_picker(&mut app);
    theme_picker_select(&mut app, "nord");
    apply_action(
        &mut app,
        Action::Startup(StartupAction::ApplyThemePickerSelection),
    );
    assert_eq!(theme::current_theme_key().as_deref(), Some("nord"));

    // Exit and reopen the picker.
    apply_action(&mut app, Action::Startup(StartupAction::ExitThemePicker));
    open_theme_picker(&mut app);

    // The current_theme_key (drives the * marker) should still be "nord".
    assert_eq!(
        theme::current_theme_key().as_deref(),
        Some("nord"),
        "current_theme_key should remain 'nord' after reopening the picker"
    );

    // The cursor should be on "nord", not on "tokyo_night".
    let cursor = app.startup_state.theme_picker_cursor;
    let active_row = app
        .startup_state
        .theme_picker_rows
        .get(cursor)
        .expect("cursor should be in range");
    assert_eq!(
        active_row.key, "nord",
        "cursor should land on the active theme (nord) when reopening the picker, not on {}",
        active_row.key
    );

    // Render and confirm the * marker appears on nord.
    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("theme picker should render after reopen");
    let nord_row = terminal
        .lines
        .iter()
        .find(|line| line.contains("Nord"))
        .cloned()
        .expect("Nord should appear in the picker");
    assert!(
        nord_row.contains('*'),
        "the * active marker should be on Nord after reopen, but the Nord row was: {nord_row:?}"
    );
}

#[test]
fn theme_picker_cursor_moves_freely_after_reopen_on_non_default_theme() {
    // Reproduces: after picking a non-default theme, exiting, and reopening,
    // moving the cursor with j/k should stay where you move it -- not snap back.
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

    // Open, navigate to nord via MoveThemePicker, apply, exit.
    open_theme_picker(&mut app);
    let nord_pos = app
        .startup_state
        .theme_picker_rows
        .iter()
        .position(|r| r.key == "nord")
        .expect("nord should be in picker");
    let tokyo_pos = app
        .startup_state
        .theme_picker_rows
        .iter()
        .position(|r| r.key == "tokyo_night")
        .expect("tokyo_night should be in picker");
    // Move cursor to nord using delta from initial position (tokyo_night).
    let delta = nord_pos as isize - tokyo_pos as isize;
    apply_action(
        &mut app,
        Action::Startup(StartupAction::MoveThemePicker(delta)),
    );
    assert_eq!(app.startup_state.theme_picker_cursor, nord_pos);
    apply_action(
        &mut app,
        Action::Startup(StartupAction::ApplyThemePickerSelection),
    );
    assert_eq!(theme::current_theme_key().as_deref(), Some("nord"));
    apply_action(&mut app, Action::Startup(StartupAction::ExitThemePicker));

    // Reopen the picker.
    open_theme_picker(&mut app);
    assert_eq!(
        app.startup_state.theme_picker_cursor, nord_pos,
        "cursor should start on nord (the active theme) after reopen"
    );

    // Now move the cursor down one row.
    apply_action(&mut app, Action::Startup(StartupAction::MoveThemePicker(1)));
    let expected = (nord_pos + 1).min(app.startup_state.theme_picker_rows.len().saturating_sub(1));
    assert_eq!(
        app.startup_state.theme_picker_cursor, expected,
        "cursor should move down from nord, not snap back to tokyo_night"
    );

    // Move it again -- still should not snap back.
    apply_action(&mut app, Action::Startup(StartupAction::MoveThemePicker(1)));
    let expected2 = (expected + 1).min(app.startup_state.theme_picker_rows.len().saturating_sub(1));
    assert_eq!(
        app.startup_state.theme_picker_cursor, expected2,
        "cursor should continue moving freely after reopen"
    );
}

#[test]
fn theme_picker_wraps_from_top_to_bottom_and_bottom_to_top() {
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
    open_theme_picker(&mut app);

    let last = app.startup_state.theme_picker_rows.len().saturating_sub(1);
    app.startup_state.theme_picker_cursor = 0;
    app.startup_state.theme_picker_scroll_offset = 0;

    assert_eq!(
        apply_action(
            &mut app,
            Action::Startup(StartupAction::MoveThemePicker(-1))
        ),
        AppOutcome::Continue
    );
    assert_eq!(app.startup_state.theme_picker_cursor, last);

    assert_eq!(
        apply_action(&mut app, Action::Startup(StartupAction::MoveThemePicker(1))),
        AppOutcome::Continue
    );
    assert_eq!(app.startup_state.theme_picker_cursor, 0);
}

#[test]
fn theme_picker_q_returns_to_originating_menu() {
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
    advance_to_first_time_menu(&mut app);

    open_theme_picker(&mut app);
    assert_eq!(
        apply_action(&mut app, Action::Startup(StartupAction::ExitThemePicker)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FirstTimeMenu);
}

#[test]
fn joined_player_theme_preference_persists_across_reload() {
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

    open_theme_picker(&mut app);
    theme_picker_select(&mut app, "tokyo_night");
    apply_action(
        &mut app,
        Action::Startup(StartupAction::ApplyThemePickerSelection),
    );
    assert_eq!(theme::current_theme_key().as_deref(), Some("tokyo_night"));
    assert_eq!(
        CampaignStore::open_default_in_dir(&fixture_dir)
            .expect("open store")
            .player_theme_preference(1)
            .expect("load preference")
            .as_deref(),
        Some("tokyo_night")
    );

    let reloaded = App::load(AppConfig {
        game_dir: fixture_dir.clone(),
        player_record_index_1_based: 1,
        export_root: None,
        queue_dir: None,
        session_timeout_secs: None,
        game_config: Default::default(),
    })
    .expect("app should reload");
    assert!(reloaded.player.is_joined);
    assert_eq!(theme::current_theme_key().as_deref(), Some("tokyo_night"));
}

#[test]
fn prejoin_theme_choice_stays_session_only_until_join_completes() {
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
    advance_to_first_time_menu(&mut app);

    open_theme_picker(&mut app);
    theme_picker_select(&mut app, "tokyo_night");
    apply_action(
        &mut app,
        Action::Startup(StartupAction::ApplyThemePickerSelection),
    );
    assert_eq!(theme::current_theme_key().as_deref(), Some("tokyo_night"));
    assert_eq!(
        CampaignStore::open_default_in_dir(&fixture_dir)
            .expect("open store")
            .player_theme_preference(1)
            .expect("load preference"),
        None
    );
}

#[test]
fn prejoin_theme_choice_persists_after_successful_join() {
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
    advance_to_first_time_menu(&mut app);

    open_theme_picker(&mut app);
    theme_picker_select(&mut app, "tokyo_night");
    apply_action(
        &mut app,
        Action::Startup(StartupAction::ApplyThemePickerSelection),
    );
    apply_action(&mut app, Action::Startup(StartupAction::ExitThemePicker));

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
    apply_action(
        &mut app,
        Action::Startup(StartupAction::AcceptFirstTimePrompt),
    );
    apply_action(
        &mut app,
        Action::Startup(StartupAction::AcceptFirstTimePrompt),
    );
    for ch in "Codex Prime".chars() {
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

    assert_eq!(app.current_screen(), ScreenId::MainMenu);
    assert_eq!(
        CampaignStore::open_default_in_dir(&fixture_dir)
            .expect("open store")
            .player_theme_preference(1)
            .expect("load preference")
            .as_deref(),
        Some("tokyo_night")
    );
}

#[test]
fn missing_theme_file_falls_back_to_classic_and_persists_fallback() {
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

    open_theme_picker(&mut app);
    theme_picker_select(&mut app, "tokyo_night");
    let missing_theme_path = fixture_dir.join("themes").join("tokyo_night.kdl");
    if missing_theme_path.exists() {
        fs::remove_file(&missing_theme_path).expect("remove theme");
    }
    apply_action(
        &mut app,
        Action::Startup(StartupAction::ApplyThemePickerSelection),
    );

    assert_eq!(
        theme::current_theme_key().as_deref(),
        Some(theme::default_theme_key())
    );
    assert_eq!(
        CampaignStore::open_default_in_dir(&fixture_dir)
            .expect("open store")
            .player_theme_preference(1)
            .expect("load preference")
            .as_deref(),
        Some(theme::default_theme_key())
    );

    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("theme picker should render");
    let expected_notice = format!(
        "Theme unavailable. Using {}.",
        theme::default_theme_display_name()
    );
    assert!(
        terminal
            .lines
            .iter()
            .all(|line| !line.contains(&expected_notice))
    );
}

#[test]
fn stale_stored_theme_preference_self_heals_to_classic_on_load() {
    let fixture_dir = temp_game_copy();
    CampaignStore::open_default_in_dir(&fixture_dir)
        .expect("open store")
        .set_player_theme_preference(1, "ghost")
        .expect("store preference");

    let app = App::load(AppConfig {
        game_dir: fixture_dir.clone(),
        player_record_index_1_based: 1,
        export_root: None,
        queue_dir: None,
        session_timeout_secs: None,
        game_config: Default::default(),
    })
    .expect("app should load");

    assert!(app.player.is_joined);
    assert_eq!(
        theme::current_theme_key().as_deref(),
        Some(theme::default_theme_key())
    );
    assert_eq!(
        CampaignStore::open_default_in_dir(&fixture_dir)
            .expect("open store")
            .player_theme_preference(1)
            .expect("load preference")
            .as_deref(),
        Some(theme::default_theme_key())
    );
}

#[test]
fn first_time_menu_status_renders_below_fixed_command_row() {
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
    advance_to_first_time_menu(&mut app);
    app.startup_state.first_time_status = Some("Only two empires remain open.".into());

    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("first-time menu should render");
    assert_eq!(
        terminal.lines[5].trim_end(),
        " FIRST TIME COMMAND <- ? L J C V <Q> ->"
    );
    assert_eq!(terminal.lines[6].trim_end(), "");
    assert_eq!(terminal.lines[7].trim_end(), "");
    assert_eq!(terminal.lines[8].trim_end(), "");
    assert!(terminal.lines[9].contains("Notice: Only two empires remain open."));
}

#[test]
fn planet_menu_matches_verified_v15_command_layout() {
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
        apply_action(&mut app, Action::Planet(PlanetAction::OpenMenu)),
        AppOutcome::Continue
    );

    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("planet menu should render");
    assert_eq!(
        terminal.line(1).trim_end(),
        " PLANET COMMANDS:                                           T>ax rate: Empire"
    );
    assert_eq!(
        terminal.line(2).trim_end(),
        "  H>elp on Options  C>OMMISSION MENU   V>iew Partial Map    S>corch planets"
    );
    assert_eq!(
        terminal.line(3).trim_end(),
        "  Q>uit: Main Menu  M>ASS-COMMISSION   P>LANET LIST         L>oad TTs w/Armies"
    );
    assert_eq!(
        terminal.line(4).trim_end(),
        "  X>pert mode       B>UILD MENU...     I>nfo about Planet   U>nload TT Armies"
    );
    assert_eq!(terminal.line(5).trim_end(), "");
}

#[test]
fn planet_menu_notice_renders_below_fixed_command_row() {
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
        apply_action(&mut app, Action::Planet(PlanetAction::OpenMenu)),
        AppOutcome::Continue
    );
    app.command_menu_notice = Some("No ships or starbases are waiting in stardock.".into());

    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("planet menu should render");
    assert_eq!(
        terminal.lines[6].trim_end(),
        " PLANET COMMAND <- ? X V C M B I P T S L U <Q> ->"
    );
    assert_eq!(terminal.lines[7].trim_end(), "");
    assert_eq!(terminal.lines[8].trim_end(), "");
    assert_eq!(terminal.lines[9].trim_end(), "");
    assert!(terminal.lines[10].contains("Notice: No ships or starbases are waiting in stardock."));
}

#[test]
fn planet_menu_expert_mode_keeps_notice_below_top_prompt() {
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
        apply_action(&mut app, Action::Planet(PlanetAction::OpenMenu)),
        AppOutcome::Continue
    );
    app.expert_mode = true;
    app.command_menu_notice = Some("No ships or starbases are waiting in stardock.".into());

    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("expert planet menu should render");
    assert_eq!(
        terminal.lines[0].trim_end(),
        " PLANET COMMAND <- ? X V C M B I P T S L U <Q> ->"
    );
    assert_eq!(terminal.lines[1].trim_end(), "");
    assert_eq!(terminal.lines[2].trim_end(), "");
    assert_eq!(terminal.lines[3].trim_end(), "");
    assert!(terminal.lines[4].contains("Notice: No ships or starbases are waiting in stardock."));
}

#[test]
fn expert_mode_survives_command_menu_navigation_and_non_menu_screens_render_normally() {
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
        apply_action(&mut app, Action::ToggleExpertMode),
        AppOutcome::Continue
    );
    assert!(app.expert_mode);

    assert_eq!(
        apply_action(&mut app, Action::Planet(PlanetAction::OpenMenu)),
        AppOutcome::Continue
    );
    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("expert planet menu should render");
    assert_eq!(
        terminal.lines[0].trim_end(),
        " PLANET COMMAND <- ? X V C M B I P T S L U <Q> ->"
    );

    assert_eq!(
        apply_action(&mut app, Action::Planet(PlanetAction::OpenBuildMenu)),
        AppOutcome::Continue
    );
    app.render(&mut terminal)
        .expect("expert build menu should render");
    assert_eq!(
        terminal.lines[0].trim_end(),
        " BUILD COMMAND <- ? X V P R C N S A D I <Q> ->"
    );
    assert_eq!(terminal.lines[1].trim_end(), "");

    assert_eq!(
        apply_action(&mut app, Action::Planet(PlanetAction::OpenBuildList)),
        AppOutcome::Continue
    );
    app.render(&mut terminal)
        .expect("empty build list should leave expert build menu visible");
    assert_eq!(
        terminal.lines[0].trim_end(),
        " BUILD COMMAND <- ? X V P R C N S A D I <Q> ->"
    );
    assert!(
        terminal
            .lines
            .iter()
            .any(|line| line.contains("Notice: No build orders are queued."))
    );
}

#[test]
fn command_menus_render_without_crashing_for_empty_empire_state() {
    let fixture_dir = temp_joined_empty_empire_copy();
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

    let mut terminal = CaptureTerminal::new();
    for action in [
        Action::Fleet(FleetAction::OpenMenu),
        Action::Fleet(FleetAction::OpenList),
        Action::Fleet(FleetAction::OpenReviewPrompt),
        Action::Fleet(FleetAction::OpenReview),
        Action::Fleet(FleetAction::OpenChangePrompt),
        Action::Fleet(FleetAction::OpenDetach),
        Action::Fleet(FleetAction::OpenEta),
        Action::Fleet(FleetAction::OpenTransportLoad),
        Action::Fleet(FleetAction::OpenTransportUnload),
        Action::Planet(PlanetAction::OpenMenu),
        Action::Planet(PlanetAction::OpenAutoCommissionPrompt),
        Action::Planet(PlanetAction::OpenCommissionMenu),
        Action::Planet(PlanetAction::OpenBuildMenu),
        Action::Planet(PlanetAction::OpenCurrentBuildPlanetInfo),
        Action::Planet(PlanetAction::OpenBuildList),
        Action::Planet(PlanetAction::OpenBuildChange),
        Action::Planet(PlanetAction::OpenBuildAbortPrompt),
        Action::Planet(PlanetAction::OpenBuildSpecify),
        Action::Planet(PlanetAction::OpenTransportPrompt(
            nc_game::screen::PlanetTransportMode::Load,
        )),
        Action::Planet(PlanetAction::OpenTransportPrompt(
            nc_game::screen::PlanetTransportMode::Unload,
        )),
        Action::Planet(PlanetAction::OpenListSortPrompt(PlanetListMode::Brief)),
        Action::Planet(PlanetAction::OpenListSortPrompt(
            PlanetListMode::BuildSelect,
        )),
        Action::Planet(PlanetAction::SubmitListSort(
            PlanetListMode::Brief,
            PlanetListSort::Location,
        )),
        Action::Planet(PlanetAction::SubmitListSort(
            PlanetListMode::BuildSelect,
            PlanetListSort::Location,
        )),
    ] {
        apply_action(&mut app, action);
        app.render(&mut terminal)
            .expect("screen should render without crashing");
    }
}
