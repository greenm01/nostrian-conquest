use crate::support::*;

fn log_in(dir: &std::path::Path) -> App {
    let mut app = App::load(AppConfig {
        game_dir: dir.to_path_buf(),
        player_record_index_1_based: 1,
        export_root: None,
        queue_dir: None,
        session_timeout_secs: None,
        game_config: GameConfig::default(),
    })
    .expect("app should load");
    advance_to_main_menu(&mut app);
    app
}

fn run_maintenance(dir: &std::path::Path) {
    let store = CampaignStore::open_default_in_dir(dir).expect("open campaign store");
    let mut maintenance = nc_engine::CampaignMaintenance::load(&store).expect("load campaign");
    // The classic fixture has the inactivity rule off; hosted games default to 3.
    maintenance.game_data.setup.autopilot_after_turns = 3;
    maintenance.run_turn().expect("maintenance should run");
    maintenance.save(&store).expect("save campaign");
}

#[test]
fn a_player_who_logs_in_every_year_never_goes_on_autopilot() {
    let dir = temp_game_copy();
    for _ in 0..6 {
        drop(log_in(&dir));
        run_maintenance(&dir);
    }
    let app = log_in(&dir);
    assert_eq!(app.current_autopilot_flag(), 0);
}

#[test]
fn a_player_who_misses_three_years_goes_on_autopilot() {
    let dir = temp_game_copy();
    drop(log_in(&dir));
    for _ in 0..4 {
        run_maintenance(&dir);
    }
    let state = latest_runtime_state(&dir);
    assert!(state.game_data.players[0].autopilot);
    let told = state
        .report_block_rows
        .iter()
        .filter(|row| row.is_visible_to_viewer(1))
        .filter(|row| row.decoded_text.contains("autopilot has taken charge"))
        .count();
    assert_eq!(told, 1, "the player is told once that autopilot took over");
}
