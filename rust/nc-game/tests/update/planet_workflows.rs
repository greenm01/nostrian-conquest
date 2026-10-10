use crate::support::*;
use nc_compat::load_classic_core_files;

#[test]
fn confirm_auto_commission_opens_paged_report_when_entries_exist() {
    let fixture_dir = temp_game_with_auto_commission_copy();
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
    assert_eq!(
        apply_action(
            &mut app,
            Action::Planet(PlanetAction::OpenAutoCommissionPrompt)
        ),
        AppOutcome::Continue
    );
    assert!(app.planet.auto_commission_prompt_active);

    assert_eq!(
        apply_action(
            &mut app,
            Action::Planet(PlanetAction::ConfirmAutoCommission)
        ),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::PlanetAutoCommissionReport);
    assert!(!app.planet.auto_commission_report_rows.is_empty());
    assert_eq!(
        app.planet.auto_commission_report_revealed_rows,
        app.planet.auto_commission_report_rows.len().min(23)
    );

    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal).expect("report should render");
    assert_eq!(terminal.line(24).trim_end(), " (slap a key)");
    assert_eq!(terminal.line(23).trim_end(), "");
    assert!(line_containing(&terminal, "Fleet").contains("commissioned from \""));
    assert!(line_containing(&terminal, "Starbase").contains("commissioned to \""));
}

#[test]
fn auto_commission_report_advances_by_page_then_returns_to_planet_menu() {
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
    app.open_planet_menu();
    app.current_screen = ScreenId::PlanetAutoCommissionReport;
    app.planet.auto_commission_report_rows = (1..=24)
        .map(|idx| {
            format!("Fleet {idx:02} commissioned from \"Foo\" in sector (08,09) with DD 01.")
        })
        .collect();
    app.planet.auto_commission_report_revealed_rows = 23;

    assert_eq!(
        apply_action(
            &mut app,
            Action::Planet(PlanetAction::AdvanceAutoCommissionReport)
        ),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::PlanetAutoCommissionReport);
    assert_eq!(app.planet.auto_commission_report_revealed_rows, 24);

    assert_eq!(
        apply_action(
            &mut app,
            Action::Planet(PlanetAction::AdvanceAutoCommissionReport)
        ),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::PlanetMenu);
    assert!(app.planet.auto_commission_report_rows.is_empty());
}

#[test]
fn planet_list_auto_commission_prompt_and_report_return_to_list() {
    let fixture_dir = temp_game_with_auto_commission_copy();
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
    app.open_planet_menu();
    app.submit_planet_list_sort(PlanetListMode::Brief, PlanetListSort::Location);

    assert_eq!(
        apply_action(
            &mut app,
            Action::Planet(PlanetAction::OpenAutoCommissionPrompt)
        ),
        AppOutcome::Continue
    );
    assert_eq!(
        app.current_screen(),
        ScreenId::PlanetList(PlanetListMode::Brief, PlanetListSort::Location)
    );

    app.render(&mut terminal)
        .expect("planet list mass-commission prompt should render");
    assert!(
        line_containing(
            &terminal,
            "Mass Commission: Commission all ships and starbases? [Y]/N ->",
        )
        .contains("Mass Commission: Commission all ships and starbases? [Y]/N ->"),
        "expected inline mass-commission prompt on planet list"
    );
    assert!(
        terminal
            .lines
            .iter()
            .all(|line| !line.contains("Automatically commission all ships and starbases")),
        "planet list should not render a post-footer auto-commission notice"
    );

    assert_eq!(
        apply_action(
            &mut app,
            Action::Planet(PlanetAction::ConfirmAutoCommission)
        ),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::PlanetAutoCommissionReport);

    while app.current_screen() == ScreenId::PlanetAutoCommissionReport {
        assert_eq!(
            apply_action(
                &mut app,
                Action::Planet(PlanetAction::AdvanceAutoCommissionReport)
            ),
            AppOutcome::Continue
        );
    }

    assert_eq!(
        app.current_screen(),
        ScreenId::PlanetList(PlanetListMode::Brief, PlanetListSort::Location)
    );
}

