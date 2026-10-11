use crate::support::*;

#[test]
fn fleet_group_order_uses_select_column_and_space_toggles_rows() {
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
        apply_action(&mut app, Action::Fleet(FleetAction::OpenMenu)),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::OpenList)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FleetList);

    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal).expect("fleet list should render");
    let top_border_line = line_containing(&terminal, "┌");
    let header_line = line_containing(&terminal, "│ID");
    let command_line = line_containing(&terminal, "COMMAND <- ? F S O C E D M T L U SPACE <Q>");
    let table_left = top_border_line
        .chars()
        .position(|ch| ch == '┌')
        .expect("group table should have a top border");
    let title_left = terminal
        .line(0)
        .chars()
        .position(|ch| ch != ' ')
        .expect("title line should contain text");
    let command_left = command_line
        .chars()
        .position(|ch| ch == 'C')
        .expect("command line should start with COMMAND");
    assert_eq!(table_left + 1, title_left);
    assert_eq!(table_left + 1, command_left);
    assert!(header_line.contains("│ID│Sel│Location│Order"));
    assert!(header_line.contains("│Target"));
    assert!(header_line.contains("│Spd│"));
    assert!(header_line.contains("ETA"));
    assert!(header_line.contains("ROE"));
    assert!(header_line.contains("ARs"));
    assert!(header_line.contains("Ships"));
    assert!(
        !terminal
            .lines
            .iter()
            .any(|line| line.contains("Selected fleets: "))
    );
    assert!(!terminal.lines.iter().any(|line| line.contains("│ X │")));

    assert_eq!(
        apply_action(
            &mut app,
            Action::Fleet(FleetAction::ToggleGroupOrderSelection)
        ),
        AppOutcome::Continue
    );
    app.render(&mut terminal)
        .expect("fleet group order selection should render");
    assert!(terminal.lines.iter().any(|line| line.contains("│ X │")));
}

#[test]
fn fleet_group_order_scrollbar_renders_just_right_of_table_border() {
    let rows = (1..=20)
        .map(|idx| FleetRow {
            fleet_record_index_1_based: idx,
            fleet_number: idx as u16,
            coords: [12, 6],
            target_coords: [12, 6],
            order_code: 3,
            current_speed: 0,
            max_speed: 3,
            eta_label: "0".to_string(),
            list_eta_label: "0".to_string(),
            rules_of_engagement: 6,
            loaded_armies: 0,
            order_label: "Patrol".to_string(),
            table_ships_label: "SC1".to_string(),
            damaged_ships: 0,
            join_host_fleet_number: None,
        })
        .collect::<Vec<_>>();
    let mut screen = FleetGroupScreen::new();
    let buffer = screen
        .render(
            nc_game::screen::ScreenGeometry::local_default(),
            &rows,
            0,
            0,
            &BTreeSet::new(),
            FleetGroupOrderMode::SelectingFleets,
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            3015,
            None,
        )
        .expect("group fleet order screen should render");
    let mut terminal = CaptureTerminal::new();
    terminal
        .render(&buffer)
        .expect("captured group fleet order screen should render");

    let top_border_line = line_containing(&terminal, "┐");
    let right_border_col = top_border_line
        .chars()
        .position(|ch| ch == '┐')
        .expect("group order table should have a right border");
    let scrollbar_col = right_border_col + 1;
    let char_at = |line: &str| line.chars().nth(scrollbar_col);
    assert!(terminal.lines.iter().any(|line| char_at(line) == Some('^')));
    assert!(terminal.lines.iter().any(|line| char_at(line) == Some('#')));
    assert!(terminal.lines.iter().any(|line| char_at(line) == Some('v')));
}

#[test]
fn fleet_group_order_ships_column_matches_ship_notation() {
    let rows = vec![FleetRow {
        fleet_record_index_1_based: 1,
        fleet_number: 4,
        coords: [12, 6],
        target_coords: [12, 6],
        order_code: 3,
        current_speed: 0,
        max_speed: 3,
        eta_label: "0".to_string(),
        list_eta_label: "0".to_string(),
        rules_of_engagement: 6,
        loaded_armies: 2,
        order_label: "Patrol".to_string(),
        table_ships_label: "SC2 BB1 DD4 TT*2".to_string(),
        damaged_ships: 0,
        join_host_fleet_number: None,
    }];
    let mut screen = FleetGroupScreen::new();
    let buffer = screen
        .render(
            nc_game::screen::ScreenGeometry::local_default(),
            &rows,
            0,
            0,
            &BTreeSet::new(),
            FleetGroupOrderMode::SelectingFleets,
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            3015,
            None,
        )
        .expect("group fleet order screen should render");
    let mut terminal = CaptureTerminal::new();
    terminal
        .render(&buffer)
        .expect("captured group fleet order screen should render");

    assert!(
        terminal
            .lines
            .iter()
            .any(|line| line.contains("SC2 BB1 DD4 TT*2")),
        "{:#?}",
        terminal.lines
    );
    assert!(
        terminal
            .lines
            .iter()
            .any(|line| line.contains("│  2│") && line.contains("SC2 BB1 DD4 TT*2")),
        "{:#?}",
        terminal.lines
    );
}

#[test]
fn fleet_group_order_ships_column_truncates_by_whole_token_with_plus_marker() {
    let rows = vec![FleetRow {
        fleet_record_index_1_based: 1,
        fleet_number: 4,
        coords: [12, 6],
        target_coords: [12, 6],
        order_code: 3,
        current_speed: 0,
        max_speed: 3,
        eta_label: "0".to_string(),
        list_eta_label: "0".to_string(),
        rules_of_engagement: 6,
        loaded_armies: 2,
        order_label: "Patrol".to_string(),
        table_ships_label: "SC2 BB4 CA3 DD5 TT*2 TT3 ETAC1".to_string(),
        damaged_ships: 0,
        join_host_fleet_number: None,
    }];
    let mut screen = FleetGroupScreen::new();
    let buffer = screen
        .render(
            nc_game::screen::ScreenGeometry::local_default(),
            &rows,
            0,
            0,
            &BTreeSet::new(),
            FleetGroupOrderMode::SelectingFleets,
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            3015,
            None,
        )
        .expect("group fleet order screen should render");
    let mut terminal = CaptureTerminal::new();
    terminal
        .render(&buffer)
        .expect("captured group fleet order screen should render");

    assert!(
        terminal
            .lines
            .iter()
            .any(|line| line.contains("SC2 BB4 CA3 DD5 +")),
        "{:#?}",
        terminal.lines
    );
    assert!(
        terminal
            .lines
            .iter()
            .all(|line| !line.contains("TT*2 TT3 ETAC1"))
    );
}

#[test]
fn fleet_group_order_ar_column_renders_zero_like_fleet_list() {
    let rows = vec![FleetRow {
        fleet_record_index_1_based: 1,
        fleet_number: 4,
        coords: [12, 6],
        target_coords: [12, 6],
        order_code: 3,
        current_speed: 0,
        max_speed: 3,
        eta_label: "0".to_string(),
        list_eta_label: "0".to_string(),
        rules_of_engagement: 6,
        loaded_armies: 0,
        order_label: "Patrol".to_string(),
        table_ships_label: "SC1".to_string(),
        damaged_ships: 0,
        join_host_fleet_number: None,
    }];
    let mut screen = FleetGroupScreen::new();
    let buffer = screen
        .render(
            nc_game::screen::ScreenGeometry::local_default(),
            &rows,
            0,
            0,
            &BTreeSet::new(),
            FleetGroupOrderMode::SelectingFleets,
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            3015,
            None,
        )
        .expect("group fleet order screen should render");
    let mut terminal = CaptureTerminal::new();
    terminal
        .render(&buffer)
        .expect("captured group fleet order screen should render");

    assert!(
        terminal
            .lines
            .iter()
            .any(|line| line.contains("│  0│") && line.contains("SC")),
        "{:#?}",
        terminal.lines
    );
}

#[test]
fn fleet_single_order_coordinate_screen_splits_loaded_and_empty_transports() {
    let mut screen = nc_game::screen::FleetSingleOrderScreen::new();
    let row = FleetRow {
        fleet_record_index_1_based: 1,
        fleet_number: 7,
        coords: [16, 13],
        target_coords: [19, 13],
        order_code: 1,
        current_speed: 3,
        max_speed: 3,
        eta_label: "1".to_string(),
        list_eta_label: "1".to_string(),
        rules_of_engagement: 6,
        loaded_armies: 2,
        order_label: "Move fleet to Sector (19,13)".to_string(),
        table_ships_label: "CA1 TT*2 TT3".to_string(),
        damaged_ships: 0,
        join_host_fleet_number: None,
    };

    let buffer = screen
        .render(
            &row,
            "Move fleet to Sector (19,13)",
            "Bombard",
            nc_game::screen::FleetSingleOrderMode::EnteringTargetX,
            "",
            "",
            "",
            "",
            "19",
            "",
            "13",
            "",
            "",
            3015,
            None,
        )
        .expect("fleet order screen renders");

    assert!(buffer.plain_line(7).contains("Ships: CA1 TT*2 TT3"));
    assert!(!buffer.plain_line(7).contains("AR="));
}

#[test]
fn fleet_single_order_named_target_screen_splits_loaded_and_empty_transports() {
    let mut screen = nc_game::screen::FleetSingleOrderScreen::new();
    let row = FleetRow {
        fleet_record_index_1_based: 1,
        fleet_number: 7,
        coords: [16, 13],
        target_coords: [19, 13],
        order_code: 13,
        current_speed: 3,
        max_speed: 3,
        eta_label: "1".to_string(),
        list_eta_label: "1".to_string(),
        rules_of_engagement: 6,
        loaded_armies: 2,
        order_label: "Join Fleet #3".to_string(),
        table_ships_label: "CA1 TT*2 TT3".to_string(),
        damaged_ships: 0,
        join_host_fleet_number: Some(3),
    };

    let buffer = screen
        .render(
            &row,
            "Join Fleet #3",
            "Join another fleet",
            nc_game::screen::FleetSingleOrderMode::EnteringTarget,
            "Enter the host fleet number for Join another fleet.",
            "Fleet # ",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            3015,
            None,
        )
        .expect("named-target fleet order screen renders");

    assert!(buffer.plain_line(7).contains("Ships: CA1 TT*2 TT3"));
    assert!(!buffer.plain_line(7).contains("AR="));
}

#[test]
fn fleet_group_order_opens_mission_picker_and_q_returns_to_group_table() {
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
        apply_action(&mut app, Action::Fleet(FleetAction::OpenMenu)),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::OpenGroupOrder)),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(
            &mut app,
            Action::Fleet(FleetAction::ToggleGroupOrderSelection)
        ),
        AppOutcome::Continue
    );
    assert_eq!(
        app.handle_key(key(KeyCode::Enter)),
        Action::Fleet(FleetAction::OpenMissionPicker)
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::OpenMissionPicker)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FleetMissionPicker);

    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("fleet mission picker should render");
    let border = terminal.line(1);
    let left_padding = border
        .find('┌')
        .expect("mission picker border should render");
    assert!(left_padding > 0, "mission picker table should be centered");
    assert!(border.trim_end().chars().count() < 80);
    assert_eq!(
        terminal.line(0).find("FLEET MISSION ORDERS:"),
        Some(left_padding + 1)
    );
    assert!(terminal.line(2).contains("No."));
    assert!(terminal.lines.iter().any(|line| line.contains("15")));
    let prompt = line_containing(&terminal, "COMMAND <- ? <Q> [");
    assert_eq!(prompt.find("COMMAND"), Some(left_padding + 1));
    assert!(prompt.contains("COMMAND <- ? <Q> ["));
    assert!(prompt.contains("->"));

    assert_eq!(
        app.handle_key(key(KeyCode::Char('q'))),
        Action::Fleet(FleetAction::OpenMissionPicker)
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::OpenMissionPicker)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FleetGroupOrder);
}
#[test]
fn fleet_order_prompt_opens_mission_picker_and_q_returns_to_order_prompt() {
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
        apply_action(&mut app, Action::Fleet(FleetAction::OpenMenu)),
        AppOutcome::Continue
    );
    assert_eq!(
        app.handle_key(key(KeyCode::Char('o'))),
        Action::Fleet(FleetAction::OpenOrder)
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::OpenOrder)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FleetMenu);

    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("fleet order prompt should render");
    let prompt = line_containing(&terminal, "COMMAND <- Order Fleet #");
    assert!(prompt.contains("Order Fleet # ["));
    assert!(prompt.contains("<Q> ->"));

    submit_fleet_menu_prompt(&mut app, Some(2));
    assert_eq!(app.current_screen(), ScreenId::FleetMissionPicker);
    assert_eq!(
        app.handle_key(key(KeyCode::PageDown)),
        Action::Fleet(FleetAction::MoveMissionPicker(8))
    );
    app.render(&mut terminal)
        .expect("fleet mission picker should render");
    let border = terminal.line(1);
    let left_padding = border
        .find('┌')
        .expect("mission picker border should render");
    assert_eq!(
        terminal.line(0).find("FLEET MISSION ORDERS:"),
        Some(left_padding + 1)
    );
    let prompt = line_containing(&terminal, "COMMAND <- ? <Q> [");
    assert_eq!(prompt.find("COMMAND"), Some(left_padding + 1));
    assert!(prompt.contains("COMMAND <- ? <Q> ["));
    assert!(prompt.contains("->"));

    assert_eq!(
        app.handle_key(key(KeyCode::Char('q'))),
        Action::Fleet(FleetAction::OpenMissionPicker)
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::OpenMissionPicker)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FleetMenu);
    app.render(&mut terminal)
        .expect("fleet menu should render after cancel");
    assert!(
        !terminal
            .lines
            .iter()
            .any(|line| line.contains("COMMAND <- Order Fleet #"))
    );
    assert_eq!(
        app.handle_key(key(KeyCode::Char('o'))),
        Action::Fleet(FleetAction::OpenOrder)
    );
}

#[test]
fn fleet_order_applies_move_order_to_selected_fleet_only() {
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
        apply_action(&mut app, Action::Fleet(FleetAction::OpenMenu)),
        AppOutcome::Continue
    );
    open_order_mission_picker_from_fleet_menu(&mut app, Some(2));
    assert_eq!(
        apply_action(
            &mut app,
            Action::Fleet(FleetAction::AppendMissionPickerChar('1'))
        ),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::SubmitMissionPicker)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FleetOrder);
    enter_fleet_order_target(&mut app, [14, 9]);
    confirm_fleet_order(&mut app, true);
    assert_eq!(app.current_screen(), ScreenId::FleetMenu);

    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("fleet menu should render success notice");
    assert!(
        !terminal
            .lines
            .iter()
            .any(|line| line.contains("COMMAND <- Order Fleet #"))
    );
    assert!(
        terminal
            .lines
            .iter()
            .any(|line| line.contains("Notice: Applied move to Fleet #2 for sector [14,9]."))
    );
    assert!(
        !terminal
            .lines
            .iter()
            .any(|line| line.contains("Error: Applied move to Fleet #2 for sector [14,9]."))
    );

    let state = latest_runtime_state(&fixture_dir);
    let ordered_fleet = state
        .game_data
        .fleets
        .iter()
        .find(|fleet| fleet.owner.0 == 1 && fleet.fleet_number.0 == 2)
        .expect("fleet #2 should exist");
    assert_eq!(ordered_fleet.order.code(), 1);
    assert_eq!(ordered_fleet.target.to_array(), [14, 9]);
    assert_eq!(
        state
            .game_data
            .fleets
            .iter()
            .filter(|fleet| !(fleet.owner.0 == 1 && fleet.fleet_number.0 == 2))
            .filter(|fleet| fleet.order.code() == 1)
            .count(),
        0
    );
}

#[test]
fn fleet_order_confirm_uses_stopped_eta_when_selected_fleet_speed_is_zero() {
    let fixture_dir = temp_game_copy();
    let mut state = latest_runtime_state(&fixture_dir);
    let current_coords = state
        .game_data
        .fleets
        .iter()
        .find(|fleet| fleet.owner.0 == 1 && fleet.fleet_number.0 == 1)
        .expect("fleet #1 should exist")
        .sector
        .to_array();
    let target = first_other_planet_coords(&state.game_data, current_coords);
    state
        .game_data
        .fleets
        .iter_mut()
        .find(|fleet| fleet.owner.0 == 1 && fleet.fleet_number.0 == 1)
        .expect("fleet #1 should exist")
        .speed = 0;
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
    let mut terminal = CaptureTerminal::new();

    advance_to_main_menu(&mut app);
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::OpenMenu)),
        AppOutcome::Continue
    );
    open_order_mission_picker_from_fleet_menu(&mut app, Some(1));
    assert_eq!(
        apply_action(
            &mut app,
            Action::Fleet(FleetAction::AppendMissionPickerChar('9'))
        ),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::SubmitMissionPicker)),
        AppOutcome::Continue
    );
    enter_fleet_order_target(&mut app, target);

    app.render(&mut terminal)
        .expect("fleet order confirm should render");
    let message = line_containing(&terminal, "Fleet 1 reaches");
    assert!(message.contains(&format!(
        "Fleet 1 reaches ({:02},{:02}) in",
        target[0], target[1]
    )));
    assert!(!message.contains("is stopped"));
}

