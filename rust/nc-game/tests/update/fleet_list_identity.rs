use crate::support::*;

fn fleet_list_app() -> (App, TempGameDir) {
    let root = temp_game_with_same_sector_fleets_copy();
    let mut app = App::load(AppConfig {
        game_dir: root.clone(),
        player_record_index_1_based: 1,
        export_root: None,
        queue_dir: None,
        session_timeout_secs: None,
        game_config: Default::default(),
    })
    .expect("load app");
    for (index, fleet) in app
        .game_data
        .fleets
        .iter_mut()
        .filter(|f| f.owner.0 == 1)
        .enumerate()
    {
        fleet.fleet_number.0 = if index == 0 { 1 } else { 10 + index as u16 };
        fleet.ships.destroyers = 4;
    }
    advance_to_main_menu(&mut app);
    app.open_fleet_list();
    (app, root)
}

#[test]
fn typed_exact_id_highlights_the_same_fleet_that_detach_opens() {
    let (mut app, _root) = fleet_list_app();
    apply_action(&mut app, Action::Fleet(FleetAction::AppendListChar('1')));
    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal).expect("render list");
    assert!(line_containing(&terminal, "COMMAND <-").contains("[01]"));
    apply_action(&mut app, Action::Fleet(FleetAction::OpenDetach));
    app.render(&mut terminal).expect("render detach");
    assert_eq!(terminal.line(2).trim(), "Fleet: Fleet #1");
}

#[test]
fn moving_after_a_typed_id_detaches_from_the_new_highlight() {
    let (mut app, _root) = fleet_list_app();
    apply_action(&mut app, Action::Fleet(FleetAction::AppendListChar('1')));
    // Default descending ID order: the row above Fleet 1 is Fleet 11.
    apply_action(&mut app, Action::Fleet(FleetAction::MoveList(-1)));
    assert!(app.fleet.list_input.is_empty());
    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal).expect("render moved list");
    assert!(line_containing(&terminal, "COMMAND <-").contains("[11]"));
    apply_action(&mut app, Action::Fleet(FleetAction::OpenDetach));
    app.render(&mut terminal).expect("render detach");
    assert_eq!(terminal.line(2).trim(), "Fleet: Fleet #11");
}
