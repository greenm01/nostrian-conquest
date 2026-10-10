use crate::support::*;

#[test]
fn fleet_detach_uses_staged_class_prompt_and_creates_new_fleet() {
    let fixture_dir = temp_game_copy();
    let mut game_data = CoreGameData::load(&fixture_dir).expect("load fixture");
    let initial_fleet_count = game_data.fleets.records.len();
    let donor = &mut game_data.fleets.records[0];
    donor.set_scout_count(1);
    donor.set_cruiser_count(1);
    donor.set_destroyer_count(4);
    donor.set_battleship_count(0);
    donor.set_troop_transport_count(4);
    donor.set_army_count(4);
    donor.set_etac_count(0);
    donor.set_current_location_coords_raw([8, 9]);
    donor.recompute_max_speed_from_composition();
    donor.set_current_speed(0);
    donor.set_rules_of_engagement(0);
    game_data.save(&fixture_dir).expect("save fixture");
    let store = CampaignStore::open_default_in_dir(&fixture_dir).expect("open campaign store");
    import_directory_snapshot(&store, &fixture_dir).expect("refresh sqlite snapshot");

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
        apply_action(&mut app, Action::Fleet(FleetAction::OpenMenu)),
        AppOutcome::Continue
    );
    open_detach_from_fleet_menu(&mut app, Some(1));
    assert_eq!(app.current_screen(), ScreenId::FleetDetach);

    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal).expect("render class prompt");
    assert_eq!(terminal.line(1).trim_end(), "");
    assert_eq!(terminal.line(2).trim_end(), " Fleet: Fleet #1");
    assert_eq!(terminal.line(3).trim_end(), "");
    assert_eq!(terminal.line(4).trim_end(), " Location: (08,09)");
    assert!(terminal.line(5).starts_with(" Orders: "));
    assert!(terminal.line(6).starts_with(" Target: "));
    assert_eq!(terminal.line(7).trim_end(), " Speed: 0");
    assert_eq!(terminal.line(8).trim_end(), " ROE: 0");
    assert!(terminal.line(10).contains("Ships: SC CA 4DD 4TT*"));
    assert!(!terminal.line(10).contains("AR="));
    assert_eq!(terminal.line(12).trim_end(), "<C>ommission, <X> Cancel");
    assert!(
        line_containing(&terminal, "Class <BB,CA,DD,TT*,TT,SC,ET,C,X>")
            .contains("Class <BB,CA,DD,TT*,TT,SC,ET,C,X>")
    );
    assert!(
        !terminal
            .lines
            .iter()
            .any(|line| line.contains("Detach ships from the selected fleet"))
    );
    assert!(line_containing(&terminal, "Staged for New Fleet: ").contains("none"));
    assert!(
        !terminal
            .lines
            .iter()
            .any(|line| line.contains("Remaining on Donor: "))
    );

    enter_detach_input(&mut app, "dd");
    submit_detach(&mut app);
    app.render(&mut terminal).expect("render quantity prompt");
    assert!(
        line_containing(&terminal, "DD to stage (max 4)")
            .contains("DD to stage (max 4) [1] <Q> ->")
    );

    submit_detach(&mut app);
    app.render(&mut terminal).expect("render staged summary");
    assert!(line_containing(&terminal, "Staged for New Fleet: ").contains("DD=1"));
    assert!(line_containing(&terminal, "Remaining on Donor: ").contains("SC=1 CA=1 DD=3 TT*=4"));

    enter_detach_input(&mut app, "c");
    submit_detach(&mut app);
    app.render(&mut terminal)
        .expect("render menu notice after commission");
    assert_eq!(app.current_screen(), ScreenId::FleetDetach);
    assert!(line_containing(&terminal, "Staged for New Fleet: ").contains("none"));
    assert!(
        !terminal
            .lines
            .iter()
            .any(|line| line.contains("Remaining on Donor: "))
    );
    let updated = latest_runtime_state(&fixture_dir).game_data;
    let first_commission_message = format!(
        "Commissioned Fleet #{:02} from Fleet #01.",
        updated
            .fleets
            .records
            .last()
            .expect("detached fleet")
            .local_slot_word_raw()
    );
    assert!(
        terminal
            .lines
            .iter()
            .any(|line| line.contains(&first_commission_message))
    );

    assert_eq!(updated.fleets.records.len(), initial_fleet_count + 1);
    assert_eq!(updated.fleets.records[0].scout_count(), 1);
    assert_eq!(updated.fleets.records[0].cruiser_count(), 1);
    assert_eq!(updated.fleets.records[0].destroyer_count(), 3);
    assert_eq!(updated.fleets.records[0].troop_transport_count(), 4);
    assert_eq!(updated.fleets.records[0].army_count(), 4);
    let detached = updated.fleets.records.last().expect("detached fleet");
    assert_eq!(detached.destroyer_count(), 1);
    assert_eq!(detached.scout_count(), 0);
    assert_eq!(detached.cruiser_count(), 0);
    assert_eq!(detached.troop_transport_count(), 0);
    assert_eq!(detached.army_count(), 0);
    assert_eq!(detached.rules_of_engagement(), 6);
    assert_eq!(updated.fleets.records[0].rules_of_engagement(), 0);
}