#[test]
fn fleet_group_order_confirm_uses_eta_when_selected_fleet_speed_is_zero() {
    let fixture_dir = temp_game_copy();
    let mut state = latest_runtime_state(&fixture_dir);
    let current_coords = state
        .game_data
        .fleets
        .iter()
        .find(|fleet| fleet.owner.0 == 1 && fleet.fleet_number.0 == 1)
        .expect("fleet #1 should exist")
        .sector
        .to_array();
    let target = first_other_planet_coords(&state.game_data, current_coords);
    state
        .game_data
        .fleets
        .iter_mut()
        .find(|fleet| fleet.owner.0 == 1 && fleet.fleet_number.0 == 1)
        .expect("fleet #1 should exist")
        .speed = 0;
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
    let mut terminal = CaptureTerminal::new();

    advance_to_main_menu(&mut app);
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::OpenMenu)),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::OpenGroupOrder)),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(
            &mut app,
            Action::Fleet(FleetAction::ToggleGroupOrderSelection)
        ),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::OpenMissionPicker)),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(
            &mut app,
            Action::Fleet(FleetAction::AppendMissionPickerChar('9'))
        ),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::SubmitMissionPicker)),
        AppOutcome::Continue
    );
    enter_fleet_group_order_target(&mut app, target);

    app.render(&mut terminal)
        .expect("fleet group order confirm should render");
    let message = line_containing(&terminal, "reaches");
    assert!(message.contains(&format!("reaches ({:02},{:02}) in", target[0], target[1],)));
    assert!(!message.contains("is stopped"));
}

#[test]
fn fleet_list_live_eta_projects_years_for_stopped_fleet_after_new_order() {
    let fixture_dir = temp_game_copy();
    let mut state = latest_runtime_state(&fixture_dir);
    let current_coords = state
        .game_data
        .fleets
        .iter()
        .find(|fleet| fleet.owner.0 == 1 && fleet.fleet_number.0 == 1)
        .expect("fleet #1 should exist")
        .sector
        .to_array();
    let target = first_other_planet_coords(&state.game_data, current_coords);
    state
        .game_data
        .fleets
        .iter_mut()
        .find(|fleet| fleet.owner.0 == 1 && fleet.fleet_number.0 == 1)
        .expect("fleet #1 should exist")
        .speed = 0;
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
    let mut terminal = CaptureTerminal::new();

    advance_to_main_menu(&mut app);
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::OpenMenu)),
        AppOutcome::Continue
    );
    open_order_mission_picker_from_fleet_menu(&mut app, Some(1));
    assert_eq!(
        apply_action(
            &mut app,
            Action::Fleet(FleetAction::AppendMissionPickerChar('9'))
        ),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::SubmitMissionPicker)),
        AppOutcome::Continue
    );
    enter_fleet_order_target(&mut app, target);
    confirm_fleet_order(&mut app, true);

    let state = latest_runtime_state(&fixture_dir);
    let (fleet_idx, ordered_fleet) = state
        .game_data
        .fleets
        .iter()
        .enumerate()
        .find(|(_, fleet)| fleet.owner.0 == 1 && fleet.fleet_number.0 == 1)
        .expect("fleet #1 should exist after ordering");
    let expected_years = match nc_engine::estimate_fleet_eta_to_destination(
        &state.game_data,
        fleet_idx,
        ordered_fleet.target.to_array(),
        false,
        true,
    ) {
        nc_engine::FleetEtaEstimate::Arrived => 0,
        nc_engine::FleetEtaEstimate::Years(years) => years,
        other => panic!("expected projected ETA for ordered fleet, got {other:?}"),
    };

    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::OpenList)),
        AppOutcome::Continue
    );
    app.render(&mut terminal).expect("fleet list should render");
    assert!(
        terminal.lines.iter().any(|line| {
            line.contains("│ 1│")
                && line.contains("View")
                && line.contains(&format!("({:02},{:02})", target[0], target[1]))
                && line.contains(&format!("│{:>4}│", expected_years))
        }),
        "{:#?}",
        terminal.lines
    );
}

#[test]
fn fleet_group_selection_live_eta_projects_years_for_stopped_fleet() {
    let fixture_dir = temp_game_copy();
    let mut state = latest_runtime_state(&fixture_dir);
    let current_coords = state
        .game_data
        .fleets
        .iter()
        .find(|fleet| fleet.owner.0 == 1 && fleet.fleet_number.0 == 1)
        .expect("fleet #1 should exist")
        .sector
        .to_array();
    let target = first_other_planet_coords(&state.game_data, current_coords);
    let fleet = state
        .game_data
        .fleets
        .iter_mut()
        .find(|fleet| fleet.owner.0 == 1 && fleet.fleet_number.0 == 1)
        .expect("fleet #1 should exist");
    fleet.speed = 0;
    fleet.order = nc_data::Order::ViewWorld;
    fleet.target = nc_data::Coords::from_array(target);
    let expected_years = match nc_engine::estimate_fleet_eta_to_destination(
        &state.game_data,
        state
            .game_data
            .fleets
            .iter()
            .position(|fleet| fleet.owner.0 == 1 && fleet.fleet_number.0 == 1)
            .expect("fleet #1 should exist"),
        target,
        false,
        true,
    ) {
        nc_engine::FleetEtaEstimate::Arrived => 0,
        nc_engine::FleetEtaEstimate::Years(years) => years,
        other => panic!("expected projected ETA for group fleet, got {other:?}"),
    };
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
    let mut terminal = CaptureTerminal::new();

    advance_to_main_menu(&mut app);
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::OpenMenu)),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::OpenGroupOrder)),
        AppOutcome::Continue
    );

    app.render(&mut terminal)
        .expect("group fleet order should render");
    assert!(
        terminal.lines.iter().any(|line| {
            line.contains("│ 1│")
                && line.contains("View")
                && line.contains(&format!("({:02},{:02})", target[0], target[1]))
                && line.contains(&format!("│{:>3}│", expected_years))
        }),
        "{:#?}",
        terminal.lines
    );
}

#[test]
fn fit_table_columns_keeps_header_width_for_blank_cells() {
    let columns = [TableColumn::right("ROE", 1), TableColumn::left("Status", 1)];
    let rows = vec![
        vec!["".to_string(), "".to_string()],
        vec!["".to_string(), "OK".to_string()],
    ];

    let fitted = fit_table_columns(&columns, &rows);

    assert_eq!(fitted[0].width, "ROE".len());
    assert_eq!(fitted[1].width, "Status".len());
}

#[test]
fn fleet_mission_requirements_match_manual_summary_table() {
    let expected = [
        (0, "Any ships", FleetMissionRequirement::Any),
        (1, "Any", FleetMissionRequirement::Any),
        (2, "Any", FleetMissionRequirement::Any),
        (3, "Any", FleetMissionRequirement::Any),
        (4, "Combat ships", FleetMissionRequirement::CombatShips),
        (5, "Combat ships", FleetMissionRequirement::CombatShips),
        (6, "Combat ships", FleetMissionRequirement::CombatShips),
        (
            7,
            "Combat + loaded transports",
            FleetMissionRequirement::CombatAndLoadedTransports,
        ),
        (
            8,
            "Loaded transports (combat recommended)",
            FleetMissionRequirement::LoadedTransports,
        ),
        (9, "Any", FleetMissionRequirement::Any),
        (
            10,
            "At least one scout",
            FleetMissionRequirement::AtLeastOneScout,
        ),
        (
            11,
            "At least one scout",
            FleetMissionRequirement::AtLeastOneScout,
        ),
        (
            12,
            "At least one ETAC",
            FleetMissionRequirement::AtLeastOneEtac,
        ),
        (13, "Any", FleetMissionRequirement::Any),
        (14, "Any", FleetMissionRequirement::Any),
        (15, "Any", FleetMissionRequirement::Any),
    ];

    for (option, (code, requirements, requirement)) in
        FLEET_MISSION_OPTIONS.iter().zip(expected.into_iter())
    {
        assert_eq!(option.code, code);
        assert_eq!(option.requirements, requirements);
        assert_eq!(option.requirement, requirement);
    }
}

#[test]
fn fleet_record_supports_manual_requirement_classes() {
    let fixture_dir = temp_game_copy();
    let mut state = latest_runtime_state(&fixture_dir);
    let fleet = state
        .game_data
        .fleets
        .get_mut(0)
        .expect("fleet 1 should exist");

    fleet.ships.battleships = 0;
    fleet.ships.cruisers = 0;
    fleet.ships.destroyers = 0;
    nc_data::set_armies_aboard(fleet, 0);
    fleet.ships.scouts = 0;
    fleet.ships.etacs = 0;

    assert!(fleet_record_supports_mission_code(fleet, 0));
    assert!(fleet_record_supports_mission_code(fleet, 15));
    assert!(!fleet_record_supports_mission_code(fleet, 4));
    assert!(!fleet_record_supports_mission_code(fleet, 8));
    assert!(!fleet_record_supports_mission_code(fleet, 10));
    assert!(!fleet_record_supports_mission_code(fleet, 12));

    fleet.ships.destroyers = 1;
    assert!(fleet_record_supports_mission_code(fleet, 4));
    assert!(fleet_record_supports_mission_code(fleet, 6));
    assert!(!fleet_record_supports_mission_code(fleet, 7));

    nc_data::set_armies_aboard(fleet, 1);
    assert!(fleet_record_supports_mission_code(fleet, 7));
    assert!(fleet_record_supports_mission_code(fleet, 8));

    fleet.ships.destroyers = 0;
    assert!(!fleet_record_supports_mission_code(fleet, 7));
    assert!(fleet_record_supports_mission_code(fleet, 8));

    fleet.ships.scouts = 1;
    assert!(fleet_record_supports_mission_code(fleet, 10));
    assert!(fleet_record_supports_mission_code(fleet, 11));

    fleet.ships.etacs = 1;
    assert!(fleet_record_supports_mission_code(fleet, 12));
}

#[test]
fn fleet_order_allows_guard_starbase_from_fleet_command() {
    let fixture_dir = temp_game_with_starbase_copy();
    let before = latest_runtime_state(&fixture_dir);
    assert_eq!(before.game_data.fleets[0].order.code(), 4);
    assert_eq!(before.game_data.fleets[1].order.code(), 5);
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
        apply_action(&mut app, Action::Fleet(FleetAction::OpenMenu)),
        AppOutcome::Continue
    );
    open_order_mission_picker_from_fleet_menu(&mut app, Some(2));
    assert_eq!(
        apply_action(
            &mut app,
            Action::Fleet(FleetAction::AppendMissionPickerChar('4'))
        ),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::SubmitMissionPicker)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FleetOrder);

    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("guard starbase target prompt should render");
    assert!(line_containing(&terminal, "Location: ").contains("Location: ("));
    assert!(line_containing(&terminal, "Current / Max Speed: ").contains("Current / Max Speed: "));
    assert!(line_containing(&terminal, "ROE: ").contains("ROE: "));
    assert!(line_containing(&terminal, "Order: ").contains("Order: "));
    assert!(line_containing(&terminal, "Ships: ").contains("Ships: "));
    assert!(
        line_containing(&terminal, "Enter the starbase number for Guard a Starbase.")
            .contains("Enter the starbase number for Guard a Starbase.")
    );
    assert!(
        !terminal
            .lines
            .iter()
            .any(|line| line.contains("New Orders: "))
    );
    assert!(
        line_containing(&terminal, "Starbase # [").contains("Starbase # [1]"),
        "{}",
        line_containing(&terminal, "Starbase # [")
    );

    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::SubmitOrder)),
        AppOutcome::Continue
    );

    let state = latest_runtime_state(&fixture_dir);
    let ordered_fleet = state
        .game_data
        .fleets
        .iter()
        .find(|fleet| fleet.owner.0 == 1 && fleet.fleet_number.0 == 2)
        .expect("fleet #2 should exist");
    assert_eq!(ordered_fleet.order.code(), 4);
    assert_eq!(
        ([
            ordered_fleet.target_depth,
            ordered_fleet.mission_target as u8
        ]),
        [1, 1]
    );
    assert_eq!(ordered_fleet.target.to_array(), [6, 5]);
}

#[test]
fn fleet_order_blocks_guard_starbase_when_player_has_no_starbases() {
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
        apply_action(&mut app, Action::Fleet(FleetAction::OpenMenu)),
        AppOutcome::Continue
    );
    open_order_mission_picker_from_fleet_menu(&mut app, Some(2));
    assert_eq!(
        apply_action(
            &mut app,
            Action::Fleet(FleetAction::AppendMissionPickerChar('4'))
        ),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::SubmitMissionPicker)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FleetMissionPicker);

    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("no-starbase guard order notice should render");
    assert!(
        terminal
            .lines
            .iter()
            .all(|line| !line.contains("You have no starbases available to guard."))
    );
}

#[test]
fn fleet_order_allows_join_another_fleet_from_fleet_command() {
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
        apply_action(&mut app, Action::Fleet(FleetAction::OpenMenu)),
        AppOutcome::Continue
    );
    open_order_mission_picker_from_fleet_menu(&mut app, Some(1));
    assert_eq!(
        apply_action(
            &mut app,
            Action::Fleet(FleetAction::AppendMissionPickerChar('1'))
        ),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(
            &mut app,
            Action::Fleet(FleetAction::AppendMissionPickerChar('3'))
        ),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::SubmitMissionPicker)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FleetOrder);

    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("join-fleet target prompt should render");
    assert!(line_containing(&terminal, "Location: ").contains("Location: ("));
    assert!(line_containing(&terminal, "Current / Max Speed: ").contains("Current / Max Speed: "));
    assert!(line_containing(&terminal, "ROE: ").contains("ROE: "));
    assert!(line_containing(&terminal, "Order: ").contains("Order: "));
    assert!(line_containing(&terminal, "Ships: ").contains("Ships: "));
    assert!(
        line_containing(
            &terminal,
            "Enter the host fleet number for Join another fleet."
        )
        .contains("Enter the host fleet number for Join another fleet.")
    );
    assert!(
        !terminal
            .lines
            .iter()
            .any(|line| line.contains("New Orders: "))
    );
    assert!(
        line_containing(&terminal, "Fleet # [").contains("Fleet # ["),
        "{}",
        line_containing(&terminal, "Fleet # [")
    );

    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::SubmitOrder)),
        AppOutcome::Continue
    );

    let state = latest_runtime_state(&fixture_dir);
    let source = state
        .game_data
        .fleets
        .iter()
        .find(|fleet| fleet.owner.0 == 1 && fleet.fleet_number.0 == 1)
        .expect("fleet #1 should exist");
    assert_eq!(source.order, nc_data::Order::JoinAnotherFleet);
    assert_ne!(source.mission_target, 0);
    let valid_host = state.game_data.fleets.iter().any(|fleet| {
        fleet.owner.0 == 1
            && fleet.fleet_number.0 == source.mission_target
            && fleet.sector.to_array() == source.target.to_array()
    });
    assert!(valid_host);
}

#[test]
fn fleet_order_allows_rendezvous_sector_from_fleet_command() {
    let fixture_dir = temp_game_copy();
    let mut state = latest_runtime_state(&fixture_dir);
    let rendezvous_target = [10, 13];
    state.game_data.fleets[1].order = nc_data::Order::RendezvousSector;
    state.game_data.fleets[1].target = nc_data::Coords::from_array(rendezvous_target);
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
        apply_action(&mut app, Action::Fleet(FleetAction::OpenMenu)),
        AppOutcome::Continue
    );
    open_order_mission_picker_from_fleet_menu(&mut app, Some(1));
    assert_eq!(
        apply_action(
            &mut app,
            Action::Fleet(FleetAction::AppendMissionPickerChar('1'))
        ),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(
            &mut app,
            Action::Fleet(FleetAction::AppendMissionPickerChar('4'))
        ),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::SubmitMissionPicker)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FleetOrder);

    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("rendezvous target prompt should render");
    assert!(
        line_containing(&terminal, "Target XX [")
            .contains(&format!("Target XX [{:02}] <Q> ->", rendezvous_target[0]))
    );

    enter_fleet_order_target(&mut app, rendezvous_target);
    confirm_fleet_order(&mut app, true);

    let state = latest_runtime_state(&fixture_dir);
    let ordered_fleet = state
        .game_data
        .fleets
        .iter()
        .find(|fleet| fleet.owner.0 == 1 && fleet.fleet_number.0 == 1)
        .expect("fleet #1 should exist");
    assert_eq!(ordered_fleet.order, nc_data::Order::RendezvousSector);
    assert_eq!(ordered_fleet.target.to_array(), rendezvous_target);
}