#[test]
fn planet_list_display_queue_opens_selected_planet_and_returns_to_list() {
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
    app.open_planet_menu();
    app.submit_planet_list_sort(PlanetListMode::Brief, PlanetListSort::Location);

    let mut rows = app.game_data.empire_planet_economy_rows(1);
    rows.sort_by_key(|row| row.coords);
    let target = rows.first().expect("owned planet");
    app.game_data
        .append_planet_build_order(target.planet_record_index_1_based, 10, 1)
        .expect("queue build order");

    assert_eq!(
        apply_action(&mut app, Action::Planet(PlanetAction::OpenBuildList)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::PlanetBuildList);

    let action = app.handle_key(key(KeyCode::Char('q')));
    assert_eq!(action, Action::Planet(PlanetAction::OpenBuildMenu));
    assert_eq!(apply_action(&mut app, action), AppOutcome::Continue);
    assert_eq!(
        app.current_screen(),
        ScreenId::PlanetList(PlanetListMode::Brief, PlanetListSort::Location)
    );
}

#[test]
fn planet_list_abort_builds_prompt_stays_inline_and_clears_queue() {
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
    app.open_planet_menu();
    app.submit_planet_list_sort(PlanetListMode::Brief, PlanetListSort::Location);

    let mut rows = app.game_data.empire_planet_economy_rows(1);
    rows.sort_by_key(|row| row.coords);
    let target = rows.first().expect("owned planet");
    app.game_data
        .append_planet_build_order(target.planet_record_index_1_based, 10, 1)
        .expect("queue build order");

    assert_eq!(
        apply_action(&mut app, Action::Planet(PlanetAction::OpenBuildAbortPrompt)),
        AppOutcome::Continue
    );
    assert_eq!(
        app.current_screen(),
        ScreenId::PlanetList(PlanetListMode::Brief, PlanetListSort::Location)
    );

    app.render(&mut terminal)
        .expect("planet list abort-build prompt should render");
    assert!(
        line_containing(&terminal, "Abort queued builds? Y/[N] ->")
            .contains("Abort queued builds? Y/[N] ->"),
        "expected inline build-abort prompt on planet list"
    );

    let action = app.handle_key(key(KeyCode::Char('y')));
    assert_eq!(action, Action::Planet(PlanetAction::ConfirmBuildAbort));
    assert_eq!(apply_action(&mut app, action), AppOutcome::Continue);
    assert_eq!(
        app.current_screen(),
        ScreenId::PlanetList(PlanetListMode::Brief, PlanetListSort::Location)
    );
    assert!(
        nc_engine::planet_build_orders(
            &app.game_data.planets.records[target.planet_record_index_1_based - 1]
        )
        .is_empty()
    );
    assert_eq!(
        app.planet.list_prompt_status.as_deref(),
        Some("Build orders aborted.")
    );
}

#[test]
fn planet_list_commission_opens_direct_draft_for_selected_planet_and_returns_to_list() {
    let fixture_dir = temp_game_with_auto_commission_copy();
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
    app.open_planet_menu();
    app.submit_planet_list_sort(PlanetListMode::Brief, PlanetListSort::Location);

    let mut rows = app
        .game_data
        .empire_planet_economy_rows(app.player.record_index_1_based);
    rows.sort_by_key(|row| row.coords);
    let target_cursor = rows
        .iter()
        .position(|row| {
            let planet = &app.game_data.planets.records[row.planet_record_index_1_based - 1];
            (0..nc_data::STARDOCK_SLOT_COUNT).any(|slot| {
                planet.stardock_count_raw(slot) > 0 && planet.stardock_kind_raw(slot) != 0
            })
        })
        .expect("fixture should have a planet with stardock units");
    app.planet.brief_cursor = target_cursor;

    assert_eq!(
        apply_action(&mut app, Action::Planet(PlanetAction::OpenCommissionMenu)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::PlanetCommissionDraft);

    let action = app.handle_key(key(KeyCode::Char('q')));
    assert_eq!(apply_action(&mut app, action), AppOutcome::Continue);
    assert_eq!(
        app.current_screen(),
        ScreenId::PlanetList(PlanetListMode::Brief, PlanetListSort::Location)
    );
}

#[test]
fn planet_list_build_specify_and_transport_stay_on_list_context() {
    let fixture_dir = temp_game_copy();
    let mut state = latest_runtime_state(&fixture_dir);
    let extra_owned_idx = state
        .game_data
        .planets
        .records
        .iter()
        .enumerate()
        .find(|(_, planet)| planet.owner_empire_slot_raw() != 1)
        .map(|(idx, _)| idx)
        .expect("fixture should have a non-owned planet");
    let extra_planet = &mut state.game_data.planets.records[extra_owned_idx];
    extra_planet.set_owner_empire_slot_raw(1);
    extra_planet.set_stored_production_points(10);
    extra_planet.set_army_count_raw(9);
    let extra_coords = extra_planet.coords_raw();

    for fleet in state
        .game_data
        .fleets
        .records
        .iter_mut()
        .filter(|fleet| fleet.owner_empire_raw() == 1)
    {
        fleet.set_troop_transport_count(0);
        fleet.set_army_count(0);
        fleet.recompute_max_speed_from_composition();
    }

    let load_fleet = state
        .game_data
        .fleets
        .records
        .iter_mut()
        .find(|fleet| fleet.owner_empire_raw() == 1 && fleet.local_slot_word_raw() == 3)
        .expect("fleet #3 should exist");
    load_fleet.set_current_location_coords_raw(extra_coords);
    load_fleet.set_troop_transport_count(5);
    load_fleet.set_army_count(0);
    load_fleet.recompute_max_speed_from_composition();
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
    app.open_planet_menu();
    app.submit_planet_list_sort(PlanetListMode::Brief, PlanetListSort::Location);

    let mut rows = app
        .game_data
        .empire_planet_economy_rows(app.player.record_index_1_based);
    rows.sort_by_key(|row| row.coords);
    let build_cursor = rows
        .iter()
        .position(|row| row.stored_production_points >= 2 && u32::from(row.build_capacity) >= 2)
        .expect("fixture should contain at least one buildable owned planet");
    let target_cursor = rows
        .iter()
        .position(|row| row.coords == extra_coords)
        .expect("extra owned planet should appear in planet list");
    app.planet.brief_cursor = build_cursor;

    assert_eq!(
        apply_action(&mut app, Action::Planet(PlanetAction::OpenBuildSpecify)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::PlanetBuildSpecify);
    let action = app.handle_key(key(KeyCode::Char('q')));
    assert_eq!(apply_action(&mut app, action), AppOutcome::Continue);
    assert_eq!(
        app.current_screen(),
        ScreenId::PlanetList(PlanetListMode::Brief, PlanetListSort::Location)
    );

    app.planet.brief_cursor = target_cursor;

    assert_eq!(
        apply_action(
            &mut app,
            Action::Planet(PlanetAction::OpenTransportPrompt(
                nc_game::screen::PlanetTransportMode::Load,
            )),
        ),
        AppOutcome::Continue
    );
    assert_eq!(
        app.current_screen(),
        ScreenId::PlanetList(PlanetListMode::Brief, PlanetListSort::Location)
    );

    app.render(&mut terminal)
        .expect("planet list load prompt should render");
    assert!(
        line_containing(&terminal, "COMMAND <- Load Armies: Fleet #")
            .contains("Load Armies: Fleet # [3]"),
        "expected inline list fleet prompt with selected-planet fleet default"
    );
    assert!(
        terminal
            .lines
            .iter()
            .all(|line| !line.contains("Load Planet") && !line.contains("Planet:")),
        "list flow should not ask for a planet or render a hanger summary"
    );

    submit_planet_transport_prompt(&mut app, None);
    terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("planet list load quantity prompt should render");
    assert!(
        terminal.lines.iter().any(|line| {
            line.contains("COMMAND <-") && line.contains("Load Armies: How many armies?")
        }),
        "expected inline quantity prompt on planet list"
    );
    assert!(
        terminal
            .lines
            .iter()
            .all(|line| !line.contains("Planet:") && !line.contains("Fleet 03")),
        "planet list should not render a post-footer transport summary"
    );

    let action = app.handle_key(key(KeyCode::Char('q')));
    assert_eq!(apply_action(&mut app, action), AppOutcome::Continue);
    assert_eq!(
        app.current_screen(),
        ScreenId::PlanetList(PlanetListMode::Brief, PlanetListSort::Location)
    );

    assert_eq!(
        apply_action(&mut app, Action::Planet(PlanetAction::OpenScorchPrompt)),
        AppOutcome::Continue
    );
    assert_eq!(
        app.current_screen(),
        ScreenId::PlanetList(PlanetListMode::Brief, PlanetListSort::Location)
    );
    terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("planet list scorch prompt should render");
    assert!(
        line_containing(
            &terminal,
            "COMMAND <- Scorch Planet: Are you sure? Y/[N] ->"
        )
        .contains("Scorch Planet: Are you sure? Y/[N] ->"),
        "expected scorch confirm inline on planet list"
    );
    assert!(
        terminal
            .lines
            .iter()
            .all(|line| !line.contains("Scorch \"")),
        "planet list should not render a post-footer scorch summary"
    );
}

#[test]
fn planet_commission_menu_renders_without_crashing_when_no_stardock_units_exist() {
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
        apply_action(&mut app, Action::Planet(PlanetAction::OpenCommissionMenu)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::PlanetMenu);

    app.render(&mut terminal).expect("render succeeds");
    assert!(
        terminal
            .lines
            .iter()
            .any(|line| { line.contains("No owned planets have units waiting in stardock.") })
    );
}

#[test]
fn planet_commission_draft_render_does_not_crash_when_picker_rows_disappear() {
    let fixture_dir = temp_game_copy();
    let mut state = latest_runtime_state(&fixture_dir);
    for planet in &mut state.game_data.planets.records {
        for slot in 0..6 {
            planet.set_stardock_count_raw(slot, 0);
        }
    }
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
    app.current_screen = ScreenId::PlanetCommissionDraft;
    app.planet.commission_draft_rows = vec![PlanetCommissionDraftRow {
        direct_slot_0_based: None,
        kind: ProductionItemKind::Destroyer,
        unit_label: "Destroyers".to_string(),
        remaining_qty: 1,
        fleet_qty: 1,
    }];

    app.render(&mut terminal)
        .expect("commission draft render succeeds");
    assert!(
        terminal
            .lines
            .iter()
            .any(|line| line.contains("DRAFT COMMISSION FLEET:"))
    );
}

#[test]
fn planet_commission_picker_render_returns_to_planet_menu_when_empty() {
    let fixture_dir = temp_game_copy();
    let mut state = latest_runtime_state(&fixture_dir);
    for planet in &mut state.game_data.planets.records {
        for slot in 0..6 {
            planet.set_stardock_count_raw(slot, 0);
        }
    }
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
    app.current_screen = ScreenId::PlanetCommissionPicker;

    app.render(&mut terminal)
        .expect("empty commission picker should redirect");
    assert_eq!(app.current_screen(), ScreenId::PlanetMenu);
}

#[test]
fn planet_commission_uses_draft_for_ships_and_direct_result_for_starbases() {
    let fixture_dir = temp_game_copy();
    let mut state = latest_runtime_state(&fixture_dir);
    let owned_planets = state
        .game_data
        .planets
        .records
        .iter()
        .enumerate()
        .filter(|(_, planet)| planet.owner_empire_slot_raw() == 1)
        .map(|(idx, _)| idx)
        .collect::<Vec<_>>();
    let mut owned_planets = owned_planets;
    if owned_planets.len() < 2 {
        let extra_idx = state
            .game_data
            .planets
            .records
            .iter()
            .enumerate()
            .find(|(_, planet)| planet.owner_empire_slot_raw() != 1)
            .map(|(idx, _)| idx)
            .expect("fixture should have a spare planet");
        state.game_data.planets.records[extra_idx].set_owner_empire_slot_raw(1);
        state.game_data.planets.records[extra_idx].set_ownership_status_raw(1);
        owned_planets.push(extra_idx);
    }
    for &planet_idx in &owned_planets {
        let planet = &mut state.game_data.planets.records[planet_idx];
        for slot in 0..6 {
            planet.set_stardock_count_raw(slot, 0);
        }
    }
    state.game_data.planets.records[owned_planets[0]].set_stardock_count_raw(0, 2);
    state.game_data.planets.records[owned_planets[1]].set_stardock_count_raw(6, 1);
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
        apply_action(&mut app, Action::Planet(PlanetAction::OpenMenu)),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Planet(PlanetAction::OpenCommissionMenu)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::PlanetCommissionPicker);

    assert_eq!(
        apply_action(&mut app, Action::Planet(PlanetAction::OpenCommissionPlanet)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::PlanetCommissionDraft);

    app.render(&mut terminal).expect("commission draft renders");
    assert!(
        terminal
            .lines
            .iter()
            .any(|line| line.contains("COMMAND <- Qty for"))
    );
    assert!(
        terminal
            .lines
            .iter()
            .all(|line| !line.contains("Set quantities for the ships you want in this fleet."))
    );

    assert_eq!(
        apply_action(
            &mut app,
            Action::Planet(PlanetAction::AppendCommissionDraftChar('2'))
        ),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(
            &mut app,
            Action::Planet(PlanetAction::SubmitCommissionDraft)
        ),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::PlanetCommissionResult);
    assert!(
        app.planet
            .commission_result_notice
            .as_deref()
            .unwrap_or("")
            .contains("Fleet")
    );

    app.render(&mut terminal)
        .expect("commission result renders");
    assert!(
        terminal
            .lines
            .iter()
            .any(|line| line.contains("Fleet") && line.contains("Commissioned"))
    );
    assert!(
        terminal
            .lines
            .iter()
            .any(|line| line.contains("(slap a key)"))
    );

    assert_eq!(
        apply_action(
            &mut app,
            Action::Planet(PlanetAction::DismissCommissionResult(KeyCode::Enter))
        ),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::PlanetCommissionPicker);
    assert_eq!(
        apply_action(&mut app, Action::Planet(PlanetAction::OpenCommissionPlanet)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::PlanetCommissionDraft);

    assert_eq!(
        apply_action(
            &mut app,
            Action::Planet(PlanetAction::SubmitCommissionDraft)
        ),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::PlanetCommissionResult);
    assert!(
        app.planet
            .commission_result_notice
            .as_deref()
            .unwrap_or("")
            .contains("Starbase")
    );

    assert_eq!(
        apply_action(
            &mut app,
            Action::Planet(PlanetAction::DismissCommissionResult(KeyCode::Enter))
        ),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::PlanetMenu);
}

#[test]
fn planet_commission_draft_keeps_intermediate_success_inline() {
    let fixture_dir = temp_game_copy();
    let mut state = latest_runtime_state(&fixture_dir);
    let owned_planets = state
        .game_data
        .planets
        .records
        .iter()
        .enumerate()
        .filter(|(_, planet)| planet.owner_empire_slot_raw() == 1)
        .map(|(idx, _)| idx)
        .collect::<Vec<_>>();
    let owned_planet = owned_planets
        .first()
        .copied()
        .expect("fixture should have an owned planet");
    for &planet_idx in &owned_planets {
        let planet = &mut state.game_data.planets.records[planet_idx];
        for slot in 0..6 {
            planet.set_stardock_count_raw(slot, 0);
        }
    }
    let planet = &mut state.game_data.planets.records[owned_planet];
    planet.set_stardock_count_raw(0, 5);
    planet.set_stardock_count_raw(2, 3);
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
        apply_action(&mut app, Action::Planet(PlanetAction::OpenMenu)),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Planet(PlanetAction::OpenCommissionMenu)),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Planet(PlanetAction::OpenCommissionPlanet)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::PlanetCommissionDraft);

    assert_eq!(
        apply_action(
            &mut app,
            Action::Planet(PlanetAction::AppendCommissionDraftChar('2'))
        ),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(
            &mut app,
            Action::Planet(PlanetAction::MoveCommissionDraftRow(1))
        ),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(
            &mut app,
            Action::Planet(PlanetAction::AppendCommissionDraftChar('1'))
        ),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(
            &mut app,
            Action::Planet(PlanetAction::SubmitCommissionDraft)
        ),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::PlanetCommissionDraft);
    assert!(
        app.planet
            .commission_draft_notice
            .as_deref()
            .unwrap_or("")
            .contains("Fleet")
    );
    assert_eq!(app.planet.commission_draft_rows.len(), 2);
    assert_eq!(app.planet.commission_draft_rows[0].remaining_qty, 3);
    assert_eq!(app.planet.commission_draft_rows[1].remaining_qty, 2);
}

#[test]
fn planet_commission_blank_submit_uses_displayed_default_qty() {
    let fixture_dir = temp_game_copy();
    let mut state = latest_runtime_state(&fixture_dir);
    let owned_planets = state
        .game_data
        .planets
        .records
        .iter()
        .enumerate()
        .filter(|(_, planet)| planet.owner_empire_slot_raw() == 1)
        .map(|(idx, _)| idx)
        .collect::<Vec<_>>();
    let owned_planet = owned_planets
        .first()
        .copied()
        .expect("fixture should have an owned planet");
    for &planet_idx in &owned_planets {
        let planet = &mut state.game_data.planets.records[planet_idx];
        for slot in 0..6 {
            planet.set_stardock_count_raw(slot, 0);
        }
    }
    let planet = &mut state.game_data.planets.records[owned_planet];
    planet.set_stardock_count_raw(2, 1);
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
        apply_action(&mut app, Action::Planet(PlanetAction::OpenMenu)),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Planet(PlanetAction::OpenCommissionMenu)),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Planet(PlanetAction::OpenCommissionPlanet)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::PlanetCommissionDraft);
    assert_eq!(app.planet.commission_draft_rows.len(), 1);
    assert_eq!(app.planet.commission_draft_rows[0].remaining_qty, 1);
    assert_eq!(app.planet.commission_draft_rows[0].fleet_qty, 0);

    assert_eq!(
        apply_action(
            &mut app,
            Action::Planet(PlanetAction::SubmitCommissionDraft)
        ),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::PlanetCommissionResult);
    assert!(
        app.planet
            .commission_result_notice
            .as_deref()
            .unwrap_or("")
            .contains("Fleet")
    );
}

