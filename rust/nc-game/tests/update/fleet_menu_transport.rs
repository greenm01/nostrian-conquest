use crate::support::*;
use nc_game::domains::fleet::screens::fleet::FleetListScreen;
use nc_game::screen::ScreenGeometry;

fn owned_fleet_mut(
    state: &mut CampaignRuntimeState,
    fleet_number: u16,
) -> &mut nc_data::FleetRecord {
    state
        .game_data
        .fleets
        .records
        .iter_mut()
        .find(|fleet| fleet.owner_empire_raw() == 1 && fleet.local_slot_word_raw() == fleet_number)
        .unwrap_or_else(|| panic!("fleet #{fleet_number} should exist"))
}

fn set_fleet_ship_profile(
    fleet: &mut nc_data::FleetRecord,
    battleships: u16,
    cruisers: u16,
    destroyers: u16,
    troop_transports: u16,
    scouts: u16,
    etacs: u16,
) {
    fleet.set_battleship_count(battleships);
    fleet.set_cruiser_count(cruisers);
    fleet.set_destroyer_count(destroyers);
    fleet.set_troop_transport_count(troop_transports);
    fleet.set_scout_count(scouts);
    fleet.set_etac_count(etacs);
}

fn assert_order_prompt_default(app: &mut App, expected: u16) {
    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("order prompt should render");
    assert!(
        line_containing(&terminal, "COMMAND <- Order Fleet #")
            .contains(&format!("Order Fleet # [{expected}] <Q> ->"))
    );
}

#[test]
fn fleet_review_opens_with_an_inline_prompt_first() {
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
    assert_eq!(app.current_screen(), ScreenId::FleetMenu);

    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("fleet menu prompt should render");
    let prompt = line_containing(&terminal, "COMMAND <- Review Fleet #");
    assert!(prompt.contains("Review Fleet # ["));
    assert!(prompt.contains("<Q> ->"));
}

#[test]
fn fleet_order_prompt_uses_smart_default_while_other_prompts_keep_strongest_default() {
    let fixture_dir = temp_game_copy();
    let mut state = latest_runtime_state(&fixture_dir);
    {
        let fleet = owned_fleet_mut(&mut state, 1);
        set_fleet_ship_profile(fleet, 0, 0, 0, 0, 0, 1);
        fleet.set_standing_order_kind(nc_data::Order::HoldPosition);
        fleet.set_standing_order_target_coords_raw([0, 0]);
    }
    {
        let fleet = owned_fleet_mut(&mut state, 2);
        set_fleet_ship_profile(fleet, 9, 0, 0, 0, 0, 0);
        fleet.set_standing_order_kind(nc_data::Order::MoveOnly);
        fleet.set_standing_order_target_coords_raw([14, 9]);
    }
    save_runtime_state(&fixture_dir, &state);
    assert_eq!(strongest_owned_fleet_number(&fixture_dir), 2);

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
    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("review prompt should render");
    assert!(
        line_containing(&terminal, "COMMAND <- Review Fleet #")
            .contains("Review Fleet # [2] <Q> ->")
    );

    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::CancelMenuPrompt)),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::OpenChangePrompt)),
        AppOutcome::Continue
    );
    app.render(&mut terminal)
        .expect("change prompt should render");
    assert!(
        line_containing(&terminal, "COMMAND <- Change Fleet #")
            .contains("Change Fleet # [2] <Q> ->")
    );

    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::CancelMenuPrompt)),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::OpenEta)),
        AppOutcome::Continue
    );
    app.render(&mut terminal).expect("eta prompt should render");
    assert!(
        line_containing(&terminal, "COMMAND <- ETA Fleet #").contains("ETA Fleet # [2] <Q> ->")
    );

    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::CancelMenuPrompt)),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::OpenOrder)),
        AppOutcome::Continue
    );
    assert_order_prompt_default(&mut app, 1);
}

#[test]
fn fleet_order_prompt_prefers_ready_etac_fleets_over_stronger_ready_non_etac_fleets() {
    let fixture_dir = temp_game_copy();
    let mut state = latest_runtime_state(&fixture_dir);
    {
        let fleet = owned_fleet_mut(&mut state, 1);
        set_fleet_ship_profile(fleet, 0, 0, 0, 0, 0, 1);
        fleet.set_standing_order_kind(nc_data::Order::HoldPosition);
        fleet.set_standing_order_target_coords_raw([0, 0]);
    }
    {
        let fleet = owned_fleet_mut(&mut state, 2);
        set_fleet_ship_profile(fleet, 9, 0, 0, 0, 0, 0);
        fleet.set_standing_order_kind(nc_data::Order::HoldPosition);
        fleet.set_standing_order_target_coords_raw([0, 0]);
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
    advance_to_main_menu(&mut app);
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::OpenMenu)),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::OpenOrder)),
        AppOutcome::Continue
    );

    assert_order_prompt_default(&mut app, 1);
}

#[test]
fn fleet_order_prompt_avoids_patrol_and_blockade_fleets_when_other_owned_fleets_exist() {
    let fixture_dir = temp_game_copy();
    let mut state = latest_runtime_state(&fixture_dir);
    {
        let fleet = owned_fleet_mut(&mut state, 1);
        set_fleet_ship_profile(fleet, 1, 0, 0, 0, 0, 0);
        fleet.set_standing_order_kind(nc_data::Order::MoveOnly);
        fleet.set_standing_order_target_coords_raw([14, 9]);
    }
    {
        let fleet = owned_fleet_mut(&mut state, 2);
        let current = fleet.current_location_coords_raw();
        set_fleet_ship_profile(fleet, 9, 0, 0, 0, 0, 1);
        fleet.set_standing_order_kind(nc_data::Order::GuardBlockadeWorld);
        fleet.set_standing_order_target_coords_raw(current);
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
    advance_to_main_menu(&mut app);
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::OpenMenu)),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::OpenOrder)),
        AppOutcome::Continue
    );

    assert_order_prompt_default(&mut app, 1);
}

#[test]
fn fleet_order_prompt_prefers_combat_only_fallback_over_active_etac_fleets() {
    let fixture_dir = temp_game_copy();
    let mut state = latest_runtime_state(&fixture_dir);
    {
        let fleet = owned_fleet_mut(&mut state, 1);
        let current = fleet.current_location_coords_raw();
        set_fleet_ship_profile(fleet, 0, 0, 0, 0, 0, 1);
        fleet.set_standing_order_kind(nc_data::Order::PatrolSector);
        fleet.set_standing_order_target_coords_raw(current);
    }
    {
        let fleet = owned_fleet_mut(&mut state, 2);
        let current = fleet.current_location_coords_raw();
        set_fleet_ship_profile(fleet, 9, 0, 0, 0, 0, 0);
        fleet.set_standing_order_kind(nc_data::Order::GuardBlockadeWorld);
        fleet.set_standing_order_target_coords_raw(current);
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
    advance_to_main_menu(&mut app);
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::OpenMenu)),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::OpenOrder)),
        AppOutcome::Continue
    );

    assert_order_prompt_default(&mut app, 2);
}

#[test]
fn fleet_order_prompt_uses_strength_when_no_ready_non_patrol_fleets_exist() {
    let fixture_dir = temp_game_copy();
    let mut state = latest_runtime_state(&fixture_dir);
    {
        let fleet = owned_fleet_mut(&mut state, 1);
        set_fleet_ship_profile(fleet, 0, 0, 0, 0, 0, 1);
        fleet.set_standing_order_kind(nc_data::Order::MoveOnly);
        fleet.set_standing_order_target_coords_raw([14, 9]);
    }
    {
        let fleet = owned_fleet_mut(&mut state, 2);
        set_fleet_ship_profile(fleet, 9, 0, 0, 0, 0, 0);
        fleet.set_standing_order_kind(nc_data::Order::ViewWorld);
        fleet.set_standing_order_target_coords_raw([13, 9]);
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
    advance_to_main_menu(&mut app);
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::OpenMenu)),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::OpenOrder)),
        AppOutcome::Continue
    );

    assert_order_prompt_default(&mut app, 2);
}

#[test]
fn fleet_order_prompt_fallback_ignores_active_same_system_colonize_orders() {
    let fixture_dir = temp_game_copy();
    let mut state = latest_runtime_state(&fixture_dir);
    for fleet_number in [1_u16, 2] {
        let fleet = owned_fleet_mut(&mut state, fleet_number);
        let current = fleet.current_location_coords_raw();
        fleet.set_standing_order_kind(nc_data::Order::ColonizeWorld);
        fleet.set_standing_order_target_coords_raw(current);
    }
    for fleet_number in [3_u16, 4] {
        let fleet = owned_fleet_mut(&mut state, fleet_number);
        let current = fleet.current_location_coords_raw();
        fleet.set_standing_order_kind(nc_data::Order::GuardBlockadeWorld);
        fleet.set_standing_order_target_coords_raw(current);
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
    advance_to_main_menu(&mut app);
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::OpenMenu)),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::OpenOrder)),
        AppOutcome::Continue
    );

    assert_order_prompt_default(&mut app, 3);
}

#[test]
fn fleet_order_prompt_fallback_ignores_mixed_combat_fleets_with_scouts_or_etacs() {
    let fixture_dir = temp_game_copy();
    let mut state = latest_runtime_state(&fixture_dir);
    {
        let fleet = owned_fleet_mut(&mut state, 1);
        set_fleet_ship_profile(fleet, 0, 4, 0, 0, 0, 1);
        fleet.set_standing_order_kind(nc_data::Order::MoveOnly);
        fleet.set_standing_order_target_coords_raw([14, 9]);
    }
    {
        let fleet = owned_fleet_mut(&mut state, 2);
        set_fleet_ship_profile(fleet, 0, 0, 3, 0, 1, 0);
        fleet.set_standing_order_kind(nc_data::Order::ViewWorld);
        fleet.set_standing_order_target_coords_raw([13, 9]);
    }
    {
        let fleet = owned_fleet_mut(&mut state, 3);
        set_fleet_ship_profile(fleet, 0, 0, 1, 0, 0, 0);
        let current = fleet.current_location_coords_raw();
        fleet.set_standing_order_kind(nc_data::Order::GuardBlockadeWorld);
        fleet.set_standing_order_target_coords_raw(current);
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
    advance_to_main_menu(&mut app);
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::OpenMenu)),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::OpenOrder)),
        AppOutcome::Continue
    );

    assert_order_prompt_default(&mut app, 3);
}

