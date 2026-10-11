use crate::support::*;
use nc_compat::{load_classic_core_files, save_classic_core_files};

#[test]
fn fleet_detach_uses_staged_class_prompt_and_creates_new_fleet() {
    let fixture_dir = temp_game_copy();
    let mut game_data = load_classic_core_files(&fixture_dir).expect("load fixture");
    let initial_fleet_count = game_data.fleets.len();
    let donor = &mut game_data.fleets[0];
    donor.ships.scouts = 1;
    donor.ships.cruisers = 1;
    donor.ships.destroyers = 4;
    donor.ships.battleships = 0;
    nc_data::set_troop_transports(donor, 4);
    nc_data::set_armies_aboard(donor, 4);
    donor.ships.etacs = 0;
    donor.sector = nc_data::Coords::from_array([8, 9]);
    nc_data::recompute_max_speed(donor);
    donor.speed = 0;
    donor.rules_of_engagement = 0;
    save_classic_core_files(&fixture_dir, &game_data).expect("save fixture");
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
    assert!(terminal.line(10).contains("Ships: SC1 CA1 DD4 TT*4"));
    assert!(!terminal.line(10).contains("AR="));
    assert_eq!(terminal.line(12).trim_end(), "<C>ommission, <X> Cancel");
    assert!(
        line_containing(&terminal, "Class <SC,CA,DD,TT*>")
            .contains("Class <SC,CA,DD,TT*>")
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
    assert!(line_containing(&terminal, "Staged for New Fleet: ").contains("DD1"));
    assert!(line_containing(&terminal, "Remaining on Donor: ").contains("SC1 CA1 DD3 TT*4"));

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
            .last()
            .expect("detached fleet")
            .fleet_number
            .0
    );
    assert!(
        terminal
            .lines
            .iter()
            .any(|line| line.contains(&first_commission_message))
    );

    assert_eq!(updated.fleets.len(), initial_fleet_count + 1);
    assert_eq!(updated.fleets[0].ships.scouts, 1);
    assert_eq!(updated.fleets[0].ships.cruisers, 1);
    assert_eq!(updated.fleets[0].ships.destroyers, 3);
    assert_eq!(nc_data::troop_transports(&updated.fleets[0]), 4);
    assert_eq!(updated.fleets[0].ships.loaded_transports, 4);
    let detached = updated.fleets.last().expect("detached fleet");
    assert_eq!(detached.ships.destroyers, 1);
    assert_eq!(detached.ships.scouts, 0);
    assert_eq!(detached.ships.cruisers, 0);
    assert_eq!(nc_data::troop_transports(&detached), 0);
    assert_eq!(detached.ships.loaded_transports, 0);
    assert_eq!(detached.rules_of_engagement, 6);
    assert_eq!(updated.fleets[0].rules_of_engagement, 0);
}

/// A detached fleet starts at ECGAME's default ROE, 6, whatever the donor's
/// ROE is; one without a warship gets 0.
#[test]
fn fleet_detach_new_fleet_starts_at_the_classic_default_roe() {
    let fixture_dir = temp_game_copy();
    let mut game_data = load_classic_core_files(&fixture_dir).expect("load fixture");
    let donor = &mut game_data.fleets[0];
    donor.ships.battleships = 0;
    donor.ships.cruisers = 1;
    donor.ships.destroyers = 2;
    nc_data::set_troop_transports(donor, 0);
    nc_data::set_armies_aboard(donor, 0);
    donor.ships.scouts = 2;
    donor.ships.etacs = 0;
    nc_data::recompute_max_speed(donor);
    donor.speed = 0;
    donor.rules_of_engagement = 3;
    save_classic_core_files(&fixture_dir, &game_data).expect("save fixture");
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
            .iter()
            .find(|fleet| {
                fleet.owner.0 == 1
                    && fleet.ships.destroyers == destroyers
                    && fleet.ships.scouts == scouts
                    && fleet.ships.cruisers == cruisers
            })
            .expect("fleet with that composition")
            .rules_of_engagement
    };
    assert_eq!(roe_of(1, 0, 0), 6, "the detached destroyer");
    assert_eq!(roe_of(0, 1, 0), 0, "the detached scout");
    assert_eq!(roe_of(1, 1, 1), 3, "the donor keeps its own ROE");
}