#[test]
fn planet_commission_blank_row_move_commits_displayed_default_qty() {
    let fixture_dir = temp_game_copy();
    let mut state = latest_runtime_state(&fixture_dir);
    let owned_planets = state
        .game_data
        .planets
        .records
        .iter()
        .enumerate()
        .filter(|(_, planet)| planet.owner_empire_slot_raw() == 1)
        .map(|(idx, _)| idx)
        .collect::<Vec<_>>();
    let owned_planet = owned_planets
        .first()
        .copied()
        .expect("fixture should have an owned planet");
    for &planet_idx in &owned_planets {
        let planet = &mut state.game_data.planets.records[planet_idx];
        for slot in 0..6 {
            planet.set_stardock_count_raw(slot, 0);
        }
    }
    let planet = &mut state.game_data.planets.records[owned_planet];
    planet.set_stardock_count_raw(0, 2);
    planet.set_stardock_count_raw(2, 1);
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
        apply_action(&mut app, Action::Planet(PlanetAction::OpenMenu)),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Planet(PlanetAction::OpenCommissionMenu)),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Planet(PlanetAction::OpenCommissionPlanet)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::PlanetCommissionDraft);
    assert_eq!(app.planet.commission_draft_rows.len(), 2);
    assert_eq!(app.planet.commission_draft_rows[0].remaining_qty, 2);
    assert_eq!(app.planet.commission_draft_rows[0].fleet_qty, 0);

    assert_eq!(
        apply_action(
            &mut app,
            Action::Planet(PlanetAction::MoveCommissionDraftRow(1))
        ),
        AppOutcome::Continue
    );
    assert_eq!(app.planet.commission_draft_cursor, 1);
    assert_eq!(app.planet.commission_draft_rows[0].fleet_qty, 2);

    assert_eq!(
        apply_action(
            &mut app,
            Action::Planet(PlanetAction::AppendCommissionDraftChar('0'))
        ),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(
            &mut app,
            Action::Planet(PlanetAction::SubmitCommissionDraft)
        ),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::PlanetCommissionDraft);
    assert!(
        app.planet
            .commission_draft_notice
            .as_deref()
            .unwrap_or("")
            .contains("Fleet")
    );
}

#[test]
fn planet_commission_result_latches_dismiss_key_until_release() {
    let fixture_dir = temp_game_copy();
    let mut state = latest_runtime_state(&fixture_dir);
    let owned_planets = state
        .game_data
        .planets
        .records
        .iter()
        .enumerate()
        .filter(|(_, planet)| planet.owner_empire_slot_raw() == 1)
        .map(|(idx, _)| idx)
        .collect::<Vec<_>>();
    let mut owned_planets = owned_planets;
    if owned_planets.len() < 2 {
        let extra_idx = state
            .game_data
            .planets
            .records
            .iter()
            .enumerate()
            .find(|(_, planet)| planet.owner_empire_slot_raw() != 1)
            .map(|(idx, _)| idx)
            .expect("fixture should have a spare planet");
        state.game_data.planets.records[extra_idx].set_owner_empire_slot_raw(1);
        state.game_data.planets.records[extra_idx].set_ownership_status_raw(1);
        owned_planets.push(extra_idx);
    }
    for &planet_idx in &owned_planets {
        let planet = &mut state.game_data.planets.records[planet_idx];
        for slot in 0..6 {
            planet.set_stardock_count_raw(slot, 0);
        }
    }
    state.game_data.planets.records[owned_planets[0]].set_stardock_count_raw(0, 2);
    state.game_data.planets.records[owned_planets[1]].set_stardock_count_raw(3, 2);
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
        apply_action(&mut app, Action::Planet(PlanetAction::OpenMenu)),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Planet(PlanetAction::OpenCommissionMenu)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::PlanetCommissionPicker);
    app.current_screen = ScreenId::PlanetCommissionResult;
    app.planet.commission_result_return_to_picker = true;
    app.planet.commission_result_notice = Some("Fleet 02 Commissioned".to_string());

    let dismiss_press = app.handle_key(key_with_kind(KeyCode::Enter, KeyEventKind::Press));
    assert_eq!(apply_action(&mut app, dismiss_press), AppOutcome::Continue);
    assert_eq!(app.current_screen(), ScreenId::PlanetCommissionPicker);

    let repeat = app.handle_key(key_with_kind(KeyCode::Enter, KeyEventKind::Repeat));
    assert_eq!(apply_action(&mut app, repeat), AppOutcome::Continue);
    assert_eq!(app.current_screen(), ScreenId::PlanetCommissionPicker);

    let fresh_press = app.handle_key(key_with_kind(KeyCode::Enter, KeyEventKind::Press));
    assert_eq!(apply_action(&mut app, fresh_press), AppOutcome::Continue);
    assert_eq!(app.current_screen(), ScreenId::PlanetCommissionDraft);
}