#[test]
fn fleet_order_persists_immediately_and_reloaded_tables_reflect_it() {
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
        apply_action(&mut app, Action::Fleet(FleetAction::OpenMenu)),
        AppOutcome::Continue
    );
    open_order_mission_picker_from_fleet_menu(&mut app, Some(2));
    assert_eq!(
        apply_action(
            &mut app,
            Action::Fleet(FleetAction::AppendMissionPickerChar('1'))
        ),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::SubmitMissionPicker)),
        AppOutcome::Continue
    );
    enter_fleet_order_target(&mut app, [14, 9]);
    confirm_fleet_order(&mut app, true);
    assert_eq!(app.current_screen(), ScreenId::FleetMenu);

    let persisted = latest_runtime_state(&fixture_dir);
    assert_eq!(persisted.game_data.fleets[1].order.code(), 1);
    assert_eq!(persisted.game_data.fleets[1].target.to_array(), [14, 9]);

    let mut reloaded = App::load(AppConfig {
        game_dir: fixture_dir.clone(),
        player_record_index_1_based: 1,
        export_root: None,
        queue_dir: None,
        session_timeout_secs: None,
        game_config: Default::default(),
    })
    .expect("reloaded app should load");
    advance_to_main_menu(&mut reloaded);
    assert_eq!(
        apply_action(&mut reloaded, Action::Fleet(FleetAction::OpenMenu)),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut reloaded, Action::Fleet(FleetAction::OpenList)),
        AppOutcome::Continue
    );
    let mut terminal = CaptureTerminal::new();
    reloaded
        .render(&mut terminal)
        .expect("reloaded fleet list should render");
    let table_text = (3..16)
        .map(|row| terminal.line(row).to_string())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(table_text.contains("(14,09)"));
}

#[test]
fn fleet_order_from_stopped_fleet_uses_max_speed() {
    let fixture_dir = temp_game_copy();
    let mut state = latest_runtime_state(&fixture_dir);
    state.game_data.fleets[1].speed = 0;
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
        apply_action(&mut app, Action::Fleet(FleetAction::OpenMenu)),
        AppOutcome::Continue
    );
    open_order_mission_picker_from_fleet_menu(&mut app, Some(2));
    assert_eq!(
        apply_action(
            &mut app,
            Action::Fleet(FleetAction::AppendMissionPickerChar('1'))
        ),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::SubmitMissionPicker)),
        AppOutcome::Continue
    );
    enter_fleet_order_target(&mut app, [14, 9]);
    confirm_fleet_order(&mut app, true);

    let persisted = latest_runtime_state(&fixture_dir);
    assert_eq!(persisted.game_data.fleets[1].order.code(), 1);
    assert_eq!(persisted.game_data.fleets[1].target.to_array(), [14, 9]);
    assert_eq!(persisted.game_data.fleets[1].speed, 3);
    assert_eq!(
        persisted.game_data.fleets[1].speed,
        persisted.game_data.fleets[1].max_speed
    );
}

#[test]
fn fleet_order_preserves_explicit_nonzero_speed() {
    let fixture_dir = temp_game_copy();
    let mut state = latest_runtime_state(&fixture_dir);
    state.game_data.fleets[1].speed = 1;
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
        apply_action(&mut app, Action::Fleet(FleetAction::OpenMenu)),
        AppOutcome::Continue
    );
    open_order_mission_picker_from_fleet_menu(&mut app, Some(2));
    assert_eq!(
        apply_action(
            &mut app,
            Action::Fleet(FleetAction::AppendMissionPickerChar('1'))
        ),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::SubmitMissionPicker)),
        AppOutcome::Continue
    );
    enter_fleet_order_target(&mut app, [14, 9]);
    confirm_fleet_order(&mut app, true);

    let persisted = latest_runtime_state(&fixture_dir);
    assert_eq!(persisted.game_data.fleets[1].order.code(), 1);
    assert_eq!(persisted.game_data.fleets[1].target.to_array(), [14, 9]);
    assert_eq!(persisted.game_data.fleets[1].speed, 1);
    assert_eq!(persisted.game_data.fleets[1].max_speed, 3);
}

#[test]
fn fleet_order_screen_uses_compact_summary_and_eta_confirm() {
    let fixture_dir = temp_game_copy();
    let mut state = latest_runtime_state(&fixture_dir);
    let bombard_target = state
        .game_data
        .planets
        .iter()
        .find(|planet| planet.owner.0 != 1)
        .expect("fixture should have a foreign world")
        .sector
        .to_array();
    state.game_data.fleets[1].order = nc_data::Order::from_code(5);
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
        apply_action(&mut app, Action::Fleet(FleetAction::OpenMenu)),
        AppOutcome::Continue
    );
    open_order_mission_picker_from_fleet_menu(&mut app, Some(2));
    assert_eq!(
        apply_action(
            &mut app,
            Action::Fleet(FleetAction::AppendMissionPickerChar('6'))
        ),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::SubmitMissionPicker)),
        AppOutcome::Continue
    );

    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("compact fleet order screen should render");
    assert!(line_containing(&terminal, "Location: ").contains("Location: ("));
    assert!(line_containing(&terminal, "Current / Max Speed: ").contains("Current / Max Speed: "));
    assert!(line_containing(&terminal, "ROE: ").contains("ROE: "));
    assert!(line_containing(&terminal, "Order: ").contains("Order: "));
    assert!(line_containing(&terminal, "Ships: ").contains("Ships: "));
    assert!(
        line_containing(&terminal, "Enter target coordinates for new order: ")
            .contains("Enter target coordinates for new order: Bombard")
    );
    let prompt = line_containing(&terminal, "Target XX ");
    assert!(prompt.contains("COMMAND <- Target XX "));
    assert!(
        !terminal
            .lines
            .iter()
            .any(|line| line.contains("New Orders: "))
    );
    assert!(
        !terminal
            .lines
            .iter()
            .any(|line| line.contains("Selected mission:"))
    );
    assert!(
        !terminal
            .lines
            .iter()
            .any(|line| line.contains("Standing order:"))
    );

    enter_fleet_order_target(&mut app, bombard_target);
    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("fleet order confirm should render");
    assert!(
        line_containing(&terminal, "Stardate: ")
            .contains(&format!("Stardate: {}", app.game_data.conquest.game_year))
    );
    assert!(line_containing(&terminal, "Confirm [Y]/N").contains("Confirm [Y]/N"));
    assert!(line_containing(&terminal, "New Orders: ").contains("New Orders: Bombard"));
    assert!(terminal.lines.iter().any(|line| line.contains(&format!(
        "reaches ({:02},{:02}) in",
        bombard_target[0], bombard_target[1]
    ))));
    assert!(
        !terminal
            .lines
            .iter()
            .any(|line| line.contains("Location: "))
    );
    assert!(
        !terminal
            .lines
            .iter()
            .any(|line| line.contains("Current / Max Speed: "))
    );
    assert!(!terminal.lines.iter().any(|line| line.contains("ROE: ")));
    assert!(!terminal.lines.iter().any(|line| line.contains("Order: ")));
    assert!(!terminal.lines.iter().any(|line| line.contains("Ships: ")));
}

#[test]
fn fleet_group_order_uses_compact_summary_and_eta_confirm() {
    let fixture_dir = temp_game_copy();
    let bombard_target = latest_runtime_state(&fixture_dir)
        .game_data
        .planets
        .iter()
        .find(|planet| planet.owner.0 != 1)
        .expect("fixture should have a foreign world")
        .sector
        .to_array();

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
        apply_action(&mut app, Action::Fleet(FleetAction::OpenMenu)),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::OpenGroupOrder)),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(
            &mut app,
            Action::Fleet(FleetAction::ToggleGroupOrderSelection)
        ),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::OpenMissionPicker)),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(
            &mut app,
            Action::Fleet(FleetAction::AppendMissionPickerChar('6'))
        ),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::SubmitMissionPicker)),
        AppOutcome::Continue
    );

    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("compact fleet group target screen should render");
    let selected = line_containing(&terminal, "Selected fleets: ")
        .trim()
        .strip_prefix("Selected fleets: ")
        .expect("selected fleets line should have prefix");
    assert_eq!(selected.split(", ").count(), 1);
    assert!(selected.len() >= 2);
    assert!(selected.chars().all(|ch| ch.is_ascii_digit()));
    assert!(
        line_containing(&terminal, "Enter target coordinates for new order: ")
            .contains("Enter target coordinates for new order: Bombard")
    );
    let prompt = line_containing(&terminal, "Target XX ");
    assert!(prompt.contains("COMMAND <- Target XX "));
    assert!(!terminal.lines.iter().any(|line| line.contains("│Sel│")));
    assert!(!terminal.lines.iter().any(|line| line.contains('│')));
    assert!(!terminal.lines.iter().any(|line| line.contains('┌')));

    enter_fleet_group_order_target(&mut app, bombard_target);
    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("compact fleet group confirm should render");
    assert!(
        line_containing(&terminal, "Stardate: ")
            .contains(&format!("Stardate: {}", app.game_data.conquest.game_year))
    );
    let selected = line_containing(&terminal, "Selected fleets: ")
        .trim()
        .strip_prefix("Selected fleets: ")
        .expect("selected fleets line should have prefix");
    assert_eq!(selected.split(", ").count(), 1);
    assert!(selected.len() >= 2);
    assert!(selected.chars().all(|ch| ch.is_ascii_digit()));
    assert!(line_containing(&terminal, "New Orders: ").contains("New Orders: Bombard"));
    assert!(terminal.lines.iter().any(|line| line.contains(&format!(
        "reaches ({:02},{:02}) in",
        bombard_target[0], bombard_target[1]
    ))));
    assert!(line_containing(&terminal, "Confirm [Y]/N").contains("Confirm [Y]/N"));
    assert!(!terminal.lines.iter().any(|line| line.contains("│Sel│")));
    assert!(!terminal.lines.iter().any(|line| line.contains('│')));
    assert!(!terminal.lines.iter().any(|line| line.contains('┌')));
}

#[test]
fn fleet_group_order_lists_selected_fleet_numbers_in_compact_target_entry() {
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
        apply_action(&mut app, Action::Fleet(FleetAction::OpenMenu)),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::OpenGroupOrder)),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(
            &mut app,
            Action::Fleet(FleetAction::ToggleGroupOrderSelection)
        ),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::MoveGroupOrder(1))),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(
            &mut app,
            Action::Fleet(FleetAction::ToggleGroupOrderSelection)
        ),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::OpenMissionPicker)),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(
            &mut app,
            Action::Fleet(FleetAction::AppendMissionPickerChar('3'))
        ),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::SubmitMissionPicker)),
        AppOutcome::Continue
    );

    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("compact fleet group target screen should render");
    let selected = line_containing(&terminal, "Selected fleets: ")
        .trim()
        .strip_prefix("Selected fleets: ")
        .expect("selected fleets line should have prefix");
    let parts = selected.split(", ").collect::<Vec<_>>();
    assert_eq!(parts.len(), 2);
    assert!(
        parts
            .iter()
            .all(|part| part.len() >= 2 && part.chars().all(|ch| ch.is_ascii_digit()))
    );
}

#[test]
fn fleet_group_order_target_y_prompt_renders_with_multiple_selected_fleets() {
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
        apply_action(&mut app, Action::Fleet(FleetAction::OpenMenu)),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::OpenGroupOrder)),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(
            &mut app,
            Action::Fleet(FleetAction::ToggleGroupOrderSelection)
        ),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::MoveGroupOrder(1))),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(
            &mut app,
            Action::Fleet(FleetAction::ToggleGroupOrderSelection)
        ),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::OpenMissionPicker)),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(
            &mut app,
            Action::Fleet(FleetAction::AppendMissionPickerChar('6'))
        ),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::SubmitMissionPicker)),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(
            &mut app,
            Action::Fleet(FleetAction::AppendGroupOrderChar('1'))
        ),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(
            &mut app,
            Action::Fleet(FleetAction::AppendGroupOrderChar('2'))
        ),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::SubmitGroupOrder)),
        AppOutcome::Continue
    );

    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("fleet group target y prompt should render");
    let selected = line_containing(&terminal, "Selected fleets: ");
    assert!(selected.contains(", "));
    assert!(
        line_containing(&terminal, "Target YY ").contains("Target YY "),
        "{:#?}",
        terminal.lines
    );
}

#[test]
fn fleet_order_scout_system_defaults_avoid_worlds_targeted_by_other_friendly_scouts() {
    let fixture_dir = temp_game_copy();
    let mut state = latest_runtime_state(&fixture_dir);
    let viewer_index = 0usize;
    let home_coords = state.game_data.planets[state.game_data.players[0].homeworld as usize - 1]
        .sector
        .to_array();
    let mut candidates = state
        .game_data
        .planets
        .iter()
        .enumerate()
        .filter(|(_, planet)| planet.owner.0 != 1)
        .map(|(idx, planet)| (idx, planet.sector.to_array()))
        .collect::<Vec<_>>();
    candidates.sort_by_key(|(_, coords)| {
        let dx = i32::from(home_coords[0]) - i32::from(coords[0]);
        let dy = i32::from(home_coords[1]) - i32::from(coords[1]);
        dx * dx + dy * dy
    });
    let claimed_coords = candidates[0].1;
    let fallback_coords = candidates[1].1;
    state.game_data.planets[candidates[0].0].owner.0 = 2;
    state.game_data.planets[candidates[1].0].owner.0 = 2;
    for fleet in state.game_data.fleets.iter_mut() {
        if fleet.owner.0 == 1 {
            fleet.order = nc_data::Order::from_code(9);
        }
    }
    let selected_fleet_number = state.game_data.fleets[0].fleet_number.0;
    state.game_data.fleets[0].ships.scouts = 1;
    state.game_data.fleets[0].order = nc_data::Order::from_code(0);
    state.game_data.fleets[1].ships.scouts = 1;
    state.game_data.fleets[1].order = nc_data::Order::ScoutSolarSystem;
    state.game_data.fleets[1].target = nc_data::Coords::from_array(claimed_coords);
    let mut planet_intel_by_viewer = (1..=state.game_data.conquest.player_count)
        .map(|viewer_empire_id| {
            CampaignStore::open_default_in_dir(&fixture_dir)
                .expect("open campaign store")
                .latest_planet_intel_for_viewer(viewer_empire_id)
                .expect("load runtime intel")
                .into_iter()
                .map(|snapshot| (snapshot.planet_record_index_1_based, snapshot))
                .collect::<BTreeMap<_, _>>()
        })
        .collect::<Vec<_>>();
    let year = state.game_data.conquest.game_year;
    planet_intel_by_viewer[viewer_index].clear();
    planet_intel_by_viewer[viewer_index].insert(
        candidates[0].0 + 1,
        partial_known_world_snapshot(
            candidates[0].0 + 1,
            &state.game_data.planets[candidates[0].0],
            2,
            year,
        ),
    );
    planet_intel_by_viewer[viewer_index].insert(
        candidates[1].0 + 1,
        partial_known_world_snapshot(
            candidates[1].0 + 1,
            &state.game_data.planets[candidates[1].0],
            2,
            year,
        ),
    );
    save_runtime_state_with_intel(&fixture_dir, &state, &planet_intel_by_viewer);

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
        apply_action(&mut app, Action::Fleet(FleetAction::OpenMenu)),
        AppOutcome::Continue
    );
    open_order_mission_picker_from_fleet_menu(&mut app, Some(selected_fleet_number));
    assert_eq!(
        apply_action(
            &mut app,
            Action::Fleet(FleetAction::AppendMissionPickerChar('1'))
        ),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(
            &mut app,
            Action::Fleet(FleetAction::AppendMissionPickerChar('1'))
        ),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::SubmitMissionPicker)),
        AppOutcome::Continue
    );

    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("scout system target prompt should render");
    assert!(
        line_containing(&terminal, "Target XX [")
            .contains(&format!("Target XX [{:02}] <Q> ->", fallback_coords[0]))
    );
}

fn latest_planet_intel_by_viewer(
    fixture_dir: &Path,
    player_count: u8,
) -> Vec<BTreeMap<usize, PlanetIntelSnapshot>> {
    (1..=player_count)
        .map(|viewer_empire_id| {
            CampaignStore::open_default_in_dir(fixture_dir)
                .expect("open campaign store")
                .latest_planet_intel_for_viewer(viewer_empire_id)
                .expect("load runtime intel")
                .into_iter()
                .map(|snapshot| (snapshot.planet_record_index_1_based, snapshot))
                .collect::<BTreeMap<_, _>>()
        })
        .collect::<Vec<_>>()
}