#[test]
fn fleet_order_prompt_prefers_newly_auto_commissioned_fleet_first() {
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
    assert_eq!(
        apply_action(
            &mut app,
            Action::Planet(PlanetAction::ConfirmAutoCommission)
        ),
        AppOutcome::Continue
    );

    app.open_fleet_menu();
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::OpenOrder)),
        AppOutcome::Continue
    );

    assert_order_prompt_default(&mut app, 5);
}

#[test]
fn fleet_order_prompt_prefers_most_recent_newly_commissioned_fleet() {
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
        apply_action(&mut app, Action::Planet(PlanetAction::OpenCommissionMenu)),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Planet(PlanetAction::OpenCommissionPlanet)),
        AppOutcome::Continue
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
    assert_eq!(app.current_screen(), ScreenId::PlanetCommissionDraft);
    assert_eq!(
        apply_action(
            &mut app,
            Action::Planet(PlanetAction::SubmitCommissionDraft)
        ),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::PlanetCommissionDraft);

    app.open_fleet_menu();
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::OpenOrder)),
        AppOutcome::Continue
    );

    assert_order_prompt_default(&mut app, 6);
}

#[test]
fn fleet_review_prompt_accepts_typed_fleet_id_and_opens_that_fleet() {
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
    open_review_from_fleet_menu(&mut app, Some(1));
    assert_eq!(app.current_screen(), ScreenId::FleetReview);
    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("fleet review detail should render");
    assert!(terminal.line(2).contains("Fleet ID: 1"));
}

#[test]
fn fleet_review_close_returns_to_menu_without_restoring_review_prompt() {
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
    open_review_from_fleet_menu(&mut app, Some(2));
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::CloseReview)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FleetMenu);

    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("fleet menu should render after closing review");
    assert!(
        !terminal
            .lines
            .iter()
            .any(|line| line.contains("COMMAND <- Review Fleet #"))
    );
    assert_eq!(
        app.handle_key(key(KeyCode::Char('r'))),
        Action::Fleet(FleetAction::OpenReviewPrompt)
    );
}

#[test]
fn fleet_review_prompt_shows_invalid_fleet_message_on_unknown_typed_id() {
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
    for ch in ['9', '9'] {
        assert_eq!(
            apply_action(
                &mut app,
                Action::Fleet(FleetAction::AppendMenuPromptChar(ch))
            ),
            AppOutcome::Continue
        );
    }
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::SubmitMenuPrompt)),
        AppOutcome::Continue
    );
    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("fleet menu prompt should render invalid id notice");
    assert!(line_containing(&terminal, "COMMAND <- Review Fleet #").contains("Review Fleet #"));
    assert!(
        terminal
            .lines
            .iter()
            .any(|line| line.contains("Fleet #99 is not in your fleet list."))
    );
}

#[test]
fn fleet_menu_load_and_unload_keys_open_fleet_transport_flow() {
    let fixture_dir = temp_game_copy();
    let mut state = latest_runtime_state(&fixture_dir);
    let homeworld_index =
        state.game_data.player.records[0].homeworld_planet_index_1_based_raw() as usize - 1;
    let home_coords = state.game_data.planets.records[homeworld_index].coords_raw();
    let fleet_one = state
        .game_data
        .fleets
        .records
        .iter_mut()
        .find(|fleet| fleet.owner_empire_raw() == 1 && fleet.local_slot_word_raw() == 1)
        .expect("fleet #1 should exist");
    fleet_one.set_current_location_coords_raw(home_coords);
    fleet_one.set_battleship_count(0);
    fleet_one.set_cruiser_count(0);
    fleet_one.set_destroyer_count(0);
    fleet_one.set_troop_transport_count(5);
    fleet_one.set_army_count(1);
    fleet_one.set_scout_count(0);
    fleet_one.set_etac_count(0);
    fleet_one.recompute_max_speed_from_composition();

    let fleet_two = state
        .game_data
        .fleets
        .records
        .iter_mut()
        .find(|fleet| fleet.owner_empire_raw() == 1 && fleet.local_slot_word_raw() == 2)
        .expect("fleet #2 should exist");
    fleet_two.set_current_location_coords_raw(home_coords);
    fleet_two.set_battleship_count(4);
    fleet_two.set_cruiser_count(0);
    fleet_two.set_destroyer_count(0);
    fleet_two.set_troop_transport_count(3);
    fleet_two.set_army_count(3);
    fleet_two.set_scout_count(0);
    fleet_two.set_etac_count(0);
    fleet_two.recompute_max_speed_from_composition();
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
        app.handle_key(key(KeyCode::Char('l'))),
        Action::Fleet(FleetAction::OpenTransportLoad)
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::OpenTransportLoad)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FleetMenu);
    assert_eq!(app.fleet.menu_prompt_default_value, "1");

    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("fleet load prompt should render");
    let prompt = line_containing(&terminal, "COMMAND <- Load Fleet #");
    assert!(prompt.contains("Load Fleet # ["));
    assert!(prompt.contains("<Q> ->"));
    submit_fleet_menu_prompt(&mut app, Some(1));
    assert_eq!(app.current_screen(), ScreenId::FleetMenu);
    app.render(&mut terminal)
        .expect("fleet load quantity prompt should render inline");
    assert!(
        terminal
            .lines
            .iter()
            .any(|line| line.contains("LOAD ARMIES ONTO TROOP TRANSPORTS:"))
    );
    assert!(
        terminal
            .lines
            .iter()
            .any(|line| line.contains("Planet:") && line.contains("Fleet 01"))
    );
    let prompt = line_containing(&terminal, "COMMAND <- How many armies to load?");
    assert!(prompt.contains("How many armies to load? [4]"));
    assert!(prompt.contains("<Q> ->"));
    assert!(
        terminal
            .lines
            .iter()
            .all(|line| !line.contains("Load Planet XX,YY"))
    );
    assert_eq!(
        app.handle_key(key(KeyCode::Char('q'))),
        Action::Fleet(FleetAction::CancelMenuPrompt)
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::CancelMenuPrompt)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FleetMenu);

    assert_eq!(
        app.handle_key(key(KeyCode::Char('u'))),
        Action::Fleet(FleetAction::OpenTransportUnload)
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::OpenTransportUnload)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FleetMenu);
    assert_eq!(app.fleet.menu_prompt_default_value, "2");
    app.render(&mut terminal)
        .expect("fleet unload prompt should render");
    let prompt = line_containing(&terminal, "COMMAND <- Unload Fleet #");
    assert!(prompt.contains("Unload Fleet # ["));
    assert!(prompt.contains("<Q> ->"));
    submit_fleet_menu_prompt(&mut app, Some(2));
    assert_eq!(app.current_screen(), ScreenId::FleetMenu);
    app.render(&mut terminal)
        .expect("fleet unload quantity prompt should render inline");
    assert!(
        terminal
            .lines
            .iter()
            .any(|line| line.contains("UNLOAD ARMIES FROM TROOP TRANSPORTS:"))
    );
    assert!(
        terminal
            .lines
            .iter()
            .any(|line| line.contains("Planet:") && line.contains("Fleet 02"))
    );
    let prompt = line_containing(&terminal, "COMMAND <- How many armies to unload?");
    assert!(prompt.contains("How many armies to unload? [3]"));
    assert!(prompt.contains("<Q> ->"));
}

#[test]
fn fleet_transport_load_prompt_rejects_fleet_not_at_owned_world() {
    let fixture_dir = temp_game_copy();
    let mut state = latest_runtime_state(&fixture_dir);
    let homeworld_index =
        state.game_data.player.records[0].homeworld_planet_index_1_based_raw() as usize - 1;
    let home_coords = state.game_data.planets.records[homeworld_index].coords_raw();
    let fleet_one = state
        .game_data
        .fleets
        .records
        .iter_mut()
        .find(|fleet| fleet.owner_empire_raw() == 1 && fleet.local_slot_word_raw() == 1)
        .expect("fleet #1 should exist");
    fleet_one.set_current_location_coords_raw([1, 1]);
    fleet_one.set_troop_transport_count(4);
    fleet_one.set_army_count(0);
    fleet_one.recompute_max_speed_from_composition();
    let fleet_two = state
        .game_data
        .fleets
        .records
        .iter_mut()
        .find(|fleet| fleet.owner_empire_raw() == 1 && fleet.local_slot_word_raw() == 2)
        .expect("fleet #2 should exist");
    fleet_two.set_current_location_coords_raw(home_coords);
    fleet_two.set_troop_transport_count(2);
    fleet_two.set_army_count(0);
    fleet_two.recompute_max_speed_from_composition();
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
        apply_action(&mut app, Action::Fleet(FleetAction::OpenTransportLoad)),
        AppOutcome::Continue
    );
    submit_fleet_menu_prompt(&mut app, Some(1));

    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("fleet load prompt should render owned-world warning");
    assert_eq!(app.current_screen(), ScreenId::FleetMenu);
    assert_eq!(
        app.fleet
            .menu_prompt_status
            .as_ref()
            .map(|feedback| feedback.message()),
        Some("That fleet is not at one of your worlds.")
    );
}

