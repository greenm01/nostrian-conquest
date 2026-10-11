use super::*;
use nc_data::STALE_SESSION_MESSAGE;

fn latest(store: &CampaignStore) -> nc_data::CampaignRuntimeState {
    store
        .load_latest_runtime_state()
        .expect("load runtime state")
        .expect("a snapshot exists")
}

/// One maintenance run: load year Y, advance, save year Y + 1.
fn run_maintenance(store: &CampaignStore) {
    let mut state = latest(store);
    nc_engine::run_maintenance_turn(&mut state.game_data).expect("maintenance should run");
    store
        .save_runtime_state_structured(
            state.game_year,
            &state.game_data,
            &state.planet_scorch_orders,
            &state.report_block_rows,
            &state.queued_mail,
        )
        .expect("maintenance should save the new year");
}

fn enter_tax_rate(app: &mut DashApp, digits: &str) {
    app.handle_key(KeyEvent::new(KeyCode::Char('x'), KeyModifiers::NONE));
    for digit in digits.chars() {
        app.handle_key(KeyEvent::new(KeyCode::Char(digit), KeyModifiers::NONE));
    }
    app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
}

#[test]
fn a_save_after_maintenance_shows_the_message_and_leaves_the_new_year_untouched() {
    let mut app = dash_app_with_store();
    let store = app.campaign_store.clone().expect("store");

    run_maintenance(&store);
    let after_maintenance = latest(&store);
    assert_eq!(
        after_maintenance.game_year,
        app.game_data.conquest.game_year + 1
    );

    enter_tax_rate(&mut app, "42");

    assert_eq!(app.popup, ActivePopup::TaxPrompt);
    assert_eq!(
        app.tax_prompt_status.as_deref(),
        Some(STALE_SESSION_MESSAGE)
    );
    assert_eq!(latest(&store), after_maintenance);

    // Every later save is refused the same way.
    app.tax_prompt_status = None;
    app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
    assert_eq!(
        app.tax_prompt_status.as_deref(),
        Some(STALE_SESSION_MESSAGE)
    );
    assert_eq!(latest(&store), after_maintenance);
}

#[test]
fn a_save_with_no_maintenance_in_between_goes_through() {
    let mut app = dash_app_with_store();
    let store = app.campaign_store.clone().expect("store");

    enter_tax_rate(&mut app, "42");

    assert_eq!(app.popup, ActivePopup::None);
    assert_eq!(latest(&store).game_data.players[0].tax_rate, 42);
}
