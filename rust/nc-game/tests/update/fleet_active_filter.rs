use crate::support::*;

fn owned_fleet_mut(state: &mut CampaignRuntimeState, fleet_number: u16) -> &mut nc_data::Fleet {
    state
        .game_data
        .fleets
        .iter_mut()
        .find(|fleet| fleet.owner_empire_raw() == 1 && fleet.local_slot_word_raw() == fleet_number)
        .unwrap_or_else(|| panic!("fleet #{fleet_number} should exist"))
}

fn clear_fleet_force(fleet: &mut nc_data::Fleet) {
    fleet.set_scout_count(0);
    fleet.set_battleship_count(0);
    fleet.set_cruiser_count(0);
    fleet.set_destroyer_count(0);
    fleet.set_troop_transport_count(0);
    fleet.set_army_count(0);
    fleet.set_etac_count(0);
    fleet.recompute_max_speed_from_composition();
}

#[test]
fn fleet_menu_shows_no_active_fleets_when_owned_slots_are_empty() {
    let fixture_dir = temp_game_copy();
    let mut state = latest_runtime_state(&fixture_dir);
    for fleet in state
        .game_data
        .fleets
        .iter_mut()
        .filter(|fleet| fleet.owner_empire_raw() == 1)
    {
        clear_fleet_force(fleet);
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

    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::OpenReviewPrompt)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FleetMenu);

    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("fleet menu should render review notice");
    assert!(
        terminal
            .lines
            .iter()
            .any(|line| line.contains("You have no active fleets."))
    );
}

#[test]
fn fleet_menu_defaults_and_review_list_skip_empty_owned_slots() {
    let fixture_dir = temp_game_copy();
    let mut state = latest_runtime_state(&fixture_dir);
    {
        let empty_fleet = owned_fleet_mut(&mut state, 2);
        clear_fleet_force(empty_fleet);
        empty_fleet.set_standing_order_kind(nc_data::Order::HoldPosition);
        empty_fleet.set_standing_order_target_coords_raw([0, 0]);
    }
    {
        let active_fleet = owned_fleet_mut(&mut state, 1);
        active_fleet.set_destroyer_count(1);
        active_fleet.set_standing_order_kind(nc_data::Order::HoldPosition);
        active_fleet.set_standing_order_target_coords_raw([0, 0]);
        active_fleet.recompute_max_speed_from_composition();
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
        apply_action(&mut app, Action::Fleet(FleetAction::OpenReviewPrompt)),
        AppOutcome::Continue
    );
    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("review prompt should render");
    assert!(
        line_containing(&terminal, "COMMAND <- Review Fleet #")
            .contains("Review Fleet # [1] <Q> ->")
    );

    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::CancelMenuPrompt)),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::OpenChangePrompt)),
        AppOutcome::Continue
    );
    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("change prompt should render");
    assert!(
        line_containing(&terminal, "COMMAND <- Change Fleet #")
            .contains("Change Fleet # [1] <Q> ->")
    );

    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::CancelMenuPrompt)),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::OpenDetach)),
        AppOutcome::Continue
    );
    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("detach prompt should render");
    assert!(
        line_containing(&terminal, "COMMAND <- Detach Fleet #")
            .contains("Detach Fleet # [1] <Q> ->")
    );

    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::CancelMenuPrompt)),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::OpenOrder)),
        AppOutcome::Continue
    );
    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("order prompt should render");
    assert!(
        line_containing(&terminal, "COMMAND <- Order Fleet #").contains("Order Fleet # [1] <Q> ->")
    );

    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::CancelMenuPrompt)),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::OpenList)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FleetList);

    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal).expect("fleet list should render");

    app.fleet.list_input = "2".to_string();
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::OpenReview)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FleetList);
    assert_eq!(
        app.fleet.list_status.as_deref(),
        Some("Fleet #2 is not in your fleet list.")
    );
}

#[test]
fn fleet_list_action_clamps_stale_cursor_before_using_visible_hold_row() {
    let fixture_dir = temp_game_copy();
    let mut state = latest_runtime_state(&fixture_dir);
    let held_fleet_number = state
        .game_data
        .fleets
        .iter()
        .find(|fleet| fleet.owner_empire_raw() == 1)
        .map(|fleet| fleet.local_slot_word_raw())
        .expect("owned fleet");
    for fleet in state
        .game_data
        .fleets
        .iter_mut()
        .filter(|fleet| fleet.owner_empire_raw() == 1)
    {
        clear_fleet_force(fleet);
    }
    {
        let held_fleet = owned_fleet_mut(&mut state, held_fleet_number);
        held_fleet.set_battleship_count(4);
        held_fleet.set_standing_order_kind(nc_data::Order::HoldPosition);
        held_fleet.set_standing_order_target_coords_raw(held_fleet.current_location_coords_raw());
        held_fleet.recompute_max_speed_from_composition();
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
    assert_eq!(app.current_screen(), ScreenId::FleetList);

    app.fleet.list_filter = nc_game::screen::FleetListFilter::Holding;
    app.fleet.cursor = 99;
    app.fleet.scroll_offset = 99;

    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("fleet list should render the held fleet");
    assert!(
        terminal
            .lines
            .iter()
            .any(|line| line.contains("FLEET LIST: ID DESCENDING HOLD"))
    );
    assert!(
        terminal
            .lines
            .iter()
            .any(|line| line.contains("Hold") && line.contains("4BB"))
    );

    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::OpenChangePrompt)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FleetList);
    assert!(app.fleet.list_dismiss_message.is_none());

    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("fleet list should render inline change prompt");
    assert!(
        terminal
            .lines
            .iter()
            .any(|line| line.contains("Change <R>OE, <I>D, or <S>peed [R] <Q> ->"))
    );
}
