use crate::support::*;

#[test]
fn navigation_hotkeys_map_ctrl_d_to_page_down_actions() {
    let root = temp_game_copy();
    let config = AppConfig {
        game_dir: root.clone(),
        player_record_index_1_based: 1,
        export_root: None,
        queue_dir: None,
        session_timeout_secs: None,
        game_config: GameConfig::default(),
    };
    let mut app = App::load(config).expect("load app");
    app.current_screen = ScreenId::PlanetDatabaseList;

    assert_eq!(
        app.handle_key(ctrl_key('d')),
        Action::Planet(PlanetAction::PageDatabaseList(-1))
    );
    assert_eq!(
        app.handle_key(ctrl_key('u')),
        Action::Planet(PlanetAction::PageDatabaseList(1))
    );
}

#[test]
fn planet_database_list_accepts_wrapped_coordinate_input() {
    let root = temp_game_copy();
    let config = AppConfig {
        game_dir: root.clone(),
        player_record_index_1_based: 1,
        export_root: None,
        queue_dir: None,
        session_timeout_secs: None,
        game_config: GameConfig::default(),
    };
    let mut app = App::load(config).expect("load app");
    advance_to_main_menu(&mut app);
    app.open_planet_database();

    let input = "{12, 3}";
    for ch in input.chars() {
        let action = app.handle_key(key(KeyCode::Char(ch)));
        assert_eq!(action, Action::Planet(PlanetAction::AppendDatabaseChar(ch)));
        assert_eq!(apply_action(&mut app, action), AppOutcome::Continue);
    }

    assert_eq!(app.planet.database_input, input);
}

#[test]
fn planet_brief_list_accepts_wrapped_coordinate_input() {
    let root = temp_game_copy();
    let config = AppConfig {
        game_dir: root.clone(),
        player_record_index_1_based: 1,
        export_root: None,
        queue_dir: None,
        session_timeout_secs: None,
        game_config: GameConfig::default(),
    };
    let mut app = App::load(config).expect("load app");
    advance_to_main_menu(&mut app);
    app.open_planet_menu();
    app.submit_planet_list_sort(PlanetListMode::Brief, PlanetListSort::Location);

    let input = "([12, 3])";
    for ch in input.chars() {
        let action = app.handle_key(key(KeyCode::Char(ch)));
        assert_eq!(action, Action::Planet(PlanetAction::AppendBriefChar(ch)));
        assert_eq!(apply_action(&mut app, action), AppOutcome::Continue);
    }

    assert_eq!(app.planet.brief_input, input);
}

#[test]
fn planet_brief_list_terminal_typed_jump_clears_footer_input() {
    let root = temp_game_copy();
    let config = AppConfig {
        game_dir: root.clone(),
        player_record_index_1_based: 1,
        export_root: None,
        queue_dir: None,
        session_timeout_secs: None,
        game_config: GameConfig::default(),
    };
    let mut app = App::load(config).expect("load app");
    advance_to_main_menu(&mut app);
    app.open_planet_menu();
    app.submit_planet_list_sort(PlanetListMode::Brief, PlanetListSort::Location);

    let mut rows = app.game_data.empire_planet_economy_rows(1);
    rows.sort_by_key(|row| row.coords);
    let match_rows = rows
        .iter()
        .map(|row| vec![nc_game::screen::format_sector_coords_table(row.coords)])
        .collect::<Vec<_>>();
    let (target_coords, input) = rows
        .iter()
        .find_map(|row| {
            let input = format!("{},{}", row.coords[0], row.coords[1]);
            nc_game::screen::table_selection::find_typed_jump(&match_rows, 0, &input)
                .filter(|matched| matched.is_terminal_exact_match)
                .map(|_| (row.coords, input))
        })
        .expect("fixture should have a terminal coordinate jump");

    for ch in input.chars() {
        let action = app.handle_key(key(KeyCode::Char(ch)));
        assert_eq!(action, Action::Planet(PlanetAction::AppendBriefChar(ch)));
        assert_eq!(apply_action(&mut app, action), AppOutcome::Continue);
    }

    assert!(app.planet.brief_input.is_empty());

    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("planet brief list should render");
    assert_eq!(
        line_containing(&terminal, "COMMAND <- ? F S B D A C M L U X <Q>").trim(),
        format!(
            "COMMAND <- ? F S B D A C M L U X <Q> [{:02},{:02}] ->",
            target_coords[0], target_coords[1]
        )
    );
}

#[test]
fn planet_database_terminal_typed_jump_clears_footer_input() {
    let root = temp_game_copy();
    let config = AppConfig {
        game_dir: root.clone(),
        player_record_index_1_based: 1,
        export_root: None,
        queue_dir: None,
        session_timeout_secs: None,
        game_config: GameConfig::default(),
    };
    let mut app = App::load(config).expect("load app");
    advance_to_main_menu(&mut app);
    app.open_planet_database();

    let mut coords_rows = nc_data::build_player_starmap_projection_from_snapshots(
        &app.game_data,
        &app.planet_intel_snapshots,
        1,
    )
    .worlds
    .into_iter()
    .map(|world| world.coords)
    .collect::<Vec<_>>();
    coords_rows.sort();
    let match_rows = coords_rows
        .iter()
        .map(|coords| vec![nc_game::screen::format_sector_coords_table(*coords)])
        .collect::<Vec<_>>();
    let (target_coords, input) = coords_rows
        .iter()
        .find_map(|coords| {
            let input = format!("{},{}", coords[0], coords[1]);
            nc_game::screen::table_selection::find_typed_jump(&match_rows, 0, &input)
                .filter(|matched| matched.is_terminal_exact_match)
                .map(|_| (*coords, input))
        })
        .expect("fixture should have a terminal database coordinate jump");

    for ch in input.chars() {
        let action = app.handle_key(key(KeyCode::Char(ch)));
        assert_eq!(action, Action::Planet(PlanetAction::AppendDatabaseChar(ch)));
        assert_eq!(apply_action(&mut app, action), AppOutcome::Continue);
    }

    assert!(app.planet.database_input.is_empty());

    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("planet database list should render");
    assert_eq!(
        line_containing(&terminal, "COMMAND <- ? F S <Q>").trim(),
        format!(
            "COMMAND <- ? F S <Q> [{:02},{:02}] ->",
            target_coords[0], target_coords[1]
        )
    );
}

#[test]
fn fleet_list_keeps_selector_input_numeric_only() {
    let root = temp_game_copy();
    let config = AppConfig {
        game_dir: root.clone(),
        player_record_index_1_based: 1,
        export_root: None,
        queue_dir: None,
        session_timeout_secs: None,
        game_config: GameConfig::default(),
    };
    let mut app = App::load(config).expect("load app");
    advance_to_main_menu(&mut app);
    app.open_fleet_list();

    assert_eq!(
        app.handle_key(key(KeyCode::Char('7'))),
        Action::Fleet(FleetAction::AppendListChar('7'))
    );
    assert_eq!(app.handle_key(key(KeyCode::Char('{'))), Action::Noop);
    assert_eq!(app.handle_key(key(KeyCode::Char(','))), Action::Noop);
    assert_eq!(
        app.handle_key(key(KeyCode::Char(' '))),
        Action::Fleet(FleetAction::ToggleGroupOrderSelection)
    );
}

#[test]
fn fleet_list_typed_jump_accepts_leading_zero_fleet_ids() {
    let root = temp_game_copy();
    let config = AppConfig {
        game_dir: root.clone(),
        player_record_index_1_based: 1,
        export_root: None,
        queue_dir: None,
        session_timeout_secs: None,
        game_config: GameConfig::default(),
    };
    let mut app = App::load(config).expect("load app");
    let mut next_other_number = 30u16;
    for fleet in app
        .game_data
        .fleets
        .records
        .iter_mut()
        .filter(|fleet| fleet.owner_empire_raw() == 1 && fleet.has_any_force())
    {
        let fleet_number = match next_other_number {
            30 => 20,
            31 => 2,
            _ => next_other_number,
        };
        fleet.set_local_slot_word_raw(fleet_number);
        next_other_number += 1;
    }
    advance_to_main_menu(&mut app);
    app.open_fleet_list();

    let action = app.handle_key(key(KeyCode::Char('0')));
    assert_eq!(action, Action::Fleet(FleetAction::AppendListChar('0')));
    assert_eq!(apply_action(&mut app, action), AppOutcome::Continue);

    let action = app.handle_key(key(KeyCode::Char('2')));
    assert_eq!(action, Action::Fleet(FleetAction::AppendListChar('2')));
    assert_eq!(apply_action(&mut app, action), AppOutcome::Continue);

    assert!(app.fleet.list_input.is_empty());
    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal).expect("fleet list should render");
    assert_eq!(
        line_containing(&terminal, "COMMAND <- ? F S O C E D M T L U SPACE <Q>").trim(),
        "COMMAND <- ? F S O C E D M T L U SPACE <Q> [02] ->"
    );
}

#[test]
fn fleet_filter_prompt_accepts_unique_prefix_and_reports_ambiguity_inline() {
    let root = temp_game_copy();
    let config = AppConfig {
        game_dir: root.clone(),
        player_record_index_1_based: 1,
        export_root: None,
        queue_dir: None,
        session_timeout_secs: None,
        game_config: GameConfig::default(),
    };
    let mut app = App::load(config).expect("load app");
    advance_to_main_menu(&mut app);
    app.open_fleet_list();

    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::OpenListFilterPrompt)),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(
            &mut app,
            Action::Fleet(FleetAction::AppendListFilterPromptChar('s'))
        ),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::SubmitListFilterPrompt)),
        AppOutcome::Continue
    );
    assert_eq!(
        app.fleet.list_filter_prompt_mode,
        nc_game::screen::FleetListFilterPromptMode::Column
    );
    assert_eq!(
        app.fleet.list_filter_prompt_status.as_deref(),
        Some(" Ambiguous: sel/shi/spd")
    );
    assert!(app.fleet.list_filter_prompt_input.is_empty());

    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal).expect("fleet list should render");
    assert!(
        line_containing(&terminal, "COMMAND <- Ambiguous: sel/shi/spd").contains("[all] <Q> ->")
    );

    assert_eq!(
        apply_action(
            &mut app,
            Action::Fleet(FleetAction::BackspaceListFilterPromptInput)
        ),
        AppOutcome::Continue
    );
    terminal = CaptureTerminal::new();
    app.render(&mut terminal).expect("fleet list should render");
    assert!(line_containing(&terminal, "COMMAND <- Filter column [?] ").contains("[all] <Q> ->"));

    assert_eq!(
        apply_action(
            &mut app,
            Action::Fleet(FleetAction::AppendListFilterPromptChar('s'))
        ),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(
            &mut app,
            Action::Fleet(FleetAction::AppendListFilterPromptChar('p'))
        ),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::SubmitListFilterPrompt)),
        AppOutcome::Continue
    );
    assert_eq!(
        app.fleet.list_filter_prompt_mode,
        nc_game::screen::FleetListFilterPromptMode::Value
    );
    assert_eq!(
        app.fleet
            .list_filter_pending_column
            .expect("pending column")
            .code,
        "spd"
    );
}

#[test]
fn planet_filter_prompt_accepts_unique_prefix_and_reports_ambiguity_inline() {
    let root = temp_game_copy();
    let config = AppConfig {
        game_dir: root.clone(),
        player_record_index_1_based: 1,
        export_root: None,
        queue_dir: None,
        session_timeout_secs: None,
        game_config: GameConfig::default(),
    };
    let mut app = App::load(config).expect("load app");
    advance_to_main_menu(&mut app);
    app.open_planet_menu();
    app.submit_planet_list_sort(PlanetListMode::Brief, PlanetListSort::Location);

    assert_eq!(
        apply_action(
            &mut app,
            Action::Planet(PlanetAction::OpenListFilterPrompt(PlanetListMode::Brief))
        ),
        AppOutcome::Continue
    );
    let action = app.handle_key(key(KeyCode::Char('s')));
    assert_eq!(apply_action(&mut app, action), AppOutcome::Continue);
    let action = app.handle_key(key(KeyCode::Enter));
    assert_eq!(apply_action(&mut app, action), AppOutcome::Continue);
    assert_eq!(
        app.planet.list_filter_prompt_mode,
        nc_game::screen::PlanetListFilterPromptMode::FilterMenu
    );
    assert_eq!(
        app.planet.list_prompt_status.as_deref(),
        Some(" Ambiguous: sbs/sta")
    );
    assert!(app.planet.list_prompt_input.is_empty());

    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("planet list should render");
    assert!(line_containing(&terminal, "COMMAND <- Ambiguous: sbs/sta").contains("[all] <Q> ->"));

    let action = app.handle_key(key(KeyCode::Backspace));
    assert_eq!(apply_action(&mut app, action), AppOutcome::Continue);
    terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("planet list should render");
    assert!(line_containing(&terminal, "COMMAND <- Filter column [?] ").contains("[all] <Q> ->"));

    let action = app.handle_key(key(KeyCode::Char('d')));
    assert_eq!(apply_action(&mut app, action), AppOutcome::Continue);
    let action = app.handle_key(key(KeyCode::Enter));
    assert_eq!(apply_action(&mut app, action), AppOutcome::Continue);
    assert_eq!(
        app.planet.list_filter_prompt_mode,
        nc_game::screen::PlanetListFilterPromptMode::ValueInput
    );
    assert_eq!(
        app.planet
            .list_filter_pending_column
            .expect("pending column")
            .code,
        "sta"
    );
}