#[test]
fn fleet_transport_unload_prompt_rejects_fleet_not_at_owned_world() {
    let fixture_dir = temp_game_copy();
    let mut state = latest_runtime_state(&fixture_dir);
    let homeworld_index =
        state.game_data.player.records[0].homeworld_planet_index_1_based_raw() as usize - 1;
    let home_coords = state.game_data.planets.records[homeworld_index].coords_raw();
    let fleet_one = state
        .game_data
        .fleets
        .records
        .iter_mut()
        .find(|fleet| fleet.owner_empire_raw() == 1 && fleet.local_slot_word_raw() == 1)
        .expect("fleet #1 should exist");
    fleet_one.set_current_location_coords_raw([1, 1]);
    fleet_one.set_troop_transport_count(4);
    fleet_one.set_army_count(2);
    fleet_one.recompute_max_speed_from_composition();
    let fleet_two = state
        .game_data
        .fleets
        .records
        .iter_mut()
        .find(|fleet| fleet.owner_empire_raw() == 1 && fleet.local_slot_word_raw() == 2)
        .expect("fleet #2 should exist");
    fleet_two.set_current_location_coords_raw(home_coords);
    fleet_two.set_troop_transport_count(2);
    fleet_two.set_army_count(1);
    fleet_two.recompute_max_speed_from_composition();
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
        apply_action(&mut app, Action::Fleet(FleetAction::OpenTransportUnload)),
        AppOutcome::Continue
    );
    submit_fleet_menu_prompt(&mut app, Some(1));

    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("fleet unload prompt should render owned-world warning");
    assert_eq!(app.current_screen(), ScreenId::FleetMenu);
    assert_eq!(
        app.fleet
            .menu_prompt_status
            .as_ref()
            .map(|feedback| feedback.message()),
        Some("That fleet is not at one of your worlds.")
    );
}

#[test]
fn fleet_transport_load_prompt_requires_armies_on_owned_world() {
    let fixture_dir = temp_game_copy();
    let mut state = latest_runtime_state(&fixture_dir);
    let homeworld_index =
        state.game_data.player.records[0].homeworld_planet_index_1_based_raw() as usize - 1;
    let home_coords = state.game_data.planets.records[homeworld_index].coords_raw();
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
    state.game_data.planets.records[extra_owned_idx].set_army_count_raw(0);
    let other_coords = state.game_data.planets.records[extra_owned_idx].coords_raw();

    let fleet_one = state
        .game_data
        .fleets
        .records
        .iter_mut()
        .find(|fleet| fleet.owner_empire_raw() == 1 && fleet.local_slot_word_raw() == 1)
        .expect("fleet #1 should exist");
    fleet_one.set_current_location_coords_raw(other_coords);
    fleet_one.set_troop_transport_count(4);
    fleet_one.set_army_count(0);
    fleet_one.recompute_max_speed_from_composition();
    let fleet_two = state
        .game_data
        .fleets
        .records
        .iter_mut()
        .find(|fleet| fleet.owner_empire_raw() == 1 && fleet.local_slot_word_raw() == 2)
        .expect("fleet #2 should exist");
    fleet_two.set_current_location_coords_raw(home_coords);
    fleet_two.set_troop_transport_count(2);
    fleet_two.set_army_count(0);
    fleet_two.recompute_max_speed_from_composition();
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
        apply_action(&mut app, Action::Fleet(FleetAction::OpenTransportLoad)),
        AppOutcome::Continue
    );
    submit_fleet_menu_prompt(&mut app, Some(1));

    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("fleet load prompt should render no-armies warning");
    assert_eq!(app.current_screen(), ScreenId::FleetMenu);
    assert_eq!(
        app.fleet
            .menu_prompt_status
            .as_ref()
            .map(|feedback| feedback.message()),
        Some("That world has no armies available to load.")
    );
}

#[test]
fn fleet_transport_unload_prompt_requires_room_on_owned_world() {
    let fixture_dir = temp_game_copy();
    let mut state = latest_runtime_state(&fixture_dir);
    let homeworld_index =
        state.game_data.player.records[0].homeworld_planet_index_1_based_raw() as usize - 1;
    let home_coords = state.game_data.planets.records[homeworld_index].coords_raw();
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
    state.game_data.planets.records[extra_owned_idx].set_army_count_raw(u8::MAX);
    let other_coords = state.game_data.planets.records[extra_owned_idx].coords_raw();

    let fleet_one = state
        .game_data
        .fleets
        .records
        .iter_mut()
        .find(|fleet| fleet.owner_empire_raw() == 1 && fleet.local_slot_word_raw() == 1)
        .expect("fleet #1 should exist");
    fleet_one.set_current_location_coords_raw(other_coords);
    fleet_one.set_troop_transport_count(4);
    fleet_one.set_army_count(2);
    fleet_one.recompute_max_speed_from_composition();
    let fleet_two = state
        .game_data
        .fleets
        .records
        .iter_mut()
        .find(|fleet| fleet.owner_empire_raw() == 1 && fleet.local_slot_word_raw() == 2)
        .expect("fleet #2 should exist");
    fleet_two.set_current_location_coords_raw(home_coords);
    fleet_two.set_troop_transport_count(2);
    fleet_two.set_army_count(1);
    fleet_two.recompute_max_speed_from_composition();
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
        apply_action(&mut app, Action::Fleet(FleetAction::OpenTransportUnload)),
        AppOutcome::Continue
    );
    submit_fleet_menu_prompt(&mut app, Some(1));

    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("fleet unload prompt should render no-room warning");
    assert_eq!(app.current_screen(), ScreenId::FleetMenu);
    assert_eq!(
        app.fleet
            .menu_prompt_status
            .as_ref()
            .map(|feedback| feedback.message()),
        Some("That world has no room to receive unloaded armies.")
    );
}

#[test]
fn fleet_menu_load_and_unload_show_menu_notice_when_no_transport_action_is_available() {
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
        apply_action(&mut app, Action::Fleet(FleetAction::OpenTransportLoad)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FleetMenu);
    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("fleet menu should render load notice");
    assert!(terminal.lines.iter().any(|line| {
        line.contains("No planets have armies and troop transports ready to load.")
    }));

    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::OpenTransportUnload)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FleetMenu);
    app.render(&mut terminal)
        .expect("fleet menu should render unload notice");
    assert!(
        terminal
            .lines
            .iter()
            .any(|line| { line.contains("No fleets have loaded armies ready to unload") })
    );
}

#[test]
fn fleet_transport_quantity_prompt_stays_inline_on_fleet_menu() {
    let fixture_dir = temp_game_copy();
    let mut state = latest_runtime_state(&fixture_dir);
    let homeworld_index =
        state.game_data.player.records[0].homeworld_planet_index_1_based_raw() as usize - 1;
    let home_coords = state.game_data.planets.records[homeworld_index].coords_raw();
    let fleet = &mut state.game_data.fleets.records[0];
    fleet.set_current_location_coords_raw(home_coords);
    fleet.set_troop_transport_count(3);
    fleet.set_army_count(1);
    fleet.recompute_max_speed_from_composition();
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
        apply_action(&mut app, Action::Fleet(FleetAction::OpenTransportLoad)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FleetMenu);
    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("fleet load prompt should render");
    submit_fleet_menu_prompt(&mut app, Some(1));
    assert_eq!(app.current_screen(), ScreenId::FleetMenu);
    app.render(&mut terminal)
        .expect("fleet load quantity prompt should render");
    assert!(
        terminal
            .lines
            .iter()
            .all(|line| !line.contains("Load Planet XX,YY"))
    );
    assert!(line_containing(&terminal, "COMMAND <- How many armies to load?").contains("<Q> ->"));
}

#[test]
fn fleet_transport_load_prompt_rejects_fleet_already_full() {
    let fixture_dir = temp_game_copy();
    let mut state = latest_runtime_state(&fixture_dir);
    let homeworld_index =
        state.game_data.player.records[0].homeworld_planet_index_1_based_raw() as usize - 1;
    let home_coords = state.game_data.planets.records[homeworld_index].coords_raw();
    state.game_data.planets.records[homeworld_index].set_army_count_raw(12);
    let fleet = state
        .game_data
        .fleets
        .records
        .iter_mut()
        .find(|fleet| fleet.owner_empire_raw() == 1 && fleet.local_slot_word_raw() == 1)
        .expect("fleet #1 should exist");
    fleet.set_current_location_coords_raw(home_coords);
    fleet.set_troop_transport_count(4);
    fleet.set_army_count(4);
    fleet.recompute_max_speed_from_composition();
    let fleet_two = state
        .game_data
        .fleets
        .records
        .iter_mut()
        .find(|fleet| fleet.owner_empire_raw() == 1 && fleet.local_slot_word_raw() == 2)
        .expect("fleet #2 should exist");
    fleet_two.set_current_location_coords_raw(home_coords);
    fleet_two.set_troop_transport_count(2);
    fleet_two.set_army_count(0);
    fleet_two.recompute_max_speed_from_composition();
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
        apply_action(&mut app, Action::Fleet(FleetAction::OpenTransportLoad)),
        AppOutcome::Continue
    );
    submit_fleet_menu_prompt(&mut app, Some(1));
    assert_eq!(
        app.fleet
            .menu_prompt_status
            .as_ref()
            .map(|feedback| feedback.message()),
        Some("That fleet's troop transports are already full.")
    );
}

