//! Detach and Transfer with crippled ships in the BBS door (t093). A plain
//! quantity stages healthy ships first and crippled ones only if the class
//! has too few healthy; `3!` stages exactly three crippled ships and is
//! rejected, with nothing staged and nothing changed, when the class has
//! fewer. The staged and remaining lines show the ships in ship notation
//! (`CA3!1`), resolved by the same rule the commit uses.

use crate::support::*;
use nc_data::{CrippledHulls, Fleet, ShipCategory, set_armies_aboard, set_troop_transports};

fn hulls(turns_1: u32, turns_2: u32, fresh: u32) -> CrippledHulls {
    CrippledHulls {
        turns: [turns_1, turns_2],
        fresh,
    }
}

fn arrange(fleet: &mut Fleet, scouts: u16, cruisers: u16, loaded: u16, empty: u16) {
    fleet.ships.scouts = scouts;
    fleet.ships.battleships = 0;
    fleet.ships.cruisers = cruisers;
    fleet.ships.destroyers = 0;
    set_troop_transports(fleet, loaded + empty);
    set_armies_aboard(fleet, loaded);
    fleet.ships.etacs = 0;
    nc_data::recompute_max_speed(fleet);
    fleet.speed = 0;
    fleet.rules_of_engagement = 0;
}

/// An app whose empire-1 fleets #1 and #2 share a sector, with `setup`
/// applied to the game data first.
fn app_with(setup: impl FnOnce(&mut CoreGameData)) -> (App, TempGameDir) {
    let root = temp_game_with_same_sector_fleets_copy();
    let mut state = latest_runtime_state(&root);
    setup(&mut state.game_data);
    save_runtime_state(&root, &state);
    let app = App::load(AppConfig {
        game_dir: root.clone(),
        player_record_index_1_based: 1,
        export_root: None,
        queue_dir: None,
        session_timeout_secs: None,
        game_config: Default::default(),
    })
    .expect("app should load");
    (app, root)
}

/// Fleet #1: a scout and twelve cruisers, three of them crippled (one this
/// pass, one with two turns left, one with one). Fleet #2: two destroyers.
fn twelve_cruisers(game_data: &mut CoreGameData) {
    arrange(&mut game_data.fleets[0], 1, 12, 0, 0);
    game_data.fleets[0].crippled.cruisers = hulls(1, 1, 1);
    arrange(&mut game_data.fleets[1], 0, 0, 0, 0);
    game_data.fleets[1].ships.destroyers = 2;
}

fn detach_app(setup: impl FnOnce(&mut CoreGameData)) -> (App, TempGameDir) {
    let (mut app, root) = app_with(setup);
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::OpenMenu)),
        AppOutcome::Continue
    );
    open_detach_from_fleet_menu(&mut app, Some(1));
    (app, root)
}

fn transfer_app(setup: impl FnOnce(&mut CoreGameData)) -> (App, TempGameDir) {
    let (mut app, root) = app_with(setup);
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::OpenMenu)),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::OpenTransfer)),
        AppOutcome::Continue
    );
    submit_fleet_menu_prompt(&mut app, Some(1));
    submit_fleet_menu_prompt(&mut app, Some(2));
    assert_eq!(app.current_screen(), ScreenId::FleetTransfer);
    (app, root)
}

fn enter_transfer_input(app: &mut App, input: &str) {
    for ch in input.chars() {
        apply_action(app, Action::Fleet(FleetAction::AppendTransferChar(ch)));
    }
}

fn submit_transfer(app: &mut App) {
    apply_action(app, Action::Fleet(FleetAction::SubmitTransfer));
}

fn detach_stage(app: &mut App, class: &str, quantity: &str) {
    enter_detach_input(app, class);
    submit_detach(app);
    enter_detach_input(app, quantity);
    submit_detach(app);
}

fn transfer_stage(app: &mut App, class: &str, quantity: &str) {
    enter_transfer_input(app, class);
    submit_transfer(app);
    enter_transfer_input(app, quantity);
    submit_transfer(app);
}