#[test]
fn database_filter_prompt_accepts_unique_prefix_and_reports_ambiguity_inline() {
    let root = temp_game_copy();
    let config = AppConfig {
        game_dir: root.clone(),
        player_record_index_1_based: 1,
        export_root: None,
        queue_dir: None,
        session_timeout_secs: None,
        game_config: GameConfig::default(),
    };
    let mut app = App::load(config).expect("load app");
    advance_to_main_menu(&mut app);
    app.open_planet_database();

    assert_eq!(
        apply_action(
            &mut app,
            Action::Planet(PlanetAction::OpenDatabaseFilterPrompt)
        ),
        AppOutcome::Continue
    );
    let action = app.handle_key(key(KeyCode::Char('s')));
    assert_eq!(apply_action(&mut app, action), AppOutcome::Continue);
    let action = app.handle_key(key(KeyCode::Enter));
    assert_eq!(apply_action(&mut app, action), AppOutcome::Continue);
    assert_eq!(
        app.planet.database_prompt_mode,
        nc_game::screen::PlanetDatabasePromptMode::FilterMenu
    );
    assert_eq!(
        app.planet.database_status.as_deref(),
        Some(" Ambiguous: sbs/sco/see")
    );
    assert!(app.planet.database_input.is_empty());

    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("planet database should render");
    assert!(
        line_containing(&terminal, "COMMAND <- Ambiguous: sbs/sco/see").contains("[all] <Q> ->")
    );

    let action = app.handle_key(key(KeyCode::Backspace));
    assert_eq!(apply_action(&mut app, action), AppOutcome::Continue);
    terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("planet database should render");
    assert!(line_containing(&terminal, "COMMAND <- Filter column [?] ").contains("[all] <Q> ->"));

    let action = app.handle_key(key(KeyCode::Char('s')));
    assert_eq!(apply_action(&mut app, action), AppOutcome::Continue);
    let action = app.handle_key(key(KeyCode::Char('c')));
    assert_eq!(apply_action(&mut app, action), AppOutcome::Continue);
    let action = app.handle_key(key(KeyCode::Enter));
    assert_eq!(apply_action(&mut app, action), AppOutcome::Continue);
    assert_eq!(
        app.planet.database_prompt_mode,
        nc_game::screen::PlanetDatabasePromptMode::FilterValueInput
    );
    assert_eq!(
        app.planet
            .database_pending_column
            .expect("pending column")
            .code,
        "sco"
    );
}

#[test]
fn empty_fleet_filter_clause_resets_to_all() {
    let root = temp_game_copy();
    let config = AppConfig {
        game_dir: root.clone(),
        player_record_index_1_based: 1,
        export_root: None,
        queue_dir: None,
        session_timeout_secs: None,
        game_config: GameConfig::default(),
    };
    let mut app = App::load(config).expect("load app");
    advance_to_main_menu(&mut app);
    app.open_fleet_list();

    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::OpenListFilterPrompt)),
        AppOutcome::Continue
    );
    for ch in ['o', 'r', 'd'] {
        assert_eq!(
            apply_action(
                &mut app,
                Action::Fleet(FleetAction::AppendListFilterPromptChar(ch))
            ),
            AppOutcome::Continue
        );
    }
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::SubmitListFilterPrompt)),
        AppOutcome::Continue
    );
    for ch in ['z', 'z', 'z', 'z'] {
        let action = app.handle_key(key(KeyCode::Char(ch)));
        assert_eq!(apply_action(&mut app, action), AppOutcome::Continue);
    }
    let action = app.handle_key(key(KeyCode::Enter));
    assert_eq!(apply_action(&mut app, action), AppOutcome::Continue);

    assert_eq!(app.current_screen, ScreenId::FleetList);
    assert!(app.fleet.list_filter_clause.is_none());
    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal).expect("fleet list should render");
    assert!(line_containing(&terminal, "FLEET LIST: ").contains(" ALL"));
}

#[test]
fn empty_planet_list_filter_clause_resets_to_all() {
    let root = temp_game_copy();
    let config = AppConfig {
        game_dir: root.clone(),
        player_record_index_1_based: 1,
        export_root: None,
        queue_dir: None,
        session_timeout_secs: None,
        game_config: GameConfig::default(),
    };
    let mut app = App::load(config).expect("load app");
    advance_to_main_menu(&mut app);
    app.open_planet_menu();
    app.submit_planet_list_sort(PlanetListMode::Brief, PlanetListSort::Location);

    assert_eq!(
        apply_action(
            &mut app,
            Action::Planet(PlanetAction::OpenListFilterPrompt(PlanetListMode::Brief))
        ),
        AppOutcome::Continue
    );
    for ch in ['p', 'l', 'a'] {
        let action = app.handle_key(key(KeyCode::Char(ch)));
        assert_eq!(apply_action(&mut app, action), AppOutcome::Continue);
    }
    let action = app.handle_key(key(KeyCode::Enter));
    assert_eq!(apply_action(&mut app, action), AppOutcome::Continue);
    for ch in ['z', 'z', 'z', 'z'] {
        let action = app.handle_key(key(KeyCode::Char(ch)));
        assert_eq!(apply_action(&mut app, action), AppOutcome::Continue);
    }
    let action = app.handle_key(key(KeyCode::Enter));
    assert_eq!(apply_action(&mut app, action), AppOutcome::Continue);

    assert_eq!(
        app.current_screen,
        ScreenId::PlanetList(PlanetListMode::Brief, PlanetListSort::Location)
    );
    assert!(app.planet.list_filter_clause.is_none());
    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("planet list should render");
    assert!(line_containing(&terminal, "PLANET LIST: ").contains(" ALL"));
}

#[test]
fn empty_database_filter_clause_resets_to_all() {
    let root = temp_game_copy();
    let config = AppConfig {
        game_dir: root.clone(),
        player_record_index_1_based: 1,
        export_root: None,
        queue_dir: None,
        session_timeout_secs: None,
        game_config: GameConfig::default(),
    };
    let mut app = App::load(config).expect("load app");
    advance_to_main_menu(&mut app);
    app.open_planet_database();

    assert_eq!(
        apply_action(
            &mut app,
            Action::Planet(PlanetAction::OpenDatabaseFilterPrompt)
        ),
        AppOutcome::Continue
    );
    for ch in ['p', 'l', 'a'] {
        let action = app.handle_key(key(KeyCode::Char(ch)));
        assert_eq!(apply_action(&mut app, action), AppOutcome::Continue);
    }
    let action = app.handle_key(key(KeyCode::Enter));
    assert_eq!(apply_action(&mut app, action), AppOutcome::Continue);
    for ch in ['z', 'z', 'z', 'z'] {
        let action = app.handle_key(key(KeyCode::Char(ch)));
        assert_eq!(apply_action(&mut app, action), AppOutcome::Continue);
    }
    let action = app.handle_key(key(KeyCode::Enter));
    assert_eq!(apply_action(&mut app, action), AppOutcome::Continue);

    assert_eq!(app.current_screen, ScreenId::PlanetDatabaseList);
    assert!(app.planet.database_filter_clause.is_none());
    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("planet database should render");
    assert!(line_containing(&terminal, "TOTAL PLANET DATABASE: ").contains(" ALL"));
}

#[test]
fn stale_fleet_filter_clause_resets_to_all_after_rows_change() {
    let root = temp_game_copy();
    let config = AppConfig {
        game_dir: root.clone(),
        player_record_index_1_based: 1,
        export_root: None,
        queue_dir: None,
        session_timeout_secs: None,
        game_config: GameConfig::default(),
    };
    let mut app = App::load(config).expect("load app");
    advance_to_main_menu(&mut app);

    let owned_fleet_indexes = app
        .game_data
        .fleets
        .records
        .iter()
        .enumerate()
        .filter_map(|(idx, fleet)| (fleet.owner_empire_raw() == 1).then_some(idx))
        .collect::<Vec<_>>();
    assert!(
        !owned_fleet_indexes.is_empty(),
        "fixture should have owned fleets"
    );
    for &idx in &owned_fleet_indexes {
        app.game_data.fleets.records[idx].set_standing_order_kind(nc_data::Order::SeekHome);
    }
    app.game_data.fleets.records[owned_fleet_indexes[0]]
        .set_standing_order_kind(nc_data::Order::HoldPosition);

    app.open_fleet_list();
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::OpenListFilterPrompt)),
        AppOutcome::Continue
    );
    for ch in ['o', 'r', 'd'] {
        assert_eq!(
            apply_action(
                &mut app,
                Action::Fleet(FleetAction::AppendListFilterPromptChar(ch))
            ),
            AppOutcome::Continue
        );
    }
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::SubmitListFilterPrompt)),
        AppOutcome::Continue
    );
    for ch in "hold".chars() {
        let action = app.handle_key(key(KeyCode::Char(ch)));
        assert_eq!(apply_action(&mut app, action), AppOutcome::Continue);
    }
    let action = app.handle_key(key(KeyCode::Enter));
    assert_eq!(apply_action(&mut app, action), AppOutcome::Continue);
    assert!(app.fleet.list_filter_clause.is_some());

    app.game_data.fleets.records[owned_fleet_indexes[0]]
        .set_standing_order_kind(nc_data::Order::SeekHome);

    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal).expect("fleet list should render");
    assert!(app.fleet.list_filter_clause.is_none());
    assert!(line_containing(&terminal, "FLEET LIST: ").contains(" ALL"));
    assert!(
        terminal
            .lines
            .iter()
            .all(|line| !line.contains("No fleets match current filter."))
    );
}

#[test]
fn stale_planet_list_filter_clause_resets_to_all_after_rows_change() {
    let root = temp_game_copy();
    let config = AppConfig {
        game_dir: root.clone(),
        player_record_index_1_based: 1,
        export_root: None,
        queue_dir: None,
        session_timeout_secs: None,
        game_config: GameConfig::default(),
    };
    let mut app = App::load(config).expect("load app");
    advance_to_main_menu(&mut app);

    let owned_planet_indexes = app
        .game_data
        .planets
        .records
        .iter()
        .enumerate()
        .filter_map(|(idx, planet)| (planet.owner_empire_slot_raw() == 1).then_some(idx))
        .collect::<Vec<_>>();
    let target_planet_idx = *owned_planet_indexes.first().expect("owned planet exists");
    for &idx in &owned_planet_indexes {
        app.game_data.planets.records[idx].set_army_count_raw(0);
    }
    app.game_data.planets.records[target_planet_idx].set_army_count_raw(5);

    app.open_planet_menu();
    app.submit_planet_list_sort(PlanetListMode::Brief, PlanetListSort::Location);
    assert_eq!(
        apply_action(
            &mut app,
            Action::Planet(PlanetAction::OpenListFilterPrompt(PlanetListMode::Brief))
        ),
        AppOutcome::Continue
    );
    for ch in ['a', 'r', 's'] {
        let action = app.handle_key(key(KeyCode::Char(ch)));
        assert_eq!(apply_action(&mut app, action), AppOutcome::Continue);
    }
    let action = app.handle_key(key(KeyCode::Enter));
    assert_eq!(apply_action(&mut app, action), AppOutcome::Continue);
    for ch in "5".chars() {
        let action = app.handle_key(key(KeyCode::Char(ch)));
        assert_eq!(apply_action(&mut app, action), AppOutcome::Continue);
    }
    let action = app.handle_key(key(KeyCode::Enter));
    assert_eq!(apply_action(&mut app, action), AppOutcome::Continue);
    assert!(app.planet.list_filter_clause.is_some());

    app.game_data.planets.records[target_planet_idx].set_army_count_raw(0);

    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("planet list should render");
    assert!(app.planet.list_filter_clause.is_none());
    assert!(line_containing(&terminal, "PLANET LIST: ").contains(" ALL"));
    assert!(
        terminal
            .lines
            .iter()
            .all(|line| !line.contains("No planets match current filter."))
    );
}

#[test]
fn stale_database_filter_clause_resets_to_all_after_rows_change() {
    let root = temp_game_copy();
    let config = AppConfig {
        game_dir: root.clone(),
        player_record_index_1_based: 1,
        export_root: None,
        queue_dir: None,
        session_timeout_secs: None,
        game_config: GameConfig::default(),
    };
    let mut app = App::load(config).expect("load app");
    advance_to_main_menu(&mut app);

    for planet in &mut app.game_data.planets.records {
        planet.set_army_count_raw(0);
    }
    let target_planet_idx = app
        .game_data
        .planets
        .records
        .iter()
        .position(|planet| planet.owner_empire_slot_raw() == 1)
        .expect("owned planet exists");
    app.game_data.planets.records[target_planet_idx].set_army_count_raw(7);

    app.open_planet_database();
    assert_eq!(
        apply_action(
            &mut app,
            Action::Planet(PlanetAction::OpenDatabaseFilterPrompt)
        ),
        AppOutcome::Continue
    );
    for ch in ['a', 'r', 's'] {
        let action = app.handle_key(key(KeyCode::Char(ch)));
        assert_eq!(apply_action(&mut app, action), AppOutcome::Continue);
    }
    let action = app.handle_key(key(KeyCode::Enter));
    assert_eq!(apply_action(&mut app, action), AppOutcome::Continue);
    for ch in "7".chars() {
        let action = app.handle_key(key(KeyCode::Char(ch)));
        assert_eq!(apply_action(&mut app, action), AppOutcome::Continue);
    }
    let action = app.handle_key(key(KeyCode::Enter));
    assert_eq!(apply_action(&mut app, action), AppOutcome::Continue);
    assert!(app.planet.database_filter_clause.is_some());

    app.game_data.planets.records[target_planet_idx].set_army_count_raw(0);

    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("planet database should render");
    assert!(app.planet.database_filter_clause.is_none());
    assert!(line_containing(&terminal, "TOTAL PLANET DATABASE: ").contains(" ALL"));
    assert!(
        terminal
            .lines
            .iter()
            .all(|line| !line.contains("No worlds match current filter."))
    );
}