#[test]
fn fleet_transport_unload_prompt_rejects_fleet_already_empty() {
    let fixture_dir = temp_game_copy();
    let mut state = latest_runtime_state(&fixture_dir);
    let homeworld_index =
        state.game_data.player.records[0].homeworld_planet_index_1_based_raw() as usize - 1;
    let home_coords = state.game_data.planets.records[homeworld_index].coords_raw();
    let fleet = state
        .game_data
        .fleets
        .records
        .iter_mut()
        .find(|fleet| fleet.owner_empire_raw() == 1 && fleet.local_slot_word_raw() == 1)
        .expect("fleet #1 should exist");
    fleet.set_current_location_coords_raw(home_coords);
    fleet.set_troop_transport_count(4);
    fleet.set_army_count(0);
    fleet.recompute_max_speed_from_composition();
    let fleet_two = state
        .game_data
        .fleets
        .records
        .iter_mut()
        .find(|fleet| fleet.owner_empire_raw() == 1 && fleet.local_slot_word_raw() == 2)
        .expect("fleet #2 should exist");
    fleet_two.set_current_location_coords_raw(home_coords);
    fleet_two.set_troop_transport_count(2);
    fleet_two.set_army_count(1);
    fleet_two.recompute_max_speed_from_composition();
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
        apply_action(&mut app, Action::Fleet(FleetAction::OpenTransportUnload)),
        AppOutcome::Continue
    );
    submit_fleet_menu_prompt(&mut app, Some(1));
    assert_eq!(
        app.fleet
            .menu_prompt_status
            .as_ref()
            .map(|feedback| feedback.message()),
        Some("That fleet's troop transports are already empty.")
    );
}

#[test]
fn fleet_transport_load_default_skips_full_fleets_and_caps_qty_by_planet_armies() {
    let fixture_dir = temp_game_copy();
    let mut state = latest_runtime_state(&fixture_dir);
    let homeworld_index =
        state.game_data.player.records[0].homeworld_planet_index_1_based_raw() as usize - 1;
    let home_planet = &mut state.game_data.planets.records[homeworld_index];
    let home_coords = home_planet.coords_raw();
    home_planet.set_army_count_raw(2);

    let fleet_one = state
        .game_data
        .fleets
        .records
        .iter_mut()
        .find(|fleet| fleet.owner_empire_raw() == 1 && fleet.local_slot_word_raw() == 1)
        .expect("fleet #1 should exist");
    fleet_one.set_current_location_coords_raw(home_coords);
    fleet_one.set_troop_transport_count(6);
    fleet_one.set_army_count(6);
    fleet_one.recompute_max_speed_from_composition();

    let fleet_two = state
        .game_data
        .fleets
        .records
        .iter_mut()
        .find(|fleet| fleet.owner_empire_raw() == 1 && fleet.local_slot_word_raw() == 2)
        .expect("fleet #2 should exist");
    fleet_two.set_current_location_coords_raw(home_coords);
    fleet_two.set_troop_transport_count(5);
    fleet_two.set_army_count(1);
    fleet_two.recompute_max_speed_from_composition();

    let fleet_three = state
        .game_data
        .fleets
        .records
        .iter_mut()
        .find(|fleet| fleet.owner_empire_raw() == 1 && fleet.local_slot_word_raw() == 3)
        .expect("fleet #3 should exist");
    fleet_three.set_current_location_coords_raw(home_coords);
    fleet_three.set_troop_transport_count(6);
    fleet_three.set_army_count(0);
    fleet_three.recompute_max_speed_from_composition();

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
        apply_action(&mut app, Action::Fleet(FleetAction::OpenTransportLoad)),
        AppOutcome::Continue
    );
    assert_eq!(app.fleet.menu_prompt_default_value, "3");
    submit_fleet_menu_prompt(&mut app, None);
    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("fleet load quantity prompt should render");
    let prompt = line_containing(&terminal, "COMMAND <- How many armies to load?");
    assert!(prompt.contains("[2]"));
}

#[test]
fn fleet_transport_unload_default_skips_empty_fleets_and_caps_qty_by_planet_capacity() {
    let fixture_dir = temp_game_copy();
    let mut state = latest_runtime_state(&fixture_dir);
    let homeworld_index =
        state.game_data.player.records[0].homeworld_planet_index_1_based_raw() as usize - 1;
    state.game_data.planets.records[homeworld_index].set_army_count_raw(253);
    let home_coords = state.game_data.planets.records[homeworld_index].coords_raw();

    let fleet_one = state
        .game_data
        .fleets
        .records
        .iter_mut()
        .find(|fleet| fleet.owner_empire_raw() == 1 && fleet.local_slot_word_raw() == 1)
        .expect("fleet #1 should exist");
    fleet_one.set_current_location_coords_raw(home_coords);
    fleet_one.set_troop_transport_count(6);
    fleet_one.set_army_count(0);
    fleet_one.recompute_max_speed_from_composition();

    let fleet_two = state
        .game_data
        .fleets
        .records
        .iter_mut()
        .find(|fleet| fleet.owner_empire_raw() == 1 && fleet.local_slot_word_raw() == 2)
        .expect("fleet #2 should exist");
    fleet_two.set_current_location_coords_raw(home_coords);
    fleet_two.set_troop_transport_count(5);
    fleet_two.set_army_count(5);
    fleet_two.recompute_max_speed_from_composition();

    let fleet_three = state
        .game_data
        .fleets
        .records
        .iter_mut()
        .find(|fleet| fleet.owner_empire_raw() == 1 && fleet.local_slot_word_raw() == 3)
        .expect("fleet #3 should exist");
    fleet_three.set_current_location_coords_raw(home_coords);
    fleet_three.set_troop_transport_count(4);
    fleet_three.set_army_count(2);
    fleet_three.recompute_max_speed_from_composition();

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
    advance_to_main_menu(&mut app);
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::OpenMenu)),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::OpenTransportUnload)),
        AppOutcome::Continue
    );
    assert_eq!(app.fleet.menu_prompt_default_value, "2");
    submit_fleet_menu_prompt(&mut app, None);
    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("fleet unload quantity prompt should render");
    let prompt = line_containing(&terminal, "COMMAND <- How many armies to unload?");
    assert!(prompt.contains("[2]"));
}

#[test]
fn planet_menu_load_and_unload_use_inline_transport_prompts() {
    let fixture_dir = temp_game_copy();
    let mut state = latest_runtime_state(&fixture_dir);
    let homeworld_index =
        state.game_data.player.records[0].homeworld_planet_index_1_based_raw() as usize - 1;
    let home_coords = state.game_data.planets.records[homeworld_index].coords_raw();
    state.game_data.planets.records[homeworld_index].set_army_count_raw(8);
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
    extra_planet.set_army_count_raw(200);
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
        .find(|fleet| fleet.owner_empire_raw() == 1 && fleet.local_slot_word_raw() == 1)
        .expect("fleet #1 should exist");
    load_fleet.set_current_location_coords_raw(home_coords);
    load_fleet.set_troop_transport_count(4);
    load_fleet.set_army_count(1);
    load_fleet.recompute_max_speed_from_composition();
    let unload_fleet = state
        .game_data
        .fleets
        .records
        .iter_mut()
        .find(|fleet| fleet.owner_empire_raw() == 1 && fleet.local_slot_word_raw() == 2)
        .expect("fleet #2 should exist");
    unload_fleet.set_current_location_coords_raw(extra_coords);
    unload_fleet.set_troop_transport_count(5);
    unload_fleet.set_army_count(3);
    unload_fleet.recompute_max_speed_from_composition();
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
        app.handle_key(key(KeyCode::Char('l'))),
        Action::Planet(PlanetAction::OpenTransportPrompt(
            nc_game::screen::PlanetTransportMode::Load,
        ))
    );
    assert_eq!(
        apply_action(
            &mut app,
            Action::Planet(PlanetAction::OpenTransportPrompt(
                nc_game::screen::PlanetTransportMode::Load,
            )),
        ),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::PlanetMenu);

    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("planet load prompt should render");
    let prompt = line_containing(&terminal, "PLANET COMMAND <- Load Fleet #");
    assert!(prompt.contains("Load Fleet # ["));
    assert!(prompt.contains("<Q> ->"));

    submit_planet_transport_prompt(&mut app, None);
    app.render(&mut terminal)
        .expect("planet load quantity prompt should render inline");
    assert!(
        terminal
            .lines
            .iter()
            .any(|line| line.contains("LOAD ARMIES ONTO TROOP TRANSPORTS:"))
    );
    assert!(
        terminal
            .lines
            .iter()
            .any(|line| line.contains("Planet:") && line.contains("Fleet "))
    );
    let prompt = line_containing(&terminal, "PLANET COMMAND <- How many armies to load?");
    assert!(prompt.contains("<Q> ->"));
    assert!(
        terminal
            .lines
            .iter()
            .all(|line| !line.contains("COMMANDS <ARROWS"))
    );

    assert_eq!(
        app.handle_key(key(KeyCode::Char('q'))),
        Action::Planet(PlanetAction::CancelTransportPrompt)
    );
    assert_eq!(
        apply_action(
            &mut app,
            Action::Planet(PlanetAction::CancelTransportPrompt)
        ),
        AppOutcome::Continue
    );
    app.render(&mut terminal)
        .expect("planet load fleet prompt should re-render");
    assert!(
        line_containing(&terminal, "PLANET COMMAND <- Load Fleet #").contains("Load Fleet # [")
    );
    assert_eq!(
        apply_action(
            &mut app,
            Action::Planet(PlanetAction::CancelTransportPrompt)
        ),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(
            &mut app,
            Action::Planet(PlanetAction::CancelTransportPrompt)
        ),
        AppOutcome::Continue
    );

    assert_eq!(
        app.handle_key(key(KeyCode::Char('u'))),
        Action::Planet(PlanetAction::OpenTransportPrompt(
            nc_game::screen::PlanetTransportMode::Unload,
        ))
    );
    assert_eq!(
        apply_action(
            &mut app,
            Action::Planet(PlanetAction::OpenTransportPrompt(
                nc_game::screen::PlanetTransportMode::Unload,
            )),
        ),
        AppOutcome::Continue
    );
    app.render(&mut terminal)
        .expect("planet unload prompt should render");
    let prompt = line_containing(&terminal, "PLANET COMMAND <- Unload Fleet #");
    assert!(prompt.contains("Unload Fleet # ["));
    submit_planet_transport_prompt(&mut app, None);
    app.render(&mut terminal)
        .expect("planet unload quantity prompt should render");
    assert!(
        line_containing(&terminal, "PLANET COMMAND <- How many armies to unload?")
            .contains("<Q> ->")
    );
}