fn screen(app: &mut App) -> CaptureTerminal {
    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal).expect("screen renders");
    terminal
}

fn detach_staged(app: &mut App) -> String {
    let terminal = screen(app);
    line_containing(&terminal, "Staged for New Fleet: ")
        .trim()
        .to_string()
}

fn detach_remaining(app: &mut App) -> String {
    let terminal = screen(app);
    line_containing(&terminal, "Remaining on Donor: ")
        .trim()
        .to_string()
}

fn status(app: &App) -> Option<String> {
    app.fleet.detach_status.clone()
}

#[test]
fn a_plain_quantity_stages_healthy_cruisers_and_the_screen_says_so() {
    let (mut app, _root) = detach_app(twelve_cruisers);
    let terminal = screen(&mut app);
    assert!(line_containing(&terminal, "Ships: ").contains("Ships: SC1 CA12!3"));
    detach_stage(&mut app, "ca", "3");
    assert!(
        detach_staged(&mut app).ends_with("CA3"),
        "{}",
        detach_staged(&mut app)
    );
    assert!(detach_remaining(&mut app).ends_with("SC1 CA9!3"));
}

#[test]
fn n_bang_stages_exactly_n_crippled_cruisers() {
    let (mut app, _root) = detach_app(twelve_cruisers);
    detach_stage(&mut app, "ca", "3!");
    assert_eq!(status(&app), None);
    assert!(
        detach_staged(&mut app).ends_with("CA3!3"),
        "{}",
        detach_staged(&mut app)
    );
    assert!(detach_remaining(&mut app).ends_with("SC1 CA9"));
}

#[test]
fn a_plain_quantity_larger_than_the_healthy_ships_shows_the_crippled_it_takes() {
    let (mut app, _root) = detach_app(|game_data| {
        arrange(&mut game_data.fleets[0], 1, 4, 0, 0);
        game_data.fleets[0].crippled.cruisers = hulls(0, 1, 1);
    });
    detach_stage(&mut app, "ca", "3");
    assert!(
        detach_staged(&mut app).ends_with("CA3!1"),
        "{}",
        detach_staged(&mut app)
    );
    assert!(detach_remaining(&mut app).ends_with("SC1 CA1!1"));
}

#[test]
fn n_bang_beyond_the_crippled_cruisers_is_rejected_and_changes_nothing() {
    let (mut app, root) = detach_app(|game_data| {
        arrange(&mut game_data.fleets[0], 1, 4, 0, 0);
        game_data.fleets[0].crippled.cruisers = hulls(0, 1, 1);
    });
    let before = latest_runtime_state(&root).game_data;
    enter_detach_input(&mut app, "ca");
    submit_detach(&mut app);
    enter_detach_input(&mut app, "3!");
    submit_detach(&mut app);
    // Two are crippled; the two healthy ships are not substituted.
    assert_eq!(
        status(&app).as_deref(),
        Some("Enter a crippled quantity from 1 to 2.")
    );
    assert_eq!(
        app.fleet.detach_selection,
        nc_data::FleetDetachSelection::default()
    );
    assert_eq!(detach_staged(&mut app), "Staged for New Fleet: none");
    let terminal = screen(&mut app);
    assert!(
        line_containing(&terminal, "CA to stage (max 4, n! = crippled only)")
            .contains("[1] <Q> ->")
    );
    // The same prompt is still open for a corrected answer.
    app.fleet.detach_input.clear();
    enter_detach_input(&mut app, "2!");
    submit_detach(&mut app);
    assert_eq!(status(&app), None);
    assert!(detach_staged(&mut app).ends_with("CA2!2"));
    assert_eq!(latest_runtime_state(&root).game_data, before);
}