/// A detached fleet starts at ECGAME's default ROE, 6, whatever the donor's
/// ROE is; one without a warship gets 0.
#[test]
fn fleet_detach_new_fleet_starts_at_the_classic_default_roe() {
    let fixture_dir = temp_game_copy();
    let mut game_data = CoreGameData::load(&fixture_dir).expect("load fixture");
    let donor = &mut game_data.fleets.records[0];
    donor.set_battleship_count(0);
    donor.set_cruiser_count(1);
    donor.set_destroyer_count(2);
    donor.set_troop_transport_count(0);
    donor.set_army_count(0);
    donor.set_scout_count(2);
    donor.set_etac_count(0);
    donor.recompute_max_speed_from_composition();
    donor.set_current_speed(0);
    donor.set_rules_of_engagement(3);
    game_data.save(&fixture_dir).expect("save fixture");
    let store = CampaignStore::open_default_in_dir(&fixture_dir).expect("open campaign store");
    import_directory_snapshot(&store, &fixture_dir).expect("refresh sqlite snapshot");

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
        apply_action(&mut app, Action::Fleet(FleetAction::OpenMenu)),
        AppOutcome::Continue
    );
    open_detach_from_fleet_menu(&mut app, Some(1));
    for class in ["dd", "sc"] {
        enter_detach_input(&mut app, class);
        submit_detach(&mut app);
        submit_detach(&mut app);
        enter_detach_input(&mut app, "c");
        submit_detach(&mut app);
    }

    let updated = latest_runtime_state(&fixture_dir).game_data;
    let roe_of = |destroyers: u16, scouts: u16, cruisers: u16| {
        updated
            .fleets
            .records
            .iter()
            .find(|fleet| {
                fleet.owner_empire_raw() == 1
                    && fleet.destroyer_count() == destroyers
                    && fleet.scout_count() == scouts
                    && fleet.cruiser_count() == cruisers
            })
            .expect("fleet with that composition")
            .rules_of_engagement()
    };
    assert_eq!(roe_of(1, 0, 0), 6, "the detached destroyer");
    assert_eq!(roe_of(0, 1, 0), 0, "the detached scout");
    assert_eq!(roe_of(1, 1, 1), 3, "the donor keeps its own ROE");
}

#[test]
fn fleet_detach_last_commissioned_message_persists_until_overwritten() {
    let fixture_dir = temp_game_copy();
    let mut game_data = CoreGameData::load(&fixture_dir).expect("load fixture");
    let donor = &mut game_data.fleets.records[0];
    donor.set_battleship_count(0);
    donor.set_cruiser_count(1);
    donor.set_destroyer_count(4);
    donor.set_troop_transport_count(4);
    donor.set_army_count(4);
    donor.set_scout_count(1);
    donor.set_etac_count(0);
    donor.recompute_max_speed_from_composition();
    donor.set_current_speed(0);
    donor.set_rules_of_engagement(0);
    game_data.save(&fixture_dir).expect("save fixture");
    let store = CampaignStore::open_default_in_dir(&fixture_dir).expect("open campaign store");
    import_directory_snapshot(&store, &fixture_dir).expect("refresh sqlite snapshot");

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
        apply_action(&mut app, Action::Fleet(FleetAction::OpenMenu)),
        AppOutcome::Continue
    );
    open_detach_from_fleet_menu(&mut app, Some(1));

    enter_detach_input(&mut app, "dd");
    submit_detach(&mut app);
    submit_detach(&mut app);
    enter_detach_input(&mut app, "c");
    submit_detach(&mut app);

    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("render after first commission");
    let first_commission_message = line_containing(&terminal, "Commissioned Fleet #")
        .trim_end()
        .to_string();

    enter_detach_input(&mut app, "zz");
    submit_detach(&mut app);
    app.render(&mut terminal)
        .expect("render empty commission warning with pinned message");
    assert_eq!(
        app.fleet.detach_status.as_deref(),
        Some("Use BB, CA, DD, TT*, TT, SC, ET, C, X, or Q.")
    );
    assert!(
        terminal
            .lines
            .iter()
            .any(|line| line.contains(&first_commission_message))
    );

    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::ClearDetachSelection)),
        AppOutcome::Continue
    );
    enter_detach_input(&mut app, "ca");
    submit_detach(&mut app);
    submit_detach(&mut app);
    enter_detach_input(&mut app, "c");
    submit_detach(&mut app);
    app.render(&mut terminal)
        .expect("render after second commission");
    let second_commission_message = line_containing(&terminal, "Commissioned Fleet #")
        .trim_end()
        .to_string();
    assert_ne!(second_commission_message, first_commission_message);
    assert!(
        !terminal
            .lines
            .iter()
            .any(|line| line.contains(&first_commission_message))
    );
}