#[test]
fn fleet_order_view_world_defaults_to_unknown_world_instead_of_closer_partial_world() {
    let fixture_dir = temp_game_copy();
    let mut state = latest_runtime_state(&fixture_dir);
    let viewer_index = 0usize;
    let selected_fleet = state
        .game_data
        .fleets
        .iter()
        .find(|fleet| fleet.owner.0 == 1)
        .expect("player should have a fleet");
    let selected_fleet_number = selected_fleet.fleet_number.0;
    let anchor = selected_fleet.sector.to_array();
    let mut candidates = state
        .game_data
        .planets
        .iter()
        .enumerate()
        .filter(|(_, planet)| planet.owner.0 != 1)
        .map(|(idx, planet)| (idx, planet.sector.to_array()))
        .collect::<Vec<_>>();
    candidates.sort_by_key(|(_, coords)| {
        let dx = i32::from(anchor[0]) - i32::from(coords[0]);
        let dy = i32::from(anchor[1]) - i32::from(coords[1]);
        dx * dx + dy * dy
    });
    let (_, partial_coords) = candidates[0];
    let (unknown_idx, unknown_coords) = candidates[1];
    for planet in state
        .game_data
        .planets
        .iter_mut()
        .filter(|planet| planet.owner.0 != 1)
    {
        planet.owner.0 = 2;
    }
    let mut planet_intel_by_viewer =
        latest_planet_intel_by_viewer(&fixture_dir, state.game_data.conquest.player_count);
    let year = state.game_data.conquest.game_year;
    planet_intel_by_viewer[viewer_index].clear();
    for (idx, planet) in state.game_data.planets.iter().enumerate() {
        if planet.owner.0 == 1 || idx == unknown_idx {
            continue;
        }
        planet_intel_by_viewer[viewer_index].insert(
            idx + 1,
            partial_known_world_snapshot(idx + 1, planet, 2, year),
        );
    }
    save_runtime_state_with_intel(&fixture_dir, &state, &planet_intel_by_viewer);

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
        apply_action(&mut app, Action::Fleet(FleetAction::OpenMenu)),
        AppOutcome::Continue
    );
    open_order_mission_picker_from_fleet_menu(&mut app, Some(selected_fleet_number));
    assert_eq!(
        apply_action(
            &mut app,
            Action::Fleet(FleetAction::AppendMissionPickerChar('9'))
        ),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::SubmitMissionPicker)),
        AppOutcome::Continue
    );

    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("view-world target prompt should render");
    assert!(
        line_containing(&terminal, "Target XX [")
            .contains(&format!("Target XX [{:02}] <Q> ->", unknown_coords[0]))
    );
    assert_ne!(partial_coords, unknown_coords);
}

#[test]
fn fleet_order_view_world_defaults_avoid_unknown_worlds_targeted_by_other_friendly_fleets() {
    let fixture_dir = temp_game_copy();
    let mut state = latest_runtime_state(&fixture_dir);
    let viewer_index = 0usize;
    let selected_fleet = state
        .game_data
        .fleets
        .iter()
        .find(|fleet| fleet.owner.0 == 1)
        .expect("player should have a fleet");
    let selected_fleet_number = selected_fleet.fleet_number.0;
    let anchor = selected_fleet.sector.to_array();
    let mut candidates = state
        .game_data
        .planets
        .iter()
        .enumerate()
        .filter(|(_, planet)| planet.owner.0 != 1)
        .map(|(idx, planet)| (idx, planet.sector.to_array()))
        .collect::<Vec<_>>();
    candidates.sort_by_key(|(_, coords)| {
        let dx = i32::from(anchor[0]) - i32::from(coords[0]);
        let dy = i32::from(anchor[1]) - i32::from(coords[1]);
        dx * dx + dy * dy
    });
    let (claimed_idx, claimed_coords) = candidates[0];
    let (fallback_idx, fallback_coords) = candidates[1];
    for planet in state
        .game_data
        .planets
        .iter_mut()
        .filter(|planet| planet.owner.0 != 1)
    {
        planet.owner.0 = 2;
    }
    let claimer = state
        .game_data
        .fleets
        .iter_mut()
        .find(|fleet| fleet.owner.0 == 1 && fleet.fleet_number.0 != selected_fleet_number)
        .expect("fixture should have another friendly fleet");
    claimer.order = nc_data::Order::MoveOnly;
    claimer.target = nc_data::Coords::from_array(claimed_coords);
    let mut planet_intel_by_viewer =
        latest_planet_intel_by_viewer(&fixture_dir, state.game_data.conquest.player_count);
    let year = state.game_data.conquest.game_year;
    planet_intel_by_viewer[viewer_index].clear();
    for (idx, planet) in state.game_data.planets.iter().enumerate() {
        if planet.owner.0 == 1 || idx == claimed_idx || idx == fallback_idx {
            continue;
        }
        planet_intel_by_viewer[viewer_index].insert(
            idx + 1,
            partial_known_world_snapshot(idx + 1, planet, 2, year),
        );
    }
    save_runtime_state_with_intel(&fixture_dir, &state, &planet_intel_by_viewer);

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
        apply_action(&mut app, Action::Fleet(FleetAction::OpenMenu)),
        AppOutcome::Continue
    );
    open_order_mission_picker_from_fleet_menu(&mut app, Some(selected_fleet_number));
    assert_eq!(
        apply_action(
            &mut app,
            Action::Fleet(FleetAction::AppendMissionPickerChar('9'))
        ),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::SubmitMissionPicker)),
        AppOutcome::Continue
    );

    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("view-world target prompt should render");
    assert!(
        line_containing(&terminal, "Target XX [")
            .contains(&format!("Target XX [{:02}] <Q> ->", fallback_coords[0]))
    );
}

#[test]
fn fleet_group_view_world_defaults_avoid_unknown_worlds_targeted_by_other_friendly_fleets() {
    let fixture_dir = temp_game_copy();
    let mut state = latest_runtime_state(&fixture_dir);
    let viewer_index = 0usize;
    for fleet in state
        .game_data
        .fleets
        .iter_mut()
        .filter(|fleet| fleet.owner.0 == 1)
    {
        fleet.order = nc_data::Order::from_code(0);
    }
    let mut owned_fleets = state
        .game_data
        .fleets
        .iter()
        .filter(|fleet| fleet.owner.0 == 1)
        .collect::<Vec<_>>();
    owned_fleets.sort_by_key(|fleet| std::cmp::Reverse(fleet.fleet_number.0));
    let anchor = owned_fleets[0].sector.to_array();
    let selected_numbers = [
        owned_fleets[0].fleet_number.0,
        owned_fleets[1].fleet_number.0,
    ];
    let claimer_number = owned_fleets
        .iter()
        .find(|fleet| !selected_numbers.contains(&fleet.fleet_number.0))
        .expect("fixture should have a non-selected friendly fleet")
        .fleet_number
        .0;
    let mut candidates = state
        .game_data
        .planets
        .iter()
        .enumerate()
        .filter(|(_, planet)| planet.owner.0 != 1)
        .map(|(idx, planet)| (idx, planet.sector.to_array()))
        .collect::<Vec<_>>();
    candidates.sort_by_key(|(_, coords)| {
        let dx = i32::from(anchor[0]) - i32::from(coords[0]);
        let dy = i32::from(anchor[1]) - i32::from(coords[1]);
        dx * dx + dy * dy
    });
    let (claimed_idx, claimed_coords) = candidates[0];
    let (fallback_idx, fallback_coords) = candidates[1];
    for planet in state
        .game_data
        .planets
        .iter_mut()
        .filter(|planet| planet.owner.0 != 1)
    {
        planet.owner.0 = 2;
    }
    let claimer = state
        .game_data
        .fleets
        .iter_mut()
        .find(|fleet| fleet.owner.0 == 1 && fleet.fleet_number.0 == claimer_number)
        .expect("claimer fleet should exist");
    claimer.order = nc_data::Order::BombardWorld;
    claimer.target = nc_data::Coords::from_array(claimed_coords);
    let mut planet_intel_by_viewer =
        latest_planet_intel_by_viewer(&fixture_dir, state.game_data.conquest.player_count);
    let year = state.game_data.conquest.game_year;
    planet_intel_by_viewer[viewer_index].clear();
    for (idx, planet) in state.game_data.planets.iter().enumerate() {
        if planet.owner.0 == 1 || idx == claimed_idx || idx == fallback_idx {
            continue;
        }
        planet_intel_by_viewer[viewer_index].insert(
            idx + 1,
            partial_known_world_snapshot(idx + 1, planet, 2, year),
        );
    }
    save_runtime_state_with_intel(&fixture_dir, &state, &planet_intel_by_viewer);

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
        apply_action(&mut app, Action::Fleet(FleetAction::OpenMenu)),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::OpenGroupOrder)),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(
            &mut app,
            Action::Fleet(FleetAction::ToggleGroupOrderSelection)
        ),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::MoveGroupOrder(1))),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(
            &mut app,
            Action::Fleet(FleetAction::ToggleGroupOrderSelection)
        ),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::OpenMissionPicker)),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(
            &mut app,
            Action::Fleet(FleetAction::AppendMissionPickerChar('9'))
        ),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::SubmitMissionPicker)),
        AppOutcome::Continue
    );

    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("group view-world target prompt should render");
    assert!(
        line_containing(&terminal, "Target XX [")
            .contains(&format!("Target XX [{:02}] <Q> ->", fallback_coords[0]))
    );
}

#[test]
fn fleet_order_view_world_falls_back_to_closest_non_owned_world_when_no_unknown_worlds_exist() {
    let fixture_dir = temp_game_copy();
    let mut state = latest_runtime_state(&fixture_dir);
    let viewer_index = 0usize;
    let selected_fleet_number = state
        .game_data
        .fleets
        .iter()
        .find(|fleet| fleet.owner.0 == 1)
        .expect("player should have a fleet")
        .fleet_number
        .0;
    let fleet_coords = state
        .game_data
        .fleets
        .iter()
        .find(|fleet| fleet.owner.0 == 1 && fleet.fleet_number.0 == selected_fleet_number)
        .expect("selected fleet should exist")
        .sector
        .to_array();
    for planet in state
        .game_data
        .planets
        .iter_mut()
        .filter(|planet| planet.owner.0 != 1)
    {
        planet.owner.0 = 2;
    }
    let mut planet_intel_by_viewer =
        latest_planet_intel_by_viewer(&fixture_dir, state.game_data.conquest.player_count);
    let year = state.game_data.conquest.game_year;
    planet_intel_by_viewer[viewer_index].clear();
    let mut fallback_candidates = Vec::new();
    for (idx, planet) in state.game_data.planets.iter().enumerate() {
        if planet.owner.0 == 1 {
            continue;
        }
        fallback_candidates.push(planet.sector.to_array());
        planet_intel_by_viewer[viewer_index].insert(
            idx + 1,
            partial_known_world_snapshot(idx + 1, planet, 2, year),
        );
    }
    fallback_candidates.sort_by_key(|coords| {
        let dx = i32::from(fleet_coords[0]) - i32::from(coords[0]);
        let dy = i32::from(fleet_coords[1]) - i32::from(coords[1]);
        dx * dx + dy * dy
    });
    let fallback_coords = fallback_candidates[0];
    save_runtime_state_with_intel(&fixture_dir, &state, &planet_intel_by_viewer);

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
        apply_action(&mut app, Action::Fleet(FleetAction::OpenMenu)),
        AppOutcome::Continue
    );
    open_order_mission_picker_from_fleet_menu(&mut app, Some(selected_fleet_number));
    assert_eq!(
        apply_action(
            &mut app,
            Action::Fleet(FleetAction::AppendMissionPickerChar('9'))
        ),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::SubmitMissionPicker)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FleetOrder);

    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("view-world target prompt should render");
    let prompt = line_containing(&terminal, "Target XX ");
    assert!(
        prompt.contains(&format!("Target XX [{:02}] <Q> ->", fallback_coords[0])),
        "{prompt}"
    );
}

#[test]
fn fleet_group_bombard_mission_defaults_to_closest_known_enemy_world() {
    let fixture_dir = temp_game_copy();
    let mut state = latest_runtime_state(&fixture_dir);
    let viewer_index = 0usize;
    let home_coords = state.game_data.planets[state.game_data.players[0].homeworld as usize - 1]
        .sector
        .to_array();
    let mut foreign_candidates = state
        .game_data
        .planets
        .iter()
        .enumerate()
        .filter(|(_, planet)| planet.owner.0 != 1)
        .map(|(idx, planet)| (idx, planet.sector.to_array()))
        .collect::<Vec<_>>();
    foreign_candidates.sort_by_key(|(_, coords)| {
        let dx = i32::from(home_coords[0]) - i32::from(coords[0]);
        let dy = i32::from(home_coords[1]) - i32::from(coords[1]);
        dx * dx + dy * dy
    });
    let (closest_idx, closest_coords) = foreign_candidates[0];
    let (other_idx, _) = foreign_candidates[1];
    state.game_data.planets[closest_idx].owner.0 = 2;
    state.game_data.planets[other_idx].owner.0 = 2;
    let mut planet_intel_by_viewer = (1..=state.game_data.conquest.player_count)
        .map(|viewer_empire_id| {
            CampaignStore::open_default_in_dir(&fixture_dir)
                .expect("open campaign store")
                .latest_planet_intel_for_viewer(viewer_empire_id)
                .expect("load runtime intel")
                .into_iter()
                .map(|snapshot| (snapshot.planet_record_index_1_based, snapshot))
                .collect::<BTreeMap<_, _>>()
        })
        .collect::<Vec<_>>();
    let year = state.game_data.conquest.game_year;
    planet_intel_by_viewer[viewer_index].insert(
        closest_idx + 1,
        partial_known_world_snapshot(
            closest_idx + 1,
            &state.game_data.planets[closest_idx],
            2,
            year,
        ),
    );
    planet_intel_by_viewer[viewer_index].insert(
        other_idx + 1,
        partial_known_world_snapshot(other_idx + 1, &state.game_data.planets[other_idx], 2, year),
    );
    save_runtime_state_with_intel(&fixture_dir, &state, &planet_intel_by_viewer);

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
        apply_action(&mut app, Action::Fleet(FleetAction::OpenMenu)),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::OpenGroupOrder)),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(
            &mut app,
            Action::Fleet(FleetAction::ToggleGroupOrderSelection)
        ),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::OpenMissionPicker)),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(
            &mut app,
            Action::Fleet(FleetAction::AppendMissionPickerChar('6'))
        ),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::SubmitMissionPicker)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FleetGroupOrder);

    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("combat target prompt should render");
    assert!(
        line_containing(&terminal, "Target XX [")
            .contains(&format!("Target XX [{:02}] <Q> ->", closest_coords[0]))
    );
}

#[test]
fn fleet_group_colonize_mission_skips_worlds_claimed_by_other_friendly_etacs() {
    let fixture_dir = temp_game_copy();
    let mut state = latest_runtime_state(&fixture_dir);
    let viewer_index = 0usize;
    let home_coords = state.game_data.planets[state.game_data.players[0].homeworld as usize - 1]
        .sector
        .to_array();
    let mut unowned_candidates = state
        .game_data
        .planets
        .iter()
        .enumerate()
        .filter(|(_, planet)| planet.owner.0 == 0)
        .map(|(idx, planet)| (idx, planet.sector.to_array()))
        .collect::<Vec<_>>();
    unowned_candidates.sort_by_key(|(_, coords)| {
        let dx = i32::from(home_coords[0]) - i32::from(coords[0]);
        let dy = i32::from(home_coords[1]) - i32::from(coords[1]);
        dx * dx + dy * dy
    });
    let claimed_coords = unowned_candidates[0].1;
    let fallback_coords = unowned_candidates[1].1;
    let mut planet_intel_by_viewer = (1..=state.game_data.conquest.player_count)
        .map(|viewer_empire_id| {
            CampaignStore::open_default_in_dir(&fixture_dir)
                .expect("open campaign store")
                .latest_planet_intel_for_viewer(viewer_empire_id)
                .expect("load runtime intel")
                .into_iter()
                .map(|snapshot| (snapshot.planet_record_index_1_based, snapshot))
                .collect::<BTreeMap<_, _>>()
        })
        .collect::<Vec<_>>();
    let year = state.game_data.conquest.game_year;
    planet_intel_by_viewer[viewer_index].insert(
        unowned_candidates[0].0 + 1,
        partial_known_world_snapshot(
            unowned_candidates[0].0 + 1,
            &state.game_data.planets[unowned_candidates[0].0],
            0,
            year,
        ),
    );
    planet_intel_by_viewer[viewer_index].insert(
        unowned_candidates[1].0 + 1,
        partial_known_world_snapshot(
            unowned_candidates[1].0 + 1,
            &state.game_data.planets[unowned_candidates[1].0],
            0,
            year,
        ),
    );
    for fleet in state.game_data.fleets.iter_mut() {
        if fleet.owner.0 == 1 {
            fleet.order = nc_data::Order::from_code(9);
        }
    }
    {
        let other_etac = state
            .game_data
            .fleets
            .iter_mut()
            .enumerate()
            .find(|(idx, fleet)| *idx != 0 && fleet.owner.0 == 1 && fleet.ships.etacs > 0)
            .map(|(_, fleet)| fleet)
            .expect("fixture should have a second ETAC fleet");
        other_etac.order = nc_data::Order::ColonizeWorld;
        other_etac.target = nc_data::Coords::from_array(claimed_coords);
    }
    let selected_etac = state
        .game_data
        .fleets
        .iter_mut()
        .find(|fleet| {
            fleet.owner.0 == 1
                && fleet.ships.etacs > 0
                && fleet.order != nc_data::Order::ColonizeWorld
        })
        .expect("fixture should have a selectable ETAC fleet");
    selected_etac.order = nc_data::Order::from_code(0);
    save_runtime_state_with_intel(&fixture_dir, &state, &planet_intel_by_viewer);

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
        apply_action(&mut app, Action::Fleet(FleetAction::OpenMenu)),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::OpenGroupOrder)),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(
            &mut app,
            Action::Fleet(FleetAction::ToggleGroupOrderSelection)
        ),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::OpenMissionPicker)),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(
            &mut app,
            Action::Fleet(FleetAction::AppendMissionPickerChar('1'))
        ),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(
            &mut app,
            Action::Fleet(FleetAction::AppendMissionPickerChar('2'))
        ),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::SubmitMissionPicker)),
        AppOutcome::Continue
    );

    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("colonize target prompt should render");
    assert!(
        line_containing(&terminal, "Target XX [")
            .contains(&format!("Target XX [{:02}] <Q> ->", fallback_coords[0]))
    );
}