#[test]
fn n_bang_on_a_class_with_no_crippled_ships_says_so() {
    let (mut app, _root) = detach_app(|game_data| {
        arrange(&mut game_data.fleets[0], 3, 2, 0, 0);
    });
    detach_stage(&mut app, "sc", "1!");
    assert_eq!(
        status(&app).as_deref(),
        Some("No crippled SC left to stage.")
    );
    assert_eq!(detach_staged(&mut app), "Staged for New Fleet: none");
    // No crippled ships in the class, so the prompt does not offer `n!`.
    let terminal = screen(&mut app);
    assert!(line_containing(&terminal, "SC to stage (max 3) [1]").contains("<Q> ->"));
}

#[test]
fn zero_and_stray_input_stage_nothing() {
    let (mut app, _root) = detach_app(twelve_cruisers);
    detach_stage(&mut app, "ca", "0!");
    assert_eq!(
        status(&app).as_deref(),
        Some("Enter a crippled quantity from 1 to 3.")
    );
    app.fleet.detach_input.clear();
    enter_detach_input(&mut app, "0");
    submit_detach(&mut app);
    assert_eq!(
        status(&app).as_deref(),
        Some("Enter a quantity from 1 to 12.")
    );
    assert_eq!(
        app.fleet.detach_selection,
        nc_data::FleetDetachSelection::default()
    );
    // `!` needs digits before it, and nothing follows it.
    app.fleet.detach_input.clear();
    enter_detach_input(&mut app, "!");
    assert_eq!(app.fleet.detach_input, "");
    enter_detach_input(&mut app, "2!!");
    assert_eq!(app.fleet.detach_input, "2!");
    enter_detach_input(&mut app, "5");
    assert_eq!(app.fleet.detach_input, "2!");
    app.fleet.detach_input.clear();
    enter_detach_input(&mut app, "1234");
    assert_eq!(app.fleet.detach_input, "123");
}

#[test]
fn plain_and_crippled_quantities_add_up_in_one_staged_selection() {
    let (mut app, _root) = detach_app(twelve_cruisers);
    detach_stage(&mut app, "ca", "2");
    detach_stage(&mut app, "ca", "1!");
    assert!(
        detach_staged(&mut app).ends_with("CA3!1"),
        "{}",
        detach_staged(&mut app)
    );
    // Only two crippled cruisers are left to stage.
    detach_stage(&mut app, "ca", "3!");
    assert_eq!(
        status(&app).as_deref(),
        Some("Enter a crippled quantity from 1 to 2.")
    );
}

#[test]
fn commissioning_moves_the_crippled_cruisers_with_their_recovery_intact() {
    let (mut app, root) = detach_app(twelve_cruisers);
    detach_stage(&mut app, "ca", "2!");
    enter_detach_input(&mut app, "c");
    submit_detach(&mut app);
    let state = latest_runtime_state(&root).game_data;
    let new_fleet = state
        .fleets
        .iter()
        .filter(|fleet| fleet.owner.0 == 1)
        .max_by_key(|fleet| fleet.fleet_number.0)
        .expect("a new fleet");
    assert_eq!(nc_data::fleet_notation(new_fleet), "CA2!2");
    // This pass's newly crippled cruiser goes first, then the longest left.
    assert_eq!(new_fleet.crippled.cruisers, hulls(0, 1, 1));
    let donor = &state.fleets[0];
    assert_eq!(nc_data::fleet_notation(donor), "SC1 CA10!1");
    assert_eq!(donor.crippled.cruisers, hulls(1, 0, 0));
}