#[test]
fn fleet_detach_commission_requires_staged_ships_and_preserves_staged_block() {
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
        apply_action(&mut app, Action::Fleet(FleetAction::OpenMenu)),
        AppOutcome::Continue
    );
    open_detach_from_fleet_menu(&mut app, Some(1));
    assert_eq!(app.current_screen(), ScreenId::FleetDetach);

    let mut terminal = CaptureTerminal::new();
    enter_detach_input(&mut app, "c");
    submit_detach(&mut app);
    app.render(&mut terminal)
        .expect("render empty commission warning");

    assert_eq!(app.current_screen(), ScreenId::FleetDetach);
    assert!(
        line_containing(&terminal, "Stage at least one ship before commissioning.")
            .contains("Stage at least one ship before commissioning.")
    );
    assert!(line_containing(&terminal, "Staged for New Fleet: ").contains("none"));
}

#[test]
fn fleet_detach_x_clears_staged_selection_without_leaving_screen() {
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
        apply_action(&mut app, Action::Fleet(FleetAction::OpenMenu)),
        AppOutcome::Continue
    );
    open_detach_from_fleet_menu(&mut app, Some(1));

    enter_detach_input(&mut app, "sc");
    submit_detach(&mut app);
    submit_detach(&mut app);
    assert_eq!(app.current_screen(), ScreenId::FleetDetach);
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::ClearDetachSelection)),
        AppOutcome::Continue
    );

    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("render cleared staged selection");
    assert_eq!(app.current_screen(), ScreenId::FleetDetach);
    assert!(line_containing(&terminal, "Staged for New Fleet: ").contains("none"));
}

#[test]
fn fleet_detach_leaves_at_least_one_ship_on_the_donor() {
    let fixture_dir = temp_game_copy();
    let mut game_data = CoreGameData::load(&fixture_dir).expect("load fixture");
    let donor = &mut game_data.fleets.records[0];
    donor.set_destroyer_count(2);
    donor.set_battleship_count(0);
    donor.set_cruiser_count(0);
    donor.set_troop_transport_count(0);
    donor.set_army_count(0);
    donor.set_scout_count(0);
    donor.set_etac_count(0);
    donor.recompute_max_speed_from_composition();
    donor.set_current_speed(0);
    game_data.save(&fixture_dir).expect("save fixture");
    let store = CampaignStore::open_default_in_dir(&fixture_dir).expect("open campaign store");
    import_directory_snapshot(&store, &fixture_dir).expect("refresh sqlite snapshot");

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
        apply_action(&mut app, Action::Fleet(FleetAction::OpenMenu)),
        AppOutcome::Continue
    );
    open_detach_from_fleet_menu(&mut app, Some(1));

    enter_detach_input(&mut app, "dd");
    submit_detach(&mut app);
    enter_detach_input(&mut app, "2");
    submit_detach(&mut app);

    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("render donor minimum warning");
    assert_eq!(app.current_screen(), ScreenId::FleetDetach);
    assert!(
        line_containing(&terminal, "Enter a quantity from 1 to 1.")
            .contains("Enter a quantity from 1 to 1.")
    );
    assert!(line_containing(&terminal, "Staged for New Fleet: ").contains("none"));
}