#[test]
fn planet_transport_load_default_chooses_best_planet_and_fleet() {
    let fixture_dir = temp_game_copy();
    let mut state = latest_runtime_state(&fixture_dir);
    let homeworld_index =
        state.game_data.player.records[0].homeworld_planet_index_1_based_raw() as usize - 1;
    let home_coords = state.game_data.planets.records[homeworld_index].coords_raw();
    state.game_data.planets.records[homeworld_index].set_army_count_raw(2);
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
    extra_planet.set_army_count_raw(10);
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

    let fleet_one = state
        .game_data
        .fleets
        .records
        .iter_mut()
        .find(|fleet| fleet.owner_empire_raw() == 1 && fleet.local_slot_word_raw() == 1)
        .expect("fleet #1 should exist");
    fleet_one.set_current_location_coords_raw(home_coords);
    fleet_one.set_troop_transport_count(4);
    fleet_one.set_army_count(0);
    fleet_one.recompute_max_speed_from_composition();

    let fleet_two = state
        .game_data
        .fleets
        .records
        .iter_mut()
        .find(|fleet| fleet.owner_empire_raw() == 1 && fleet.local_slot_word_raw() == 2)
        .expect("fleet #2 should exist");
    fleet_two.set_current_location_coords_raw(extra_coords);
    fleet_two.set_troop_transport_count(3);
    fleet_two.set_army_count(1);
    fleet_two.recompute_max_speed_from_composition();

    let fleet_three = state
        .game_data
        .fleets
        .records
        .iter_mut()
        .find(|fleet| fleet.owner_empire_raw() == 1 && fleet.local_slot_word_raw() == 3)
        .expect("fleet #3 should exist");
    fleet_three.set_current_location_coords_raw(extra_coords);
    fleet_three.set_troop_transport_count(5);
    fleet_three.set_army_count(0);
    fleet_three.recompute_max_speed_from_composition();

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
        apply_action(
            &mut app,
            Action::Planet(PlanetAction::OpenTransportPrompt(
                nc_game::screen::PlanetTransportMode::Load,
            )),
        ),
        AppOutcome::Continue
    );
    assert_eq!(app.planet.transport_prompt_default_value, "3");

    let mut terminal = CaptureTerminal::new();
    submit_planet_transport_prompt(&mut app, None);
    app.render(&mut terminal)
        .expect("planet load quantity prompt should render");
    assert!(
        line_containing(&terminal, "PLANET COMMAND <- How many armies to load?").contains("[5]")
    );
}

#[test]
fn planet_transport_unload_default_chooses_best_planet_and_fleet() {
    let fixture_dir = temp_game_copy();
    let mut state = latest_runtime_state(&fixture_dir);
    let homeworld_index =
        state.game_data.player.records[0].homeworld_planet_index_1_based_raw() as usize - 1;
    let home_coords = state.game_data.planets.records[homeworld_index].coords_raw();
    state.game_data.planets.records[homeworld_index].set_army_count_raw(250);
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
    extra_planet.set_army_count_raw(200);
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

    let fleet_one = state
        .game_data
        .fleets
        .records
        .iter_mut()
        .find(|fleet| fleet.owner_empire_raw() == 1 && fleet.local_slot_word_raw() == 1)
        .expect("fleet #1 should exist");
    fleet_one.set_current_location_coords_raw(home_coords);
    fleet_one.set_troop_transport_count(6);
    fleet_one.set_army_count(2);
    fleet_one.recompute_max_speed_from_composition();

    let fleet_two = state
        .game_data
        .fleets
        .records
        .iter_mut()
        .find(|fleet| fleet.owner_empire_raw() == 1 && fleet.local_slot_word_raw() == 2)
        .expect("fleet #2 should exist");
    fleet_two.set_current_location_coords_raw(extra_coords);
    fleet_two.set_troop_transport_count(5);
    fleet_two.set_army_count(1);
    fleet_two.recompute_max_speed_from_composition();

    let fleet_three = state
        .game_data
        .fleets
        .records
        .iter_mut()
        .find(|fleet| fleet.owner_empire_raw() == 1 && fleet.local_slot_word_raw() == 3)
        .expect("fleet #3 should exist");
    fleet_three.set_current_location_coords_raw(extra_coords);
    fleet_three.set_troop_transport_count(4);
    fleet_three.set_army_count(4);
    fleet_three.recompute_max_speed_from_composition();

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
        apply_action(
            &mut app,
            Action::Planet(PlanetAction::OpenTransportPrompt(
                nc_game::screen::PlanetTransportMode::Unload,
            )),
        ),
        AppOutcome::Continue
    );
    assert_eq!(app.planet.transport_prompt_default_value, "3");

    let mut terminal = CaptureTerminal::new();
    submit_planet_transport_prompt(&mut app, None);
    app.render(&mut terminal)
        .expect("planet unload quantity prompt should render");
    assert!(
        line_containing(&terminal, "PLANET COMMAND <- How many armies to unload?").contains("[4]")
    );
}

#[test]
fn planet_transport_load_prompt_rejects_fleet_not_at_owned_world() {
    let fixture_dir = temp_game_copy();
    let mut state = latest_runtime_state(&fixture_dir);
    let homeworld_index =
        state.game_data.player.records[0].homeworld_planet_index_1_based_raw() as usize - 1;
    let home_coords = state.game_data.planets.records[homeworld_index].coords_raw();
    state.game_data.planets.records[homeworld_index].set_army_count_raw(8);
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

    let fleet_one = state
        .game_data
        .fleets
        .records
        .iter_mut()
        .find(|fleet| fleet.owner_empire_raw() == 1 && fleet.local_slot_word_raw() == 1)
        .expect("fleet #1 should exist");
    fleet_one.set_current_location_coords_raw(home_coords);
    fleet_one.set_troop_transport_count(4);
    fleet_one.set_army_count(0);
    fleet_one.recompute_max_speed_from_composition();

    let fleet_two = state
        .game_data
        .fleets
        .records
        .iter_mut()
        .find(|fleet| fleet.owner_empire_raw() == 1 && fleet.local_slot_word_raw() == 2)
        .expect("fleet #2 should exist");
    fleet_two.set_current_location_coords_raw([1, 1]);
    fleet_two.set_troop_transport_count(4);
    fleet_two.set_army_count(0);
    fleet_two.recompute_max_speed_from_composition();

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
        apply_action(
            &mut app,
            Action::Planet(PlanetAction::OpenTransportPrompt(
                nc_game::screen::PlanetTransportMode::Load,
            )),
        ),
        AppOutcome::Continue
    );
    submit_planet_transport_prompt_value(&mut app, "2");
    let expected = "That fleet is not at one of your worlds.";
    assert_eq!(app.planet.transport_status.as_deref(), Some(expected));
}

#[test]
fn fleet_menu_long_notice_wraps_instead_of_clipping() {
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
        apply_action(&mut app, Action::Fleet(FleetAction::OpenTransportUnload)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FleetMenu);

    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("fleet menu should render wrapped notice");
    assert_eq!(
        terminal.lines[7].trim_end(),
        " FLEET COMMAND <- ? X V S F R E C I D T O M L U <Q> ->"
    );
    assert_eq!(terminal.lines[8].trim_end(), "");
    assert_eq!(terminal.lines[9].trim_end(), "");
    assert_eq!(terminal.lines[10].trim_end(), "");
    let wrapped_notice = [
        &terminal.lines[11],
        &terminal.lines[12],
        &terminal.lines[13],
    ]
    .into_iter()
    .flat_map(|line| line.split_whitespace())
    .collect::<Vec<_>>()
    .join(" ");
    assert!(
        wrapped_notice.contains(
            "No fleets have loaded armies ready to unload onto planets with free capacity."
        )
    );
}

#[test]
fn fleet_menu_x_toggles_expert_mode_and_hides_menu_chrome() {
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
        app.handle_key(key(KeyCode::Char('x'))),
        Action::ToggleExpertMode
    );
    assert_eq!(
        apply_action(&mut app, Action::ToggleExpertMode),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FleetMenu);
    assert!(app.expert_mode);

    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("expert fleet menu should render");
    assert_eq!(
        terminal.lines[0].trim_end(),
        " FLEET COMMAND <- ? X V S F R E C I D T O M L U <Q> ->"
    );
    assert_eq!(terminal.lines[1].trim_end(), "");
}