#[test]
fn planet_build_menu_and_subscreens_render_without_crashing_when_no_owned_planets_exist() {
    let fixture_dir = temp_joined_no_assets_copy();
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
        apply_action(&mut app, Action::Planet(PlanetAction::OpenBuildMenu)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::PlanetMenu);
    app.render(&mut terminal)
        .expect("planet menu render succeeds");
    assert!(
        terminal
            .lines
            .iter()
            .any(|line| line.contains("No owned planets available"))
    );

    assert_eq!(
        apply_action(
            &mut app,
            Action::Planet(PlanetAction::OpenCurrentBuildPlanetInfo)
        ),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::PlanetMenu);
    app.render(&mut terminal)
        .expect("build current-planet info fallback render succeeds");

    assert_eq!(
        apply_action(&mut app, Action::Planet(PlanetAction::OpenBuildList)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::PlanetMenu);
    app.render(&mut terminal)
        .expect("build list fallback render succeeds");

    assert_eq!(
        apply_action(&mut app, Action::Planet(PlanetAction::OpenBuildAbortPrompt)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::PlanetMenu);
    app.render(&mut terminal)
        .expect("build abort fallback render succeeds");

    assert_eq!(
        apply_action(&mut app, Action::Planet(PlanetAction::OpenBuildSpecify)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::PlanetMenu);
    app.render(&mut terminal)
        .expect("build specify fallback render succeeds");
}

#[test]
fn planet_build_menu_matches_verified_v15_command_layout() {
    let fixture_dir = temp_game_copy();
    let direct = load_classic_core_files(&fixture_dir).expect("reload direct joined fixture");
    let direct_homeworld = direct
        .planets
        .records
        .iter()
        .find(|planet| {
            planet.owner_empire_slot_raw() == 1 && planet.status_or_name_summary() == "Codex Prime"
        })
        .expect("direct joined homeworld exists");
    assert_eq!(direct_homeworld.present_production_points(), Some(100));
    assert_eq!(direct_homeworld.stored_production_points(), 50);
    let runtime = latest_runtime_state(&fixture_dir);
    let homeworld = runtime
        .game_data
        .planets
        .records
        .iter()
        .find(|planet| {
            planet.owner_empire_slot_raw() == 1 && planet.status_or_name_summary() == "Codex Prime"
        })
        .expect("joined homeworld exists");
    assert_eq!(homeworld.present_production_points(), Some(100));
    assert_eq!(homeworld.stored_production_points(), 50);
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
        apply_action(&mut app, Action::Planet(PlanetAction::OpenBuildMenu)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::PlanetBuildMenu);

    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("planet build menu should render");
    assert_eq!(
        terminal.line(0).trim_end(),
        " BUILD ON CURRENT PLANET: \"Codex Prime\" IN SYSTEM [16,13]:"
    );
    assert_eq!(
        terminal.line(2).trim_end(),
        "  H>elp with commands        P>lanets, List your         S>pecify Build Orders"
    );
    assert_eq!(
        terminal.line(3).trim_end(),
        "  Q>uit to Planet Menu       R>eview current planet      A>bort planet's builds"
    );
    assert_eq!(
        terminal.line(4).trim_end(),
        "  X>pert mode ON/OFF         C>hange current planet      D>isplay queue"
    );
    assert_eq!(
        terminal.line(5).trim_end(),
        "  V>iew partial star map     N>ext planet                I>nfo about a Planet"
    );
    assert_eq!(
        terminal.line(7).trim_end(),
        " BUILD COMMAND <- ? X V P R C N S A D I <Q> ->"
    );
    assert_eq!(
        terminal.line(13).trim_end(),
        " There are no starbases orbiting planet \"Codex Prime\"."
    );
    assert_eq!(
        terminal.line(14).trim_end(),
        " Standard building restrictions apply."
    );
    assert_eq!(
        terminal.line(15).trim_end(),
        " Spent: 0 of 50 PP.  Budget remaining: 50."
    );
    assert_eq!(terminal.lines[17].trim_end(), " Building: 0   Docked: 0");
}

#[test]
fn repeated_same_kind_build_submissions_merge_into_one_player_visible_total() {
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
        apply_action(&mut app, Action::Planet(PlanetAction::OpenBuildMenu)),
        AppOutcome::Continue
    );

    app.open_planet_build_specify();
    app.append_planet_build_unit_char('1');
    app.submit_planet_build_unit();
    app.append_planet_build_quantity_char('2');
    app.submit_planet_build_quantity()
        .expect("first build quantity should submit");

    app.open_planet_build_specify();
    app.append_planet_build_unit_char('1');
    app.submit_planet_build_unit();
    app.append_planet_build_quantity_char('3');
    app.submit_planet_build_quantity()
        .expect("second build quantity should submit");

    let planet_idx = app
        .game_data
        .planet_record_index_at_coords([16, 13])
        .expect("current build planet should exist");
    let planet = &app.game_data.planets.records[planet_idx];
    assert_eq!(planet.build_kind_raw(0), 1);
    assert_eq!(planet.build_count_raw(0), 25);
    assert!((1..10).all(|slot| planet.build_count_raw(slot) == 0));
    assert!((1..10).all(|slot| planet.build_kind_raw(slot) == 0));

    app.open_planet_build_menu();
    app.render(&mut terminal).expect("render succeeds");
    assert_eq!(terminal.line(17).trim_end(), " Building: 5   Docked: 0");

    assert_eq!(
        apply_action(
            &mut app,
            Action::Planet(PlanetAction::OpenCurrentBuildPlanetInfo)
        ),
        AppOutcome::Continue
    );
    terminal = CaptureTerminal::new();
    app.render(&mut terminal).expect("render succeeds");
    assert!(line_containing(&terminal, "Building").contains("5-DD"));
}

#[test]
fn empty_build_quantity_submission_uses_max_affordable_default() {
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
        apply_action(&mut app, Action::Planet(PlanetAction::OpenBuildMenu)),
        AppOutcome::Continue
    );

    app.open_planet_build_specify();
    app.append_planet_build_unit_char('1');
    app.submit_planet_build_unit();
    app.submit_planet_build_quantity()
        .expect("empty quantity submit should use default max");

    let planet_idx = app
        .game_data
        .planet_record_index_at_coords([16, 13])
        .expect("current build planet should exist");
    let planet = &app.game_data.planets.records[planet_idx];
    assert_eq!(planet.build_kind_raw(0), 1);
    assert_eq!(planet.build_count_raw(0), 50);
    assert!((1..10).all(|slot| planet.build_count_raw(slot) == 0));
}

#[test]
fn build_specify_from_menu_stays_on_build_menu_when_no_budget_remains() {
    let fixture_dir = temp_game_copy();
    let mut state = latest_runtime_state(&fixture_dir);
    let homeworld = state
        .game_data
        .planets
        .records
        .iter_mut()
        .find(|planet| planet.owner_empire_slot_raw() == 1)
        .expect("owned planet exists");
    homeworld.set_stored_production_points(1);
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
        apply_action(&mut app, Action::Planet(PlanetAction::OpenBuildMenu)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::PlanetBuildMenu);

    app.open_planet_build_specify();

    assert_eq!(app.current_screen(), ScreenId::PlanetBuildMenu);
    app.render(&mut terminal)
        .expect("build menu should render no-budget notice");
    assert!(
        line_containing(&terminal, "Notice: ").contains("No build budget remains."),
        "expected no-budget notice in build menu"
    );
}

#[test]
fn successful_build_that_exhausts_budget_returns_to_build_menu_with_notice() {
    let fixture_dir = temp_game_copy();
    let mut state = latest_runtime_state(&fixture_dir);
    let homeworld = state
        .game_data
        .planets
        .records
        .iter_mut()
        .find(|planet| planet.owner_empire_slot_raw() == 1)
        .expect("owned planet exists");
    homeworld.set_stored_production_points(2);
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
        apply_action(&mut app, Action::Planet(PlanetAction::OpenBuildMenu)),
        AppOutcome::Continue
    );

    app.open_planet_build_specify();
    assert_eq!(app.current_screen(), ScreenId::PlanetBuildSpecify);
    app.append_planet_build_unit_char('9');
    app.submit_planet_build_unit();
    assert_eq!(app.current_screen(), ScreenId::PlanetBuildQuantity);
    app.append_planet_build_quantity_char('1');
    app.submit_planet_build_quantity()
        .expect("quantity submit should succeed");

    assert_eq!(app.current_screen(), ScreenId::PlanetBuildMenu);
    app.render(&mut terminal)
        .expect("build menu should render exhaustion notice");
    let notice = line_containing(&terminal, "Notice: ");
    assert!(notice.contains("No build budget remains."));
    assert!(!notice.contains("Queued 1 Army."));
}

#[test]
fn build_quantity_from_points_uses_remaining_whole_units_for_partial_orders() {
    assert_eq!(
        nc_game::screen::build_quantity_from_points(ProductionItemKind::Etac, 30),
        2
    );
    assert_eq!(
        nc_game::screen::build_quantity_from_points(ProductionItemKind::Destroyer, 12),
        3
    );
}

#[test]
fn fleet_list_stays_on_fleet_menu_with_notice_when_no_fleets_exist() {
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

    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::OpenMenu)),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::OpenList)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FleetMenu);

    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("fleet menu should render empty-fleet notice");
    assert!(
        terminal
            .lines
            .iter()
            .any(|line| line.contains("You have no active fleets."))
    );
}