#[test]
fn fleet_order_colonize_rejects_duplicate_friendly_target() {
    let fixture_dir = temp_game_copy();
    let mut state = latest_runtime_state(&fixture_dir);
    let viewer_index = 0usize;
    let home_coords = state.game_data.planets[state.game_data.players[0].homeworld as usize - 1]
        .sector
        .to_array();
    let mut unowned_candidates = state
        .game_data
        .planets
        .iter()
        .enumerate()
        .filter(|(_, planet)| planet.owner.0 == 0)
        .map(|(idx, planet)| (idx, planet.sector.to_array()))
        .collect::<Vec<_>>();
    unowned_candidates.sort_by_key(|(_, coords)| {
        let dx = i32::from(home_coords[0]) - i32::from(coords[0]);
        let dy = i32::from(home_coords[1]) - i32::from(coords[1]);
        dx * dx + dy * dy
    });
    let claimed_coords = unowned_candidates[0].1;
    let mut planet_intel_by_viewer = (1..=state.game_data.conquest.player_count)
        .map(|viewer_empire_id| {
            CampaignStore::open_default_in_dir(&fixture_dir)
                .expect("open campaign store")
                .latest_planet_intel_for_viewer(viewer_empire_id)
                .expect("load runtime intel")
                .into_iter()
                .map(|snapshot| (snapshot.planet_record_index_1_based, snapshot))
                .collect::<BTreeMap<_, _>>()
        })
        .collect::<Vec<_>>();
    let year = state.game_data.conquest.game_year;
    planet_intel_by_viewer[viewer_index].insert(
        unowned_candidates[0].0 + 1,
        partial_known_world_snapshot(
            unowned_candidates[0].0 + 1,
            &state.game_data.planets[unowned_candidates[0].0],
            0,
            year,
        ),
    );
    for fleet in state.game_data.fleets.iter_mut() {
        if fleet.owner.0 == 1 {
            fleet.order = nc_data::Order::from_code(9);
        }
    }
    {
        let other_etac = state
            .game_data
            .fleets
            .iter_mut()
            .enumerate()
            .find(|(idx, fleet)| *idx != 0 && fleet.owner.0 == 1 && fleet.ships.etacs > 0)
            .map(|(_, fleet)| fleet)
            .expect("fixture should have a second ETAC fleet");
        other_etac.order = nc_data::Order::ColonizeWorld;
        other_etac.target = nc_data::Coords::from_array(claimed_coords);
    }
    let selected_fleet_number = {
        let selected_etac = state
            .game_data
            .fleets
            .iter_mut()
            .find(|fleet| {
                fleet.owner.0 == 1
                    && fleet.ships.etacs > 0
                    && fleet.order != nc_data::Order::ColonizeWorld
            })
            .expect("fixture should have a selectable ETAC fleet");
        selected_etac.order = nc_data::Order::from_code(0);
        selected_etac.fleet_number.0
    };
    save_runtime_state_with_intel(&fixture_dir, &state, &planet_intel_by_viewer);

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
        apply_action(&mut app, Action::Fleet(FleetAction::OpenMenu)),
        AppOutcome::Continue
    );
    open_order_mission_picker_from_fleet_menu(&mut app, Some(selected_fleet_number));
    assert_eq!(
        apply_action(
            &mut app,
            Action::Fleet(FleetAction::AppendMissionPickerChar('1'))
        ),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(
            &mut app,
            Action::Fleet(FleetAction::AppendMissionPickerChar('2'))
        ),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::SubmitMissionPicker)),
        AppOutcome::Continue
    );
    enter_fleet_order_target(&mut app, claimed_coords);

    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("duplicate colonize validation should render");
    assert!(
        terminal
            .lines
            .iter()
            .any(|line| { line.contains("already ordered to colonize") })
    );
    assert!(
        !terminal
            .lines
            .iter()
            .any(|line| line.contains("Confirm [Y]/N"))
    );

    let state = latest_runtime_state(&fixture_dir);
    let selected_fleet = state
        .game_data
        .fleets
        .iter()
        .find(|fleet| fleet.owner.0 == 1 && fleet.fleet_number.0 == selected_fleet_number)
        .expect("selected fleet should still exist");
    assert_ne!(selected_fleet.order, nc_data::Order::ColonizeWorld);
}

#[test]
fn fleet_group_colonize_rejects_multiple_selected_etacs_for_one_target() {
    let fixture_dir = temp_game_copy();
    let mut state = latest_runtime_state(&fixture_dir);
    let (target_index, target) = state
        .game_data
        .planets
        .iter()
        .enumerate()
        .find(|(_, planet)| planet.owner.0 == 0)
        .map(|(idx, planet)| (idx, planet.sector.to_array()))
        .expect("fixture should have an unowned planet");
    let mut planet_intel_by_viewer = (1..=state.game_data.conquest.player_count)
        .map(|viewer_empire_id| {
            CampaignStore::open_default_in_dir(&fixture_dir)
                .expect("open campaign store")
                .latest_planet_intel_for_viewer(viewer_empire_id)
                .expect("load runtime intel")
                .into_iter()
                .map(|snapshot| (snapshot.planet_record_index_1_based, snapshot))
                .collect::<BTreeMap<_, _>>()
        })
        .collect::<Vec<_>>();
    planet_intel_by_viewer[0].insert(
        target_index + 1,
        partial_known_world_snapshot(
            target_index + 1,
            &state.game_data.planets[target_index],
            0,
            state.game_data.conquest.game_year,
        ),
    );
    for fleet in state.game_data.fleets.iter_mut() {
        if fleet.owner.0 == 1 {
            fleet.order = nc_data::Order::from_code(9);
        }
    }
    let owned_fleet_indexes = state
        .game_data
        .fleets
        .iter()
        .enumerate()
        .filter(|(_, fleet)| fleet.owner.0 == 1)
        .map(|(idx, _)| idx)
        .take(2)
        .collect::<Vec<_>>();
    for idx in &owned_fleet_indexes {
        state.game_data.fleets[*idx].ships.etacs = 1;
        state.game_data.fleets[*idx].order = nc_data::Order::from_code(0);
    }
    save_runtime_state_with_intel(&fixture_dir, &state, &planet_intel_by_viewer);

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
        apply_action(&mut app, Action::Fleet(FleetAction::OpenMenu)),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::OpenGroupOrder)),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(
            &mut app,
            Action::Fleet(FleetAction::ToggleGroupOrderSelection)
        ),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::MoveGroupOrder(1))),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(
            &mut app,
            Action::Fleet(FleetAction::ToggleGroupOrderSelection)
        ),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::OpenMissionPicker)),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(
            &mut app,
            Action::Fleet(FleetAction::AppendMissionPickerChar('1'))
        ),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(
            &mut app,
            Action::Fleet(FleetAction::AppendMissionPickerChar('2'))
        ),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::SubmitMissionPicker)),
        AppOutcome::Continue
    );
    enter_fleet_group_order_target(&mut app, target);

    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("group duplicate colonize validation should render");
    assert!(terminal.lines.iter().any(|line| {
        line.contains("You cannot order multiple ETAC fleets to colonize the same world.")
    }));
    assert!(
        !terminal
            .lines
            .iter()
            .any(|line| line.contains("Confirm [Y]/N"))
    );

    let state = latest_runtime_state(&fixture_dir);
    assert_eq!(
        state
            .game_data
            .fleets
            .iter()
            .filter(|fleet| {
                fleet.owner.0 == 1
                    && fleet.order == nc_data::Order::ColonizeWorld
                    && fleet.target.to_array() == target
            })
            .count(),
        0
    );
}

#[test]
fn fleet_group_colonize_mission_allows_hidden_colonized_worlds_as_targets() {
    let fixture_dir = temp_game_copy();
    let mut state = latest_runtime_state(&fixture_dir);
    let home_coords = state.game_data.planets[state.game_data.players[0].homeworld as usize - 1]
        .sector
        .to_array();
    let mut candidates = state
        .game_data
        .planets
        .iter()
        .enumerate()
        .filter(|(_, planet)| planet.owner.0 == 0)
        .map(|(idx, planet)| (idx, planet.sector.to_array()))
        .collect::<Vec<_>>();
    candidates.sort_by_key(|(_, coords)| {
        let dx = i32::from(home_coords[0]) - i32::from(coords[0]);
        let dy = i32::from(home_coords[1]) - i32::from(coords[1]);
        dx * dx + dy * dy
    });
    let hidden_colonized_idx = candidates[0].0;
    let hidden_colonized_coords = candidates[0].1;
    state.game_data.planets[hidden_colonized_idx].owner.0 = 2;
    let mut planet_intel_by_viewer = (1..=state.game_data.conquest.player_count)
        .map(|viewer_empire_id| {
            CampaignStore::open_default_in_dir(&fixture_dir)
                .expect("open campaign store")
                .latest_planet_intel_for_viewer(viewer_empire_id)
                .expect("load runtime intel")
                .into_iter()
                .map(|snapshot| (snapshot.planet_record_index_1_based, snapshot))
                .collect::<BTreeMap<_, _>>()
        })
        .collect::<Vec<_>>();
    planet_intel_by_viewer[0].remove(&(hidden_colonized_idx + 1));
    for fleet in state.game_data.fleets.iter_mut() {
        if fleet.owner.0 == 1 {
            fleet.order = nc_data::Order::from_code(9);
        }
    }
    let selected_etac = state
        .game_data
        .fleets
        .iter_mut()
        .find(|fleet| fleet.owner.0 == 1 && fleet.ships.etacs > 0)
        .expect("fixture should have a selectable ETAC fleet");
    selected_etac.order = nc_data::Order::from_code(0);
    save_runtime_state_with_intel(&fixture_dir, &state, &planet_intel_by_viewer);

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
        apply_action(&mut app, Action::Fleet(FleetAction::OpenMenu)),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::OpenGroupOrder)),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(
            &mut app,
            Action::Fleet(FleetAction::ToggleGroupOrderSelection)
        ),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::OpenMissionPicker)),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(
            &mut app,
            Action::Fleet(FleetAction::AppendMissionPickerChar('1'))
        ),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(
            &mut app,
            Action::Fleet(FleetAction::AppendMissionPickerChar('2'))
        ),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::SubmitMissionPicker)),
        AppOutcome::Continue
    );

    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("colonize target prompt should render");
    assert!(line_containing(&terminal, "Target XX [").contains(&format!(
        "Target XX [{:02}] <Q> ->",
        hidden_colonized_coords[0]
    )));
}

#[test]
fn fleet_order_colonize_defaults_to_hidden_unknown_owner_worlds_in_planet_database() {
    let fixture_dir = temp_game_copy();
    let mut state = latest_runtime_state(&fixture_dir);
    let home_coords = state.game_data.planets[state.game_data.players[0].homeworld as usize - 1]
        .sector
        .to_array();
    let mut candidates = state
        .game_data
        .planets
        .iter()
        .enumerate()
        .filter(|(_, planet)| planet.owner.0 == 0)
        .map(|(idx, planet)| (idx, planet.sector.to_array()))
        .collect::<Vec<_>>();
    candidates.sort_by_key(|(_, coords)| {
        let dx = i32::from(home_coords[0]) - i32::from(coords[0]);
        let dy = i32::from(home_coords[1]) - i32::from(coords[1]);
        dx * dx + dy * dy
    });
    let hidden_target = candidates[0].1;
    let hidden_target_index = candidates[0].0;
    let mut planet_intel_by_viewer = (1..=state.game_data.conquest.player_count)
        .map(|viewer_empire_id| {
            CampaignStore::open_default_in_dir(&fixture_dir)
                .expect("open campaign store")
                .latest_planet_intel_for_viewer(viewer_empire_id)
                .expect("load runtime intel")
                .into_iter()
                .map(|snapshot| (snapshot.planet_record_index_1_based, snapshot))
                .collect::<BTreeMap<_, _>>()
        })
        .collect::<Vec<_>>();
    planet_intel_by_viewer[0].remove(&(hidden_target_index + 1));
    for fleet in state.game_data.fleets.iter_mut() {
        if fleet.owner.0 == 1 {
            fleet.order = nc_data::Order::from_code(9);
        }
    }
    let selected_etac = state
        .game_data
        .fleets
        .iter_mut()
        .find(|fleet| fleet.owner.0 == 1 && fleet.ships.etacs > 0)
        .expect("fixture should have a selectable ETAC fleet");
    let selected_fleet_number = selected_etac.fleet_number.0;
    selected_etac.order = nc_data::Order::from_code(0);
    save_runtime_state_with_intel(&fixture_dir, &state, &planet_intel_by_viewer);

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
        apply_action(&mut app, Action::Fleet(FleetAction::OpenMenu)),
        AppOutcome::Continue
    );
    open_order_mission_picker_from_fleet_menu(&mut app, Some(selected_fleet_number));
    assert_eq!(
        apply_action(
            &mut app,
            Action::Fleet(FleetAction::AppendMissionPickerChar('1'))
        ),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(
            &mut app,
            Action::Fleet(FleetAction::AppendMissionPickerChar('2'))
        ),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::SubmitMissionPicker)),
        AppOutcome::Continue
    );

    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("single-fleet colonize target prompt should render");
    assert!(
        line_containing(&terminal, "Target XX [")
            .contains(&format!("Target XX [{:02}] <Q> ->", hidden_target[0]))
    );
}

#[test]
fn fleet_group_bombard_shows_no_default_when_enemy_worlds_are_unknown_in_database() {
    let fixture_dir = temp_game_copy();
    let state = latest_runtime_state(&fixture_dir);
    let mut planet_intel_by_viewer = (1..=state.game_data.conquest.player_count)
        .map(|viewer_empire_id| {
            CampaignStore::open_default_in_dir(&fixture_dir)
                .expect("open campaign store")
                .latest_planet_intel_for_viewer(viewer_empire_id)
                .expect("load runtime intel")
                .into_iter()
                .map(|snapshot| (snapshot.planet_record_index_1_based, snapshot))
                .collect::<BTreeMap<_, _>>()
        })
        .collect::<Vec<_>>();
    planet_intel_by_viewer[0].clear();
    save_runtime_state_with_intel(&fixture_dir, &state, &planet_intel_by_viewer);

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
        apply_action(&mut app, Action::Fleet(FleetAction::OpenMenu)),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::OpenGroupOrder)),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(
            &mut app,
            Action::Fleet(FleetAction::ToggleGroupOrderSelection)
        ),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::OpenMissionPicker)),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(
            &mut app,
            Action::Fleet(FleetAction::AppendMissionPickerChar('6'))
        ),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::SubmitMissionPicker)),
        AppOutcome::Continue
    );

    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("bombard target prompt should render");
    let prompt = line_containing(&terminal, "Target XX ");
    assert!(prompt.contains("Target XX "));
    assert!(!prompt.contains('['));
}

#[test]
fn fleet_mission_picker_rejects_missions_not_supported_by_all_selected_fleets() {
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
        apply_action(&mut app, Action::Fleet(FleetAction::OpenMenu)),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::OpenGroupOrder)),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(
            &mut app,
            Action::Fleet(FleetAction::ToggleGroupOrderSelection)
        ),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::MoveGroupOrder(6))),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(
            &mut app,
            Action::Fleet(FleetAction::ToggleGroupOrderSelection)
        ),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::OpenMissionPicker)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FleetMissionPicker);
    assert_eq!(
        apply_action(
            &mut app,
            Action::Fleet(FleetAction::AppendMissionPickerChar('1'))
        ),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(
            &mut app,
            Action::Fleet(FleetAction::AppendMissionPickerChar('0'))
        ),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::SubmitMissionPicker)),
        AppOutcome::Continue
    );

    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("disabled mission rejection should render");
    assert!(line_containing(&terminal, "COMMAND <- ? <Q>").contains("COMMAND"));
    assert!(
        terminal
            .lines
            .iter()
            .all(|line| !line.contains("That mission does not apply to all selected fleets."))
    );
}

#[test]
fn fleet_group_order_rejects_empty_sector_for_world_targeting_mission() {
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
        apply_action(&mut app, Action::Fleet(FleetAction::OpenMenu)),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::OpenGroupOrder)),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(
            &mut app,
            Action::Fleet(FleetAction::ToggleGroupOrderSelection)
        ),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::OpenMissionPicker)),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(
            &mut app,
            Action::Fleet(FleetAction::AppendMissionPickerChar('9'))
        ),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::SubmitMissionPicker)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FleetGroupOrder);
    enter_fleet_group_order_target(&mut app, [1, 1]);

    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("world-target validation should render");
    assert!(terminal.lines.iter().any(|line| {
        line.contains("That mission requires a system with a planet at the target coordinates.")
    }));
    assert_eq!(app.current_screen(), ScreenId::FleetGroupOrder);
    let prompt = line_containing(&terminal, "Target XX ");
    assert!(prompt.contains("Target XX "));
    assert!(
        !terminal
            .lines
            .iter()
            .any(|line| line.contains("Target YY "))
    );
}