#[test]
fn transports_stage_by_cargo_and_the_confirmation_keeps_the_star() {
    let (mut app, root) = detach_app(|game_data| {
        arrange(&mut game_data.fleets[0], 1, 2, 2, 2);
        game_data.fleets[0].crippled.loaded_transports = hulls(0, 1, 0);
        game_data.fleets[0].crippled.empty_transports = hulls(1, 0, 0);
    });
    let terminal = screen(&mut app);
    assert!(line_containing(&terminal, "Ships: ").contains("Ships: SC1 CA2 TT*2!1 TT2!1"));
    // Each cargo state has its own crippled count: one empty transport is
    // crippled, so `2!` on the empty class is refused and `1!` is taken.
    detach_stage(&mut app, "tt", "2!");
    assert_eq!(
        status(&app).as_deref(),
        Some("Enter a crippled quantity from 1 to 1.")
    );
    apply_action(&mut app, Action::Fleet(FleetAction::CancelDetach));
    detach_stage(&mut app, "tt", "1!");
    assert_eq!(status(&app), None);
    assert!(
        detach_staged(&mut app).ends_with("TT1!1"),
        "{}",
        detach_staged(&mut app)
    );
    detach_stage(&mut app, "tt*", "1!");
    assert!(detach_staged(&mut app).ends_with("TT*1!1 TT1!1"));
    assert!(detach_remaining(&mut app).ends_with("SC1 CA2 TT*1 TT1"));
    detach_stage(&mut app, "tt", "1");
    assert!(detach_staged(&mut app).ends_with("TT*1!1 TT2!1"));
    enter_detach_input(&mut app, "c");
    submit_detach(&mut app);
    let state = latest_runtime_state(&root).game_data;
    let new_fleet = state
        .fleets
        .iter()
        .filter(|fleet| fleet.owner.0 == 1)
        .max_by_key(|fleet| fleet.fleet_number.0)
        .expect("a new fleet");
    assert_eq!(nc_data::fleet_notation(new_fleet), "TT*1!1 TT2!1");
    assert_eq!(new_fleet.ships.loaded_transports, 1);
    assert_eq!(new_fleet.crippled.loaded_transports, hulls(0, 1, 0));
    assert_eq!(new_fleet.crippled.empty_transports, hulls(1, 0, 0));
    let armies: u32 = state
        .fleets
        .iter()
        .filter(|fleet| fleet.owner.0 == 1)
        .map(|fleet| u32::from(fleet.ships.loaded_transports))
        .sum();
    assert_eq!(armies, 2, "no army is lost or made up");
}

#[test]
fn etac_is_the_class_code_and_et_still_works() {
    let (mut app, _root) = detach_app(|game_data| {
        arrange(&mut game_data.fleets[0], 1, 2, 0, 0);
        game_data.fleets[0].ships.etacs = 2;
    });
    let terminal = screen(&mut app);
    assert!(line_containing(&terminal, "Class <BB,CA,DD,TT*,TT,SC,ETAC,C,X>").contains("<Q> ->"));
    detach_stage(&mut app, "etac", "1");
    assert!(detach_staged(&mut app).ends_with("ETAC1"));
    detach_stage(&mut app, "et", "1");
    assert!(detach_staged(&mut app).ends_with("ETAC2"));
}

#[test]
fn transfer_takes_the_same_selection_rules() {
    let (mut app, root) = transfer_app(twelve_cruisers);
    let terminal = screen(&mut app);
    assert!(line_containing(&terminal, "Ships: ").contains("Ships: SC1 CA12!3"));
    transfer_stage(&mut app, "ca", "3!");
    let terminal = screen(&mut app);
    assert!(
        line_containing(&terminal, "Staged to Transfer: ").contains("CA3!3"),
        "{:#?}",
        terminal.lines
    );
    assert!(line_containing(&terminal, "Remaining on Source: ").contains("SC1 CA9"));
    assert!(line_containing(&terminal, "Destination After Transfer: ").contains("CA3!3 DD2"));
    // All three crippled cruisers are staged: another `1!` is rejected.
    transfer_stage(&mut app, "ca", "1!");
    assert_eq!(
        app.fleet.transfer_status.as_deref(),
        Some("No crippled CA left to stage.")
    );
    apply_action(&mut app, Action::Fleet(FleetAction::CancelTransfer));
    enter_transfer_input(&mut app, "c");
    submit_transfer(&mut app);
    let state = latest_runtime_state(&root).game_data;
    assert_eq!(nc_data::fleet_notation(&state.fleets[0]), "SC1 CA9");
    assert_eq!(nc_data::fleet_notation(&state.fleets[1]), "CA3!3 DD2");
    assert_eq!(state.fleets[1].crippled.cruisers, hulls(1, 1, 1));
}