#[test]
fn planet_list_commands_stay_on_planet_menu_with_notice_when_no_owned_planets_exist() {
    let fixture_dir = temp_joined_no_assets_copy();
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
    assert_eq!(
        apply_action(
            &mut app,
            Action::Planet(PlanetAction::OpenListSortPrompt(PlanetListMode::Brief))
        ),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::PlanetMenu);

    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("planet menu should render empty-planet notice");
    assert!(
        terminal
            .lines
            .iter()
            .any(|line| line.contains("You do not currently control any planets."))
    );

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
    assert_eq!(app.current_screen(), ScreenId::PlanetMenu);
}

#[test]
fn build_menu_planet_list_selects_build_target_and_returns_to_build_menu() {
    let fixture_dir = temp_game_copy();
    let mut state = latest_runtime_state(&fixture_dir);
    let extra_owned_idx = state
        .game_data
        .planets
        .records
        .iter()
        .enumerate()
        .find(|(_, planet)| planet.owner_empire_slot_raw() != 1)
        .map(|(idx, _)| idx)
        .expect("fixture should have a non-owned planet");
    state.game_data.planets.records[extra_owned_idx].set_owner_empire_slot_raw(1);
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
        apply_action(&mut app, Action::Planet(PlanetAction::OpenBuildMenu)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::PlanetBuildMenu);

    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal).expect("build menu should render");
    let build_title = terminal.line(0).trim_end().to_string();
    assert_eq!(
        build_title,
        " BUILD ON CURRENT PLANET: \"Codex Prime\" IN SYSTEM [16,13]:"
    );

    assert_eq!(
        app.handle_key(key(KeyCode::Char('p'))),
        Action::Planet(PlanetAction::SubmitListSort(
            PlanetListMode::BuildSelect,
            PlanetListSort::CurrentProduction
        ))
    );
    assert_eq!(
        apply_action(
            &mut app,
            Action::Planet(PlanetAction::SubmitListSort(
                PlanetListMode::BuildSelect,
                PlanetListSort::CurrentProduction
            ))
        ),
        AppOutcome::Continue
    );
    assert_eq!(
        app.current_screen(),
        ScreenId::PlanetList(
            PlanetListMode::BuildSelect,
            PlanetListSort::CurrentProduction
        )
    );

    assert_eq!(
        app.handle_key(key(KeyCode::Char('s'))),
        Action::Planet(PlanetAction::OpenListSortPrompt(
            PlanetListMode::BuildSelect
        ))
    );
    assert_eq!(
        apply_action(
            &mut app,
            Action::Planet(PlanetAction::OpenListSortPrompt(
                PlanetListMode::BuildSelect
            ))
        ),
        AppOutcome::Continue
    );
    assert_eq!(
        app.current_screen(),
        ScreenId::PlanetListSortPrompt(PlanetListMode::BuildSelect)
    );
    assert_eq!(
        app.handle_key(key(KeyCode::Char('q'))),
        Action::Planet(PlanetAction::CloseListSortPrompt(
            PlanetListMode::BuildSelect
        ))
    );
    assert_eq!(
        apply_action(
            &mut app,
            Action::Planet(PlanetAction::CloseListSortPrompt(
                PlanetListMode::BuildSelect
            ))
        ),
        AppOutcome::Continue
    );
    assert_eq!(
        app.current_screen(),
        ScreenId::PlanetList(
            PlanetListMode::BuildSelect,
            PlanetListSort::CurrentProduction
        )
    );

    assert_eq!(
        app.handle_key(key(KeyCode::Enter)),
        Action::Planet(PlanetAction::SubmitBriefInput)
    );
    assert_eq!(
        apply_action(&mut app, Action::Planet(PlanetAction::SubmitBriefInput)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::PlanetBuildMenu);
    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("build menu should render after selecting current planet");
    assert_eq!(terminal.line(0).trim_end(), build_title);

    assert_eq!(
        app.handle_key(key(KeyCode::Char('p'))),
        Action::Planet(PlanetAction::SubmitListSort(
            PlanetListMode::BuildSelect,
            PlanetListSort::CurrentProduction
        ))
    );
    assert_eq!(
        apply_action(
            &mut app,
            Action::Planet(PlanetAction::SubmitListSort(
                PlanetListMode::BuildSelect,
                PlanetListSort::CurrentProduction
            ))
        ),
        AppOutcome::Continue
    );
    assert_eq!(
        app.handle_key(key(KeyCode::Down)),
        Action::Planet(PlanetAction::MoveBrief(1))
    );
    assert_eq!(
        apply_action(&mut app, Action::Planet(PlanetAction::MoveBrief(1))),
        AppOutcome::Continue
    );
    assert_eq!(
        app.handle_key(key(KeyCode::Enter)),
        Action::Planet(PlanetAction::SubmitBriefInput)
    );
    assert_eq!(
        apply_action(&mut app, Action::Planet(PlanetAction::SubmitBriefInput)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::PlanetBuildMenu);
    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("build menu should render after choosing a new planet");
    let selected_title = terminal.line(0).trim_end().to_string();
    assert_ne!(selected_title, build_title);

    assert_eq!(
        app.handle_key(key(KeyCode::Char('p'))),
        Action::Planet(PlanetAction::SubmitListSort(
            PlanetListMode::BuildSelect,
            PlanetListSort::CurrentProduction
        ))
    );
    assert_eq!(
        apply_action(
            &mut app,
            Action::Planet(PlanetAction::SubmitListSort(
                PlanetListMode::BuildSelect,
                PlanetListSort::CurrentProduction
            ))
        ),
        AppOutcome::Continue
    );
    assert_eq!(
        app.handle_key(key(KeyCode::Char('q'))),
        Action::Planet(PlanetAction::OpenBuildMenu)
    );
    assert_eq!(
        apply_action(&mut app, Action::Planet(PlanetAction::OpenBuildMenu)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::PlanetBuildMenu);
    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("build menu should render after canceling build-select list");
    assert_eq!(terminal.line(0).trim_end(), selected_title);
}

#[test]
fn planet_database_render_uses_classic_stacked_headers() {
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
        apply_action(&mut app, Action::Planet(PlanetAction::OpenDatabase)),
        AppOutcome::Continue
    );

    app.render(&mut terminal).expect("render succeeds");
    let title_col = terminal
        .line(0)
        .find("TOTAL PLANET DATABASE:")
        .expect("title col");
    let border_col = terminal.line(1).find('┌').expect("table col");
    assert_eq!(title_col, border_col + 1);
    assert!(terminal.line(2).contains("│Coord"));
    assert!(terminal.line(2).contains("Max"));
    assert!(terminal.line(2).contains("Year"));
    assert!(terminal.line(2).contains("Curr"));
    assert!(terminal.line(2).contains("Trsry"));
    assert_eq!(terminal.line(2).matches('│').count(), 12);
    assert!(terminal.line(3).contains("(XX,YY)"));
    assert!(terminal.line(3).contains("Planet Name"));
    assert!(terminal.line(3).contains("Prod"));
    assert!(terminal.line(3).contains("Seen"));
    assert!(terminal.line(3).contains("Scout"));
    assert!(terminal.line(3).contains("ARs"));
    assert!(terminal.line(3).contains("GBs"));
    assert!(terminal.line(3).contains("SBs"));
    assert!(!terminal.line(3).contains("Intel"));
    assert!(terminal.lines.iter().any(|line| line.contains("3000")));
    let prompt = line_containing(&terminal, "COMMAND <- ");
    assert_eq!(
        prompt.find("COMMAND").expect("commands col"),
        border_col + 1
    );
    assert!(prompt.contains("["));
    assert!(prompt.contains("->"));
}

#[test]
fn planet_database_filter_and_sort_prompts_render_distinct_command_lines() {
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
        apply_action(&mut app, Action::Planet(PlanetAction::OpenDatabase)),
        AppOutcome::Continue
    );
    app.render(&mut terminal).expect("render succeeds");
    let browse_prompt_row = terminal
        .lines
        .iter()
        .position(|line| line.contains("COMMAND <- "))
        .expect("browse prompt row");

    assert_eq!(
        apply_action(
            &mut app,
            Action::Planet(PlanetAction::OpenDatabaseFilterPrompt)
        ),
        AppOutcome::Continue
    );
    app.render(&mut terminal).expect("render succeeds");
    assert_eq!(
        terminal
            .lines
            .iter()
            .position(|line| line.contains("COMMAND <- Filter column [?] "))
            .expect("filter prompt row"),
        browse_prompt_row
    );
    let prompt = line_containing(&terminal, "COMMAND <- Filter column [?] ").trim();
    assert!(prompt.starts_with("COMMAND <- Filter column [?] ["));
    assert!(prompt.ends_with("] <Q> ->"));

    assert_eq!(
        apply_action(
            &mut app,
            Action::Planet(PlanetAction::SubmitDatabaseFilter(
                nc_game::screen::PlanetDatabaseFilterMode::Range,
            ))
        ),
        AppOutcome::Continue
    );
    terminal = CaptureTerminal::new();
    app.render(&mut terminal).expect("render succeeds");
    let prompt = line_containing(&terminal, "COMMAND <- Filter coo ").trim();
    assert!(prompt.starts_with("COMMAND <- Filter coo ["));
    assert!(prompt.ends_with("] <Q> ->"));
    assert_eq!(
        terminal
            .lines
            .iter()
            .position(|line| line.contains("COMMAND <- Filter coo "))
            .expect("range prompt row"),
        browse_prompt_row
    );

    assert_eq!(
        apply_action(&mut app, Action::Planet(PlanetAction::OpenDatabase)),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(
            &mut app,
            Action::Planet(PlanetAction::OpenDatabaseSortPrompt)
        ),
        AppOutcome::Continue
    );
    terminal = CaptureTerminal::new();
    app.render(&mut terminal).expect("render succeeds");
    assert_eq!(
        line_containing(&terminal, "COMMAND <- Sort column [?]").trim(),
        "COMMAND <- Sort column [?] [coo] <Q> ->"
    );
    assert_eq!(
        terminal
            .lines
            .iter()
            .position(|line| line.contains("COMMAND <- Sort column [?]"))
            .expect("sort prompt row"),
        browse_prompt_row
    );
}

