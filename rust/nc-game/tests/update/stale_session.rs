use crate::support::*;
use nc_data::STALE_SESSION_MESSAGE;

fn load_player_one(fixture_dir: &Path) -> App {
    App::load(AppConfig {
        game_dir: fixture_dir.to_path_buf(),
        player_record_index_1_based: 1,
        export_root: None,
        queue_dir: None,
        session_timeout_secs: None,
        game_config: Default::default(),
    })
    .expect("app should load")
}

fn render(app: &mut App) -> CaptureTerminal {
    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal).expect("screen should render");
    terminal
}

fn notice_is_shown(app: &mut App) -> bool {
    render(app)
        .lines
        .iter()
        .any(|line| line.contains(STALE_SESSION_MESSAGE))
}

/// One maintenance on the stored campaign, as `maint-rust` runs it: it loads
/// year Y and saves year Y + 1.
fn run_maintenance(fixture_dir: &Path) {
    let store = CampaignStore::open_default_in_dir(fixture_dir).expect("open campaign store");
    let mut runtime = latest_runtime_state(fixture_dir);
    nc_engine::run_maintenance_turn(&mut runtime.game_data).expect("maintenance should run");
    store
        .save_runtime_state_structured(
            runtime.game_year,
            &runtime.game_data,
            &runtime.report_block_rows,
            &runtime.queued_mail,
        )
        .expect("maintenance should save the new year");
}

#[test]
fn a_save_after_maintenance_is_refused_and_the_player_is_told() {
    let fixture_dir = temp_game_copy();
    let mut app = load_player_one(&fixture_dir);
    let autopilot_before = app.game_data.players[0].autopilot;

    run_maintenance(&fixture_dir);
    let after_maintenance = latest_runtime_state(&fixture_dir);
    assert_eq!(
        after_maintenance.game_year,
        app.game_data.conquest.game_year + 1
    );

    assert_eq!(
        apply_action(&mut app, Action::ToggleAutopilot),
        AppOutcome::Continue
    );

    assert!(app.session_stale);
    assert_eq!(app.current_screen(), ScreenId::MainMenu);
    assert!(notice_is_shown(&mut app));
    assert_eq!(
        latest_runtime_state(&fixture_dir),
        after_maintenance,
        "the refused save must leave the new year untouched"
    );
    assert_eq!(
        after_maintenance.game_data.players[0].autopilot,
        autopilot_before
    );
}

#[test]
fn a_stale_session_keeps_refusing_without_touching_the_store() {
    let fixture_dir = temp_game_copy();
    let mut app = load_player_one(&fixture_dir);
    run_maintenance(&fixture_dir);
    apply_action(&mut app, Action::ToggleAutopilot);
    let after_maintenance = latest_runtime_state(&fixture_dir);

    // Browsing carries on and shows no notice; trying to save shows it again.
    apply_action(&mut app, Action::OpenGeneralMenu);
    assert_eq!(app.current_screen(), ScreenId::GeneralMenu);
    assert!(!notice_is_shown(&mut app));
    apply_action(&mut app, Action::ToggleAutopilot);

    assert_eq!(app.current_screen(), ScreenId::MainMenu);
    assert!(notice_is_shown(&mut app));
    assert_eq!(latest_runtime_state(&fixture_dir), after_maintenance);
}

#[test]
fn a_save_with_no_maintenance_in_between_goes_through() {
    let fixture_dir = temp_game_copy();
    let mut app = load_player_one(&fixture_dir);
    let autopilot_before = app.game_data.players[0].autopilot;

    apply_action(&mut app, Action::ToggleAutopilot);

    assert!(!app.session_stale);
    assert!(!notice_is_shown(&mut app));
    assert_eq!(
        latest_runtime_state(&fixture_dir).game_data.players[0].autopilot,
        !autopilot_before
    );
}