#[test]
fn fleet_detach_final_commission_returns_to_menu_with_new_fleet_number_notice() {
    let fixture_dir = temp_game_copy();
    let mut game_data = CoreGameData::load(&fixture_dir).expect("load fixture");
    let donor = &mut game_data.fleets.records[0];
    donor.set_destroyer_count(2);
    donor.set_battleship_count(0);
    donor.set_cruiser_count(0);
    donor.set_troop_transport_count(0);
    donor.set_army_count(0);
    donor.set_scout_count(0);
    donor.set_etac_count(0);
    donor.recompute_max_speed_from_composition();
    donor.set_current_speed(0);
    game_data.save(&fixture_dir).expect("save fixture");
    let store = CampaignStore::open_default_in_dir(&fixture_dir).expect("open campaign store");
    import_directory_snapshot(&store, &fixture_dir).expect("refresh sqlite snapshot");

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
        apply_action(&mut app, Action::Fleet(FleetAction::OpenMenu)),
        AppOutcome::Continue
    );
    open_detach_from_fleet_menu(&mut app, Some(1));

    enter_detach_input(&mut app, "dd");
    submit_detach(&mut app);
    submit_detach(&mut app);
    enter_detach_input(&mut app, "c");
    submit_detach(&mut app);

    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("render fleet menu notice after final detach");
    let updated = latest_runtime_state(&app.game_dir).game_data;
    let new_fleet_number = updated
        .fleets
        .records
        .last()
        .expect("detached fleet")
        .local_slot_word_raw();

    assert_eq!(app.current_screen(), ScreenId::FleetMenu);
    assert!(line_containing(&terminal, "Notice: ").contains(&format!(
        "Detached ships from Fleet #01 into Fleet #{new_fleet_number:02}."
    )));
}

#[test]
fn fleet_detach_invalidated_colonize_donor_resets_to_hold() {
    let fixture_dir = temp_game_copy();
    let mut game_data = CoreGameData::load(&fixture_dir).expect("load fixture");
    let donor = &mut game_data.fleets.records[0];
    donor.set_battleship_count(0);
    donor.set_cruiser_count(1);
    donor.set_destroyer_count(0);
    donor.set_troop_transport_count(0);
    donor.set_army_count(0);
    donor.set_scout_count(0);
    donor.set_etac_count(1);
    donor.set_current_location_coords_raw([8, 9]);
    donor.set_standing_order_kind(nc_data::Order::ColonizeWorld);
    donor.set_standing_order_target_coords_raw([15, 13]);
    donor.recompute_max_speed_from_composition();
    donor.set_current_speed(3);
    game_data.save(&fixture_dir).expect("save fixture");
    let store = CampaignStore::open_default_in_dir(&fixture_dir).expect("open campaign store");
    import_directory_snapshot(&store, &fixture_dir).expect("refresh sqlite snapshot");

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
        apply_action(&mut app, Action::Fleet(FleetAction::OpenMenu)),
        AppOutcome::Continue
    );
    open_detach_from_fleet_menu(&mut app, Some(1));

    enter_detach_input(&mut app, "et");
    submit_detach(&mut app);
    submit_detach(&mut app);
    enter_detach_input(&mut app, "c");
    submit_detach(&mut app);

    let updated = latest_runtime_state(&fixture_dir).game_data;
    let donor = &updated.fleets.records[0];
    assert_eq!(donor.etac_count(), 0);
    assert_eq!(donor.cruiser_count(), 1);
    assert_eq!(donor.standing_order_kind(), nc_data::Order::HoldPosition);
    assert_eq!(donor.current_speed(), 0);
    assert_eq!(donor.standing_order_target_coords_raw(), [8, 9]);
}

#[test]
fn fleet_detach_prompt_reports_missing_fleet_number() {
    let fixture_dir = temp_game_copy();
    let mut game_data = CoreGameData::load(&fixture_dir).expect("load fixture");
    let fleet_one = game_data
        .fleets
        .records
        .iter_mut()
        .find(|fleet| fleet.owner_empire_raw() == 1 && fleet.local_slot_word_raw() == 1)
        .expect("fleet #1 should exist");
    fleet_one.set_battleship_count(2);
    fleet_one.set_cruiser_count(0);
    fleet_one.set_destroyer_count(0);
    fleet_one.set_troop_transport_count(0);
    fleet_one.set_army_count(0);
    fleet_one.set_scout_count(0);
    fleet_one.set_etac_count(0);
    let fleet_two = game_data
        .fleets
        .records
        .iter_mut()
        .find(|fleet| fleet.owner_empire_raw() == 1 && fleet.local_slot_word_raw() == 2)
        .expect("fleet #2 should exist");
    fleet_two.set_battleship_count(0);
    fleet_two.set_cruiser_count(0);
    fleet_two.set_destroyer_count(6);
    fleet_two.set_troop_transport_count(0);
    fleet_two.set_army_count(0);
    fleet_two.set_scout_count(0);
    fleet_two.set_etac_count(0);
    game_data.save(&fixture_dir).expect("save fixture");
    let store = CampaignStore::open_default_in_dir(&fixture_dir).expect("open campaign store");
    import_directory_snapshot(&store, &fixture_dir).expect("refresh sqlite snapshot");

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
        apply_action(&mut app, Action::Fleet(FleetAction::OpenMenu)),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::OpenDetach)),
        AppOutcome::Continue
    );
    assert_eq!(app.fleet.menu_prompt_default_value, "2");
    submit_fleet_menu_prompt_value(&mut app, "99");

    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("render detach prompt missing fleet notice");
    assert_eq!(app.current_screen(), ScreenId::FleetMenu);
    assert_eq!(app.fleet.menu_prompt_default_value, "2");
    assert!(line_containing(&terminal, "COMMAND <- Detach Fleet #").contains("Detach Fleet #"));
    assert!(
        terminal
            .lines
            .iter()
            .any(|line| line.contains("Fleet #99 is not in your fleet list."))
    );
}