#[test]
fn planet_database_filters_and_sorts_with_independent_f_and_s_prompts() {
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

    let sample_worlds = app
        .game_data
        .planets
        .records
        .iter()
        .enumerate()
        .filter(|(_, planet)| planet.owner_empire_slot_raw() != 1)
        .take(3)
        .map(|(idx, planet)| (idx + 1, planet.coords_raw()))
        .collect::<Vec<_>>();
    assert_eq!(
        sample_worlds.len(),
        3,
        "fixture should contain non-owned worlds"
    );

    let owners = [Some(3), Some(0), Some(2)];
    let max_prods = [Some(90), Some(220), Some(150)];
    for (sample_idx, ((planet_record_index_1_based, _), (owner, max_prod))) in sample_worlds
        .iter()
        .zip(owners.into_iter().zip(max_prods))
        .enumerate()
    {
        app.planet_intel_snapshots.insert(
            *planet_record_index_1_based,
            PlanetIntelSnapshot {
                planet_record_index_1_based: *planet_record_index_1_based,
                intel_tier: IntelTier::Full,
                compat_is_orbit_seed: false,
                last_intel_year: Some(3000),
                seen_year: Some(3000),
                scout_year: Some(3000),
                known_name: Some(format!("Sample {}", sample_idx + 1)),
                known_owner_empire_id: owner,
                known_potential_production: max_prod,
                known_armies: Some(1),
                known_ground_batteries: Some(1),
                known_starbase_count: Some(0),
                known_current_production: Some(1),
                known_stored_points: Some(1),
                known_docked_summary: Some("Nothing".to_string()),
                known_orbit_summary: Some("Nothing".to_string()),
                compat_word_1e: None,
            },
        );
    }

    advance_to_main_menu(&mut app);
    assert_eq!(
        apply_action(&mut app, Action::Planet(PlanetAction::OpenDatabase)),
        AppOutcome::Continue
    );

    assert_eq!(
        apply_action(
            &mut app,
            Action::Planet(PlanetAction::OpenDatabaseFilterPrompt)
        ),
        AppOutcome::Continue
    );
    for ch in ['m', 'a', 'x'] {
        assert_eq!(
            apply_action(
                &mut app,
                Action::Planet(PlanetAction::AppendDatabaseChar(ch))
            ),
            AppOutcome::Continue
        );
    }
    assert_eq!(
        apply_action(
            &mut app,
            Action::Planet(PlanetAction::SubmitDatabaseFilterPrompt)
        ),
        AppOutcome::Continue
    );
    for ch in ['>', '=', '1', '0', '0'] {
        assert_eq!(
            apply_action(
                &mut app,
                Action::Planet(PlanetAction::AppendDatabaseChar(ch))
            ),
            AppOutcome::Continue
        );
    }
    assert_eq!(
        apply_action(
            &mut app,
            Action::Planet(PlanetAction::SubmitDatabaseFilterPrompt)
        ),
        AppOutcome::Continue
    );

    app.render(&mut terminal).expect("render succeeds");
    assert!(
        terminal.lines.iter().any(|line| line.contains(&format!(
            "({:02},{:02})",
            sample_worlds[1].1[0], sample_worlds[1].1[1]
        ))),
        "220 max-production world should survive the default >=100 filter"
    );
    assert!(
        !terminal.lines.iter().any(|line| line.contains(&format!(
            "({:02},{:02})",
            sample_worlds[0].1[0], sample_worlds[0].1[1]
        ))),
        "90 max-production world should be filtered out"
    );

    assert_eq!(
        apply_action(
            &mut app,
            Action::Planet(PlanetAction::OpenDatabaseFilterPrompt)
        ),
        AppOutcome::Continue
    );
    for ch in ['o', 'w', 'n'] {
        assert_eq!(
            apply_action(
                &mut app,
                Action::Planet(PlanetAction::AppendDatabaseChar(ch))
            ),
            AppOutcome::Continue
        );
    }
    assert_eq!(
        apply_action(
            &mut app,
            Action::Planet(PlanetAction::SubmitDatabaseFilterPrompt)
        ),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(
            &mut app,
            Action::Planet(PlanetAction::AppendDatabaseChar('3'))
        ),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(
            &mut app,
            Action::Planet(PlanetAction::SubmitDatabaseFilterPrompt)
        ),
        AppOutcome::Continue
    );

    terminal = CaptureTerminal::new();
    app.render(&mut terminal).expect("render succeeds");
    assert!(
        terminal
            .lines
            .iter()
            .any(|line| line.contains("TOTAL PLANET DATABASE:") && line.contains(" ALL")),
        "empty database filters should reset to ALL"
    );

    let range_anchor = sample_worlds[0].1;
    assert_eq!(
        apply_action(
            &mut app,
            Action::Planet(PlanetAction::OpenDatabaseFilterPrompt)
        ),
        AppOutcome::Continue
    );
    for ch in ['c', 'o', 'o'] {
        assert_eq!(
            apply_action(
                &mut app,
                Action::Planet(PlanetAction::AppendDatabaseChar(ch))
            ),
            AppOutcome::Continue
        );
    }
    assert_eq!(
        apply_action(
            &mut app,
            Action::Planet(PlanetAction::SubmitDatabaseFilterPrompt)
        ),
        AppOutcome::Continue
    );
    for ch in format!("{},{}", range_anchor[0], range_anchor[1]).chars() {
        assert_eq!(
            apply_action(
                &mut app,
                Action::Planet(PlanetAction::AppendDatabaseChar(ch))
            ),
            AppOutcome::Continue
        );
    }
    for ch in "/0".chars() {
        assert_eq!(
            apply_action(
                &mut app,
                Action::Planet(PlanetAction::AppendDatabaseChar(ch))
            ),
            AppOutcome::Continue
        );
    }
    assert_eq!(
        apply_action(
            &mut app,
            Action::Planet(PlanetAction::SubmitDatabaseFilterPrompt)
        ),
        AppOutcome::Continue
    );

    terminal = CaptureTerminal::new();
    app.render(&mut terminal).expect("render succeeds");
    assert!(
        terminal
            .lines
            .iter()
            .any(|line| line.contains(&format!("({:02},{:02})", range_anchor[0], range_anchor[1]))),
        "exact anchor world should remain when filtering at radius 0"
    );
    assert!(
        !terminal.lines.iter().any(|line| line.contains(&format!(
            "({:02},{:02})",
            sample_worlds[1].1[0], sample_worlds[1].1[1]
        ))),
        "non-anchor worlds should be filtered out at radius 0"
    );

    assert_eq!(
        apply_action(
            &mut app,
            Action::Planet(PlanetAction::OpenDatabaseFilterPrompt)
        ),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(
            &mut app,
            Action::Planet(PlanetAction::SubmitDatabaseFilterPrompt)
        ),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(
            &mut app,
            Action::Planet(PlanetAction::OpenDatabaseSortPrompt)
        ),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(
            &mut app,
            Action::Planet(PlanetAction::SubmitDatabaseSort(
                nc_game::screen::PlanetDatabaseSortMode::MaxProduction,
            ))
        ),
        AppOutcome::Continue
    );

    terminal = CaptureTerminal::new();
    app.render(&mut terminal).expect("render succeeds");
    let max_positions = sample_worlds
        .iter()
        .map(|(_, coords)| {
            terminal
                .lines
                .iter()
                .position(|line| line.contains(&format!("({:02},{:02})", coords[0], coords[1])))
                .expect("max-prod row present")
        })
        .collect::<Vec<_>>();
    assert!(
        max_positions[1] < max_positions[2] && max_positions[2] < max_positions[0],
        "sort should order 220 > 150 > 90 after clearing filters: {max_positions:?}"
    );
}