#[test]
fn fleet_group_order_allows_owned_planet_for_blockade_mission() {
    let fixture_dir = temp_game_copy();
    let mut state = latest_runtime_state(&fixture_dir);
    let target_idx = state
        .game_data
        .planets
        .iter()
        .enumerate()
        .find(|(_, planet)| planet.owner.0 != 1)
        .map(|(idx, _)| idx)
        .expect("fixture should have a foreign world");
    state.game_data.planets[target_idx].owner.0 = 1;
    let owned_target = state.game_data.planets[target_idx].sector.to_array();
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
        apply_action(&mut app, Action::Fleet(FleetAction::OpenMenu)),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::OpenGroupOrder)),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(
            &mut app,
            Action::Fleet(FleetAction::ToggleGroupOrderSelection)
        ),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::OpenMissionPicker)),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(
            &mut app,
            Action::Fleet(FleetAction::AppendMissionPickerChar('5'))
        ),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::SubmitMissionPicker)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FleetGroupOrder);
    enter_fleet_group_order_target(&mut app, owned_target);
    confirm_fleet_group_order(&mut app, true);

    let state = latest_runtime_state(&fixture_dir);
    let ordered_fleet = state
        .game_data
        .fleets
        .iter()
        .find(|fleet| {
            fleet.owner.0 == 1
                && fleet.order == nc_data::Order::GuardBlockadeWorld
                && fleet.target.to_array() == owned_target
        })
        .expect("one selected fleet should accept an owned blockade target");
    assert_eq!(ordered_fleet.order, nc_data::Order::GuardBlockadeWorld);
    assert_eq!(ordered_fleet.target.to_array(), owned_target);
}

#[test]
fn fleet_order_blockade_mission_defaults_to_closest_owned_planet() {
    let fixture_dir = temp_game_copy();
    let mut state = latest_runtime_state(&fixture_dir);
    let target_idx = state
        .game_data
        .planets
        .iter()
        .enumerate()
        .find(|(_, planet)| planet.owner.0 != 1)
        .map(|(idx, _)| idx)
        .expect("fixture should have a non-owned world");
    state.game_data.planets[target_idx].owner.0 = 1;
    let owned_target = state.game_data.planets[target_idx].sector.to_array();
    let selected_fleet = state
        .game_data
        .fleets
        .iter_mut()
        .find(|fleet| fleet.owner.0 == 1 && fleet.fleet_number.0 == 1)
        .expect("player 1 fleet #1 should exist");
    selected_fleet.sector = nc_data::Coords::from_array(owned_target);
    selected_fleet.target = nc_data::Coords::from_array(owned_target);
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
        apply_action(&mut app, Action::Fleet(FleetAction::OpenMenu)),
        AppOutcome::Continue
    );
    open_order_mission_picker_from_fleet_menu(&mut app, Some(1));
    assert_eq!(
        apply_action(
            &mut app,
            Action::Fleet(FleetAction::AppendMissionPickerChar('5'))
        ),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::SubmitMissionPicker)),
        AppOutcome::Continue
    );

    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("blockade target prompt should render");
    assert!(
        line_containing(&terminal, "Target XX [")
            .contains(&format!("Target XX [{:02}] <Q> ->", owned_target[0]))
    );
}

#[test]
fn fleet_group_order_rejects_owned_planet_for_scout_mission() {
    let fixture_dir = temp_game_copy();
    let mut state = latest_runtime_state(&fixture_dir);
    for fleet in state.game_data.fleets.iter_mut() {
        if fleet.owner.0 == 1 {
            fleet.order = nc_data::Order::from_code(9);
        }
    }
    let scout_fleet = state
        .game_data
        .fleets
        .get_mut(0)
        .expect("fleet 1 should exist");
    scout_fleet.ships.scouts = 1;
    scout_fleet.order = nc_data::Order::from_code(0);
    let enemy_idx = state
        .game_data
        .planets
        .iter()
        .enumerate()
        .find(|(_, planet)| planet.owner.0 != 1)
        .map(|(idx, _)| idx)
        .expect("fixture should have a foreign world");
    state.game_data.planets[enemy_idx].owner.0 = 1;
    let owned_target = state.game_data.planets[enemy_idx].sector.to_array();
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
        apply_action(&mut app, Action::Fleet(FleetAction::OpenMenu)),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::OpenGroupOrder)),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(
            &mut app,
            Action::Fleet(FleetAction::ToggleGroupOrderSelection)
        ),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::OpenMissionPicker)),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(
            &mut app,
            Action::Fleet(FleetAction::AppendMissionPickerChar('1'))
        ),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(
            &mut app,
            Action::Fleet(FleetAction::AppendMissionPickerChar('0'))
        ),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::SubmitMissionPicker)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FleetGroupOrder);
    enter_fleet_group_order_target(&mut app, owned_target);

    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("owned scout target rejection should render");
    assert!(terminal.lines.iter().any(|line| {
        line.contains("You cannot order scouts to target your own planet or system.")
    }));
    let prompt = line_containing(&terminal, "Target XX ");
    assert!(prompt.contains("Target XX "));
    assert!(
        !terminal
            .lines
            .iter()
            .any(|line| line.contains("Target YY "))
    );
}

#[test]
fn fleet_order_rejects_owned_planet_for_scout_system_mission() {
    let fixture_dir = temp_game_copy();
    let mut state = latest_runtime_state(&fixture_dir);
    for fleet in state.game_data.fleets.iter_mut() {
        if fleet.owner.0 == 1 {
            fleet.order = nc_data::Order::from_code(9);
        }
    }
    let scout_fleet = state
        .game_data
        .fleets
        .get_mut(0)
        .expect("fleet 1 should exist");
    scout_fleet.ships.scouts = 1;
    scout_fleet.order = nc_data::Order::from_code(0);
    let owned_target = state.game_data.planets[state.game_data.players[0].homeworld as usize - 1]
        .sector
        .to_array();
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
        apply_action(&mut app, Action::Fleet(FleetAction::OpenMenu)),
        AppOutcome::Continue
    );
    open_order_mission_picker_from_fleet_menu(&mut app, Some(1));
    assert_eq!(
        apply_action(
            &mut app,
            Action::Fleet(FleetAction::AppendMissionPickerChar('1'))
        ),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(
            &mut app,
            Action::Fleet(FleetAction::AppendMissionPickerChar('1'))
        ),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::SubmitMissionPicker)),
        AppOutcome::Continue
    );
    enter_fleet_order_target(&mut app, owned_target);

    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("owned scout-system target rejection should render");
    assert!(
        terminal
            .lines
            .iter()
            .any(|line| { line.contains("You cannot scout your own planet or system.") })
    );
    let prompt = line_containing(&terminal, "Target XX ");
    assert!(prompt.contains("Target XX "));
    assert!(
        !terminal
            .lines
            .iter()
            .any(|line| line.contains("Target YY "))
    );
}

#[test]
fn fleet_order_salvage_defaults_to_closest_owned_planet() {
    let fixture_dir = temp_game_copy();
    let mut state = latest_runtime_state(&fixture_dir);
    let extra_owned_idx = state
        .game_data
        .planets
        .iter()
        .enumerate()
        .find(|(_, planet)| planet.owner.0 != 1)
        .map(|(idx, _)| idx)
        .expect("fixture should have a non-owned planet");
    state.game_data.planets[extra_owned_idx].owner.0 = 1;
    let nearest_owned = state.game_data.planets[extra_owned_idx].sector.to_array();
    let selected_fleet = state
        .game_data
        .fleets
        .iter_mut()
        .find(|fleet| fleet.owner.0 == 1 && fleet.fleet_number.0 == 1)
        .expect("player 1 fleet #1 should exist");
    selected_fleet.sector = nc_data::Coords::from_array(nearest_owned);
    selected_fleet.target = nc_data::Coords::from_array(nearest_owned);
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
        apply_action(&mut app, Action::Fleet(FleetAction::OpenMenu)),
        AppOutcome::Continue
    );
    open_order_mission_picker_from_fleet_menu(&mut app, Some(1));
    assert_eq!(
        apply_action(
            &mut app,
            Action::Fleet(FleetAction::AppendMissionPickerChar('1'))
        ),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(
            &mut app,
            Action::Fleet(FleetAction::AppendMissionPickerChar('5'))
        ),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::SubmitMissionPicker)),
        AppOutcome::Continue
    );

    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("salvage target prompt should render");
    assert!(
        line_containing(&terminal, "Target XX [")
            .contains(&format!("Target XX [{:02}] <Q> ->", nearest_owned[0]))
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::SubmitOrder)),
        AppOutcome::Continue
    );
    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("salvage target y prompt should render");
    assert!(
        line_containing(&terminal, "Target YY [")
            .contains(&format!("Target YY [{:02}] <Q> ->", nearest_owned[1]))
    );
}

#[test]
fn fleet_order_patrol_defaults_x_and_y_to_current_sector() {
    let fixture_dir = temp_game_copy();
    let state = latest_runtime_state(&fixture_dir);
    let patrol_fleet = state
        .game_data
        .fleets
        .iter()
        .find(|fleet| fleet.owner.0 == 1)
        .expect("fixture should have a player fleet");
    let patrol_coords = patrol_fleet.sector.to_array();
    let patrol_number = patrol_fleet.fleet_number.0;

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
        apply_action(&mut app, Action::Fleet(FleetAction::OpenMenu)),
        AppOutcome::Continue
    );
    open_order_mission_picker_from_fleet_menu(&mut app, Some(patrol_number));
    assert_eq!(
        apply_action(
            &mut app,
            Action::Fleet(FleetAction::AppendMissionPickerChar('3'))
        ),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::SubmitMissionPicker)),
        AppOutcome::Continue
    );

    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("patrol target x prompt should render");
    assert!(
        line_containing(&terminal, "Target XX [")
            .contains(&format!("Target XX [{:02}] <Q> ->", patrol_coords[0]))
    );

    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::SubmitOrder)),
        AppOutcome::Continue
    );
    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("patrol target y prompt should render");
    assert!(
        line_containing(&terminal, "Target YY [")
            .contains(&format!("Target YY [{:02}] <Q> ->", patrol_coords[1]))
    );

    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::SubmitOrder)),
        AppOutcome::Continue
    );
    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("patrol confirm should render");
    assert!(line_containing(&terminal, "Confirm [Y]/N").contains("Confirm [Y]/N"));
    assert!(line_containing(&terminal, "New Orders: ").contains("New Orders: Patrol"));
}

#[test]
fn fleet_order_hold_defaults_to_current_sector_and_persists_target() {
    let fixture_dir = temp_game_copy();
    let state = latest_runtime_state(&fixture_dir);
    let hold_fleet = state
        .game_data
        .fleets
        .iter()
        .find(|fleet| fleet.owner.0 == 1 && fleet.fleet_number.0 == 1)
        .expect("fleet #1 should exist");
    let hold_coords = hold_fleet.sector.to_array();
    let fleet_number = hold_fleet.fleet_number.0;

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
        apply_action(&mut app, Action::Fleet(FleetAction::OpenMenu)),
        AppOutcome::Continue
    );
    open_order_mission_picker_from_fleet_menu(&mut app, Some(fleet_number));
    assert_eq!(
        apply_action(
            &mut app,
            Action::Fleet(FleetAction::AppendMissionPickerChar('0'))
        ),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::SubmitMissionPicker)),
        AppOutcome::Continue
    );

    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("hold target x prompt should render");
    assert!(
        line_containing(&terminal, "Target XX [")
            .contains(&format!("Target XX [{:02}] <Q> ->", hold_coords[0]))
    );

    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::SubmitOrder)),
        AppOutcome::Continue
    );
    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("hold target y prompt should render");
    assert!(
        line_containing(&terminal, "Target YY [")
            .contains(&format!("Target YY [{:02}] <Q> ->", hold_coords[1]))
    );

    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::SubmitOrder)),
        AppOutcome::Continue
    );
    confirm_fleet_order(&mut app, true);

    let state = latest_runtime_state(&fixture_dir);
    let ordered_fleet = state
        .game_data
        .fleets
        .iter()
        .find(|fleet| fleet.owner.0 == 1 && fleet.fleet_number.0 == fleet_number)
        .expect("fleet should still exist");
    assert_eq!(ordered_fleet.order.code(), 0);
    assert_eq!(ordered_fleet.target.to_array(), hold_coords);
}

#[test]
fn fleet_list_eta_is_zero_for_hold_orders_even_with_stale_off_sector_target() {
    let fixture_dir = temp_game_copy();
    let mut state = latest_runtime_state(&fixture_dir);
    let fleet = state
        .game_data
        .fleets
        .iter_mut()
        .find(|fleet| fleet.owner.0 == 1 && fleet.fleet_number.0 == 1)
        .expect("fleet #1 should exist");
    fleet.speed = 3;
    fleet.order = nc_data::Order::HoldPosition;
    fleet.target = nc_data::Coords::from_array([14, 9]);
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
    let mut terminal = CaptureTerminal::new();

    advance_to_main_menu(&mut app);
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::OpenMenu)),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::OpenList)),
        AppOutcome::Continue
    );
    app.render(&mut terminal).expect("fleet list should render");
    assert!(
        terminal.lines.iter().any(|line| {
            line.contains("│ 1│") && line.contains("Hold") && line.contains("│  0│")
        }),
        "{:#?}",
        terminal.lines
    );
}

#[test]
fn fleet_list_eta_is_zero_when_non_hold_target_matches_current_location() {
    let fixture_dir = temp_game_copy();
    let mut state = latest_runtime_state(&fixture_dir);
    let fleet = state
        .game_data
        .fleets
        .iter_mut()
        .find(|fleet| fleet.owner.0 == 1 && fleet.fleet_number.0 == 1)
        .expect("fleet #1 should exist");
    let current_coords = fleet.sector.to_array();
    fleet.speed = 3;
    fleet.order = nc_data::Order::PatrolSector;
    fleet.target = nc_data::Coords::from_array(current_coords);
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
    let mut terminal = CaptureTerminal::new();

    advance_to_main_menu(&mut app);
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::OpenMenu)),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::OpenList)),
        AppOutcome::Continue
    );
    app.render(&mut terminal).expect("fleet list should render");
    assert!(
        terminal.lines.iter().any(|line| {
            line.contains("│ 1│")
                && line.contains("Patrol")
                && line.contains(&format!(
                    "({:02},{:02})",
                    current_coords[0], current_coords[1]
                ))
                && line.contains("│  0│")
        }),
        "{:#?}",
        terminal.lines
    );
}

#[test]
fn fleet_order_seek_home_targets_the_nearest_owned_world_without_a_prompt() {
    let fixture_dir = temp_game_copy();
    let (fleet_number, home) = park_player_1_fleet_away(&fixture_dir, 2);
    let mut app = load_player_1_app(&fixture_dir);
    open_order_mission_picker_from_fleet_menu(&mut app, Some(fleet_number));
    submit_mission_code(&mut app, "2");

    // ECGAME asks for no coordinates; the order goes straight to the
    // confirmation.
    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("seek home confirmation should render");
    assert!(
        !terminal
            .lines
            .iter()
            .any(|line| line.contains("Target XX ["))
    );
    confirm_fleet_order(&mut app, true);

    let state = latest_runtime_state(&fixture_dir);
    let ordered_fleet = state
        .game_data
        .fleets
        .iter()
        .find(|fleet| fleet.owner.0 == 1 && fleet.fleet_number.0 == fleet_number)
        .expect("fleet should still exist");
    assert_eq!(ordered_fleet.order.code(), 2);
    assert_eq!(ordered_fleet.target.to_array(), home);
}

#[test]
fn fleet_order_seek_home_aborts_in_the_only_home_system() {
    let fixture_dir = temp_game_copy();
    let mut app = load_player_1_app(&fixture_dir);
    open_order_mission_picker_from_fleet_menu(&mut app, Some(2));
    submit_mission_code(&mut app, "2");

    // ECGAME says why, aborts the order, and changes nothing.
    assert_eq!(app.current_screen(), ScreenId::FleetMenu);
    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal).expect("fleet menu should render");
    assert!(terminal.lines.iter().any(|line| {
        line.contains("The 2nd fleet is already in your only controlled star system.")
    }));
    let state = latest_runtime_state(&fixture_dir);
    let fleet = state
        .game_data
        .fleets
        .iter()
        .find(|fleet| fleet.owner.0 == 1 && fleet.fleet_number.0 == 2)
        .expect("fleet should still exist");
    assert_ne!(fleet.order.code(), 2);
}

/// Player 1's worlds in PLANETS.DAT order: the home world, then the second.
fn player_1_worlds(fixture_dir: &std::path::Path) -> Vec<[u8; 2]> {
    latest_runtime_state(fixture_dir)
        .game_data
        .planets
        .iter()
        .filter(|planet| planet.owner.0 == 1)
        .map(|planet| planet.sector.to_array())
        .collect()
}