#[test]
fn unknown_filter_column_uses_slap_key_notice_across_table_prompts() {
    let root = temp_game_copy();
    let config = AppConfig {
        game_dir: root.clone(),
        player_record_index_1_based: 1,
        export_root: None,
        queue_dir: None,
        session_timeout_secs: None,
        game_config: GameConfig::default(),
    };
    let mut app = App::load(config).expect("load app");
    advance_to_main_menu(&mut app);

    app.open_fleet_list();
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::OpenListFilterPrompt)),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(
            &mut app,
            Action::Fleet(FleetAction::AppendListFilterPromptChar('p'))
        ),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::SubmitListFilterPrompt)),
        AppOutcome::Continue
    );
    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("fleet filter prompt should render");
    assert!(
        line_containing(&terminal, "COMMAND <-")
            .contains("Enter a valid column name/code or ALL (slap a key)")
    );
    let action = app.handle_key(key(KeyCode::Char('x')));
    assert_eq!(
        action,
        Action::Fleet(FleetAction::DismissListFilterPromptNotice)
    );
    assert_eq!(apply_action(&mut app, action), AppOutcome::Continue);
    terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("fleet filter prompt should render");
    assert!(line_containing(&terminal, "COMMAND <- Filter column [?] ").contains("[all] <Q> ->"));

    app.open_planet_menu();
    app.submit_planet_list_sort(PlanetListMode::Brief, PlanetListSort::Location);
    assert_eq!(
        apply_action(
            &mut app,
            Action::Planet(PlanetAction::OpenListFilterPrompt(PlanetListMode::Brief))
        ),
        AppOutcome::Continue
    );
    let action = app.handle_key(key(KeyCode::Char('z')));
    assert_eq!(apply_action(&mut app, action), AppOutcome::Continue);
    let action = app.handle_key(key(KeyCode::Enter));
    assert_eq!(apply_action(&mut app, action), AppOutcome::Continue);
    terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("planet filter prompt should render");
    assert!(
        line_containing(&terminal, "COMMAND <-")
            .contains("Enter a valid column name/code or ALL (slap a key)")
    );
    let action = app.handle_key(key(KeyCode::Char('x')));
    assert_eq!(
        action,
        Action::Planet(PlanetAction::DismissListFilterPromptNotice)
    );
    assert_eq!(apply_action(&mut app, action), AppOutcome::Continue);

    app.open_planet_database();
    assert_eq!(
        apply_action(
            &mut app,
            Action::Planet(PlanetAction::OpenDatabaseFilterPrompt)
        ),
        AppOutcome::Continue
    );
    let action = app.handle_key(key(KeyCode::Char('z')));
    assert_eq!(apply_action(&mut app, action), AppOutcome::Continue);
    let action = app.handle_key(key(KeyCode::Enter));
    assert_eq!(apply_action(&mut app, action), AppOutcome::Continue);
    terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("database filter prompt should render");
    assert!(
        line_containing(&terminal, "COMMAND <-")
            .contains("Enter a valid column name/code or ALL (slap a key)")
    );
    let action = app.handle_key(key(KeyCode::Char('x')));
    assert_eq!(
        action,
        Action::Planet(PlanetAction::DismissDatabaseFilterPromptNotice)
    );
    assert_eq!(apply_action(&mut app, action), AppOutcome::Continue);
}

#[test]
fn q_closes_filter_prompts_instead_of_becoming_input() {
    let root = temp_game_copy();
    let config = AppConfig {
        game_dir: root.clone(),
        player_record_index_1_based: 1,
        export_root: None,
        queue_dir: None,
        session_timeout_secs: None,
        game_config: GameConfig::default(),
    };
    let mut app = App::load(config).expect("load app");
    advance_to_main_menu(&mut app);

    app.open_fleet_list();
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::OpenListFilterPrompt)),
        AppOutcome::Continue
    );
    let action = app.handle_key(key(KeyCode::Char('q')));
    assert_eq!(action, Action::Fleet(FleetAction::CloseListPrompt));
    assert_eq!(apply_action(&mut app, action), AppOutcome::Continue);
    assert_eq!(app.current_screen(), ScreenId::FleetList);

    app.open_planet_menu();
    app.submit_planet_list_sort(PlanetListMode::Brief, PlanetListSort::Location);
    assert_eq!(
        apply_action(
            &mut app,
            Action::Planet(PlanetAction::OpenListFilterPrompt(PlanetListMode::Brief))
        ),
        AppOutcome::Continue
    );
    let action = app.handle_key(key(KeyCode::Char('q')));
    assert_eq!(
        action,
        Action::Planet(PlanetAction::CloseListFilterPrompt(PlanetListMode::Brief))
    );
    assert_eq!(apply_action(&mut app, action), AppOutcome::Continue);
    assert_eq!(
        app.current_screen(),
        ScreenId::PlanetList(PlanetListMode::Brief, app.planet.list_sort)
    );

    app.open_planet_database();
    assert_eq!(
        apply_action(
            &mut app,
            Action::Planet(PlanetAction::OpenDatabaseFilterPrompt)
        ),
        AppOutcome::Continue
    );
    let action = app.handle_key(key(KeyCode::Char('q')));
    assert_eq!(action, Action::Planet(PlanetAction::OpenDatabase));
    assert_eq!(apply_action(&mut app, action), AppOutcome::Continue);
    assert_eq!(app.current_screen(), ScreenId::PlanetDatabaseList);
}

#[test]
fn auto_commission_prompt_enter_defaults_to_yes() {
    let root = temp_game_with_auto_commission_copy();
    let config = AppConfig {
        game_dir: root.clone(),
        player_record_index_1_based: 1,
        export_root: None,
        queue_dir: None,
        session_timeout_secs: None,
        game_config: GameConfig::default(),
    };
    let mut app = App::load(config).expect("load app");
    advance_to_main_menu(&mut app);
    app.open_planet_menu();
    assert_eq!(
        apply_action(
            &mut app,
            Action::Planet(PlanetAction::OpenAutoCommissionPrompt)
        ),
        AppOutcome::Continue
    );
    assert!(app.planet.auto_commission_prompt_active);

    let action = app.handle_key(key(KeyCode::Enter));
    assert_eq!(action, Action::Planet(PlanetAction::ConfirmAutoCommission));
}

#[test]
fn planet_list_hotkeys_open_direct_row_actions_and_return_to_list() {
    let root = temp_game_copy();
    let config = AppConfig {
        game_dir: root.clone(),
        player_record_index_1_based: 1,
        export_root: None,
        queue_dir: None,
        session_timeout_secs: None,
        game_config: GameConfig::default(),
    };
    let mut app = App::load(config).expect("load app");
    advance_to_main_menu(&mut app);
    app.open_planet_menu();
    app.submit_planet_list_sort(PlanetListMode::Brief, PlanetListSort::Location);

    let list_screen = ScreenId::PlanetList(PlanetListMode::Brief, PlanetListSort::Location);
    assert_eq!(app.current_screen(), list_screen);

    assert_eq!(
        app.handle_key(key(KeyCode::Char('b'))),
        Action::Planet(PlanetAction::OpenBuildSpecify)
    );
    let action = app.handle_key(key(KeyCode::Char('b')));
    assert_eq!(apply_action(&mut app, action), AppOutcome::Continue);
    assert_eq!(app.current_screen(), ScreenId::PlanetBuildSpecify);
    let action = app.handle_key(key(KeyCode::Char('q')));
    assert_eq!(apply_action(&mut app, action), AppOutcome::Continue);
    assert_eq!(app.current_screen(), list_screen);

    assert_eq!(
        app.handle_key(key(KeyCode::Char('i'))),
        Action::Planet(PlanetAction::SubmitBriefInput)
    );
    let action = app.handle_key(key(KeyCode::Char('i')));
    assert_eq!(apply_action(&mut app, action), AppOutcome::Continue);
    assert_eq!(app.current_screen(), ScreenId::PlanetInfoDetail);
    let action = app.handle_key(key(KeyCode::Enter));
    assert_eq!(apply_action(&mut app, action), AppOutcome::Continue);
    assert_eq!(app.current_screen(), list_screen);

    assert_eq!(
        app.handle_key(key(KeyCode::Char('x'))),
        Action::Planet(PlanetAction::OpenScorchPrompt)
    );
    let action = app.handle_key(key(KeyCode::Char('x')));
    assert_eq!(apply_action(&mut app, action), AppOutcome::Continue);
    assert_eq!(app.current_screen(), list_screen);
    let action = app.handle_key(key(KeyCode::Enter));
    assert_eq!(apply_action(&mut app, action), AppOutcome::Continue);
    assert_eq!(app.current_screen(), list_screen);

    assert_eq!(
        app.handle_key(key(KeyCode::Char('s'))),
        Action::Planet(PlanetAction::OpenListSortPrompt(PlanetListMode::Brief))
    );
}

#[test]
fn planet_list_build_hotkey_shows_notice_when_selected_planet_has_no_build_budget() {
    let fixture_dir = temp_game_copy();
    let mut runtime = latest_runtime_state(&fixture_dir);
    let homeworld = runtime
        .game_data
        .planets
        .records
        .iter_mut()
        .find(|planet| planet.owner_empire_slot_raw() == 1)
        .expect("owned planet exists");
    homeworld.set_stored_production_points(1);
    save_runtime_state(&fixture_dir, &runtime);

    let config = AppConfig {
        game_dir: fixture_dir.clone(),
        player_record_index_1_based: 1,
        export_root: None,
        queue_dir: None,
        session_timeout_secs: None,
        game_config: GameConfig::default(),
    };
    let mut app = App::load(config).expect("load app");
    let mut terminal = CaptureTerminal::new();
    advance_to_main_menu(&mut app);
    app.open_planet_menu();
    app.submit_planet_list_sort(PlanetListMode::Brief, PlanetListSort::Location);

    let list_screen = ScreenId::PlanetList(PlanetListMode::Brief, PlanetListSort::Location);
    assert_eq!(app.current_screen(), list_screen);

    let action = app.handle_key(key(KeyCode::Char('b')));
    assert_eq!(action, Action::Planet(PlanetAction::OpenBuildSpecify));
    assert_eq!(apply_action(&mut app, action), AppOutcome::Continue);
    assert_eq!(app.current_screen(), list_screen);

    app.render(&mut terminal)
        .expect("planet list should render no-budget notice");
    assert!(
        line_containing(&terminal, "No build budget remains.").contains("No build budget remains.")
    );
}

#[test]
fn compose_body_treats_hjkl_as_text_and_keeps_arrow_navigation() {
    let screen = MessageComposeScreen::new();

    assert_eq!(
        screen.handle_body_key(key(KeyCode::Char('h'))),
        Action::Messaging(MessagingAction::AppendComposeBodyChar('h'))
    );
    assert_eq!(
        screen.handle_body_key(key(KeyCode::Char('j'))),
        Action::Messaging(MessagingAction::AppendComposeBodyChar('j'))
    );
    assert_eq!(
        screen.handle_body_key(key(KeyCode::Char('k'))),
        Action::Messaging(MessagingAction::AppendComposeBodyChar('k'))
    );
    assert_eq!(
        screen.handle_body_key(key(KeyCode::Char('l'))),
        Action::Messaging(MessagingAction::AppendComposeBodyChar('l'))
    );
    assert_eq!(
        screen.handle_body_key(key(KeyCode::Left)),
        Action::Messaging(MessagingAction::MoveComposeBodyCursorLeft)
    );
    assert_eq!(
        screen.handle_body_key(key(KeyCode::Down)),
        Action::Messaging(MessagingAction::MoveComposeBodyCursorDown)
    );
    assert_eq!(
        screen.handle_body_key(key(KeyCode::Up)),
        Action::Messaging(MessagingAction::MoveComposeBodyCursorUp)
    );
    assert_eq!(
        screen.handle_body_key(key(KeyCode::Right)),
        Action::Messaging(MessagingAction::MoveComposeBodyCursorRight)
    );
}