#[test]
fn planet_database_renders_unowned_and_unknown_owner_rows_distinctly() {
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

    let sample_worlds = app
        .game_data
        .planets
        .records
        .iter()
        .enumerate()
        .filter(|(_, planet)| planet.owner_empire_slot_raw() != 1)
        .take(2)
        .map(|(idx, planet)| (idx + 1, planet.coords_raw()))
        .collect::<Vec<_>>();
    assert_eq!(
        sample_worlds.len(),
        2,
        "fixture should contain non-owned worlds"
    );

    app.planet_intel_snapshots.insert(
        sample_worlds[0].0,
        PlanetIntelSnapshot {
            planet_record_index_1_based: sample_worlds[0].0,
            intel_tier: IntelTier::Partial,
            compat_is_orbit_seed: false,
            last_intel_year: Some(3000),
            seen_year: Some(3000),
            scout_year: Some(3000),
            known_name: Some("Known Unowned".to_string()),
            known_owner_empire_id: Some(0),
            known_potential_production: Some(76),
            known_armies: None,
            known_ground_batteries: None,
            known_starbase_count: None,
            known_current_production: None,
            known_stored_points: None,
            known_docked_summary: None,
            known_orbit_summary: None,
            compat_word_1e: None,
        },
    );

    advance_to_main_menu(&mut app);
    assert_eq!(
        apply_action(&mut app, Action::Planet(PlanetAction::OpenDatabase)),
        AppOutcome::Continue
    );

    app.render(&mut terminal).expect("render succeeds");
    assert!(
        line_containing(&terminal, "Known Unowned").contains("Unowned"),
        "{}",
        line_containing(&terminal, "Known Unowned")
    );
    let unknown_coords = format!(
        "({:02},{:02})",
        sample_worlds[1].1[0], sample_worlds[1].1[1]
    );
    assert!(
        line_containing(&terminal, &unknown_coords).contains("?"),
        "{}",
        line_containing(&terminal, &unknown_coords)
    );
    assert!(!terminal.lines.iter().any(|line| line.contains("#0")));
}

#[test]
fn planet_database_renders_icd_owner_label_for_civil_disorder_worlds() {
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

    let sample_world = app
        .game_data
        .planets
        .records
        .iter()
        .enumerate()
        .find(|(_, planet)| planet.owner_empire_slot_raw() != 1)
        .map(|(idx, _)| idx + 1)
        .expect("fixture should contain a non-owned world");

    app.game_data.player.records[1].set_civil_disorder_mode();
    app.planet_intel_snapshots.insert(
        sample_world,
        PlanetIntelSnapshot {
            planet_record_index_1_based: sample_world,
            intel_tier: IntelTier::Partial,
            compat_is_orbit_seed: false,
            last_intel_year: Some(3000),
            seen_year: Some(3000),
            scout_year: Some(3000),
            known_name: None,
            known_owner_empire_id: Some(2),
            known_potential_production: None,
            known_armies: None,
            known_ground_batteries: None,
            known_starbase_count: None,
            known_current_production: None,
            known_stored_points: None,
            known_docked_summary: None,
            known_orbit_summary: None,
            compat_word_1e: None,
        },
    );

    advance_to_main_menu(&mut app);
    assert_eq!(
        apply_action(&mut app, Action::Planet(PlanetAction::OpenDatabase)),
        AppOutcome::Continue
    );

    app.render(&mut terminal).expect("render succeeds");
    assert!(
        terminal.lines.iter().any(|line| line.contains("ICD")),
        "{}",
        terminal.lines.join("\n")
    );
}

#[test]
fn planet_menu_tax_prompt_renders_inline_command_and_warning_stack() {
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
    let current_tax = app.game_data.player.records[app.player.record_index_1_based - 1].tax_rate();

    advance_to_main_menu(&mut app);
    assert_eq!(
        apply_action(&mut app, Action::Planet(PlanetAction::OpenMenu)),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Planet(PlanetAction::OpenTaxPrompt)),
        AppOutcome::Continue
    );

    app.render(&mut terminal).expect("render succeeds");
    assert_eq!(
        line_containing(&terminal, "PLANET COMMAND <- Empire tax rate").trim_end(),
        format!(" PLANET COMMAND <- Empire tax rate (0 - 100) [{current_tax}] <Q> ->")
    );
    assert_eq!(terminal.line(7).trim_end(), "");
    assert_eq!(
        line_containing(&terminal, "PLANET TAX: ").trim_end(),
        " PLANET TAX: Set empire tax rate."
    );
    assert!(
        line_containing(&terminal, "Warning: ")
            .contains("Taxes in excess of 65% may actually REDUCE"),
        "expected inline tax warning block below the helper message"
    );
}

#[test]
fn planet_menu_tax_prompt_stays_inline_for_errors_and_returns_to_menu_on_success() {
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
        apply_action(&mut app, Action::Planet(PlanetAction::OpenMenu)),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Planet(PlanetAction::OpenTaxPrompt)),
        AppOutcome::Continue
    );
    for ch in ['9', '9', '9'] {
        assert_eq!(
            apply_action(&mut app, Action::Planet(PlanetAction::AppendTaxChar(ch))),
            AppOutcome::Continue
        );
    }
    assert_eq!(
        apply_action(&mut app, Action::Planet(PlanetAction::SubmitTax)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::PlanetMenu);

    app.render(&mut terminal).expect("render succeeds");
    assert!(
        line_containing(&terminal, "Error: ").contains("Enter an integer tax rate from 0 to 100."),
        "expected inline tax validation error"
    );

    assert_eq!(
        apply_action(&mut app, Action::Planet(PlanetAction::BackspaceTaxInput)),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Planet(PlanetAction::BackspaceTaxInput)),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Planet(PlanetAction::BackspaceTaxInput)),
        AppOutcome::Continue
    );
    for ch in ['6', '5'] {
        assert_eq!(
            apply_action(&mut app, Action::Planet(PlanetAction::AppendTaxChar(ch))),
            AppOutcome::Continue
        );
    }
    assert_eq!(
        apply_action(&mut app, Action::Planet(PlanetAction::SubmitTax)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::PlanetMenu);

    app.render(&mut terminal).expect("render succeeds");
    assert!(
        !terminal
            .lines
            .iter()
            .any(|line| line.contains("PLANET COMMAND <- Empire tax rate"))
    );
    assert!(
        line_containing(&terminal, "Notice: ").contains("Empire tax rate set to 65%."),
        "expected command-menu success notice after saving tax"
    );
}

#[test]
fn planet_menu_scorch_prompt_defaults_to_lowest_owned_planet_and_validates_ownership() {
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
        apply_action(&mut app, Action::Planet(PlanetAction::OpenMenu)),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Planet(PlanetAction::OpenScorchPrompt)),
        AppOutcome::Continue
    );

    let expected_default = app
        .game_data
        .empire_planet_economy_rows(app.player.record_index_1_based)
        .into_iter()
        .min_by(|left, right| {
            left.present_production
                .cmp(&right.present_production)
                .then_with(|| left.coords.cmp(&right.coords))
        })
        .expect("owned planet exists")
        .coords;

    app.render(&mut terminal).expect("render succeeds");
    assert_eq!(
        line_containing(&terminal, "PLANET COMMAND <- Scorch Planet XX").trim_end(),
        format!(
            " PLANET COMMAND <- Scorch Planet XX [{}] <Q> ->",
            nc_game::screen::format_sector_coords_default(expected_default)
        )
    );

    let foreign_coords = app
        .game_data
        .planets
        .records
        .iter()
        .find(|planet| planet.owner_empire_slot_raw() as usize != app.player.record_index_1_based)
        .map(|planet| planet.coords_raw())
        .expect("non-owned planet exists");
    for ch in foreign_coords[0].to_string().chars() {
        assert_eq!(
            apply_action(
                &mut app,
                Action::Planet(PlanetAction::AppendScorchPromptChar(ch))
            ),
            AppOutcome::Continue
        );
    }
    assert_eq!(
        apply_action(
            &mut app,
            Action::Planet(PlanetAction::AppendScorchPromptChar(','))
        ),
        AppOutcome::Continue
    );
    for ch in foreign_coords[1].to_string().chars() {
        assert_eq!(
            apply_action(
                &mut app,
                Action::Planet(PlanetAction::AppendScorchPromptChar(ch))
            ),
            AppOutcome::Continue
        );
    }
    assert_eq!(
        apply_action(&mut app, Action::Planet(PlanetAction::SubmitScorchPrompt)),
        AppOutcome::Continue
    );

    terminal = CaptureTerminal::new();
    app.render(&mut terminal).expect("render succeeds");
    assert!(
        line_containing(&terminal, "Error: ").contains(&format!(
            "Planet [{},{}] is not one of your worlds.",
            foreign_coords[0], foreign_coords[1]
        )),
        "expected inline ownership validation error"
    );
}