#[test]
fn fleet_merge_defaults_to_largest_eligible_source_and_smallest_host() {
    let fixture_dir = temp_game_copy();
    let mut state = latest_runtime_state(&fixture_dir);

    let fleet_one = state
        .game_data
        .fleets
        .records
        .iter_mut()
        .find(|fleet| fleet.owner_empire_raw() == 1 && fleet.local_slot_word_raw() == 1)
        .expect("fleet #1 should exist");
    fleet_one.set_current_location_coords_raw([1, 1]);
    fleet_one.set_destroyer_count(4);
    fleet_one.set_cruiser_count(0);
    fleet_one.set_battleship_count(0);
    fleet_one.set_troop_transport_count(0);
    fleet_one.set_scout_count(0);
    fleet_one.set_etac_count(0);

    let fleet_two = state
        .game_data
        .fleets
        .records
        .iter_mut()
        .find(|fleet| fleet.owner_empire_raw() == 1 && fleet.local_slot_word_raw() == 2)
        .expect("fleet #2 should exist");
    fleet_two.set_current_location_coords_raw([8, 8]);
    fleet_two.set_destroyer_count(2);
    fleet_two.set_cruiser_count(0);
    fleet_two.set_battleship_count(0);
    fleet_two.set_troop_transport_count(0);
    fleet_two.set_scout_count(0);
    fleet_two.set_etac_count(0);

    let fleet_three = state
        .game_data
        .fleets
        .records
        .iter_mut()
        .find(|fleet| fleet.owner_empire_raw() == 1 && fleet.local_slot_word_raw() == 3)
        .expect("fleet #3 should exist");
    fleet_three.set_current_location_coords_raw([8, 8]);
    fleet_three.set_destroyer_count(5);
    fleet_three.set_cruiser_count(0);
    fleet_three.set_battleship_count(0);
    fleet_three.set_troop_transport_count(0);
    fleet_three.set_scout_count(0);
    fleet_three.set_etac_count(0);

    let fleet_four = state
        .game_data
        .fleets
        .records
        .iter_mut()
        .find(|fleet| fleet.owner_empire_raw() == 1 && fleet.local_slot_word_raw() == 4)
        .expect("fleet #4 should exist");
    fleet_four.set_current_location_coords_raw([8, 8]);
    fleet_four.set_destroyer_count(7);
    fleet_four.set_cruiser_count(0);
    fleet_four.set_battleship_count(0);
    fleet_four.set_troop_transport_count(0);
    fleet_four.set_scout_count(0);
    fleet_four.set_etac_count(0);

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
        apply_action(&mut app, Action::Fleet(FleetAction::OpenMerge)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FleetMenu);
    assert_eq!(app.fleet.menu_prompt_default_value, "4");

    submit_fleet_menu_prompt(&mut app, None);
    assert_eq!(app.current_screen(), ScreenId::FleetMenu);
    assert_eq!(app.fleet.menu_prompt_default_value, "2");
}

#[test]
fn fleet_merge_source_rejects_fleet_without_lower_numbered_host() {
    let fixture_dir = temp_game_copy();
    let mut state = latest_runtime_state(&fixture_dir);

    let fleet_one = state
        .game_data
        .fleets
        .records
        .iter_mut()
        .find(|fleet| fleet.owner_empire_raw() == 1 && fleet.local_slot_word_raw() == 1)
        .expect("fleet #1 should exist");
    fleet_one.set_current_location_coords_raw([8, 8]);

    let fleet_two = state
        .game_data
        .fleets
        .records
        .iter_mut()
        .find(|fleet| fleet.owner_empire_raw() == 1 && fleet.local_slot_word_raw() == 2)
        .expect("fleet #2 should exist");
    fleet_two.set_current_location_coords_raw([8, 8]);

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
        apply_action(&mut app, Action::Fleet(FleetAction::OpenMerge)),
        AppOutcome::Continue
    );

    submit_fleet_menu_prompt(&mut app, Some(1));
    assert_eq!(
        app.fleet
            .menu_prompt_status
            .as_ref()
            .map(|feedback| feedback.message()),
        Some("Fleets must be co-located in the same sector.")
    );
}

#[test]
fn fleet_merge_host_rejects_non_colocated_fleet() {
    let fixture_dir = temp_game_copy();
    let mut state = latest_runtime_state(&fixture_dir);

    let fleet_one = state
        .game_data
        .fleets
        .records
        .iter_mut()
        .find(|fleet| fleet.owner_empire_raw() == 1 && fleet.local_slot_word_raw() == 1)
        .expect("fleet #1 should exist");
    fleet_one.set_current_location_coords_raw([1, 1]);

    let fleet_two = state
        .game_data
        .fleets
        .records
        .iter_mut()
        .find(|fleet| fleet.owner_empire_raw() == 1 && fleet.local_slot_word_raw() == 2)
        .expect("fleet #2 should exist");
    fleet_two.set_current_location_coords_raw([8, 8]);

    let fleet_three = state
        .game_data
        .fleets
        .records
        .iter_mut()
        .find(|fleet| fleet.owner_empire_raw() == 1 && fleet.local_slot_word_raw() == 3)
        .expect("fleet #3 should exist");
    fleet_three.set_current_location_coords_raw([8, 8]);

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
        apply_action(&mut app, Action::Fleet(FleetAction::OpenMerge)),
        AppOutcome::Continue
    );

    submit_fleet_menu_prompt(&mut app, Some(3));
    submit_fleet_menu_prompt(&mut app, Some(1));
    assert_eq!(
        app.fleet
            .menu_prompt_status
            .as_ref()
            .map(|feedback| feedback.message()),
        Some("Fleet #1 is not in the same sector as Fleet #3.")
    );
}

#[test]
fn fleet_merge_auto_swaps_higher_numbered_fleet_into_lower_host() {
    let fixture_dir = temp_game_copy();
    let mut state = latest_runtime_state(&fixture_dir);

    let fleet_two = state
        .game_data
        .fleets
        .records
        .iter_mut()
        .find(|fleet| fleet.owner_empire_raw() == 1 && fleet.local_slot_word_raw() == 2)
        .expect("fleet #2 should exist");
    fleet_two.set_current_location_coords_raw([8, 8]);

    let fleet_three = state
        .game_data
        .fleets
        .records
        .iter_mut()
        .find(|fleet| fleet.owner_empire_raw() == 1 && fleet.local_slot_word_raw() == 3)
        .expect("fleet #3 should exist");
    fleet_three.set_current_location_coords_raw([8, 8]);

    let fleet_four = state
        .game_data
        .fleets
        .records
        .iter_mut()
        .find(|fleet| fleet.owner_empire_raw() == 1 && fleet.local_slot_word_raw() == 4)
        .expect("fleet #4 should exist");
    fleet_four.set_current_location_coords_raw([8, 8]);

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
        apply_action(&mut app, Action::Fleet(FleetAction::OpenMerge)),
        AppOutcome::Continue
    );

    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("merge source prompt should render");
    assert!(line_containing(&terminal, "COMMAND <- Merge Fleet #").contains("Merge Fleet # ["));

    submit_fleet_menu_prompt(&mut app, Some(3));
    app.render(&mut terminal)
        .expect("merge host prompt should render");
    assert!(line_containing(&terminal, "COMMAND <- Into Fleet #").contains("Into Fleet # ["));

    submit_fleet_menu_prompt(&mut app, Some(4));
    assert_eq!(app.current_screen(), ScreenId::FleetMenu);
    app.render(&mut terminal)
        .expect("fleet menu should render merge success notice");
    assert!(
        terminal
            .lines
            .iter()
            .any(|line| line.contains("Fleet #4 ordered to join Fleet #3."))
    );

    let state = latest_runtime_state(&fixture_dir);
    let source = state
        .game_data
        .fleets
        .records
        .iter()
        .find(|fleet| fleet.owner_empire_raw() == 1 && fleet.local_slot_word_raw() == 4)
        .expect("fleet #4 should exist");
    let host = state
        .game_data
        .fleets
        .records
        .iter()
        .find(|fleet| fleet.owner_empire_raw() == 1 && fleet.local_slot_word_raw() == 3)
        .expect("fleet #3 should exist");
    assert_eq!(
        source.standing_order_kind(),
        nc_data::Order::JoinAnotherFleet
    );
    assert_eq!(source.join_host_fleet_id_raw(), host.fleet_id_word_raw());
    assert_eq!(
        source.standing_order_target_coords_raw(),
        host.current_location_coords_raw()
    );
}

#[test]
fn fleet_table_zero_pads_numbers_to_current_max_width() {
    let mut screen = nc_game::screen::FleetListScreen::new();
    let rows = vec![
        FleetRow {
            fleet_record_index_1_based: 1,
            fleet_number: 1,
            coords: [16, 13],
            target_coords: [16, 13],
            order_code: 0,
            current_speed: 0,
            max_speed: 3,
            eta_label: "0".to_string(),
            list_eta_label: "0".to_string(),
            rules_of_engagement: 6,
            loaded_armies: 0,
            order_label: "Hold".to_string(),
            composition_label: "CA=1".to_string(),
            table_ships_label: "CA".to_string(),
            join_host_fleet_number: None,
        },
        FleetRow {
            fleet_record_index_1_based: 2,
            fleet_number: 10,
            coords: [17, 13],
            target_coords: [17, 13],
            order_code: 0,
            current_speed: 0,
            max_speed: 3,
            eta_label: "0".to_string(),
            list_eta_label: "0".to_string(),
            rules_of_engagement: 6,
            loaded_armies: 0,
            order_label: "Hold".to_string(),
            composition_label: "DD=1".to_string(),
            table_ships_label: "DD".to_string(),
            join_host_fleet_number: None,
        },
        FleetRow {
            fleet_record_index_1_based: 3,
            fleet_number: 100,
            coords: [18, 13],
            target_coords: [18, 13],
            order_code: 0,
            current_speed: 0,
            max_speed: 3,
            eta_label: "0".to_string(),
            list_eta_label: "0".to_string(),
            rules_of_engagement: 6,
            loaded_armies: 0,
            order_label: "Hold".to_string(),
            composition_label: "BB=1".to_string(),
            table_ships_label: "BB".to_string(),
            join_host_fleet_number: None,
        },
    ];

    let buffer = screen
        .render(
            nc_game::screen::ScreenGeometry::local_default(),
            &rows,
            nc_game::screen::FleetListSort::Id,
            nc_game::screen::SortDirection::Desc,
            nc_game::screen::FleetListFilter::All,
            0,
            0,
            "",
            None,
            None,
            None,
            "",
            "",
            None,
        )
        .expect("fleet list renders");

    assert!(buffer.plain_line(4).contains("│001│"));
    assert!(buffer.plain_line(5).contains("│010│"));
    assert!(buffer.plain_line(6).contains("│100│"));
}