/// Order fleet 2, which sits in player 1's home system, to Seek Home, answer
/// ECGAME's question, and confirm; returns the fleet's new target.
fn seek_home_from_the_home_system(fixture_dir: &std::path::Path, answer: Option<char>) -> [u8; 2] {
    let mut app = load_player_1_app(fixture_dir);
    open_order_mission_picker_from_fleet_menu(&mut app, Some(2));
    submit_mission_code(&mut app, "2");

    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("seek home question should render");
    assert!(
        terminal
            .lines
            .iter()
            .any(|line| line.contains("The 2nd fleet is already in a star system you control."))
    );
    assert!(terminal.lines.iter().any(|line| {
        line.contains("Do you wish this fleet to seek the next closest system? [Y]")
    }));
    if let Some(ch) = answer {
        assert_eq!(
            apply_action(&mut app, Action::Fleet(FleetAction::AppendOrderChar(ch))),
            AppOutcome::Continue
        );
    }
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::SubmitOrder)),
        AppOutcome::Continue
    );
    confirm_fleet_order(&mut app, true);

    latest_runtime_state(fixture_dir)
        .game_data
        .fleets
        .iter()
        .find(|fleet| fleet.owner.0 == 1 && fleet.fleet_number.0 == 2)
        .map(|fleet| {
            assert_eq!(fleet.order.code(), 2);
            fleet.target.to_array()
        })
        .expect("fleet should still exist")
}

#[test]
fn fleet_order_seek_home_in_the_home_system_defaults_to_the_next_closest() {
    let fixture_dir = temp_game_copy();
    give_player_1_a_second_world(&fixture_dir);
    let worlds = player_1_worlds(&fixture_dir);
    assert_eq!(
        seek_home_from_the_home_system(&fixture_dir, None),
        worlds[1]
    );
}

#[test]
fn fleet_order_seek_home_in_the_home_system_stays_on_no() {
    let fixture_dir = temp_game_copy();
    give_player_1_a_second_world(&fixture_dir);
    let worlds = player_1_worlds(&fixture_dir);
    assert_eq!(
        seek_home_from_the_home_system(&fixture_dir, Some('n')),
        worlds[0]
    );
}

#[test]
fn fleet_order_salvage_with_one_owned_world_needs_no_target() {
    let fixture_dir = temp_game_copy();
    let (fleet_number, home) = park_player_1_fleet_away(&fixture_dir, 1);
    let mut app = load_player_1_app(&fixture_dir);
    open_order_mission_picker_from_fleet_menu(&mut app, Some(fleet_number));
    submit_mission_code(&mut app, "15");
    confirm_fleet_order(&mut app, true);

    let state = latest_runtime_state(&fixture_dir);
    let ordered_fleet = state
        .game_data
        .fleets
        .iter()
        .find(|fleet| fleet.owner.0 == 1 && fleet.fleet_number.0 == fleet_number)
        .expect("fleet should still exist");
    assert_eq!(ordered_fleet.order.code(), 15);
    assert_eq!(ordered_fleet.target.to_array(), home);
}

/// Move player 1's fleet `fleet_number` to rest at (1,1); returns its number
/// and the player's only world.
fn park_player_1_fleet_away(fixture_dir: &std::path::Path, fleet_number: u16) -> (u16, [u8; 2]) {
    let mut state = latest_runtime_state(fixture_dir);
    let home = state
        .game_data
        .planets
        .iter()
        .find(|planet| planet.owner.0 == 1)
        .map(|planet| planet.sector.to_array())
        .expect("player 1 owns a world");
    let fleet = state
        .game_data
        .fleets
        .iter_mut()
        .find(|fleet| fleet.owner.0 == 1 && fleet.fleet_number.0 == fleet_number)
        .expect("fleet should exist");
    fleet.sector = nc_data::Coords::from_array([1, 1]);
    nc_data::set_fleet_at_rest(fleet);
    save_runtime_state(fixture_dir, &state);
    (fleet_number, home)
}

/// Give player 1 a second world so Salvage asks for coordinates.
fn give_player_1_a_second_world(fixture_dir: &std::path::Path) {
    let mut state = latest_runtime_state(fixture_dir);
    let planet = state
        .game_data
        .planets
        .iter_mut()
        .rev()
        .find(|planet| planet.owner.0 == 0)
        .expect("fixture should have an unowned planet");
    planet.owner.0 = 1;
    save_runtime_state(fixture_dir, &state);
}

fn load_player_1_app(fixture_dir: &std::path::Path) -> App {
    let mut app = App::load(AppConfig {
        game_dir: fixture_dir.to_path_buf(),
        player_record_index_1_based: 1,
        export_root: None,
        queue_dir: None,
        session_timeout_secs: None,
        game_config: Default::default(),
    })
    .expect("app should load");
    advance_to_main_menu(&mut app);
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::OpenMenu)),
        AppOutcome::Continue
    );
    app
}

fn submit_mission_code(app: &mut App, code: &str) {
    for ch in code.chars() {
        assert_eq!(
            apply_action(app, Action::Fleet(FleetAction::AppendMissionPickerChar(ch))),
            AppOutcome::Continue
        );
    }
    assert_eq!(
        apply_action(app, Action::Fleet(FleetAction::SubmitMissionPicker)),
        AppOutcome::Continue
    );
}

#[test]
fn fleet_order_salvage_rejects_empty_sector_target() {
    let fixture_dir = temp_game_copy();
    give_player_1_a_second_world(&fixture_dir);
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
        apply_action(&mut app, Action::Fleet(FleetAction::OpenMenu)),
        AppOutcome::Continue
    );
    open_order_mission_picker_from_fleet_menu(&mut app, Some(1));
    assert_eq!(
        apply_action(
            &mut app,
            Action::Fleet(FleetAction::AppendMissionPickerChar('1'))
        ),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(
            &mut app,
            Action::Fleet(FleetAction::AppendMissionPickerChar('5'))
        ),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::SubmitMissionPicker)),
        AppOutcome::Continue
    );
    enter_fleet_order_target(&mut app, [1, 1]);

    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("salvage empty-sector validation should render");
    assert!(
        line_containing(&terminal, "That mission needs a system with a planet")
            .contains("That mission needs a system with a planet at the target.")
    );
}

#[test]
fn fleet_order_salvage_rejects_foreign_planet_target() {
    let fixture_dir = temp_game_copy();
    let mut state = latest_runtime_state(&fixture_dir);
    let foreign_idx = state
        .game_data
        .planets
        .iter()
        .enumerate()
        .find(|(_, planet)| planet.owner.0 == 0)
        .map(|(idx, _)| idx)
        .expect("fixture should have an unowned planet");
    state.game_data.planets[foreign_idx].owner.0 = 2;
    let mut planet_intel_by_viewer = (1..=state.game_data.conquest.player_count)
        .map(|viewer_empire_id| {
            CampaignStore::open_default_in_dir(&fixture_dir)
                .expect("open campaign store")
                .latest_planet_intel_for_viewer(viewer_empire_id)
                .expect("load runtime intel")
                .into_iter()
                .map(|snapshot| (snapshot.planet_record_index_1_based, snapshot))
                .collect::<BTreeMap<_, _>>()
        })
        .collect::<Vec<_>>();
    planet_intel_by_viewer[0].insert(
        foreign_idx + 1,
        partial_known_world_snapshot(
            foreign_idx + 1,
            &state.game_data.planets[foreign_idx],
            2,
            state.game_data.conquest.game_year,
        ),
    );
    let foreign_target = state.game_data.planets[foreign_idx].sector.to_array();
    save_runtime_state_with_intel(&fixture_dir, &state, &planet_intel_by_viewer);
    give_player_1_a_second_world(&fixture_dir);

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
        apply_action(&mut app, Action::Fleet(FleetAction::OpenMenu)),
        AppOutcome::Continue
    );
    open_order_mission_picker_from_fleet_menu(&mut app, Some(1));
    assert_eq!(
        apply_action(
            &mut app,
            Action::Fleet(FleetAction::AppendMissionPickerChar('1'))
        ),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(
            &mut app,
            Action::Fleet(FleetAction::AppendMissionPickerChar('5'))
        ),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::SubmitMissionPicker)),
        AppOutcome::Continue
    );
    enter_fleet_order_target(&mut app, foreign_target);

    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("salvage foreign-planet validation should render");
    assert!(
        line_containing(
            &terminal,
            "That mission requires one of your owned planets."
        )
        .contains("That mission requires one of your owned planets.")
    );
}

#[test]
fn fleet_order_salvage_rejects_unowned_planet_target() {
    let fixture_dir = temp_game_copy();
    give_player_1_a_second_world(&fixture_dir);
    let state = latest_runtime_state(&fixture_dir);
    let unowned_target = state
        .game_data
        .planets
        .iter()
        .find(|planet| planet.owner.0 == 0)
        .map(|planet| planet.sector.to_array())
        .expect("fixture should have an unowned planet");

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
        apply_action(&mut app, Action::Fleet(FleetAction::OpenMenu)),
        AppOutcome::Continue
    );
    open_order_mission_picker_from_fleet_menu(&mut app, Some(1));
    assert_eq!(
        apply_action(
            &mut app,
            Action::Fleet(FleetAction::AppendMissionPickerChar('1'))
        ),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(
            &mut app,
            Action::Fleet(FleetAction::AppendMissionPickerChar('5'))
        ),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::SubmitMissionPicker)),
        AppOutcome::Continue
    );
    enter_fleet_order_target(&mut app, unowned_target);

    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("salvage unowned-planet validation should render");
    assert!(
        line_containing(
            &terminal,
            "That mission requires one of your owned planets."
        )
        .contains("That mission requires one of your owned planets.")
    );
}

#[test]
fn fleet_group_order_allows_manual_combat_target_without_known_enemy_world() {
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
        apply_action(&mut app, Action::Fleet(FleetAction::OpenMenu)),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::OpenGroupOrder)),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(
            &mut app,
            Action::Fleet(FleetAction::ToggleGroupOrderSelection)
        ),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::OpenMissionPicker)),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(
            &mut app,
            Action::Fleet(FleetAction::AppendMissionPickerChar('5'))
        ),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::SubmitMissionPicker)),
        AppOutcome::Continue
    );

    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("combat target prompt should render");
    assert_eq!(app.current_screen(), ScreenId::FleetGroupOrder);
    let prompt = line_containing(&terminal, "Target XX [");
    assert!(prompt.contains("Target XX ["));
    assert!(!prompt.contains("Notice:"));
}

#[test]
fn fleet_group_order_allows_manual_scout_target_without_known_enemy_world() {
    let fixture_dir = temp_game_copy();
    let mut state = latest_runtime_state(&fixture_dir);
    for fleet in state.game_data.fleets.iter_mut() {
        if fleet.owner.0 == 1 {
            fleet.order = nc_data::Order::from_code(9);
        }
    }
    let scout_fleet = state
        .game_data
        .fleets
        .get_mut(0)
        .expect("fleet 1 should exist");
    scout_fleet.ships.scouts = 1;
    scout_fleet.order = nc_data::Order::from_code(0);
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
        apply_action(&mut app, Action::Fleet(FleetAction::OpenMenu)),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::OpenGroupOrder)),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(
            &mut app,
            Action::Fleet(FleetAction::ToggleGroupOrderSelection)
        ),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::OpenMissionPicker)),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(
            &mut app,
            Action::Fleet(FleetAction::AppendMissionPickerChar('1'))
        ),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(
            &mut app,
            Action::Fleet(FleetAction::AppendMissionPickerChar('0'))
        ),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::SubmitMissionPicker)),
        AppOutcome::Continue
    );

    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("scout target prompt should render");
    assert_eq!(app.current_screen(), ScreenId::FleetGroupOrder);
    let prompt = line_containing(&terminal, "Target XX [");
    assert!(prompt.contains("Target XX ["));
    assert!(!prompt.contains("Notice:"));
}

#[test]
fn fleet_group_patrol_defaults_x_and_y_to_selected_sector() {
    let fixture_dir = temp_game_copy();
    let state = latest_runtime_state(&fixture_dir);
    let patrol_fleet = state
        .game_data
        .fleets
        .iter()
        .find(|fleet| fleet.owner.0 == 1)
        .expect("fixture should have a player fleet");
    let patrol_coords = patrol_fleet.sector.to_array();

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
        apply_action(&mut app, Action::Fleet(FleetAction::OpenMenu)),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::OpenGroupOrder)),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(
            &mut app,
            Action::Fleet(FleetAction::ToggleGroupOrderSelection)
        ),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::OpenMissionPicker)),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(
            &mut app,
            Action::Fleet(FleetAction::AppendMissionPickerChar('3'))
        ),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::SubmitMissionPicker)),
        AppOutcome::Continue
    );

    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("group patrol target x prompt should render");
    assert!(
        line_containing(&terminal, "Target XX [")
            .contains(&format!("Target XX [{:02}] <Q> ->", patrol_coords[0]))
    );

    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::SubmitGroupOrder)),
        AppOutcome::Continue
    );
    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("group patrol target y prompt should render");
    assert!(
        line_containing(&terminal, "Target YY [")
            .contains(&format!("Target YY [{:02}] <Q> ->", patrol_coords[1]))
    );

    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::SubmitGroupOrder)),
        AppOutcome::Continue
    );
    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("group patrol confirm should render");
    assert!(line_containing(&terminal, "Confirm [Y]/N").contains("Confirm [Y]/N"));
    assert!(line_containing(&terminal, "New Orders: ").contains("New Orders: Patrol"));
}

#[test]
fn fleet_group_hold_defaults_to_selected_sector_and_persists_target() {
    let fixture_dir = temp_game_copy();
    let state = latest_runtime_state(&fixture_dir);
    let hold_fleet = state
        .game_data
        .fleets
        .iter()
        .find(|fleet| fleet.owner.0 == 1)
        .expect("fixture should have a player fleet");
    let hold_coords = hold_fleet.sector.to_array();

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
        apply_action(&mut app, Action::Fleet(FleetAction::OpenMenu)),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::OpenGroupOrder)),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(
            &mut app,
            Action::Fleet(FleetAction::ToggleGroupOrderSelection)
        ),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::OpenMissionPicker)),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(
            &mut app,
            Action::Fleet(FleetAction::AppendMissionPickerChar('0'))
        ),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::SubmitMissionPicker)),
        AppOutcome::Continue
    );

    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("group hold target x prompt should render");
    assert!(
        line_containing(&terminal, "Target XX [")
            .contains(&format!("Target XX [{:02}] <Q> ->", hold_coords[0]))
    );

    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::SubmitGroupOrder)),
        AppOutcome::Continue
    );
    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("group hold target y prompt should render");
    assert!(
        line_containing(&terminal, "Target YY [")
            .contains(&format!("Target YY [{:02}] <Q> ->", hold_coords[1]))
    );

    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::SubmitGroupOrder)),
        AppOutcome::Continue
    );
    confirm_fleet_group_order(&mut app, true);

    let state = latest_runtime_state(&fixture_dir);
    assert_eq!(
        state
            .game_data
            .fleets
            .iter()
            .filter(|fleet| {
                fleet.owner.0 == 1
                    && fleet.order.code() == 0
                    && fleet.target.to_array() == hold_coords
            })
            .count(),
        1
    );
}

#[test]
fn fleet_group_seek_home_targets_the_nearest_owned_world_without_a_prompt() {
    let fixture_dir = temp_game_copy();
    let mut home = [0, 0];
    for fleet_number in 1..=4 {
        home = park_player_1_fleet_away(&fixture_dir, fleet_number).1;
    }
    let mut app = load_player_1_app(&fixture_dir);
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::OpenGroupOrder)),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(
            &mut app,
            Action::Fleet(FleetAction::ToggleGroupOrderSelection)
        ),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::OpenMissionPicker)),
        AppOutcome::Continue
    );
    submit_mission_code(&mut app, "2");

    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("group seek home confirmation should render");
    assert!(
        !terminal
            .lines
            .iter()
            .any(|line| line.contains("Target XX ["))
    );
    confirm_fleet_group_order(&mut app, true);

    let state = latest_runtime_state(&fixture_dir);
    let ordered = state
        .game_data
        .fleets
        .iter()
        .filter(|fleet| fleet.owner.0 == 1)
        .map(|fleet| {
            (
                fleet.fleet_number.0,
                fleet.order.code(),
                fleet.target.to_array(),
            )
        })
        .collect::<Vec<_>>();
    assert_eq!(
        ordered
            .iter()
            .filter(|(_, order, target)| *order == 2 && *target == home)
            .count(),
        1,
        "{ordered:?}"
    );
}

#[test]
fn fleet_group_seek_home_of_one_home_fleet_asks_for_the_next_closest() {
    let fixture_dir = temp_game_copy();
    give_player_1_a_second_world(&fixture_dir);
    let worlds = player_1_worlds(&fixture_dir);
    let mut app = load_player_1_app(&fixture_dir);
    for action in [
        FleetAction::OpenGroupOrder,
        FleetAction::ToggleGroupOrderSelection,
        FleetAction::OpenMissionPicker,
    ] {
        assert_eq!(
            apply_action(&mut app, Action::Fleet(action)),
            AppOutcome::Continue
        );
    }
    submit_mission_code(&mut app, "2");

    // ECGAME treats a group order of one fleet as a single order.
    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("group seek home question should render");
    assert!(terminal.lines.iter().any(|line| {
        line.contains("Do you wish this fleet to seek the next closest system? [Y]")
    }));
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::SubmitGroupOrder)),
        AppOutcome::Continue
    );
    confirm_fleet_group_order(&mut app, true);

    let state = latest_runtime_state(&fixture_dir);
    assert!(state.game_data.fleets.iter().any(|fleet| {
        fleet.owner.0 == 1 && fleet.order.code() == 2 && fleet.target.to_array() == worlds[1]
    }));
}

