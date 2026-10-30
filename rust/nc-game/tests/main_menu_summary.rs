use nc_data::GameStateBuilder;
use nc_game::model::MainMenuSummary;

/// Maintenance sets the classic message pointers when only reports wait,
/// for ECGAME's login. Without mail, the client must not offer messages.
#[test]
fn reports_alone_do_not_make_messages_pending() {
    let mut game_data = GameStateBuilder::new()
        .with_player_count(4)
        .with_year(3000)
        .build_initialized_baseline()
        .expect("baseline should build");
    nc_data::set_login_reviewables_present(&mut game_data.players[0], true);

    let summary = MainMenuSummary::from_game_data(&game_data, 1, true, false);
    assert!(summary.pending_results);
    assert!(!summary.pending_messages);

    let with_mail = MainMenuSummary::from_game_data(&game_data, 1, true, true);
    assert!(with_mail.pending_messages);
}