#[test]
fn apply_action_switches_between_client_screens() {
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
        app.current_screen(),
        ScreenId::Startup(StartupPhase::Splash)
    );

    assert_eq!(
        apply_action(&mut app, Action::Startup(StartupAction::Advance)),
        AppOutcome::Continue
    );
    assert_eq!(
        app.current_screen(),
        ScreenId::Startup(StartupPhase::Splash)
    );

    assert_eq!(
        apply_action(&mut app, Action::Startup(StartupAction::Advance)),
        AppOutcome::Continue
    );
    assert_eq!(
        app.current_screen(),
        ScreenId::Startup(StartupPhase::LoginSummary)
    );

    assert_eq!(
        apply_action(&mut app, Action::Startup(StartupAction::Advance)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::MainMenu);

    assert_eq!(
        apply_action(&mut app, Action::OpenGeneralMenu),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::GeneralMenu);

    // Continue on the joined-player surface after startup.
    assert_eq!(
        apply_action(&mut app, Action::OpenMainMenu),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::MainMenu);

    assert_eq!(
        apply_action(&mut app, Action::OpenGeneralMenu),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::GeneralMenu);

    assert_eq!(
        apply_action(&mut app, Action::Starmap(StarmapAction::OpenFull)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::Starmap);

    assert_eq!(
        apply_action(&mut app, Action::OpenGeneralMenu),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::GeneralMenu);

    assert_eq!(
        apply_action(&mut app, Action::OpenPopupHelp),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::GeneralMenu);
    assert!(app.popup_help.is_some());
    assert_eq!(
        apply_action(&mut app, Action::DismissPopupHelp),
        AppOutcome::Continue
    );
    assert!(app.popup_help.is_none());

    assert_eq!(
        apply_action(&mut app, Action::Planet(PlanetAction::OpenMenu)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::PlanetMenu);

    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::OpenMenu)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FleetMenu);

    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::OpenList)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FleetList);

    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::OpenReviewPrompt)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FleetList);
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::OpenReview)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FleetReview);

    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::OpenChangePrompt)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FleetMenu);

    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::OpenDetach)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FleetMenu);

    assert_eq!(
        apply_action(&mut app, Action::OpenPopupHelp),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FleetMenu);
    assert!(app.popup_help.is_some());
    assert_eq!(
        apply_action(&mut app, Action::DismissPopupHelp),
        AppOutcome::Continue
    );
    assert!(app.popup_help.is_none());

    assert_eq!(
        apply_action(
            &mut app,
            Action::Planet(PlanetAction::OpenAutoCommissionPrompt)
        ),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::PlanetMenu);

    assert_eq!(
        apply_action(
            &mut app,
            Action::Planet(PlanetAction::ConfirmAutoCommission)
        ),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::PlanetMenu);

    assert_eq!(
        apply_action(&mut app, Action::Planet(PlanetAction::OpenCommissionMenu)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::PlanetMenu);

    assert_eq!(
        apply_action(&mut app, Action::Planet(PlanetAction::OpenBuildMenu)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::PlanetBuildMenu);

    assert_eq!(
        apply_action(&mut app, Action::OpenPopupHelp),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::PlanetBuildMenu);
    assert!(app.popup_help.is_some());
    assert_eq!(
        apply_action(&mut app, Action::DismissPopupHelp),
        AppOutcome::Continue
    );
    assert!(app.popup_help.is_none());

    assert_eq!(
        apply_action(
            &mut app,
            Action::Planet(PlanetAction::OpenCurrentBuildPlanetInfo)
        ),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::PlanetInfoDetail);

    assert_eq!(
        apply_action(&mut app, Action::Planet(PlanetAction::OpenBuildList)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::PlanetBuildMenu);
    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("build menu should render list-empty notice");
    assert!(line_containing(&terminal, "Notice: ").contains("No build orders are queued."));

    assert_eq!(
        apply_action(&mut app, Action::Planet(PlanetAction::OpenBuildAbortPrompt)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::PlanetBuildMenu);
    app.render(&mut terminal)
        .expect("build menu should render abort-empty notice");
    assert!(line_containing(&terminal, "Notice: ").contains("No build orders are queued."));

    assert_eq!(
        apply_action(&mut app, Action::Planet(PlanetAction::OpenBuildSpecify)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::PlanetBuildSpecify);

    assert_eq!(
        apply_action(&mut app, Action::Planet(PlanetAction::OpenTaxPrompt)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::PlanetMenu);

    assert_eq!(
        apply_action(&mut app, Action::Planet(PlanetAction::BackspaceTaxInput)),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Planet(PlanetAction::BackspaceTaxInput)),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Planet(PlanetAction::AppendTaxChar('6'))),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Planet(PlanetAction::AppendTaxChar('5'))),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Planet(PlanetAction::SubmitTax)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::PlanetMenu);

    assert_eq!(
        apply_action(&mut app, Action::Planet(PlanetAction::OpenDatabase)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::PlanetDatabaseList);

    assert_eq!(
        apply_action(&mut app, Action::Planet(PlanetAction::SubmitDatabaseLookup)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::PlanetInfoDetail);

    assert_eq!(
        apply_action(
            &mut app,
            Action::Planet(PlanetAction::SubmitListSort(
                PlanetListMode::Brief,
                PlanetListSort::CurrentProduction
            ))
        ),
        AppOutcome::Continue
    );
    assert_eq!(
        app.current_screen(),
        ScreenId::PlanetList(PlanetListMode::Brief, PlanetListSort::CurrentProduction)
    );

    assert_eq!(
        apply_action(
            &mut app,
            Action::Planet(PlanetAction::SubmitListSort(
                PlanetListMode::Brief,
                PlanetListSort::Location
            ))
        ),
        AppOutcome::Continue
    );
    assert_eq!(
        app.current_screen(),
        ScreenId::PlanetList(PlanetListMode::Brief, PlanetListSort::Location)
    );

    assert_eq!(
        apply_action(&mut app, Action::OpenGeneralMenu),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::GeneralMenu);

    assert_eq!(
        apply_action(
            &mut app,
            Action::Planet(PlanetAction::OpenInfoPrompt(CommandMenu::General))
        ),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::GeneralMenu);
    assert_eq!(app.planet_info_input(), "");

    assert_eq!(
        apply_action(&mut app, Action::Planet(PlanetAction::SubmitInfoPrompt)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::PlanetInfoDetail);
    assert_eq!(app.selected_planet_info(), Some(14));

    assert_eq!(
        apply_action(
            &mut app,
            Action::Starmap(StarmapAction::OpenPartialView(CommandMenu::General))
        ),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::PartialStarmapView);

    assert_eq!(
        apply_action(&mut app, Action::Startup(StartupAction::OpenReports)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::Reports);

    assert_eq!(
        apply_action(&mut app, Action::Empire(EmpireAction::OpenStatus)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::EmpireStatus);

    assert_eq!(
        apply_action(&mut app, Action::Empire(EmpireAction::OpenProfile)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::EmpireProfile);

    assert_eq!(
        apply_action(
            &mut app,
            Action::Empire(EmpireAction::OpenRankingsTable(
                EmpireProductionRankingSort::Production
            ))
        ),
        AppOutcome::Continue
    );
    assert_eq!(
        app.current_screen(),
        ScreenId::Rankings(EmpireProductionRankingSort::Production)
    );

    assert_eq!(
        apply_action(&mut app, Action::OpenMainMenu),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::MainMenu);
}

#[test]
fn app_load_persists_all_setup_backed_config_fields_into_runtime_snapshot() {
    let fixture_dir = temp_first_time_game_copy();
    let before_setup = latest_runtime_state(&fixture_dir).game_data.setup.clone();
    let config = GameConfig {
        game_name: "Config Persistence Test".to_string(),
        theme: None,
        setup_overrides: GameSetupOverrides {
            snoop_enabled: Some(false),
            session_max_idle_minutes: Some(19),
            session_minimum_time_minutes: Some(7),
            session_local_timeout: Some(true),
            session_remote_timeout: Some(false),
            inactivity_autopilot_after_turns: Some(5),
        },
        reservations: Vec::new(),
    };

    let first = App::load(AppConfig {
        game_dir: fixture_dir.clone(),
        player_record_index_1_based: 1,
        export_root: None,
        queue_dir: None,
        session_timeout_secs: None,
        game_config: config.clone(),
    })
    .expect("app should load and persist config");
    assert_ne!(
        before_setup.snoop_enabled(),
        first.game_data.setup.snoop_enabled()
    );
    assert_ne!(
        before_setup.max_time_between_keys_minutes_raw(),
        first.game_data.setup.max_time_between_keys_minutes_raw()
    );
    assert_ne!(
        before_setup.minimum_time_granted_minutes_raw(),
        first.game_data.setup.minimum_time_granted_minutes_raw()
    );
    assert_ne!(
        before_setup.local_timeout_enabled(),
        first.game_data.setup.local_timeout_enabled()
    );
    assert_ne!(
        before_setup.remote_timeout_enabled(),
        first.game_data.setup.remote_timeout_enabled()
    );
    assert_ne!(
        before_setup.autopilot_inactive_turns_raw(),
        first.game_data.setup.autopilot_inactive_turns_raw()
    );

    let persisted = latest_runtime_state(&fixture_dir);
    let setup = &persisted.game_data.setup;
    assert!(!setup.snoop_enabled());
    assert_eq!(setup.max_time_between_keys_minutes_raw(), 19);
    assert_eq!(setup.minimum_time_granted_minutes_raw(), 7);
    assert!(setup.local_timeout_enabled());
    assert!(!setup.remote_timeout_enabled());
    assert_eq!(setup.autopilot_inactive_turns_raw(), 5);

    let second = App::load(AppConfig {
        game_dir: fixture_dir.clone(),
        player_record_index_1_based: 1,
        export_root: None,
        queue_dir: None,
        session_timeout_secs: None,
        game_config: config,
    })
    .expect("second load should reuse persisted config snapshot");
    assert_eq!(second.game_data.setup, first.game_data.setup);
}

#[test]
fn apply_action_quit_exits_loop() {
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

    assert_eq!(apply_action(&mut app, Action::Quit), AppOutcome::Quit);
    assert_eq!(
        app.current_screen(),
        ScreenId::Startup(StartupPhase::Splash)
    );
}

#[test]
fn main_menu_keys_open_existing_shared_screens_and_return_to_main() {
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
        app.handle_key(key(KeyCode::Char('b'))),
        Action::Empire(EmpireAction::OpenStatus)
    );
    assert_eq!(
        apply_action(&mut app, Action::Empire(EmpireAction::OpenStatus)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::EmpireStatus);
    assert_eq!(
        app.handle_key(key(KeyCode::Enter)),
        Action::ReturnToCommandMenu
    );
    assert_eq!(
        apply_action(&mut app, Action::ReturnToCommandMenu),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::MainMenu);

    assert_eq!(
        app.handle_key(key(KeyCode::Char('f'))),
        Action::Fleet(FleetAction::OpenMenu)
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::OpenMenu)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FleetMenu);
    assert_eq!(
        app.handle_key(key(KeyCode::Char('h'))),
        Action::OpenPopupHelp
    );
    assert_eq!(
        apply_action(&mut app, Action::OpenPopupHelp),
        AppOutcome::Continue
    );
    assert!(app.popup_help.is_some());
    assert_eq!(
        app.handle_key(key(KeyCode::Enter)),
        Action::DismissPopupHelp
    );
    assert_eq!(
        apply_action(&mut app, Action::DismissPopupHelp),
        AppOutcome::Continue
    );
    assert!(app.popup_help.is_none());
    assert_eq!(app.current_screen(), ScreenId::FleetMenu);
    assert_eq!(
        app.handle_key(key(KeyCode::Char('i'))),
        Action::Planet(PlanetAction::OpenInfoPrompt(CommandMenu::Fleet))
    );
    assert_eq!(
        apply_action(
            &mut app,
            Action::Planet(PlanetAction::OpenInfoPrompt(CommandMenu::Fleet))
        ),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FleetMenu);
    assert_eq!(
        app.handle_key(key(KeyCode::Char('q'))),
        Action::Planet(PlanetAction::CloseInfoPrompt)
    );
    assert_eq!(
        apply_action(&mut app, Action::Planet(PlanetAction::CloseInfoPrompt)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FleetMenu);
    assert_eq!(
        app.handle_key(key(KeyCode::Char('v'))),
        Action::Starmap(StarmapAction::OpenPartialView(CommandMenu::Fleet))
    );
    assert_eq!(
        apply_action(
            &mut app,
            Action::Starmap(StarmapAction::OpenPartialView(CommandMenu::Fleet))
        ),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::PartialStarmapView);
    assert_eq!(
        app.handle_key(key(KeyCode::Enter)),
        Action::Starmap(StarmapAction::OpenPlanetInfoAtCenter)
    );
    assert_eq!(
        apply_action(
            &mut app,
            Action::Starmap(StarmapAction::OpenPlanetInfoAtCenter)
        ),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::PlanetInfoDetail);
    assert_eq!(
        app.handle_key(key(KeyCode::Char(' '))),
        Action::ReturnToCommandMenu
    );
    assert_eq!(
        apply_action(&mut app, Action::ReturnToCommandMenu),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::PartialStarmapView);
    assert_eq!(
        app.handle_key(key(KeyCode::Char('q'))),
        Action::ReturnToCommandMenu
    );
    assert_eq!(
        apply_action(&mut app, Action::ReturnToCommandMenu),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FleetMenu);
    assert_eq!(
        app.handle_key(key(KeyCode::Char('c'))),
        Action::Fleet(FleetAction::OpenChangePrompt)
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::OpenChangePrompt)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FleetMenu);
    submit_fleet_menu_prompt(&mut app, Some(1));
    submit_fleet_menu_prompt_value(&mut app, "R");
    submit_fleet_menu_prompt_value(&mut app, "4");
    assert_eq!(app.current_fleet_roe_by_id(1), Some(4));
    assert_eq!(
        app.handle_key(key(KeyCode::Char('q'))),
        Action::OpenMainMenu
    );
    assert_eq!(
        apply_action(&mut app, Action::OpenMainMenu),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::MainMenu);
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::OpenMenu)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FleetMenu);
    assert_eq!(
        app.handle_key(key(KeyCode::Char('e'))),
        Action::Fleet(FleetAction::OpenEta)
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::OpenEta)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FleetMenu);
    assert_eq!(
        app.handle_key(key(KeyCode::Char('q'))),
        Action::Fleet(FleetAction::CancelMenuPrompt)
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::CancelMenuPrompt)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FleetMenu);
    assert_eq!(app.handle_key(key(KeyCode::Char('b'))), Action::Noop);
    assert_eq!(
        app.handle_key(key(KeyCode::Char('f'))),
        Action::Fleet(FleetAction::OpenList)
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::OpenList)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FleetList);
    assert_eq!(
        app.handle_key(key(KeyCode::Enter)),
        Action::Fleet(FleetAction::OpenReview)
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::OpenReview)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FleetReview);
    assert_eq!(
        app.handle_key(key(KeyCode::Esc)),
        Action::Fleet(FleetAction::CloseReview)
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::CloseReview)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FleetList);
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::OpenMenu)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FleetMenu);
    assert_eq!(
        app.handle_key(key(KeyCode::Char('r'))),
        Action::Fleet(FleetAction::OpenReviewPrompt)
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::OpenReviewPrompt)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FleetMenu);
    assert_eq!(
        app.handle_key(key(KeyCode::Char('7'))),
        Action::Fleet(FleetAction::AppendMenuPromptChar('7'))
    );
    assert_eq!(
        app.handle_key(key(KeyCode::Backspace)),
        Action::Fleet(FleetAction::BackspaceMenuPromptInput)
    );
    assert_eq!(
        app.handle_key(key(KeyCode::Enter)),
        Action::Fleet(FleetAction::SubmitMenuPrompt)
    );
    assert_eq!(
        app.handle_key(key(KeyCode::Char('q'))),
        Action::Fleet(FleetAction::CancelMenuPrompt)
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::OpenMenu)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FleetMenu);
    assert_eq!(
        app.handle_key(key(KeyCode::Char('q'))),
        Action::OpenMainMenu
    );
    assert_eq!(
        apply_action(&mut app, Action::OpenMainMenu),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::MainMenu);

    assert_eq!(
        app.handle_key(key(KeyCode::Char('i'))),
        Action::Planet(PlanetAction::OpenInfoPrompt(CommandMenu::Main))
    );
    assert_eq!(
        apply_action(
            &mut app,
            Action::Planet(PlanetAction::OpenInfoPrompt(CommandMenu::Main))
        ),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::MainMenu);
    assert_eq!(
        app.handle_key(key(KeyCode::Char('q'))),
        Action::Planet(PlanetAction::CloseInfoPrompt)
    );
    assert_eq!(
        apply_action(&mut app, Action::Planet(PlanetAction::CloseInfoPrompt)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::MainMenu);

    assert_eq!(
        apply_action(
            &mut app,
            Action::Planet(PlanetAction::OpenInfoPrompt(CommandMenu::Main))
        ),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::MainMenu);
    assert_eq!(
        apply_action(&mut app, Action::Planet(PlanetAction::SubmitInfoPrompt)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::PlanetInfoDetail);
    assert_eq!(
        app.handle_key(key(KeyCode::Enter)),
        Action::ReturnToCommandMenu
    );
    assert_eq!(
        apply_action(&mut app, Action::ReturnToCommandMenu),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::MainMenu);

    assert_eq!(
        app.handle_key(key(KeyCode::Char('v'))),
        Action::Starmap(StarmapAction::OpenPartialView(CommandMenu::Main))
    );
    assert_eq!(
        apply_action(
            &mut app,
            Action::Starmap(StarmapAction::OpenPartialView(CommandMenu::Main))
        ),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::PartialStarmapView);
    assert_eq!(
        app.handle_key(key(KeyCode::Enter)),
        Action::Starmap(StarmapAction::OpenPlanetInfoAtCenter)
    );
    assert_eq!(
        apply_action(
            &mut app,
            Action::Starmap(StarmapAction::OpenPlanetInfoAtCenter)
        ),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::PlanetInfoDetail);
    assert_eq!(
        apply_action(&mut app, Action::ReturnToCommandMenu),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::PartialStarmapView);
    assert_eq!(
        app.handle_key(key(KeyCode::Esc)),
        Action::ReturnToCommandMenu
    );
    assert_eq!(
        apply_action(&mut app, Action::ReturnToCommandMenu),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::MainMenu);

    assert_eq!(
        app.handle_key(key(KeyCode::Char('t'))),
        Action::Planet(PlanetAction::OpenDatabase)
    );
    assert_eq!(
        apply_action(&mut app, Action::Planet(PlanetAction::OpenDatabase)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::PlanetDatabaseList);
    assert_eq!(
        app.handle_key(key(KeyCode::Enter)),
        Action::Planet(PlanetAction::SubmitDatabaseLookup)
    );
    assert_eq!(
        apply_action(&mut app, Action::Planet(PlanetAction::SubmitDatabaseLookup)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::PlanetInfoDetail);
    assert_eq!(
        app.handle_key(key(KeyCode::Char('q'))),
        Action::ReturnToCommandMenu
    );
    assert_eq!(
        apply_action(&mut app, Action::ReturnToCommandMenu),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::PlanetDatabaseList);
    assert_eq!(
        app.handle_key(key(KeyCode::Char('q'))),
        Action::ReturnToCommandMenu
    );
    assert_eq!(
        apply_action(&mut app, Action::ReturnToCommandMenu),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::MainMenu);
}

#[test]
fn fleet_review_detail_from_menu_uses_dismiss_prompt_and_returns_to_menu() {
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
        apply_action(&mut app, Action::Fleet(FleetAction::OpenReviewPrompt)),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::SubmitMenuPrompt)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FleetReview);
    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("fleet review should render");
    assert!(
        terminal
            .lines
            .iter()
            .any(|line| line.contains("(slap a key)"))
    );
    assert!(
        terminal
            .lines
            .iter()
            .all(|line| !line.contains("Fleet Record #:"))
    );
    assert_eq!(terminal.line(10).trim_end(), "");
    assert_eq!(terminal.line(11).trim_end(), " (slap a key)");
    assert_eq!(
        app.handle_key(key(KeyCode::Char('x'))),
        Action::Fleet(FleetAction::CloseReview)
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::CloseReview)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FleetMenu);
    assert_eq!(
        app.handle_key(key(KeyCode::Char('r'))),
        Action::Fleet(FleetAction::OpenReviewPrompt)
    );
}