#[test]
fn fleet_detach_prompt_reports_single_ship_fleet_as_ineligible() {
    let fixture_dir = temp_game_copy();
    let mut game_data = CoreGameData::load(&fixture_dir).expect("load fixture");
    let donor = &mut game_data.fleets.records[0];
    donor.set_destroyer_count(1);
    donor.set_battleship_count(0);
    donor.set_cruiser_count(0);
    donor.set_troop_transport_count(0);
    donor.set_army_count(0);
    donor.set_scout_count(0);
    donor.set_etac_count(0);
    donor.recompute_max_speed_from_composition();
    donor.set_current_speed(0);
    let fallback = game_data
        .fleets
        .records
        .iter_mut()
        .find(|fleet| fleet.owner_empire_raw() == 1 && fleet.local_slot_word_raw() == 2)
        .expect("fleet #2 should exist");
    fallback.set_battleship_count(0);
    fallback.set_cruiser_count(0);
    fallback.set_destroyer_count(4);
    fallback.set_troop_transport_count(0);
    fallback.set_army_count(0);
    fallback.set_scout_count(0);
    fallback.set_etac_count(0);
    game_data.save(&fixture_dir).expect("save fixture");
    let store = CampaignStore::open_default_in_dir(&fixture_dir).expect("open campaign store");
    import_directory_snapshot(&store, &fixture_dir).expect("refresh sqlite snapshot");

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
        apply_action(&mut app, Action::Fleet(FleetAction::OpenMenu)),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::OpenDetach)),
        AppOutcome::Continue
    );
    assert_eq!(app.fleet.menu_prompt_default_value, "2");
    submit_fleet_menu_prompt(&mut app, Some(1));

    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("render detach prompt single-ship notice");
    assert_eq!(app.current_screen(), ScreenId::FleetMenu);
    assert_eq!(app.fleet.menu_prompt_default_value, "2");
    assert_eq!(
        app.fleet
            .menu_prompt_status
            .as_ref()
            .map(|feedback| feedback.message()),
        Some("Fleet #1 has only one ship and is not eligible to detach any ships.")
    );
}

#[test]
fn fleet_detach_prompt_defaults_to_largest_owned_fleet_by_ship_total() {
    let fixture_dir = temp_game_copy();
    let mut game_data = CoreGameData::load(&fixture_dir).expect("load fixture");
    let fleet_one = game_data
        .fleets
        .records
        .iter_mut()
        .find(|fleet| fleet.owner_empire_raw() == 1 && fleet.local_slot_word_raw() == 1)
        .expect("fleet #1 should exist");
    fleet_one.set_battleship_count(3);
    fleet_one.set_cruiser_count(0);
    fleet_one.set_destroyer_count(0);
    fleet_one.set_troop_transport_count(0);
    fleet_one.set_army_count(0);
    fleet_one.set_scout_count(0);
    fleet_one.set_etac_count(0);
    let fleet_two = game_data
        .fleets
        .records
        .iter_mut()
        .find(|fleet| fleet.owner_empire_raw() == 1 && fleet.local_slot_word_raw() == 2)
        .expect("fleet #2 should exist");
    fleet_two.set_battleship_count(0);
    fleet_two.set_cruiser_count(0);
    fleet_two.set_destroyer_count(5);
    fleet_two.set_troop_transport_count(0);
    fleet_two.set_army_count(0);
    fleet_two.set_scout_count(0);
    fleet_two.set_etac_count(0);
    game_data.save(&fixture_dir).expect("save fixture");
    let store = CampaignStore::open_default_in_dir(&fixture_dir).expect("open campaign store");
    import_directory_snapshot(&store, &fixture_dir).expect("refresh sqlite snapshot");

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
        apply_action(&mut app, Action::Fleet(FleetAction::OpenMenu)),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::OpenDetach)),
        AppOutcome::Continue
    );
    assert_eq!(app.fleet.menu_prompt_default_value, "2");
}