#[test]
fn fleet_detach_last_commissioned_message_persists_until_overwritten() {
    let fixture_dir = temp_game_copy();
    let mut game_data = load_classic_core_files(&fixture_dir).expect("load fixture");
    let donor = &mut game_data.fleets[0];
    donor.ships.battleships = 0;
    donor.ships.cruisers = 1;
    donor.ships.destroyers = 4;
    nc_data::set_troop_transports(donor, 4);
    nc_data::set_armies_aboard(donor, 4);
    donor.ships.scouts = 1;
    donor.ships.etacs = 0;
    nc_data::recompute_max_speed(donor);
    donor.speed = 0;
    donor.rules_of_engagement = 0;
    save_classic_core_files(&fixture_dir, &game_data).expect("save fixture");
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
        Some("Choose a listed class, C, X, or Q.")
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
    let mut game_data = load_classic_core_files(&fixture_dir).expect("load fixture");
    let donor = &mut game_data.fleets[0];
    donor.ships.destroyers = 2;
    donor.ships.battleships = 0;
    donor.ships.cruisers = 0;
    nc_data::set_troop_transports(donor, 0);
    nc_data::set_armies_aboard(donor, 0);
    donor.ships.scouts = 0;
    donor.ships.etacs = 0;
    nc_data::recompute_max_speed(donor);
    donor.speed = 0;
    save_classic_core_files(&fixture_dir, &game_data).expect("save fixture");
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
    let mut game_data = load_classic_core_files(&fixture_dir).expect("load fixture");
    let donor = &mut game_data.fleets[0];
    donor.ships.destroyers = 2;
    donor.ships.battleships = 0;
    donor.ships.cruisers = 0;
    nc_data::set_troop_transports(donor, 0);
    nc_data::set_armies_aboard(donor, 0);
    donor.ships.scouts = 0;
    donor.ships.etacs = 0;
    nc_data::recompute_max_speed(donor);
    donor.speed = 0;
    save_classic_core_files(&fixture_dir, &game_data).expect("save fixture");
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
        .last()
        .expect("detached fleet")
        .fleet_number
        .0;

    assert_eq!(app.current_screen(), ScreenId::FleetMenu);
    assert!(line_containing(&terminal, "Notice: ").contains(&format!(
        "Detached ships from Fleet #01 into Fleet #{new_fleet_number:02}."
    )));
}

#[test]
fn fleet_detach_invalidated_colonize_donor_resets_to_hold() {
    let fixture_dir = temp_game_copy();
    let mut game_data = load_classic_core_files(&fixture_dir).expect("load fixture");
    let donor = &mut game_data.fleets[0];
    donor.ships.battleships = 0;
    donor.ships.cruisers = 1;
    donor.ships.destroyers = 0;
    nc_data::set_troop_transports(donor, 0);
    nc_data::set_armies_aboard(donor, 0);
    donor.ships.scouts = 0;
    donor.ships.etacs = 1;
    donor.sector = nc_data::Coords::from_array([8, 9]);
    donor.order = nc_data::Order::ColonizeWorld;
    donor.target = nc_data::Coords::from_array([15, 13]);
    nc_data::recompute_max_speed(donor);
    donor.speed = 3;
    save_classic_core_files(&fixture_dir, &game_data).expect("save fixture");
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
    let donor = &updated.fleets[0];
    assert_eq!(donor.ships.etacs, 0);
    assert_eq!(donor.ships.cruisers, 1);
    assert_eq!(donor.order, nc_data::Order::HoldPosition);
    assert_eq!(donor.speed, 0);
    assert_eq!(donor.target.to_array(), [8, 9]);
}