#[test]
fn fleet_review_from_list_uses_dismiss_prompt_and_any_key_returns_to_list() {
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
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::OpenReview)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FleetReview);

    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("fleet review should render");
    assert!(
        terminal
            .lines
            .iter()
            .all(|line| !line.contains("Fleet Record #:"))
    );
    assert_eq!(terminal.line(10).trim_end(), "");
    assert_eq!(terminal.line(11).trim_end(), " (slap a key)");
    assert!(!terminal.line(11).contains("COMMAND <-"));

    assert_eq!(
        app.handle_key(key(KeyCode::Left)),
        Action::Fleet(FleetAction::CloseReview)
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::CloseReview)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FleetList);
}

#[test]
fn fleet_menu_matches_verified_v15_command_layout() {
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

    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal).expect("fleet menu should render");
    assert_eq!(terminal.line(1).trim_end(), " FLEET COMMAND CENTER:");
    assert_eq!(
        terminal.line(2).trim_end(),
        "  H>elp on Options   S>TARBASE MENU...   C>hg ROE,ID,Speed   O>rder a Fleet"
    );
    assert_eq!(
        terminal.line(3).trim_end(),
        "  Q>uit: Main Menu   E>TA Calc           I>nfo about Planet  M>erge a Fleet"
    );
    assert_eq!(
        terminal.line(4).trim_end(),
        "  X>pert Mode        F>LEET LIST         D>etach Ships       L>oad TTs w/Armies"
    );
    assert_eq!(
        terminal.line(5).trim_end(),
        "  V>iew Partial Map  R>eview a Fleet     T>ransfer Ships     U>nload TT Armies"
    );
}

#[test]
fn starbase_menu_matches_verified_v15_command_layout() {
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
        apply_action(&mut app, Action::Starbase(StarbaseAction::OpenMenu)),
        AppOutcome::Continue
    );

    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("starbase menu should render");
    assert_eq!(
        terminal.line(0).trim_end(),
        "STARBASE CONTROL:            X>pert mode ON/OFF     V>iew Partial Star Map"
    );
    assert_eq!(
        terminal.line(1).trim_end(),
        "  H>elp with commands        S>tarbases-List        I>nfo about a Planet"
    );
    assert_eq!(
        terminal.line(2).trim_end(),
        "  Q>uit to Fleet Command     R>eview a Starbase     M>ove/Halt Starbase"
    );
    assert_eq!(
        line_containing(&terminal, "STARBASE COMMAND <-").trim_end(),
        " STARBASE COMMAND <- ? X S R V I M <Q> ->"
    );
}

#[test]
fn starbase_review_matches_verified_v15_review_content() {
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
        apply_action(&mut app, Action::Starbase(StarbaseAction::OpenMenu)),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Starbase(StarbaseAction::OpenReviewSelect)),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(
            &mut app,
            Action::Starbase(StarbaseAction::SubmitReviewSelect)
        ),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::StarbaseReview);

    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("starbase review should render");
    assert_eq!(terminal.line(3).trim_end(), " Starbase ID: Starbase 1");
    assert_eq!(
        terminal.line(4).trim_end(),
        " Location:    World in Solar System [ 6, 5]"
    );
    assert_eq!(
        terminal.line(5).trim_end(),
        " Destination: World in Solar System [ 6, 5]"
    );
    assert_eq!(
        terminal.line(6).trim_end(),
        " Operation:   Protection & Enhancement"
    );
    assert_eq!(
        terminal.line(7).trim_end(),
        " ETA:         Starbase 1 has already arrived and is in operation."
    );
    assert_eq!(terminal.line(8).trim_end(), " Escort:      The 1st Fleet");
}

#[test]
fn starbase_list_and_review_ignore_stale_escort_linkage_without_live_guard_orders() {
    let fixture_dir = temp_game_with_starbase_copy();
    let mut runtime = latest_runtime_state(&fixture_dir);
    runtime.game_data.fleets.records[0].set_standing_order_kind(nc_data::Order::HoldPosition);
    runtime.game_data.fleets.records[0].set_standing_order_target_coords_raw([6, 5]);
    runtime.game_data.fleets.records[0].set_mission_aux_bytes([0, 0]);
    runtime.game_data.fleets.records[0].set_current_location_coords_raw([6, 5]);
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
        apply_action(&mut app, Action::Fleet(FleetAction::OpenMenu)),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Starbase(StarbaseAction::OpenMenu)),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Starbase(StarbaseAction::OpenList)),
        AppOutcome::Continue
    );

    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("starbase list should render");
    let list_row = line_containing(&terminal, "System(06,05)");
    assert!(list_row.contains("N/A"));

    assert_eq!(
        apply_action(&mut app, Action::Starbase(StarbaseAction::OpenReview)),
        AppOutcome::Continue
    );
    app.render(&mut terminal)
        .expect("starbase review should render");
    assert_eq!(terminal.line(8).trim_end(), " Escort:      N/A");
}

#[test]
fn starbase_list_summarizes_multiple_live_guard_fleets_and_review_lists_them() {
    let fixture_dir = temp_game_with_starbase_copy();
    let mut runtime = latest_runtime_state(&fixture_dir);
    runtime.game_data.fleets.records[1].set_current_location_coords_raw([6, 5]);
    runtime.game_data.fleets.records[1].set_standing_order_kind(nc_data::Order::GuardStarbase);
    runtime.game_data.fleets.records[1].set_standing_order_target_coords_raw([6, 5]);
    runtime.game_data.fleets.records[1].set_mission_aux_bytes([1, 1]);
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
        apply_action(&mut app, Action::Fleet(FleetAction::OpenMenu)),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Starbase(StarbaseAction::OpenMenu)),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Starbase(StarbaseAction::OpenList)),
        AppOutcome::Continue
    );

    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("starbase list should render");
    let list_row = line_containing(&terminal, "System(06,05)");
    assert!(list_row.contains("2 guards"));

    assert_eq!(
        apply_action(&mut app, Action::Starbase(StarbaseAction::OpenReview)),
        AppOutcome::Continue
    );
    app.render(&mut terminal)
        .expect("starbase review should render");
    assert_eq!(
        terminal.line(8).trim_end(),
        " Escort:      Guard Fleets 1 and 2"
    );
}

#[test]
fn starbase_list_wraps_selection_from_top_to_bottom_and_bottom_to_top() {
    let fixture_dir = temp_game_with_starbase_copy();
    let mut runtime = latest_runtime_state(&fixture_dir);
    {
        let mut base = runtime.game_data.bases.records[0].clone();
        base.set_base_id_raw(2);
        base.set_owner_empire_raw(1);
        base.set_coords_raw([9, 9]);
        base.set_target_coords_raw([9, 9]);
        runtime.game_data.bases.records.push(base);
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
    advance_to_main_menu(&mut app);
    assert_eq!(
        apply_action(&mut app, Action::Starbase(StarbaseAction::OpenMenu)),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Starbase(StarbaseAction::OpenList)),
        AppOutcome::Continue
    );

    assert_eq!(app.starbase.cursor, 0);
    assert_eq!(
        apply_action(&mut app, Action::Starbase(StarbaseAction::MoveSelect(-1))),
        AppOutcome::Continue
    );
    assert_eq!(app.starbase.cursor, 1);

    assert_eq!(
        apply_action(&mut app, Action::Starbase(StarbaseAction::MoveSelect(1))),
        AppOutcome::Continue
    );
    assert_eq!(app.starbase.cursor, 0);
}

#[test]
fn starbase_list_and_review_show_numeric_transit_eta() {
    let fixture_dir = temp_game_with_starbase_copy();
    let mut runtime = latest_runtime_state(&fixture_dir);
    runtime.game_data.bases.records[0].set_coords_raw([15, 13]);
    runtime.game_data.bases.records[0].set_target_coords_raw([2, 12]);
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
        apply_action(&mut app, Action::Fleet(FleetAction::OpenMenu)),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Starbase(StarbaseAction::OpenMenu)),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Starbase(StarbaseAction::OpenList)),
        AppOutcome::Continue
    );

    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("starbase list should render");
    let list_row = line_containing(&terminal, "System(15,13)");
    assert!(list_row.contains("System(15,13)"));
    assert!(list_row.contains("System(02,12)"));
    assert!(list_row.contains("16"));
    assert!(list_row.contains("Starbase in transit"));

    assert_eq!(
        apply_action(&mut app, Action::Starbase(StarbaseAction::OpenReview)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::StarbaseReview);

    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("starbase review should render");
    assert_eq!(
        terminal.line(7).trim_end(),
        " ETA:         Starbase 1 is in transit with ETA 16 years."
    );
}