#[test]
fn fleet_group_order_applies_move_order_to_selected_fleets() {
    let fixture_dir = temp_game_copy();
    let mut state = latest_runtime_state(&fixture_dir);
    for fleet in state.game_data.fleets.iter_mut() {
        if fleet.owner.0 == 1 {
            fleet.order = nc_data::Order::from_code(9);
        }
    }
    state.game_data.fleets[0].order = nc_data::Order::from_code(0);
    state.game_data.fleets[1].order = nc_data::Order::from_code(0);
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
        apply_action(&mut app, Action::Fleet(FleetAction::OpenMenu)),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::OpenGroupOrder)),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(
            &mut app,
            Action::Fleet(FleetAction::ToggleGroupOrderSelection)
        ),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::MoveGroupOrder(1))),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(
            &mut app,
            Action::Fleet(FleetAction::ToggleGroupOrderSelection)
        ),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::OpenMissionPicker)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FleetMissionPicker);
    assert_eq!(
        apply_action(
            &mut app,
            Action::Fleet(FleetAction::AppendMissionPickerChar('1'))
        ),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::SubmitMissionPicker)),
        AppOutcome::Continue
    );
    enter_fleet_group_order_target(&mut app, [10, 13]);
    confirm_fleet_group_order(&mut app, true);
    assert_eq!(app.current_screen(), ScreenId::FleetGroupOrder);

    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("fleet group order should render normal command line");
    assert!(line_containing(&terminal, "COMMAND <- ? SPACE <Q>").contains("COMMAND"));
    assert!(!terminal.lines.iter().any(|line| line.contains("Applied ")));
    assert!(
        !terminal
            .lines
            .iter()
            .any(|line| line.contains("Selected fleets: "))
    );

    let state = latest_runtime_state(&fixture_dir);
    assert_eq!(state.game_data.fleets[0].order.code(), 1);
    assert_eq!(state.game_data.fleets[0].target.to_array(), [10, 13]);
    assert_eq!(state.game_data.fleets[1].order.code(), 1);
    assert_eq!(state.game_data.fleets[1].target.to_array(), [10, 13]);
}

#[test]
fn fleet_group_order_accepts_join_fleet_mission_number() {
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
        apply_action(&mut app, Action::Fleet(FleetAction::OpenMenu)),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::OpenGroupOrder)),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(
            &mut app,
            Action::Fleet(FleetAction::ToggleGroupOrderSelection)
        ),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::OpenMissionPicker)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FleetMissionPicker);
    assert_eq!(
        apply_action(
            &mut app,
            Action::Fleet(FleetAction::AppendMissionPickerChar('1'))
        ),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(
            &mut app,
            Action::Fleet(FleetAction::AppendMissionPickerChar('3'))
        ),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::SubmitMissionPicker)),
        AppOutcome::Continue
    );

    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("fleet group join target prompt should render");
    assert_eq!(app.current_screen(), ScreenId::FleetGroupOrder);
    let selected = line_containing(&terminal, "Selected fleets: ")
        .trim()
        .strip_prefix("Selected fleets: ")
        .expect("selected fleets line should have prefix");
    assert_eq!(selected.split(", ").count(), 1);
    assert!(selected.len() >= 2);
    assert!(selected.chars().all(|ch| ch.is_ascii_digit()));
    assert!(
        line_containing(
            &terminal,
            "Enter the host fleet number for Join another fleet."
        )
        .contains("Enter the host fleet number for Join another fleet.")
    );
    assert!(
        !terminal
            .lines
            .iter()
            .any(|line| line.contains("Enter target for new order: Join fleet"))
    );
    assert!(
        line_containing(&terminal, "Fleet # [").contains("Fleet # ["),
        "{}",
        line_containing(&terminal, "Fleet # [")
    );

    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::SubmitGroupOrder)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FleetGroupOrder);

    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("fleet group join should return to normal command line");
    assert!(line_containing(&terminal, "COMMAND <- ? SPACE <Q>").contains("COMMAND"));
    assert!(!terminal.lines.iter().any(|line| line.contains("Applied ")));
    assert!(
        !terminal
            .lines
            .iter()
            .any(|line| line.contains("Selected fleets: "))
    );

    let state = latest_runtime_state(&fixture_dir);
    let joined_fleets = state
        .game_data
        .fleets
        .iter()
        .filter(|fleet| fleet.owner.0 == 1 && fleet.order.code() == 13)
        .collect::<Vec<_>>();
    assert_eq!(joined_fleets.len(), 1);
    let ordered_fleet = joined_fleets[0];
    assert_eq!(ordered_fleet.order.code(), 13);
    assert_ne!(ordered_fleet.mission_target, 0);
    assert_ne!(ordered_fleet.mission_target, ordered_fleet.fleet_number.0);
    assert_eq!(ordered_fleet.target.to_array(), [16, 13]);
}

#[test]
fn fleet_group_order_persists_rendezvous_target_for_selected_fleets() {
    let fixture_dir = temp_game_copy();
    let state = latest_runtime_state(&fixture_dir);
    let rendezvous_target = state.game_data.fleets[0].sector.to_array();
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
        apply_action(&mut app, Action::Fleet(FleetAction::OpenMenu)),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::OpenGroupOrder)),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(
            &mut app,
            Action::Fleet(FleetAction::ToggleGroupOrderSelection)
        ),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::MoveGroupOrder(1))),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(
            &mut app,
            Action::Fleet(FleetAction::ToggleGroupOrderSelection)
        ),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::OpenMissionPicker)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FleetMissionPicker);
    assert_eq!(
        apply_action(
            &mut app,
            Action::Fleet(FleetAction::AppendMissionPickerChar('1'))
        ),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(
            &mut app,
            Action::Fleet(FleetAction::AppendMissionPickerChar('4'))
        ),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::SubmitMissionPicker)),
        AppOutcome::Continue
    );

    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("fleet group rendezvous target prompt should render");
    assert!(
        line_containing(&terminal, "Target XX [")
            .contains(&format!("Target XX [{:02}] <Q> ->", rendezvous_target[0]))
    );

    enter_fleet_group_order_target(&mut app, rendezvous_target);
    confirm_fleet_group_order(&mut app, true);
    assert_eq!(app.current_screen(), ScreenId::FleetGroupOrder);

    let state = latest_runtime_state(&fixture_dir);
    assert_eq!(
        state
            .game_data
            .fleets
            .iter()
            .filter(|fleet| {
                fleet.owner.0 == 1
                    && fleet.order == nc_data::Order::RendezvousSector
                    && fleet.target.to_array() == rendezvous_target
            })
            .count(),
        2
    );
}

#[test]
fn fleet_group_guard_starbase_target_prompt_uses_named_target_layout() {
    let fixture_dir = temp_game_with_starbase_copy();
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
        apply_action(&mut app, Action::Fleet(FleetAction::OpenMenu)),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::OpenGroupOrder)),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(
            &mut app,
            Action::Fleet(FleetAction::ToggleGroupOrderSelection)
        ),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::OpenMissionPicker)),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(
            &mut app,
            Action::Fleet(FleetAction::AppendMissionPickerChar('4'))
        ),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::SubmitMissionPicker)),
        AppOutcome::Continue
    );

    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("fleet group guard-starbase target prompt should render");
    assert_eq!(app.current_screen(), ScreenId::FleetGroupOrder);
    let selected = line_containing(&terminal, "Selected fleets: ")
        .trim()
        .strip_prefix("Selected fleets: ")
        .expect("selected fleets line should have prefix");
    assert_eq!(selected.split(", ").count(), 1);
    assert!(selected.len() >= 2);
    assert!(selected.chars().all(|ch| ch.is_ascii_digit()));
    assert!(
        line_containing(&terminal, "Enter the starbase number for Guard a Starbase.")
            .contains("Enter the starbase number for Guard a Starbase.")
    );
    assert!(
        !terminal
            .lines
            .iter()
            .any(|line| line.contains("Enter target for new order: Guard starbase"))
    );
    assert!(
        line_containing(&terminal, "Starbase # [").contains("Starbase # ["),
        "{}",
        line_containing(&terminal, "Starbase # [")
    );
}

#[test]
fn fleet_change_roe_accepts_typed_fleet_selection_and_q_cancels_prompt() {
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
        apply_action(&mut app, Action::Fleet(FleetAction::OpenMenu)),
        AppOutcome::Continue
    );
    open_change_value_prompt_from_fleet_menu(&mut app, Some(4), 'R');
    assert_eq!(
        app.handle_key(key(KeyCode::Char('q'))),
        Action::Fleet(FleetAction::CancelMenuPrompt)
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::CancelMenuPrompt)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FleetMenu);

    open_change_value_prompt_from_fleet_menu(&mut app, Some(4), 'R');
    submit_fleet_menu_prompt_value(&mut app, "7");
    assert_eq!(app.current_fleet_roe_by_id(4), Some(7));
    assert_eq!(app.current_fleet_roe_by_id(1), Some(6));
}

#[test]
fn fleet_change_field_prompt_uses_angle_bracket_commands_and_default() {
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

    advance_to_main_menu(&mut app);
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::OpenMenu)),
        AppOutcome::Continue
    );
    open_change_field_prompt_from_fleet_menu(&mut app, Some(4));

    app.render(&mut terminal).expect("fleet menu should render");
    assert!(
        line_containing(&terminal, "COMMAND <- Change")
            .contains("Change <R>OE, <I>D, or <S>peed [R] <Q> ->")
    );
}

#[test]
fn fleet_change_roe_empty_enter_accepts_displayed_default() {
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
        apply_action(&mut app, Action::Fleet(FleetAction::OpenMenu)),
        AppOutcome::Continue
    );
    open_change_value_prompt_from_fleet_menu(&mut app, Some(4), 'R');
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::SubmitMenuPrompt)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FleetMenu);
    assert_eq!(app.current_fleet_roe_by_id(4), Some(6));
}

#[test]
fn fleet_change_success_returns_to_menu_with_notice() {
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

    advance_to_main_menu(&mut app);
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::OpenMenu)),
        AppOutcome::Continue
    );
    open_change_value_prompt_from_fleet_menu(&mut app, Some(4), 'R');
    submit_fleet_menu_prompt_value(&mut app, "9");

    app.render(&mut terminal).expect("render succeeds");
    assert!(
        line_containing(&terminal, "Fleet #4 ROE set to 9.").contains("Fleet #4 ROE set to 9.")
    );
}

#[test]
fn fleet_change_roe_rejects_support_only_fleet_with_updated_message() {
    let fixture_dir = temp_game_copy();
    let mut state = latest_runtime_state(&fixture_dir);
    let fleet = state
        .game_data
        .fleets
        .iter_mut()
        .find(|fleet| fleet.owner.0 == 1 && fleet.fleet_number.0 == 4)
        .expect("player 1 fleet #4 should exist");
    fleet.ships.destroyers = 0;
    fleet.ships.cruisers = 0;
    fleet.ships.battleships = 0;
    fleet.ships.scouts = 1;
    nc_data::set_troop_transports(fleet, 0);
    nc_data::set_armies_aboard(fleet, 0);
    fleet.ships.etacs = 0;
    nc_data::recompute_max_speed(fleet);
    fleet.speed = 0;
    fleet.rules_of_engagement = 0;
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
    let mut terminal = CaptureTerminal::new();

    advance_to_main_menu(&mut app);
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::OpenMenu)),
        AppOutcome::Continue
    );
    open_change_value_prompt_from_fleet_menu(&mut app, Some(4), 'R');
    submit_fleet_menu_prompt_value(&mut app, "6");

    app.render(&mut terminal).expect("render succeeds");
    assert!(
        line_containing(&terminal, "Support-only fleets must use ROE 0.")
            .contains("Support-only fleets must use ROE 0.")
    );
    assert_eq!(app.current_fleet_roe_by_id(4), Some(0));
}

#[test]
fn fleet_list_checked_change_clears_only_successful_fleets_on_partial_roe_update() {
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

    {
        let combat = app
            .game_data
            .fleets
            .iter_mut()
            .find(|fleet| fleet.owner.0 == 1 && fleet.fleet_number.0 == 4)
            .expect("player 1 fleet #4 should exist");
        combat.ships.destroyers = 1;
        combat.ships.cruisers = 0;
        combat.ships.battleships = 0;
        combat.ships.scouts = 0;
        nc_data::set_troop_transports(combat, 0);
        nc_data::set_armies_aboard(combat, 0);
        combat.ships.etacs = 0;
        nc_data::recompute_max_speed(combat);
        combat.rules_of_engagement = 0;

        let support = app
            .game_data
            .fleets
            .iter_mut()
            .find(|fleet| fleet.owner.0 == 1 && fleet.fleet_number.0 == 1)
            .expect("player 1 fleet #1 should exist");
        support.ships.destroyers = 0;
        support.ships.cruisers = 0;
        support.ships.battleships = 0;
        support.ships.scouts = 0;
        nc_data::set_troop_transports(support, 1);
        nc_data::set_armies_aboard(support, 1);
        support.ships.etacs = 0;
        nc_data::recompute_max_speed(support);
        support.rules_of_engagement = 0;
    }
    let combat_record_index = app
        .game_data
        .fleets
        .iter()
        .position(|fleet| fleet.owner.0 == 1 && fleet.fleet_number.0 == 4)
        .expect("fleet #4 record index")
        + 1;
    let support_record_index = app
        .game_data
        .fleets
        .iter()
        .position(|fleet| fleet.owner.0 == 1 && fleet.fleet_number.0 == 1)
        .expect("fleet #1 record index")
        + 1;

    advance_to_main_menu(&mut app);
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::OpenMenu)),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::OpenList)),
        AppOutcome::Continue
    );
    app.fleet
        .group_selected_fleets
        .extend([combat_record_index, support_record_index]);

    let action = app.handle_key(key(KeyCode::Char('c')));
    assert_eq!(apply_action(&mut app, action), AppOutcome::Continue);
    let action = app.handle_key(key(KeyCode::Char('r')));
    assert_eq!(apply_action(&mut app, action), AppOutcome::Continue);
    submit_fleet_menu_prompt_value(&mut app, "6");

    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal).expect("render succeeds");
    assert_eq!(app.current_fleet_roe_by_id(4), Some(6));
    assert_eq!(app.current_fleet_roe_by_id(1), Some(0));
    assert_eq!(app.fleet.group_selected_fleets.len(), 1);
    let remaining_record = *app
        .fleet
        .group_selected_fleets
        .iter()
        .next()
        .expect("one fleet should remain selected");
    assert_eq!(remaining_record, support_record_index);
}

#[test]
fn fleet_change_id_updates_visible_fleet_number_inline() {
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

    advance_to_main_menu(&mut app);
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::OpenMenu)),
        AppOutcome::Continue
    );
    open_change_value_prompt_from_fleet_menu(&mut app, Some(4), 'I');
    submit_fleet_menu_prompt_value(&mut app, "12");

    app.render(&mut terminal).expect("fleet menu should render");
    assert!(
        line_containing(&terminal, "Fleet #4 renumbered to Fleet #12.")
            .contains("Fleet #4 renumbered to Fleet #12.")
    );

    let state = latest_runtime_state(&fixture_dir);
    assert!(
        state
            .game_data
            .fleets
            .iter()
            .any(|fleet| fleet.owner.0 == 1 && fleet.fleet_number.0 == 12)
    );
}

#[test]
fn fleet_change_id_rejects_duplicate_fleet_number_inline() {
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

    advance_to_main_menu(&mut app);
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::OpenMenu)),
        AppOutcome::Continue
    );
    open_change_value_prompt_from_fleet_menu(&mut app, Some(4), 'I');
    submit_fleet_menu_prompt_value(&mut app, "1");

    app.render(&mut terminal)
        .expect("change prompt should render");
    assert_eq!(app.current_screen(), ScreenId::FleetMenu);
    assert!(
        line_containing(&terminal, "Fleet ID is already in use.")
            .contains("Fleet ID is already in use.")
    );
    assert!(
        line_containing(&terminal, "COMMAND <- New Fleet ID").contains("New Fleet ID [4] <Q> ->")
    );
}

#[test]
fn fleet_change_speed_updates_current_speed_inline() {
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

    advance_to_main_menu(&mut app);
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::OpenMenu)),
        AppOutcome::Continue
    );
    open_change_value_prompt_from_fleet_menu(&mut app, Some(4), 'S');
    submit_fleet_menu_prompt_value(&mut app, "0");

    app.render(&mut terminal).expect("fleet menu should render");
    assert!(
        line_containing(&terminal, "Fleet #4 speed set to 0.").contains("Fleet #4 speed set to 0.")
    );

    let state = latest_runtime_state(&fixture_dir);
    let fleet = state
        .game_data
        .fleets
        .iter()
        .find(|fleet| fleet.owner.0 == 1 && fleet.fleet_number.0 == 4)
        .expect("fleet #4 should exist");
    assert_eq!(fleet.speed, 0);
}