#[test]
fn fleet_detach_prompt_reports_missing_fleet_number() {
    let fixture_dir = temp_game_copy();
    let mut game_data = load_classic_core_files(&fixture_dir).expect("load fixture");
    let fleet_one = game_data
        .fleets
        .iter_mut()
        .find(|fleet| fleet.owner.0 == 1 && fleet.fleet_number.0 == 1)
        .expect("fleet #1 should exist");
    fleet_one.ships.battleships = 2;
    fleet_one.ships.cruisers = 0;
    fleet_one.ships.destroyers = 0;
    nc_data::set_troop_transports(fleet_one, 0);
    nc_data::set_armies_aboard(fleet_one, 0);
    fleet_one.ships.scouts = 0;
    fleet_one.ships.etacs = 0;
    let fleet_two = game_data
        .fleets
        .iter_mut()
        .find(|fleet| fleet.owner.0 == 1 && fleet.fleet_number.0 == 2)
        .expect("fleet #2 should exist");
    fleet_two.ships.battleships = 0;
    fleet_two.ships.cruisers = 0;
    fleet_two.ships.destroyers = 6;
    nc_data::set_troop_transports(fleet_two, 0);
    nc_data::set_armies_aboard(fleet_two, 0);
    fleet_two.ships.scouts = 0;
    fleet_two.ships.etacs = 0;
    save_classic_core_files(&fixture_dir, &game_data).expect("save fixture");
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
    let mut game_data = load_classic_core_files(&fixture_dir).expect("load fixture");
    let donor = &mut game_data.fleets[0];
    donor.ships.destroyers = 1;
    donor.ships.battleships = 0;
    donor.ships.cruisers = 0;
    nc_data::set_troop_transports(donor, 0);
    nc_data::set_armies_aboard(donor, 0);
    donor.ships.scouts = 0;
    donor.ships.etacs = 0;
    nc_data::recompute_max_speed(donor);
    donor.speed = 0;
    let fallback = game_data
        .fleets
        .iter_mut()
        .find(|fleet| fleet.owner.0 == 1 && fleet.fleet_number.0 == 2)
        .expect("fleet #2 should exist");
    fallback.ships.battleships = 0;
    fallback.ships.cruisers = 0;
    fallback.ships.destroyers = 4;
    nc_data::set_troop_transports(fallback, 0);
    nc_data::set_armies_aboard(fallback, 0);
    fallback.ships.scouts = 0;
    fallback.ships.etacs = 0;
    save_classic_core_files(&fixture_dir, &game_data).expect("save fixture");
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
    let mut game_data = load_classic_core_files(&fixture_dir).expect("load fixture");
    let fleet_one = game_data
        .fleets
        .iter_mut()
        .find(|fleet| fleet.owner.0 == 1 && fleet.fleet_number.0 == 1)
        .expect("fleet #1 should exist");
    fleet_one.ships.battleships = 3;
    fleet_one.ships.cruisers = 0;
    fleet_one.ships.destroyers = 0;
    nc_data::set_troop_transports(fleet_one, 0);
    nc_data::set_armies_aboard(fleet_one, 0);
    fleet_one.ships.scouts = 0;
    fleet_one.ships.etacs = 0;
    let fleet_two = game_data
        .fleets
        .iter_mut()
        .find(|fleet| fleet.owner.0 == 1 && fleet.fleet_number.0 == 2)
        .expect("fleet #2 should exist");
    fleet_two.ships.battleships = 0;
    fleet_two.ships.cruisers = 0;
    fleet_two.ships.destroyers = 5;
    nc_data::set_troop_transports(fleet_two, 0);
    nc_data::set_armies_aboard(fleet_two, 0);
    fleet_two.ships.scouts = 0;
    fleet_two.ships.etacs = 0;
    save_classic_core_files(&fixture_dir, &game_data).expect("save fixture");
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