#[test]
fn starbase_move_prompt_accepts_non_planet_sector_and_persists_report() {
    let fixture_dir = temp_game_with_starbase_copy();
    let mut runtime = latest_runtime_state(&fixture_dir);
    runtime.game_data.fleets.records[1].set_current_location_coords_raw([6, 5]);
    runtime.game_data.fleets.records[1].set_standing_order_kind(nc_data::Order::GuardStarbase);
    runtime.game_data.fleets.records[1].set_standing_order_target_coords_raw([6, 5]);
    runtime.game_data.fleets.records[1].set_mission_aux_bytes([1, 1]);
    save_runtime_state(&fixture_dir, &runtime);
    let destination = first_empty_sector(&runtime.game_data);

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
    apply_action(&mut app, Action::Fleet(FleetAction::OpenMenu));
    apply_action(&mut app, Action::Starbase(StarbaseAction::OpenMenu));
    apply_action(&mut app, Action::Starbase(StarbaseAction::OpenMovePrompt));

    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("starbase move base prompt should render");
    assert!(
        line_containing(&terminal, "STARBASE COMMAND <- Starbase #")
            .contains("Starbase # [1] <Q> ->")
    );

    apply_action(&mut app, Action::Starbase(StarbaseAction::SubmitMovePrompt));
    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("starbase move decision prompt should render");
    assert!(
        line_containing(&terminal, "STARBASE COMMAND <- <H>alt or")
            .contains("<H>alt or [M]ove <Q> ->")
    );

    apply_action(&mut app, Action::Starbase(StarbaseAction::SubmitMovePrompt));
    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("starbase move destination prompt should render");
    assert!(
        line_containing(&terminal, "STARBASE COMMAND <- Destination")
            .contains("Destination [06,05] <Q> ->")
    );

    for ch in format!("{:02},{:02}", destination[0], destination[1]).chars() {
        apply_action(
            &mut app,
            Action::Starbase(StarbaseAction::AppendMovePromptChar(ch)),
        );
    }
    apply_action(&mut app, Action::Starbase(StarbaseAction::SubmitMovePrompt));

    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("starbase menu notice should render after move");
    assert!(line_containing(&terminal, "Notice: ").contains(&format!(
        "Starbase #1 ordered to move to ({:02},{:02}).",
        destination[0], destination[1]
    )));

    let persisted = latest_runtime_state(&fixture_dir);
    assert_eq!(
        persisted.game_data.bases.records[0].coords_raw(),
        [6, 5],
        "live base location should stay unchanged until movement execution exists"
    );
    assert_eq!(
        persisted.game_data.bases.records[0].target_coords_raw(),
        destination
    );
    let latest_report = persisted
        .report_block_rows
        .last()
        .expect("move should append a report block");
    assert!(latest_report.decoded_text.contains(&format!(
        "Starbase 1 is moving to ({:02},{:02}).",
        destination[0], destination[1]
    )));
    assert!(
        latest_report
            .decoded_text
            .contains("Guard Fleets 1 and 2 will follow it.")
    );
}

#[test]
fn starbase_help_uses_move_wording_not_hauling() {
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
    for _ in 0..64 {
        if app.current_screen() == ScreenId::MainMenu {
            break;
        }
        app.advance_startup();
    }
    assert_eq!(app.current_screen(), ScreenId::MainMenu);
    apply_action(&mut app, Action::Fleet(FleetAction::OpenMenu));
    apply_action(&mut app, Action::Starbase(StarbaseAction::OpenMenu));
    apply_action(&mut app, Action::OpenPopupHelp);

    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("starbase help should render");
    assert!(
        line_containing(&terminal, "move to a new location").contains("move to a new location")
    );
    assert!(
        !terminal.lines.iter().any(|line| line.contains("hauled")),
        "player-facing help should not mention hauling"
    );
}

#[test]
fn planet_build_list_help_mentions_delete_hotkey() {
    let root = temp_game_copy();
    let config = AppConfig {
        game_dir: root.clone(),
        player_record_index_1_based: 1,
        export_root: None,
        queue_dir: None,
        session_timeout_secs: None,
        game_config: GameConfig::default(),
    };
    let mut app = App::load(config).expect("load app");
    app.current_screen = ScreenId::PlanetBuildList;
    apply_action(&mut app, Action::OpenPopupHelp);

    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("planet build list help should render");
    let delete_line = line_containing(&terminal, "delete highlighted build order");
    assert!(
        delete_line.contains('D'),
        "planet build list helper should advertise the delete hotkey as its own row"
    );
}

#[test]
fn fleet_list_help_mentions_row_actions_without_review_hotkey() {
    let root = temp_game_copy();
    let config = AppConfig {
        game_dir: root.clone(),
        player_record_index_1_based: 1,
        export_root: None,
        queue_dir: None,
        session_timeout_secs: None,
        game_config: GameConfig::default(),
    };
    let mut app = App::load(config).expect("load app");
    app.current_screen = ScreenId::FleetList;
    apply_action(&mut app, Action::OpenPopupHelp);

    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("fleet list help should render");
    assert!(
        line_containing(&terminal, "Filter List").contains("F"),
        "fleet list helper should advertise F filter"
    );
    assert!(
        line_containing(&terminal, "Sort List").contains("S"),
        "fleet list helper should advertise S sort"
    );
    assert!(
        line_containing(&terminal, "review highlighted fleet").contains("Enter"),
        "fleet list helper should advertise Enter review"
    );
    assert!(
        line_containing(&terminal, "assign orders").contains("O"),
        "fleet list helper should advertise O for orders"
    );
    assert!(
        line_containing(&terminal, "change checked fleets").contains("C"),
        "fleet list helper should advertise C for change"
    );
    assert!(
        line_containing(&terminal, "travel time").contains("E"),
        "fleet list helper should advertise E for ETA"
    );
    assert!(
        line_containing(&terminal, "merge checked fleets").contains("M"),
        "fleet list helper should advertise M for merge"
    );
    assert!(
        !terminal
            .lines
            .iter()
            .any(|line| line.contains("R") && line.contains("review highlighted fleet")),
        "fleet list helper should not advertise a redundant R review hotkey"
    );
}

#[test]
fn planet_list_help_mentions_row_actions_and_sort_hotkey() {
    let root = temp_game_copy();
    let config = AppConfig {
        game_dir: root.clone(),
        player_record_index_1_based: 1,
        export_root: None,
        queue_dir: None,
        session_timeout_secs: None,
        game_config: GameConfig::default(),
    };
    let mut app = App::load(config).expect("load app");
    app.current_screen = ScreenId::PlanetList(PlanetListMode::Brief, PlanetListSort::Location);
    apply_action(&mut app, Action::OpenPopupHelp);

    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("planet list help should render");
    assert!(
        line_containing(&terminal, "Filter List").contains("F"),
        "planet list helper should advertise F filter"
    );
    assert!(
        line_containing(&terminal, "Sort List").contains("S"),
        "planet list helper should advertise S sort"
    );
    assert!(
        line_containing(&terminal, "review highlighted planet").contains("I"),
        "planet list helper should advertise I review"
    );
    assert!(
        terminal
            .lines
            .iter()
            .any(|line| line.contains("Enter") && line.contains("review highlighted planet")),
        "planet list helper should advertise Enter review on its own row"
    );
    assert!(
        line_containing(&terminal, "new build orders").contains("B"),
        "planet list helper should advertise B for build specify"
    );
    assert!(
        line_containing(&terminal, "queued build orders").contains("D"),
        "planet list helper should advertise D for display queue"
    );
    assert!(
        line_containing(&terminal, "abort queued build orders").contains("A"),
        "planet list helper should advertise A for build abort"
    );
    assert!(
        line_containing(&terminal, "manually commission").contains("C"),
        "planet list helper should advertise C for commission"
    );
    assert!(
        line_containing(&terminal, "mass commission").contains("M"),
        "planet list helper should advertise M for mass commission"
    );
    assert!(
        line_containing(&terminal, "load armies").contains("L"),
        "planet list helper should advertise L for load"
    );
    assert!(
        line_containing(&terminal, "scorch earth").contains("X"),
        "planet list helper should advertise X for scorch"
    );
}

#[test]
fn planet_database_help_shortens_sort_and_filter_hotkeys() {
    let root = temp_game_copy();
    let config = AppConfig {
        game_dir: root.clone(),
        player_record_index_1_based: 1,
        export_root: None,
        queue_dir: None,
        session_timeout_secs: None,
        game_config: GameConfig::default(),
    };
    let mut app = App::load(config).expect("load app");
    app.current_screen = ScreenId::PlanetDatabaseList;
    apply_action(&mut app, Action::OpenPopupHelp);

    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("planet database help should render");
    assert!(
        line_containing(&terminal, "Filter List").contains("F"),
        "planet database helper should advertise F filter"
    );
    assert!(
        line_containing(&terminal, "Sort List").contains("S"),
        "planet database helper should advertise S sort"
    );
}

#[test]
fn fleet_list_change_prompt_uses_overlay_keys_and_returns_to_list() {
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
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::OpenChangePrompt)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FleetList);
    assert_eq!(
        app.handle_key(key(KeyCode::Char('q'))),
        Action::Fleet(FleetAction::CancelMenuPrompt)
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::CancelMenuPrompt)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FleetList);

    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::OpenChangePrompt)),
        AppOutcome::Continue
    );
    assert_eq!(
        app.handle_key(key(KeyCode::Char('r'))),
        Action::Fleet(FleetAction::AppendMenuPromptChar('r'))
    );
    assert_eq!(
        apply_action(
            &mut app,
            Action::Fleet(FleetAction::AppendMenuPromptChar('r'))
        ),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FleetList);
    assert_eq!(
        app.handle_key(key(KeyCode::Backspace)),
        Action::Fleet(FleetAction::BackspaceMenuPromptInput)
    );
    assert_eq!(
        app.handle_key(key(KeyCode::Char('0'))),
        Action::Fleet(FleetAction::AppendMenuPromptChar('0'))
    );
    assert_eq!(
        apply_action(
            &mut app,
            Action::Fleet(FleetAction::AppendMenuPromptChar('0'))
        ),
        AppOutcome::Continue
    );
    assert_eq!(
        app.handle_key(key(KeyCode::Enter)),
        Action::Fleet(FleetAction::SubmitMenuPrompt)
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::SubmitMenuPrompt)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FleetList);

    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("fleet list should render updated roe value");
    assert!(
        terminal
            .lines
            .iter()
            .any(|line| line.contains("│  0│") && line.contains("Guard/Blkd")),
        "{:#?}",
        terminal.lines
    );
    assert!(
        terminal
            .lines
            .iter()
            .all(|line| !line.contains("ROE set to 0."))
    );
}

#[test]
fn fleet_list_order_success_returns_to_list() {
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
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::OpenOrder)),
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
    assert_eq!(app.current_screen(), ScreenId::FleetOrder);
    enter_fleet_order_target(&mut app, [14, 9]);
    confirm_fleet_order(&mut app, true);
    assert_eq!(app.current_screen(), ScreenId::FleetList);

    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("fleet list should render updated order state");
    assert!(
        terminal
            .lines
            .iter()
            .any(|line| line.contains("Move") && line.contains("(14,09)")),
        "{:#?}",
        terminal.lines
    );
    assert!(
        terminal
            .lines
            .iter()
            .all(|line| !line.contains("Applied move to Fleet #")),
        "{:#?}",
        terminal.lines
    );
}

#[test]
fn fleet_eta_result_dismiss_returns_to_fleet_list_when_launched_from_list() {
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
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::OpenEta)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FleetEta);
    for ch in ['1', '0', ',', '1', '3'] {
        assert_eq!(
            apply_action(&mut app, Action::Fleet(FleetAction::AppendEtaChar(ch))),
            AppOutcome::Continue
        );
    }
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::SubmitEta)),
        AppOutcome::Continue
    );
    assert_eq!(
        app.fleet.eta_mode,
        nc_game::screen::FleetEtaMode::ConfirmingSystemEntry
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::AppendEtaChar('y'))),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FleetEta);
    assert_eq!(
        app.fleet.eta_mode,
        nc_game::screen::FleetEtaMode::ShowingResult
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::SubmitEta)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FleetList);
}

#[test]
fn fleet_list_transfer_cancel_stays_in_fleet_flow() {
    let fixture_dir = temp_game_with_same_sector_fleets_copy();
    let mut state = latest_runtime_state(&fixture_dir);
    let donor = state
        .game_data
        .fleets
        .records
        .iter_mut()
        .find(|fleet| fleet.owner_empire_raw() == 1 && fleet.local_slot_word_raw() == 4)
        .expect("fleet #4 should exist");
    donor.set_battleship_count(0);
    donor.set_cruiser_count(0);
    donor.set_destroyer_count(0);
    donor.set_troop_transport_count(2);
    donor.set_army_count(0);
    donor.set_scout_count(0);
    donor.set_etac_count(0);
    donor.recompute_max_speed_from_composition();
    let host = state
        .game_data
        .fleets
        .records
        .iter_mut()
        .find(|fleet| fleet.owner_empire_raw() == 1 && fleet.local_slot_word_raw() == 3)
        .expect("fleet #3 should exist");
    host.set_battleship_count(0);
    host.set_cruiser_count(0);
    host.set_destroyer_count(1);
    host.set_troop_transport_count(0);
    host.set_army_count(0);
    host.set_scout_count(0);
    host.set_etac_count(0);
    host.recompute_max_speed_from_composition();
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
        apply_action(&mut app, Action::Fleet(FleetAction::OpenList)),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::OpenTransfer)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FleetList);
    assert_eq!(
        app.fleet.menu_prompt_mode,
        Some(nc_game::domains::fleet::state::FleetMenuPromptMode::TransferHost)
    );
    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("fleet list transfer host prompt should render");
    assert!(
        line_containing(&terminal, "COMMAND <- Transfer To Fleet #")
            .contains("Transfer To Fleet # ["),
        "{:#?}",
        terminal.lines
    );
    assert_eq!(
        app.handle_key(key(KeyCode::Char('q'))),
        Action::Fleet(FleetAction::CancelMenuPrompt)
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::CancelMenuPrompt)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FleetList);

    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::OpenTransfer)),
        AppOutcome::Continue
    );
    submit_fleet_menu_prompt(&mut app, Some(3));
    assert_eq!(app.current_screen(), ScreenId::FleetTransfer);
    assert_eq!(
        app.handle_key(key(KeyCode::Char('q'))),
        Action::Fleet(FleetAction::CancelTransfer)
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::CancelTransfer)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FleetList);
}