#[test]
fn transfer_plain_quantity_and_rejection_leave_the_state_alone() {
    let (mut app, root) = transfer_app(|game_data| {
        twelve_cruisers(game_data);
        game_data.fleets[0].crippled.cruisers = hulls(0, 0, 0);
    });
    let before = latest_runtime_state(&root).game_data;
    transfer_stage(&mut app, "ca", "1!");
    assert_eq!(
        app.fleet.transfer_status.as_deref(),
        Some("No crippled CA left to stage.")
    );
    assert_eq!(
        app.fleet.transfer_selection,
        nc_data::FleetDetachSelection::default()
    );
    assert_eq!(latest_runtime_state(&root).game_data, before);
    apply_action(&mut app, Action::Fleet(FleetAction::CancelTransfer));
    // `3` stages healthy cruisers.
    transfer_stage(&mut app, "ca", "3");
    let terminal = screen(&mut app);
    assert!(line_containing(&terminal, "Staged to Transfer: ").contains("CA3"));
    assert!(!line_containing(&terminal, "Staged to Transfer: ").contains('!'));
}

#[test]
fn damage_changes_no_row_and_no_prompt_position() {
    let mut healthy = detach_app(|game_data| {
        twelve_cruisers(game_data);
        game_data.fleets[0].crippled.cruisers = hulls(0, 0, 0);
    });
    let mut damaged = detach_app(twelve_cruisers);
    let (healthy, damaged) = (screen(&mut healthy.0), screen(&mut damaged.0));
    assert_eq!(healthy.lines.len(), 25);
    assert_eq!(damaged.lines.len(), 25);
    let prompt_row = |terminal: &CaptureTerminal| {
        terminal
            .lines
            .iter()
            .position(|line| line.contains("Class <BB,CA,DD,TT*,TT,SC,ETAC,C,X>"))
            .expect("class prompt")
    };
    assert_eq!(prompt_row(&healthy), prompt_row(&damaged));
    for (row, (a, b)) in healthy.lines.iter().zip(&damaged.lines).enumerate() {
        assert!(
            a.chars().count() <= 80 && b.chars().count() <= 80,
            "row {row}"
        );
        if !a.contains("Ships: ") {
            assert_eq!(a, b, "row {row}");
        }
    }
}

#[test]
fn the_category_helpers_agree_on_every_door_class() {
    // The door's class codes and the shared categories are one list.
    for category in ShipCategory::ALL {
        assert_eq!(ShipCategory::from_code(category.code()), Some(category));
    }
}

fn help_screen(mut app: App) -> CaptureTerminal {
    apply_action(&mut app, Action::OpenPopupHelp);
    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal).expect("help renders");
    assert_eq!(terminal.lines.len(), 25);
    terminal
}

#[test]
fn the_fleet_list_help_repeats_the_notation_legend() {
    let (mut app, _root) = app_with(twelve_cruisers);
    app.current_screen = ScreenId::FleetList;
    let terminal = help_screen(app);
    assert!(line_containing(&terminal, "CA12!3 = 12 cruisers, 3 crippled").contains("Ships"));
    assert!(line_containing(&terminal, "TT* = loaded transports").contains("Ships"));
    assert!(line_containing(&terminal, "cut-short Ships cell").contains("+!"));
}

#[test]
fn the_detach_and_transfer_help_explain_n_bang_and_the_notation() {
    let (detach, _detach_root) = detach_app(twelve_cruisers);
    let (transfer, _transfer_root) = transfer_app(twelve_cruisers);
    for app in [detach, transfer] {
        let terminal = help_screen(app);
        assert!(line_containing(&terminal, "stage exactly three crippled ships").contains("3!"));
        assert!(line_containing(&terminal, "healthy ones first").contains('3'));
        assert!(line_containing(&terminal, "CA3!1 = 3 cruisers, 1 crippled").contains("Ships"));
    }
}