#[test]
fn fleet_list_table_uses_order_target_eta_columns_and_current_speed() {
    let mut screen = nc_game::screen::FleetListScreen::new();
    let rows = vec![FleetRow {
        fleet_record_index_1_based: 1,
        fleet_number: 4,
        coords: [8, 9],
        target_coords: [16, 13],
        order_code: 5,
        current_speed: 2,
        max_speed: 6,
        eta_label: "3000".to_string(),
        list_eta_label: "0".to_string(),
        rules_of_engagement: 6,
        loaded_armies: 3,
        order_label: "Guard/blockade world in System (16,13)".to_string(),
        composition_label: "DD=1".to_string(),
        table_ships_label: "DD".to_string(),
        join_host_fleet_number: None,
    }];

    let buffer = screen
        .render(
            nc_game::screen::ScreenGeometry::local_default(),
            &rows,
            nc_game::screen::FleetListSort::Id,
            nc_game::screen::SortDirection::Desc,
            nc_game::screen::FleetListFilter::All,
            0,
            0,
            "",
            None,
            None,
            None,
            "",
            "",
            None,
        )
        .expect("fleet list renders");

    assert_eq!(buffer.plain_line(0), " FLEET LIST: ID DESCENDING ALL");
    assert!(!buffer.plain_line(1).contains("ENTER reviews a fleet."));
    assert!(buffer.plain_line(1).starts_with("┌"));
    assert!(buffer.plain_line(1).ends_with("┐"));
    assert!(buffer.plain_line(2).contains("│ID│Sel│Location│Order"));
    assert!(buffer.plain_line(2).contains("│Target"));
    assert!(buffer.plain_line(2).contains("│Spd│"));
    assert!(buffer.plain_line(2).contains("ETA"));
    assert!(buffer.plain_line(2).contains("ROE"));
    assert!(buffer.plain_line(2).contains("ARs"));
    assert!(buffer.plain_line(2).contains("Ships"));
    assert!(buffer.plain_line(2).contains("│ROE│"));
    assert!(buffer.plain_line(2).contains("│ARs│"));
    assert!(buffer.plain_line(4).contains("Guard/Blkd"));
    assert!(buffer.plain_line(4).contains("(16,13)"));
    assert!(buffer.plain_line(4).contains("│  2│"));
    assert!(buffer.plain_line(4).contains("│  3│"));
    assert!(!buffer.plain_line(4).contains("2/6"));
    assert!(buffer.plain_line(4).contains("0"));
    assert!(buffer.plain_line(4).contains("DD"));
    assert_eq!(
        buffer.plain_line(6),
        " COMMAND <- ? F S O C E D M T L U SPACE <Q> [4] ->"
    );
}

#[test]
fn fleet_list_ships_column_shows_sparse_counted_tokens() {
    let mut screen = nc_game::screen::FleetListScreen::new();
    let rows = vec![FleetRow {
        fleet_record_index_1_based: 1,
        fleet_number: 4,
        coords: [8, 9],
        target_coords: [16, 13],
        order_code: 5,
        current_speed: 2,
        max_speed: 6,
        eta_label: "3000".to_string(),
        list_eta_label: "0".to_string(),
        rules_of_engagement: 6,
        loaded_armies: 2,
        order_label: "Guard/blockade world in System (16,13)".to_string(),
        composition_label: "SC=2 BB=1 DD=4 AR=2 ET=1".to_string(),
        table_ships_label: "2SC BB 4DD 2TT* ET".to_string(),
        join_host_fleet_number: None,
    }];

    let buffer = screen
        .render(
            nc_game::screen::ScreenGeometry::local_default(),
            &rows,
            nc_game::screen::FleetListSort::Id,
            nc_game::screen::SortDirection::Desc,
            nc_game::screen::FleetListFilter::All,
            0,
            0,
            "",
            None,
            None,
            None,
            "",
            "",
            None,
        )
        .expect("fleet list renders");

    assert!(buffer.plain_line(4).contains("2SC BB 4DD 2TT* ET"));
}

#[test]
fn fleet_list_ships_column_truncates_by_whole_token_with_plus_marker() {
    let mut screen = nc_game::screen::FleetListScreen::new();
    let rows = vec![FleetRow {
        fleet_record_index_1_based: 1,
        fleet_number: 4,
        coords: [8, 9],
        target_coords: [16, 13],
        order_code: 5,
        current_speed: 2,
        max_speed: 6,
        eta_label: "3000".to_string(),
        list_eta_label: "0".to_string(),
        rules_of_engagement: 6,
        loaded_armies: 2,
        order_label: "Guard/blockade world in System (16,13)".to_string(),
        composition_label: "SC=2 BB=4 CA=3 DD=5 TT=5 AR=2 ET=1".to_string(),
        table_ships_label: "2SC 4BB 3CA 5DD 2TT* 3TT ET".to_string(),
        join_host_fleet_number: None,
    }];

    let buffer = screen
        .render(
            nc_game::screen::ScreenGeometry::local_default(),
            &rows,
            nc_game::screen::FleetListSort::Id,
            nc_game::screen::SortDirection::Desc,
            nc_game::screen::FleetListFilter::All,
            0,
            0,
            "",
            None,
            None,
            None,
            "",
            "",
            None,
        )
        .expect("fleet list renders");

    let ships_cell = buffer.plain_line(4);
    assert!(ships_cell.contains("+"));
    assert!(ships_cell.contains("2SC"));
    assert!(!ships_cell.contains("3TT"));
    assert!(!ships_cell.contains(" ET"));
}

#[test]
fn fleet_list_eta_column_shows_turns_remaining_for_arrived_fleets() {
    let fixture_dir = temp_game_copy();
    let mut state = latest_runtime_state(&fixture_dir);
    state.game_data.conquest.set_game_year(3007);
    for fleet in state
        .game_data
        .fleets
        .records
        .iter_mut()
        .filter(|fleet| fleet.owner_empire_raw() == 1)
    {
        fleet.set_current_location_coords_raw([16, 13]);
        fleet.set_standing_order_target_coords_raw([16, 13]);
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
    advance_to_main_menu(&mut app);
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::OpenMenu)),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::OpenList)),
        AppOutcome::Continue
    );

    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal).expect("fleet list should render");
    let right_border_col = terminal
        .line(1)
        .chars()
        .position(|ch| ch == '┐')
        .expect("fleet list should have a right border");
    let scrollbar_col = (1..=22).find_map(|row| {
        terminal
            .line(row)
            .chars()
            .nth(right_border_col + 1)
            .filter(|ch| matches!(ch, '^' | '|' | '#' | 'v'))
            .map(|_| right_border_col + 1)
    });
    assert!(right_border_col < 80);
    if let Some(scrollbar_col) = scrollbar_col {
        assert_eq!(scrollbar_col, right_border_col + 1);
    }
    assert!(
        terminal
            .lines
            .iter()
            .filter(|line| line.contains("(16,13)"))
            .any(|line| line.contains("│  0│")),
        "{:#?}",
        terminal.lines
    );
}

#[test]
fn fleet_list_table_renders_x_for_unreachable_eta_label() {
    let mut screen = nc_game::screen::FleetListScreen::new();
    let rows = vec![FleetRow {
        fleet_record_index_1_based: 1,
        fleet_number: 1,
        coords: [8, 9],
        target_coords: [0, 0],
        order_code: 1,
        current_speed: 3,
        max_speed: 6,
        eta_label: "N/A".to_string(),
        list_eta_label: "X".to_string(),
        rules_of_engagement: 6,
        loaded_armies: 0,
        order_label: "Move fleet to Sector (0,0)".to_string(),
        composition_label: "DD=1".to_string(),
        table_ships_label: "DD".to_string(),
        join_host_fleet_number: None,
    }];

    let buffer = screen
        .render(
            nc_game::screen::ScreenGeometry::local_default(),
            &rows,
            nc_game::screen::FleetListSort::Id,
            nc_game::screen::SortDirection::Desc,
            nc_game::screen::FleetListFilter::All,
            0,
            0,
            "",
            None,
            None,
            None,
            "",
            "",
            None,
        )
        .expect("fleet list renders");

    assert!(buffer.plain_line(4).contains("Move"));
    assert!(buffer.plain_line(4).contains("│   X│"));
}

#[test]
fn fleet_list_load_keeps_confirmation_when_table_does_not_show_army_load_state() {
    let fixture_dir = temp_game_copy();
    let mut state = latest_runtime_state(&fixture_dir);
    let homeworld_index =
        state.game_data.player.records[0].homeworld_planet_index_1_based_raw() as usize - 1;
    let home_coords = state.game_data.planets.records[homeworld_index].coords_raw();
    let fleet = state
        .game_data
        .fleets
        .records
        .iter_mut()
        .find(|fleet| fleet.owner_empire_raw() == 1 && fleet.local_slot_word_raw() == 4)
        .expect("fleet #4 should exist");
    fleet.set_current_location_coords_raw(home_coords);
    fleet.set_standing_order_target_coords_raw(home_coords);
    fleet.set_battleship_count(0);
    fleet.set_cruiser_count(0);
    fleet.set_destroyer_count(0);
    fleet.set_troop_transport_count(5);
    fleet.set_army_count(0);
    fleet.set_scout_count(0);
    fleet.set_etac_count(0);
    fleet.recompute_max_speed_from_composition();
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
        apply_action(&mut app, Action::Fleet(FleetAction::OpenTransportLoad)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FleetList);
    submit_fleet_menu_prompt_value(&mut app, "1");
    assert_eq!(app.current_screen(), ScreenId::FleetList);

    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("fleet list should render load confirmation");
    assert!(
        terminal
            .lines
            .iter()
            .any(|line| line.contains("Loaded 1 armies") && line.contains("Fleet #4")),
        "{:#?}",
        terminal.lines
    );
}