#[test]
fn fleet_list_transfer_host_error_uses_slap_a_key_latch_and_preserves_prompt() {
    let fixture_dir = temp_game_with_same_sector_fleets_copy();
    let mut state = latest_runtime_state(&fixture_dir);
    let donor = state
        .game_data
        .fleets
        .records
        .iter_mut()
        .find(|fleet| fleet.owner_empire_raw() == 1 && fleet.local_slot_word_raw() == 4)
        .expect("fleet #4 should exist");
    donor.set_battleship_count(0);
    donor.set_cruiser_count(0);
    donor.set_destroyer_count(0);
    donor.set_troop_transport_count(2);
    donor.set_army_count(0);
    donor.set_scout_count(0);
    donor.set_etac_count(0);
    donor.recompute_max_speed_from_composition();
    let host = state
        .game_data
        .fleets
        .records
        .iter_mut()
        .find(|fleet| fleet.owner_empire_raw() == 1 && fleet.local_slot_word_raw() == 3)
        .expect("fleet #3 should exist");
    host.set_battleship_count(0);
    host.set_cruiser_count(0);
    host.set_destroyer_count(1);
    host.set_troop_transport_count(0);
    host.set_army_count(0);
    host.set_scout_count(0);
    host.set_etac_count(0);
    host.recompute_max_speed_from_composition();
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
    apply_action(&mut app, Action::Fleet(FleetAction::OpenMenu));
    apply_action(&mut app, Action::Fleet(FleetAction::OpenList));
    apply_action(&mut app, Action::Fleet(FleetAction::OpenTransfer));
    submit_fleet_menu_prompt(&mut app, Some(99));
    assert_eq!(app.current_screen(), ScreenId::FleetList);
    assert_eq!(
        app.fleet.menu_prompt_mode,
        Some(nc_game::domains::fleet::state::FleetMenuPromptMode::TransferHost)
    );

    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("fleet list transfer error should render");
    let command_line = line_containing(&terminal, "COMMAND <-");
    assert!(command_line.contains("(slap a key)"));
    assert!(command_line.contains("Fleet #99 is not in your fleet list."));

    assert_eq!(
        app.handle_key(key(KeyCode::Char('?'))),
        Action::Fleet(FleetAction::DismissMessage)
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::DismissMessage)),
        AppOutcome::Continue
    );
    assert!(app.popup_help.is_none());
    assert_eq!(app.current_screen(), ScreenId::FleetList);
    assert_eq!(
        app.fleet.menu_prompt_mode,
        Some(nc_game::domains::fleet::state::FleetMenuPromptMode::TransferHost)
    );

    terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("fleet list transfer host prompt should return after dismiss");
    assert!(
        line_containing(&terminal, "COMMAND <- Transfer To Fleet #")
            .contains("Transfer To Fleet # [")
    );
}

#[test]
fn fleet_list_transfer_donor_validation_uses_short_footer_notice_for_one_ship_fleet() {
    let fixture_dir = temp_game_with_same_sector_fleets_copy();
    let mut state = latest_runtime_state(&fixture_dir);
    let donor = state
        .game_data
        .fleets
        .records
        .iter_mut()
        .find(|fleet| fleet.owner_empire_raw() == 1 && fleet.local_slot_word_raw() == 4)
        .expect("fleet #4 should exist");
    donor.set_battleship_count(0);
    donor.set_cruiser_count(0);
    donor.set_destroyer_count(0);
    donor.set_troop_transport_count(0);
    donor.set_army_count(0);
    donor.set_scout_count(1);
    donor.set_etac_count(0);
    donor.recompute_max_speed_from_composition();
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
    apply_action(&mut app, Action::Fleet(FleetAction::OpenMenu));
    apply_action(&mut app, Action::Fleet(FleetAction::OpenList));
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::OpenTransfer)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FleetList);
    assert!(app.fleet.menu_prompt_mode.is_none());

    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("fleet list transfer donor error should render");
    let command_line = line_containing(&terminal, "COMMAND <-");
    assert!(command_line.contains("(slap a key)"));
    assert!(command_line.contains("Use merge instead"));

    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::DismissMessage)),
        AppOutcome::Continue
    );
    terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("fleet list should return after dismiss");
    assert!(!line_containing(&terminal, "COMMAND <-").contains("(slap a key)"));
}

#[test]
fn fleet_list_transfer_donor_validation_uses_short_footer_notice_without_host_fleet() {
    let fixture_dir = temp_game_copy();
    let mut state = latest_runtime_state(&fixture_dir);
    let donor = state
        .game_data
        .fleets
        .records
        .iter_mut()
        .find(|fleet| fleet.owner_empire_raw() == 1 && fleet.local_slot_word_raw() == 4)
        .expect("fleet #4 should exist");
    donor.set_current_location_coords_raw([1, 1]);
    donor.set_standing_order_target_coords_raw([1, 1]);
    donor.set_battleship_count(0);
    donor.set_cruiser_count(0);
    donor.set_destroyer_count(0);
    donor.set_troop_transport_count(2);
    donor.set_army_count(0);
    donor.set_scout_count(0);
    donor.set_etac_count(0);
    donor.recompute_max_speed_from_composition();
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
    apply_action(&mut app, Action::Fleet(FleetAction::OpenMenu));
    apply_action(&mut app, Action::Fleet(FleetAction::OpenList));
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::OpenTransfer)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FleetList);
    assert!(app.fleet.menu_prompt_mode.is_none());

    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("fleet list transfer donor error should render");
    let command_line = line_containing(&terminal, "COMMAND <-");
    assert!(command_line.contains("(slap a key)"));
    assert!(command_line.contains("Unable to transfer"));
}

#[test]
fn starbase_move_prompt_errors_hang_directly_under_command_line() {
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
    apply_action(&mut app, Action::Fleet(FleetAction::OpenMenu));
    apply_action(&mut app, Action::Starbase(StarbaseAction::OpenMenu));
    apply_action(&mut app, Action::Starbase(StarbaseAction::OpenMovePrompt));
    apply_action(&mut app, Action::Starbase(StarbaseAction::SubmitMovePrompt));
    apply_action(&mut app, Action::Starbase(StarbaseAction::SubmitMovePrompt));
    for ch in "99,99".chars() {
        apply_action(
            &mut app,
            Action::Starbase(StarbaseAction::AppendMovePromptChar(ch)),
        );
    }
    apply_action(&mut app, Action::Starbase(StarbaseAction::SubmitMovePrompt));

    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("starbase move error should render");
    let error_line = terminal
        .lines
        .iter()
        .find(|line| line.contains("Error: "))
        .expect("error hanger should render");
    assert!(error_line.contains("Enter coordinates within 1.."));
    let error_row = terminal
        .lines
        .iter()
        .position(|line| line.contains("Error: "))
        .expect("error hanger should render");
    assert_eq!(error_row, 6, "error hanger should sit under the prompt");
}

#[test]
fn starbase_halt_prompt_resets_destination_to_live_location() {
    let fixture_dir = temp_game_with_starbase_copy();
    let mut runtime = latest_runtime_state(&fixture_dir);
    runtime.game_data.bases.records[0].set_target_coords_raw([2, 12]);
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
    apply_action(&mut app, Action::Fleet(FleetAction::OpenMenu));
    apply_action(&mut app, Action::Starbase(StarbaseAction::OpenMenu));
    apply_action(&mut app, Action::Starbase(StarbaseAction::OpenMovePrompt));
    apply_action(&mut app, Action::Starbase(StarbaseAction::SubmitMovePrompt));
    apply_action(
        &mut app,
        Action::Starbase(StarbaseAction::AppendMovePromptChar('H')),
    );
    apply_action(&mut app, Action::Starbase(StarbaseAction::SubmitMovePrompt));

    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("halt confirm should render");
    assert!(
        line_containing(&terminal, "STARBASE COMMAND <- Halt this starbase?")
            .contains("Halt this starbase? [Y]/N ->")
    );

    apply_action(&mut app, Action::Starbase(StarbaseAction::SubmitMovePrompt));
    let persisted = latest_runtime_state(&fixture_dir);
    assert_eq!(
        persisted.game_data.bases.records[0].target_coords_raw(),
        [6, 5]
    );

    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("starbase halt notice should render");
    assert!(line_containing(&terminal, "Notice: ").contains("Starbase #1 halted at (06,05)."));
}

#[test]
fn fleet_transfer_uses_two_inline_fleet_prompts_before_quantity_entry() {
    let fixture_dir = temp_game_with_same_sector_fleets_copy();
    let mut state = latest_runtime_state(&fixture_dir);
    state.game_data.fleets.records[0].set_troop_transport_count(2);
    state.game_data.fleets.records[0].set_army_count(2);
    state.game_data.fleets.records[0].recompute_max_speed_from_composition();
    state.game_data.fleets.records[1].set_troop_transport_count(1);
    state.game_data.fleets.records[1].set_army_count(1);
    state.game_data.fleets.records[1].recompute_max_speed_from_composition();
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
        apply_action(&mut app, Action::Fleet(FleetAction::OpenTransfer)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FleetMenu);

    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("transfer donor prompt should render");
    assert!(
        line_containing(&terminal, "COMMAND <- Transfer From Fleet #")
            .contains("Transfer From Fleet # [")
    );

    submit_fleet_menu_prompt(&mut app, Some(1));
    assert_eq!(app.current_screen(), ScreenId::FleetMenu);
    app.render(&mut terminal)
        .expect("transfer host prompt should render");
    assert!(
        line_containing(&terminal, "COMMAND <- Transfer To Fleet #")
            .contains("Transfer To Fleet # [")
    );

    submit_fleet_menu_prompt(&mut app, Some(2));
    assert_eq!(app.current_screen(), ScreenId::FleetTransfer);
    app.render(&mut terminal)
        .expect("transfer quantity screen should render");
    assert_eq!(
        terminal.line(0).trim_end(),
        "TRANSFER SHIPS BETWEEN FLEETS:"
    );
    assert!(line_containing(&terminal, "Source Fleet: Fleet #1").contains("Source Fleet:"));
    assert!(
        line_containing(&terminal, "Destination Fleet: Fleet #2").contains("Destination Fleet:")
    );
    assert!(
        terminal
            .lines
            .iter()
            .any(|line| line.contains("Ships: ") && line.contains("TT*"))
    );
    assert!(terminal.lines.iter().all(|line| !line.contains("AR=")));
    assert!(line_containing(&terminal, "Class <BB,CA,DD,TT*,TT,SC,ET,C,X>").contains("<Q> ->"));
    assert!(
        terminal
            .lines
            .iter()
            .any(|line| line.contains("Staged to Transfer: none"))
    );
}

#[test]
fn fleet_transfer_source_prompt_defaults_to_largest_eligible_fleet() {
    let fixture_dir = temp_game_with_same_sector_fleets_copy();
    let mut state = latest_runtime_state(&fixture_dir);
    state.game_data.fleets.records[0].set_battleship_count(1);
    state.game_data.fleets.records[0].set_cruiser_count(1);
    state.game_data.fleets.records[0].set_destroyer_count(1);
    state.game_data.fleets.records[1].set_battleship_count(0);
    state.game_data.fleets.records[1].set_cruiser_count(0);
    state.game_data.fleets.records[1].set_destroyer_count(0);
    state.game_data.fleets.records[1].set_scout_count(1);
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
    for _ in 0..64 {
        if app.current_screen() == ScreenId::MainMenu {
            break;
        }
        app.advance_startup();
    }
    assert_eq!(app.current_screen(), ScreenId::MainMenu);
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::OpenMenu)),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::OpenTransfer)),
        AppOutcome::Continue
    );
    assert_eq!(app.fleet.menu_prompt_default_value, "1");
}

#[test]
fn fleet_transfer_source_prompt_rejects_one_ship_fleet() {
    let fixture_dir = temp_game_with_same_sector_fleets_copy();
    let mut state = latest_runtime_state(&fixture_dir);
    state.game_data.fleets.records[0].set_battleship_count(0);
    state.game_data.fleets.records[0].set_cruiser_count(0);
    state.game_data.fleets.records[0].set_destroyer_count(0);
    state.game_data.fleets.records[0].set_troop_transport_count(0);
    state.game_data.fleets.records[0].set_scout_count(1);
    state.game_data.fleets.records[0].set_etac_count(0);
    state.game_data.fleets.records[0].recompute_max_speed_from_composition();
    state.game_data.fleets.records[1].set_scout_count(2);
    state.game_data.fleets.records[1].recompute_max_speed_from_composition();
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
        apply_action(&mut app, Action::Fleet(FleetAction::OpenTransfer)),
        AppOutcome::Continue
    );
    submit_fleet_menu_prompt(&mut app, Some(1));
    assert_eq!(
        app.fleet
            .menu_prompt_status
            .as_ref()
            .map(|feedback| feedback.message()),
        Some("Use merge instead")
    );
}

#[test]
fn fleet_list_detach_single_ship_uses_short_footer_notice() {
    let fixture_dir = temp_game_with_same_sector_fleets_copy();
    let mut state = latest_runtime_state(&fixture_dir);
    let donor = state
        .game_data
        .fleets
        .records
        .iter_mut()
        .find(|fleet| fleet.owner_empire_raw() == 1 && fleet.local_slot_word_raw() == 4)
        .expect("fleet #4 should exist");
    donor.set_battleship_count(0);
    donor.set_cruiser_count(0);
    donor.set_destroyer_count(0);
    donor.set_troop_transport_count(0);
    donor.set_army_count(0);
    donor.set_scout_count(1);
    donor.set_etac_count(0);
    donor.recompute_max_speed_from_composition();
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
    apply_action(&mut app, Action::Fleet(FleetAction::OpenMenu));
    apply_action(&mut app, Action::Fleet(FleetAction::OpenList));
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::OpenDetach)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FleetList);

    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("fleet list detach donor error should render");
    let command_line = line_containing(&terminal, "COMMAND <-");
    assert!(command_line.contains("(slap a key)"));
    assert!(command_line.contains("Unable to detach"));
}