#[test]
fn planet_menu_scorch_three_confirms_persist_order_and_update_planet_info_status() {
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
        apply_action(&mut app, Action::Planet(PlanetAction::OpenMenu)),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Planet(PlanetAction::OpenScorchPrompt)),
        AppOutcome::Continue
    );

    let expected_coords = app
        .game_data
        .empire_planet_economy_rows(app.player.record_index_1_based)
        .into_iter()
        .min_by(|left, right| {
            left.present_production
                .cmp(&right.present_production)
                .then_with(|| left.coords.cmp(&right.coords))
        })
        .expect("owned planet exists")
        .coords;
    let expected_record_index = app
        .game_data
        .planet_record_index_at_coords(expected_coords)
        .expect("selected scorch planet index");
    let expected_name =
        app.game_data.planets.records[expected_record_index].status_or_name_summary();

    assert_eq!(
        apply_action(&mut app, Action::Planet(PlanetAction::SubmitScorchPrompt)),
        AppOutcome::Continue
    );
    app.render(&mut terminal).expect("render succeeds");
    assert!(line_containing(&terminal, "SETTING SCORCH-EARTH POLICY:").contains("SCORCH-EARTH"));
    assert!(
        line_containing(&terminal, "Are you sure? Y/[N] ->").contains("Are you sure? Y/[N] ->")
    );

    assert_eq!(
        apply_action(&mut app, Action::Planet(PlanetAction::SubmitScorchPrompt)),
        AppOutcome::Continue
    );
    terminal = CaptureTerminal::new();
    app.render(&mut terminal).expect("render succeeds");
    assert!(
        line_containing(&terminal, "Are you really sure? Y/[N] ->")
            .contains("Are you really sure? Y/[N] ->")
    );

    assert_eq!(
        apply_action(&mut app, Action::Planet(PlanetAction::SubmitScorchPrompt)),
        AppOutcome::Continue
    );
    terminal = CaptureTerminal::new();
    app.render(&mut terminal).expect("render succeeds");
    assert!(
        line_containing(
            &terminal,
            "Are you sure-sure? Last chance to bail! Y/[N] ->"
        )
        .contains("Are you sure-sure? Last chance to bail! Y/[N] ->")
    );

    assert_eq!(
        apply_action(&mut app, Action::Planet(PlanetAction::SubmitScorchPrompt)),
        AppOutcome::Continue
    );
    assert!(
        app.planet_scorch_orders
            .contains(&(expected_record_index + 1))
    );

    terminal = CaptureTerminal::new();
    app.render(&mut terminal).expect("render succeeds");
    assert!(
        line_containing(&terminal, "Notice: ")
            .contains(&format!("Planet \"{expected_name}\" is scorched!")),
        "expected success notice after final confirmation"
    );

    let scorched_planet = &app.game_data.planets.records[expected_record_index];
    assert_eq!(scorched_planet.present_production_points(), Some(0));
    assert_eq!(scorched_planet.potential_production_points(), 0);
    assert_eq!(scorched_planet.stored_production_points(), 0);
    assert!((0..10).all(|slot| scorched_planet.build_count_raw(slot) == 0));
    assert!((0..10).all(|slot| scorched_planet.build_kind_raw(slot) == 0));
    assert!(
        (0..nc_data::STARDOCK_SLOT_COUNT).all(|slot| scorched_planet.stardock_count_raw(slot) == 0)
    );
    assert!(
        (0..nc_data::STARDOCK_SLOT_COUNT).all(|slot| scorched_planet.stardock_kind_raw(slot) == 0)
    );

    let reloaded = latest_runtime_state(&fixture_dir);
    assert!(
        reloaded
            .planet_scorch_orders
            .contains(&(expected_record_index + 1))
    );
    let reloaded_planet = &reloaded.game_data.planets.records[expected_record_index];
    assert_eq!(reloaded_planet.present_production_points(), Some(0));
    assert_eq!(reloaded_planet.potential_production_points(), 0);
    assert_eq!(reloaded_planet.stored_production_points(), 0);

    app.open_planet_info_detail_at_coords(expected_coords, Some(ScreenId::PlanetMenu))
        .expect("open planet info detail");
    terminal = CaptureTerminal::new();
    app.render(&mut terminal).expect("render succeeds");
    assert!(
        line_containing(&terminal, "Production").contains('0'),
        "production line should show zero"
    );
    assert!(
        line_containing(&terminal, "Potential Production").contains('0'),
        "potential production line should show zero"
    );
    assert!(
        line_containing(&terminal, "Treasury").contains('0'),
        "stored points line should show zero"
    );
    let build_queue_line = line_containing(&terminal, "Building");
    let stardock_line = line_containing(&terminal, "Docked");
    assert!(build_queue_line.contains("Nothing"));
    assert!(stardock_line.contains("Nothing"));
    assert_eq!(
        build_queue_line.find(':'),
        stardock_line.find(':'),
        "build queue and stardock separators should line up"
    );
    assert!(
        line_containing(&terminal, "Planet is scorched!").contains("Planet is scorched!"),
        "status line should report the scorched planet"
    );
}

#[test]
fn planet_menu_scorch_confirm_enter_honors_default_no_and_bails() {
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
    assert_eq!(
        apply_action(&mut app, Action::Planet(PlanetAction::OpenScorchPrompt)),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Planet(PlanetAction::SubmitScorchPrompt)),
        AppOutcome::Continue
    );
    assert_eq!(
        app.handle_key(key(KeyCode::Enter)),
        Action::Planet(PlanetAction::CancelScorchPrompt)
    );
    let enter_action = app.handle_key(key(KeyCode::Enter));
    assert_eq!(apply_action(&mut app, enter_action), AppOutcome::Continue);
    assert_eq!(app.current_screen, ScreenId::PlanetMenu);
    assert!(app.planet.scorch_prompt_mode.is_none());
    assert!(app.planet_scorch_orders.is_empty());
}

#[test]
fn planet_build_specify_uses_bottom_command_line_default_prompt() {
    let mut screen = PlanetBuildScreen::new();
    let view = PlanetBuildMenuView {
        row: EmpirePlanetEconomyRow {
            planet_record_index_1_based: 1,
            planet_name: "Loki".to_string(),
            coords: [16, 13],
            present_production: 50,
            potential_production: 75,
            stored_production_points: 40,
            build_capacity: 50,
            yearly_tax_revenue: 10,
            yearly_growth_delta: 5,
            armies: 10,
            ground_batteries: 5,
            has_friendly_starbase: false,
            is_homeworld_seed: false,
        },
        committed_points: 10,
        budget: 40,
        points_left: 30,
        building_count: 1,
        docked_count: 0,
    };
    let orders = vec![PlanetBuildOrder {
        kind: ProductionItemKind::Destroyer,
        points_remaining: 5,
    }];

    let buffer = screen
        .render_specify(&view, &orders, "", None, None)
        .expect("build specify renders");

    assert!((0..buffer.height()).any(|row| {
        buffer
            .plain_line(row)
            .contains("COMMAND <- Unit number or 0 if done")
    }));
    assert!((0..buffer.height()).any(|row| buffer.plain_line(row).contains("[0] <Q> ->")));
    assert!(!(0..buffer.height()).any(|row| {
        buffer
            .plain_line(row)
            .contains("You have spent 10 out of 40 points.")
    }));
}

#[test]
fn planet_build_quantity_uses_bottom_command_line_default_prompt() {
    let mut screen = PlanetBuildScreen::new();
    let view = PlanetBuildMenuView {
        row: EmpirePlanetEconomyRow {
            planet_record_index_1_based: 1,
            planet_name: "Loki".to_string(),
            coords: [16, 13],
            present_production: 50,
            potential_production: 75,
            stored_production_points: 40,
            build_capacity: 50,
            yearly_tax_revenue: 10,
            yearly_growth_delta: 5,
            armies: 10,
            ground_batteries: 5,
            has_friendly_starbase: false,
            is_homeworld_seed: false,
        },
        committed_points: 10,
        budget: 40,
        points_left: 30,
        building_count: 1,
        docked_count: 0,
    };
    let orders = vec![PlanetBuildOrder {
        kind: ProductionItemKind::Destroyer,
        points_remaining: 5,
    }];

    let buffer = screen
        .render_quantity_prompt(
            &view,
            &orders,
            nc_game::screen::build_unit_spec(1).expect("destroyer spec"),
            6,
            "",
            None,
        )
        .expect("build quantity renders");

    assert!((0..buffer.height()).any(|row| {
        buffer
            .plain_line(row)
            .contains("COMMAND <- How many new destroyers to build")
    }));
    assert!((0..buffer.height()).any(|row| buffer.plain_line(row).contains("[6] <Q> ->")));
    assert!(!(0..buffer.height()).any(|row| {
        buffer
            .plain_line(row)
            .contains("You have spent 10 out of 40 points.")
    }));
}

#[test]
fn planet_build_specify_renders_success_as_notice_not_error() {
    let mut screen = PlanetBuildScreen::new();
    let view = PlanetBuildMenuView {
        row: EmpirePlanetEconomyRow {
            planet_record_index_1_based: 1,
            planet_name: "Loki".to_string(),
            coords: [16, 13],
            present_production: 50,
            potential_production: 75,
            stored_production_points: 40,
            build_capacity: 50,
            yearly_tax_revenue: 10,
            yearly_growth_delta: 5,
            armies: 10,
            ground_batteries: 5,
            has_friendly_starbase: false,
            is_homeworld_seed: false,
        },
        committed_points: 10,
        budget: 40,
        points_left: 30,
        building_count: 1,
        docked_count: 0,
    };
    let orders = vec![PlanetBuildOrder {
        kind: ProductionItemKind::Destroyer,
        points_remaining: 10,
    }];

    let buffer = screen
        .render_specify(&view, &orders, "", None, Some("Queued 2 Destroyers."))
        .expect("build specify renders with notice");

    assert!(!(0..buffer.height()).any(|row| {
        buffer
            .plain_line(row)
            .contains("Notice: Queued 2 Destroyers.")
    }));
    assert!(!(0..buffer.height()).any(|row| buffer.plain_line(row).contains("Error:")));
}