#[test]
fn fleet_list_transport_error_uses_slap_a_key_footer_and_consumes_next_key() {
    let fixture_dir = temp_game_copy();
    let mut state = latest_runtime_state(&fixture_dir);
    let homeworld_index =
        state.game_data.player.records[0].homeworld_planet_index_1_based_raw() as usize - 1;
    let home_coords = state.game_data.planets.records[homeworld_index].coords_raw();
    let fleet = state
        .game_data
        .fleets
        .records
        .iter_mut()
        .find(|fleet| fleet.owner_empire_raw() == 1 && fleet.local_slot_word_raw() == 4)
        .expect("fleet #4 should exist");
    fleet.set_current_location_coords_raw(home_coords);
    fleet.set_standing_order_target_coords_raw(home_coords);
    fleet.set_battleship_count(1);
    fleet.set_cruiser_count(0);
    fleet.set_destroyer_count(0);
    fleet.set_troop_transport_count(0);
    fleet.set_army_count(0);
    fleet.set_scout_count(0);
    fleet.set_etac_count(0);
    fleet.recompute_max_speed_from_composition();
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
        apply_action(&mut app, Action::Fleet(FleetAction::OpenTransportLoad)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FleetList);

    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("fleet list transport error should render");
    let command_line = line_containing(&terminal, "COMMAND <-");
    assert!(command_line.contains("(slap a key)"));
    assert!(command_line.contains("Unable to load armies"));

    assert_eq!(
        app.handle_key(key(KeyCode::Enter)),
        Action::Fleet(FleetAction::DismissMessage)
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::DismissMessage)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FleetList);

    terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("fleet list should render without the dismiss latch");
    assert!(!line_containing(&terminal, "COMMAND <-").contains("(slap a key)"));
}

#[test]
fn fleet_list_load_quantity_prompt_keeps_scrollbar_gutter() {
    let mut fleet_list = FleetListScreen::new();
    let rows = (0..24)
        .map(|idx| FleetRow {
            fleet_record_index_1_based: idx + 1,
            fleet_number: (24 - idx) as u16,
            coords: [((idx % 31) + 1) as u8, (((idx * 3) % 31) + 1) as u8],
            target_coords: [((idx % 31) + 1) as u8, (((idx * 3) % 31) + 1) as u8],
            order_code: nc_data::Order::HoldPosition.to_raw(),
            current_speed: 0,
            max_speed: 9,
            eta_label: "0".to_string(),
            list_eta_label: "0".to_string(),
            rules_of_engagement: 6,
            loaded_armies: 0,
            order_label: "Hold".to_string(),
            composition_label: "TT".to_string(),
            table_ships_label: "TT".to_string(),
            join_host_fleet_number: None,
        })
        .collect::<Vec<_>>();

    let mut terminal = CaptureTerminal::new();
    let buffer = fleet_list
        .render(
            ScreenGeometry::local_default(),
            &rows,
            nc_game::screen::FleetListSort::Id,
            nc_game::screen::SortDirection::Desc,
            nc_game::screen::FleetListFilter::All,
            0,
            0,
            "",
            None,
            None,
            Some("How many armies to load? "),
            "1",
            "",
            None,
        )
        .expect("fleet list load quantity prompt should render");
    terminal
        .render(&buffer)
        .expect("capture should record buffer");
    let right_border_col = terminal
        .line(1)
        .chars()
        .position(|ch| ch == '┐')
        .expect("fleet list should have a right border");
    let scrollbar_col = (1..=22).find_map(|row| {
        terminal
            .line(row)
            .chars()
            .nth(right_border_col + 1)
            .filter(|ch| matches!(ch, '^' | '|' | '#' | 'v'))
            .map(|_| right_border_col + 1)
    });
    assert!(right_border_col < 79);
    assert_eq!(scrollbar_col, Some(right_border_col + 1));
    assert!(line_containing(&terminal, "COMMAND <- How many armies to load?").contains("<Q> ->"));
}

#[test]
fn fleet_list_sorts_descending_and_typed_fleet_number_opens_review() {
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

    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal).expect("fleet list should render");
    assert!(terminal.line(4).contains("│ 4│"));
    assert_eq!(
        line_containing(&terminal, "COMMAND <- ? F S O C E D M T L U SPACE <Q> [").trim_end(),
        " COMMAND <- ? F S O C E D M T L U SPACE <Q> [4] ->"
    );

    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::AppendListChar('1'))),
        AppOutcome::Continue
    );
    assert!(app.fleet.list_input.is_empty());
    app.render(&mut terminal)
        .expect("fleet list should render typed fleet input");
    assert_eq!(
        line_containing(&terminal, "COMMAND <- ? F S O C E D M T L U SPACE <Q> [").trim_end(),
        " COMMAND <- ? F S O C E D M T L U SPACE <Q> [1] ->"
    );

    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::OpenReview)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FleetReview);
    app.render(&mut terminal)
        .expect("fleet review should render");
    assert!(
        line_containing(&terminal, "Fleet ID: ").contains("Fleet ID: 1"),
        "{:#?}",
        terminal.lines
    );
}

#[test]
fn fleet_review_screen_splits_loaded_and_empty_transports() {
    let mut screen = nc_game::screen::FleetReviewScreen::new();
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
        composition_label: "CA=1 TT=5 AR=2".to_string(),
        table_ships_label: "CA 2TT* 3TT".to_string(),
        join_host_fleet_number: None,
    };

    let buffer = screen
        .render(&row, 0, 1, false)
        .expect("fleet review screen renders");

    assert!(buffer.plain_line(9).contains("Composition: CA 2TT* 3TT"));
    assert!(!buffer.plain_line(9).contains("AR="));
}

#[test]
fn fleet_eta_screen_renders_bottom_line_prompt() {
    let mut screen = nc_game::screen::FleetEtaScreen::new();
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
        composition_label: "CA=1 TT=5 AR=2".to_string(),
        table_ships_label: "CA 2TT* 3TT".to_string(),
        join_host_fleet_number: None,
    };

    let buffer = screen
        .render(
            &row,
            nc_game::screen::FleetEtaMode::EnteringDestination,
            [19, 13],
            "",
            "",
            None,
        )
        .expect("fleet eta screen renders");

    assert_eq!(buffer.plain_line(0), " CALCULATE FLEET ETA:");
    assert_eq!(buffer.plain_line(2).trim_end(), " Fleet ID: 7");
    assert_eq!(buffer.plain_line(4).trim_end(), " Location: (16,13)");
    assert_eq!(buffer.plain_line(8).trim_end(), " Target: (19,13)");
    assert!(buffer.plain_line(10).contains("Ships: CA 2TT* 3TT"));
    assert!(!buffer.plain_line(10).contains("AR="));
    assert!(
        buffer
            .plain_line(12)
            .contains("COMMAND <- Destination [19,13] <Q> ->")
    );
}

#[test]
fn fleet_eta_accepts_typed_fleet_destination_and_default_include_system() {
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
    open_eta_from_fleet_menu(&mut app, Some(4));
    assert_eq!(app.current_screen(), ScreenId::FleetEta);
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::AppendEtaChar('1'))),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::AppendEtaChar('0'))),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::AppendEtaChar(','))),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::AppendEtaChar('1'))),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::AppendEtaChar('3'))),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::SubmitEta)),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::SubmitEta)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FleetEta);
    assert_eq!(
        app.handle_key(key(KeyCode::Enter)),
        Action::Fleet(FleetAction::SubmitEta)
    );
}

#[test]
fn fleet_eta_result_dismiss_returns_to_primary_fleet_menu() {
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
    open_eta_from_fleet_menu(&mut app, Some(4));
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
        apply_action(&mut app, Action::Fleet(FleetAction::SubmitEta)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FleetEta);

    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::SubmitEta)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FleetMenu);

    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("fleet menu should render after dismissing eta result");
    assert!(
        line_containing(
            &terminal,
            "FLEET COMMAND <- ? X V S F R E C I D T O M L U <Q> ->"
        )
        .contains("FLEET COMMAND <- ? X V S F R E C I D T O M L U <Q> ->")
    );
    assert!(
        !terminal
            .lines
            .iter()
            .any(|line| line.contains("COMMAND <- ETA Fleet #")),
        "{:#?}",
        terminal.lines
    );
}

#[test]
fn fleet_eta_uses_max_speed_when_selected_fleet_is_stopped() {
    let fixture_dir = temp_game_copy();
    let mut state = latest_runtime_state(&fixture_dir);
    let fleet = state
        .game_data
        .fleets
        .records
        .get_mut(0)
        .expect("fleet 1 should exist");
    let current_coords = fleet.current_location_coords_raw();
    fleet.set_current_speed(0);
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
    open_eta_from_fleet_menu(&mut app, Some(1));
    for ch in format!("{},{}", current_coords[0], current_coords[1]).chars() {
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
        apply_action(&mut app, Action::Fleet(FleetAction::SubmitEta)),
        AppOutcome::Continue
    );

    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("fleet eta result should render");
    let prompt = line_containing(&terminal, "Fleet 1 reaches [");
    assert!(
        prompt.contains(&format!(
            "Fleet 1 reaches [{},{}] in 0 year(s)",
            current_coords[0], current_coords[1]
        )),
        "{}",
        prompt
    );
    assert!(!prompt.contains("is stopped"));
}

#[test]
fn fleet_eta_allows_empty_sector_targets_for_resting_hold_fleets() {
    let fixture_dir = temp_game_copy();
    let mut state = latest_runtime_state(&fixture_dir);
    let fleet = state
        .game_data
        .fleets
        .records
        .get_mut(0)
        .expect("fleet 1 should exist");
    fleet.set_current_speed(0);
    fleet.set_standing_order_kind(nc_data::Order::HoldPosition);
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
    open_eta_from_fleet_menu(&mut app, Some(1));
    for ch in ['1', ',', '1'] {
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
        apply_action(&mut app, Action::Fleet(FleetAction::SubmitEta)),
        AppOutcome::Continue
    );

    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("fleet eta empty-sector result should render");
    let prompt = line_containing(&terminal, "Fleet 1 reaches [1,1] in");
    assert!(prompt.contains("Fleet 1 reaches [1,1] in"));
    assert!(!prompt.contains("No route found"));
}