#[test]
fn fleet_list_transfer_uses_typed_fleet_number_for_single_scout_validation() {
    let fixture_dir = temp_game_with_same_sector_fleets_copy();
    let mut state = latest_runtime_state(&fixture_dir);
    let typed_target = state
        .game_data
        .fleets
        .records
        .iter_mut()
        .find(|fleet| fleet.owner_empire_raw() == 1 && fleet.local_slot_word_raw() == 1)
        .expect("fleet #1 should exist");
    typed_target.set_battleship_count(0);
    typed_target.set_cruiser_count(0);
    typed_target.set_destroyer_count(0);
    typed_target.set_troop_transport_count(0);
    typed_target.set_army_count(0);
    typed_target.set_scout_count(1);
    typed_target.set_etac_count(0);
    typed_target.recompute_max_speed_from_composition();
    let cursor_row = state
        .game_data
        .fleets
        .records
        .iter_mut()
        .find(|fleet| fleet.owner_empire_raw() == 1 && fleet.local_slot_word_raw() == 4)
        .expect("fleet #4 should exist");
    cursor_row.set_troop_transport_count(2);
    cursor_row.set_scout_count(0);
    cursor_row.set_etac_count(0);
    cursor_row.recompute_max_speed_from_composition();
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
    apply_action(&mut app, Action::Fleet(FleetAction::OpenMenu));
    apply_action(&mut app, Action::Fleet(FleetAction::OpenList));
    apply_action(&mut app, Action::Fleet(FleetAction::AppendListChar('1')));
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::OpenTransfer)),
        AppOutcome::Continue
    );

    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("fleet list transfer typed selection error should render");
    let command_line = line_containing(&terminal, "COMMAND <-");
    assert!(command_line.contains("(slap a key)"));
    assert!(command_line.contains("Use merge instead"));
}

#[test]
fn fleet_list_detach_uses_typed_fleet_number_for_single_scout_validation() {
    let fixture_dir = temp_game_with_same_sector_fleets_copy();
    let mut state = latest_runtime_state(&fixture_dir);
    let typed_target = state
        .game_data
        .fleets
        .records
        .iter_mut()
        .find(|fleet| fleet.owner_empire_raw() == 1 && fleet.local_slot_word_raw() == 1)
        .expect("fleet #1 should exist");
    typed_target.set_battleship_count(0);
    typed_target.set_cruiser_count(0);
    typed_target.set_destroyer_count(0);
    typed_target.set_troop_transport_count(0);
    typed_target.set_army_count(0);
    typed_target.set_scout_count(1);
    typed_target.set_etac_count(0);
    typed_target.recompute_max_speed_from_composition();
    let cursor_row = state
        .game_data
        .fleets
        .records
        .iter_mut()
        .find(|fleet| fleet.owner_empire_raw() == 1 && fleet.local_slot_word_raw() == 4)
        .expect("fleet #4 should exist");
    cursor_row.set_troop_transport_count(2);
    cursor_row.set_scout_count(0);
    cursor_row.set_etac_count(0);
    cursor_row.recompute_max_speed_from_composition();
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
    apply_action(&mut app, Action::Fleet(FleetAction::OpenMenu));
    apply_action(&mut app, Action::Fleet(FleetAction::OpenList));
    apply_action(&mut app, Action::Fleet(FleetAction::AppendListChar('1')));
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::OpenDetach)),
        AppOutcome::Continue
    );

    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("fleet list detach typed selection error should render");
    let command_line = line_containing(&terminal, "COMMAND <-");
    assert!(command_line.contains("(slap a key)"));
    assert!(command_line.contains("Unable to detach"));
}

#[test]
fn fleet_transfer_destination_prompt_defaults_to_smallest_colocated_fleet() {
    let fixture_dir = temp_game_with_same_sector_fleets_copy();
    let mut state = latest_runtime_state(&fixture_dir);
    state.game_data.fleets.records[0].set_battleship_count(1);
    state.game_data.fleets.records[0].set_cruiser_count(1);
    state.game_data.fleets.records[0].set_destroyer_count(2);
    state.game_data.fleets.records[1].set_battleship_count(0);
    state.game_data.fleets.records[1].set_cruiser_count(0);
    state.game_data.fleets.records[1].set_destroyer_count(1);
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
        apply_action(&mut app, Action::Fleet(FleetAction::OpenTransfer)),
        AppOutcome::Continue
    );
    submit_fleet_menu_prompt(&mut app, Some(1));
    assert_eq!(app.fleet.menu_prompt_default_value, "2");
}

#[test]
fn fleet_transfer_destination_prompt_rejects_non_colocated_fleet() {
    let fixture_dir = temp_game_with_same_sector_fleets_copy();
    let mut state = latest_runtime_state(&fixture_dir);
    state.game_data.fleets.records[2].set_current_location_coords_raw([1, 1]);
    state.game_data.fleets.records[2].set_standing_order_target_coords_raw([1, 1]);
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
        apply_action(&mut app, Action::Fleet(FleetAction::OpenTransfer)),
        AppOutcome::Continue
    );
    submit_fleet_menu_prompt(&mut app, Some(1));
    submit_fleet_menu_prompt(&mut app, Some(3));
    assert_eq!(
        app.fleet
            .menu_prompt_status
            .as_ref()
            .map(|feedback| feedback.message()),
        Some("Fleet #3 is not in the same sector as Fleet #1.")
    );
}

#[test]
fn general_rankings_opens_production_table_and_returns_to_general_menu() {
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
    assert_eq!(app.current_screen(), ScreenId::GeneralMenu);

    assert_eq!(
        app.handle_key(key(KeyCode::Char('o'))),
        Action::Empire(EmpireAction::OpenRankingsTable(
            EmpireProductionRankingSort::Production
        ))
    );
    assert_eq!(
        apply_action(
            &mut app,
            Action::Empire(EmpireAction::OpenRankingsTable(
                EmpireProductionRankingSort::Production
            ))
        ),
        AppOutcome::Continue
    );
    assert_eq!(
        app.current_screen(),
        ScreenId::Rankings(EmpireProductionRankingSort::Production)
    );
    assert_eq!(
        app.handle_key(key(KeyCode::Enter)),
        Action::ReturnToCommandMenu
    );
    assert_eq!(
        apply_action(&mut app, Action::ReturnToCommandMenu),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::GeneralMenu);
}

#[test]
fn apply_action_toggles_autopilot_and_enemy_relation() {
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

    let initial_autopilot = app.current_autopilot_flag();
    assert_eq!(
        apply_action(&mut app, Action::ToggleAutopilot),
        AppOutcome::Continue
    );
    assert_eq!(
        app.current_autopilot_flag(),
        if initial_autopilot == 0 { 1 } else { 0 }
    );

    assert_eq!(
        apply_action(&mut app, Action::Empire(EmpireAction::OpenEnemies)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::Enemies);
    assert_eq!(
        app.current_relation_to(2),
        Some(DiplomaticRelation::Neutral)
    );

    assert_eq!(
        apply_action(
            &mut app,
            Action::Empire(EmpireAction::AppendEnemiesChar('2'))
        ),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Empire(EmpireAction::SubmitEnemiesInput)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_relation_to(2), Some(DiplomaticRelation::Enemy));
}

#[test]
fn returning_login_clears_only_inactivity_auto_enabled_autopilot() {
    let fixture_dir = temp_game_copy();
    let store = CampaignStore::open_default_in_dir(&fixture_dir).expect("open campaign store");
    let mut state = latest_runtime_state(&fixture_dir);
    state.game_data.player.records[0].set_autopilot_flag(1);
    state.game_data.player.records[0].set_last_run_year_raw(2997);
    let planet_intel_by_viewer = (1..=state.game_data.conquest.player_count())
        .map(|viewer_empire_id| {
            store
                .latest_planet_intel_for_viewer(viewer_empire_id)
                .expect("load runtime intel")
                .into_iter()
                .map(|snapshot| (snapshot.planet_record_index_1_based, snapshot))
                .collect::<BTreeMap<_, _>>()
        })
        .collect::<Vec<_>>();
    let mut player_activity_states = store
        .latest_player_activity_states(state.game_data.conquest.player_count())
        .expect("load player activity");
    player_activity_states[0].last_participation_year = 2997;
    player_activity_states[0].inactivity_autopilot_pending_clear = true;
    save_runtime_state_with_intel_and_activity(
        &fixture_dir,
        &state,
        &planet_intel_by_viewer,
        &player_activity_states,
    );

    let mut app = App::load(AppConfig {
        game_dir: fixture_dir.clone(),
        player_record_index_1_based: 1,
        export_root: None,
        queue_dir: None,
        session_timeout_secs: None,
        game_config: GameConfig::default(),
    })
    .expect("app should load");

    advance_to_main_menu(&mut app);
    assert_eq!(app.current_autopilot_flag(), 0);

    let reloaded = latest_runtime_state(&fixture_dir);
    assert_eq!(
        reloaded.game_data.player.records[0].last_run_year_raw(),
        3000
    );
    let activity = CampaignStore::open_default_in_dir(&fixture_dir)
        .expect("open campaign store")
        .latest_player_activity_states(reloaded.game_data.conquest.player_count())
        .expect("load player activity");
    assert_eq!(activity[0].last_participation_year, 3000);
    assert!(!activity[0].inactivity_autopilot_pending_clear);
}

#[test]
fn returning_login_preserves_manual_autopilot() {
    let fixture_dir = temp_game_copy();
    let store = CampaignStore::open_default_in_dir(&fixture_dir).expect("open campaign store");
    let mut state = latest_runtime_state(&fixture_dir);
    state.game_data.player.records[0].set_autopilot_flag(1);
    state.game_data.player.records[0].set_last_run_year_raw(2997);
    let planet_intel_by_viewer = (1..=state.game_data.conquest.player_count())
        .map(|viewer_empire_id| {
            store
                .latest_planet_intel_for_viewer(viewer_empire_id)
                .expect("load runtime intel")
                .into_iter()
                .map(|snapshot| (snapshot.planet_record_index_1_based, snapshot))
                .collect::<BTreeMap<_, _>>()
        })
        .collect::<Vec<_>>();
    let mut player_activity_states = store
        .latest_player_activity_states(state.game_data.conquest.player_count())
        .expect("load player activity");
    player_activity_states[0].last_participation_year = 2997;
    player_activity_states[0].inactivity_autopilot_pending_clear = false;
    save_runtime_state_with_intel_and_activity(
        &fixture_dir,
        &state,
        &planet_intel_by_viewer,
        &player_activity_states,
    );

    let mut app = App::load(AppConfig {
        game_dir: fixture_dir.clone(),
        player_record_index_1_based: 1,
        export_root: None,
        queue_dir: None,
        session_timeout_secs: None,
        game_config: GameConfig::default(),
    })
    .expect("app should load");

    advance_to_main_menu(&mut app);
    assert_eq!(app.current_autopilot_flag(), 1);

    let reloaded = latest_runtime_state(&fixture_dir);
    assert_eq!(
        reloaded.game_data.player.records[0].last_run_year_raw(),
        3000
    );
    let activity = CampaignStore::open_default_in_dir(&fixture_dir)
        .expect("open campaign store")
        .latest_player_activity_states(reloaded.game_data.conquest.player_count())
        .expect("load player activity");
    assert_eq!(activity[0].last_participation_year, 3000);
    assert!(!activity[0].inactivity_autopilot_pending_clear);
}

#[test]
fn apply_action_clamps_enemies_scroll_to_visible_window() {
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
        apply_action(&mut app, Action::Empire(EmpireAction::OpenEnemies)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::Enemies);

    for _ in 0..50 {
        assert_eq!(
            apply_action(&mut app, Action::Empire(EmpireAction::ScrollEnemies(1))),
            AppOutcome::Continue
        );
    }

    assert_eq!(app.enemies_scroll_offset(), 0);
}

#[test]
fn enemies_typed_empire_number_moves_selector_bar_immediately() {
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
        apply_action(&mut app, Action::Empire(EmpireAction::OpenEnemies)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::Enemies);
    assert_eq!(app.empire.enemies_cursor, 0);

    assert_eq!(
        apply_action(
            &mut app,
            Action::Empire(EmpireAction::AppendEnemiesChar('3'))
        ),
        AppOutcome::Continue
    );

    assert_eq!(app.empire.enemies_cursor, 1);
    assert_eq!(app.enemies_scroll_offset(), 0);
}

#[test]
fn apply_action_deletes_reviewables() {
    let fixture_dir = temp_game_copy();
    let mut runtime = latest_runtime_state(&fixture_dir);
    set_runtime_report_blocks(&mut runtime, b"test results");
    runtime.queued_mail.push(incoming_mail(
        2,
        1,
        runtime.game_data.conquest.game_year().saturating_sub(1),
        "Orders",
        "test messages",
    ));
    runtime.game_data.player.records[0].raw[0x30] = 1;
    runtime.game_data.player.records[0].raw[0x34] = 1;
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
            Action::Messaging(MessagingAction::OpenDeleteReviewables)
        ),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::GeneralMenu);
    assert!(app.messaging.delete_reviewables_prompt_active);

    assert_eq!(
        apply_action(
            &mut app,
            Action::Messaging(MessagingAction::ConfirmDeleteReviewables)
        ),
        AppOutcome::Continue
    );

    let runtime = latest_runtime_state(&fixture_dir);
    assert!(
        runtime
            .report_block_rows
            .iter()
            .all(|row| row.recipient_deleted)
    );
    assert_eq!(runtime.queued_mail.len(), 1);
    assert!(runtime.queued_mail[0].recipient_deleted);
    assert_eq!(runtime.game_data.player.records[0].raw[0x30], 0);
    assert_eq!(runtime.game_data.player.records[0].raw[0x34], 0);
    assert!(!app.messaging.delete_reviewables_prompt_active);
}
