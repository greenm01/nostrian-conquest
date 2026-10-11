use super::*;

pub(crate) fn open_theme_picker(app: &mut App) {
    assert_eq!(
        apply_action(app, Action::Startup(StartupAction::OpenThemePicker)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::ThemePicker);
}

pub(crate) fn theme_picker_select(app: &mut App, key: &str) {
    let Some(cursor) = app
        .startup_state
        .theme_picker_rows
        .iter()
        .position(|row| row.key == key)
    else {
        panic!("theme picker should contain {key}");
    };
    app.startup_state.theme_picker_cursor = cursor;
}

pub(crate) fn enable_door_mode(app: &mut App) {
    app.door_mode = true;
    app.screen_geometry = nc_game::screen::ScreenGeometry::for_door(Some(24));
    theme::apply_door_theme();
}

pub(crate) fn key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

pub(crate) fn ctrl_key(ch: char) -> KeyEvent {
    KeyEvent::new(KeyCode::Char(ch), KeyModifiers::CONTROL)
}

pub(crate) fn key_with_kind(code: KeyCode, kind: KeyEventKind) -> KeyEvent {
    KeyEvent {
        code,
        modifiers: KeyModifiers::NONE,
        kind,
        state: KeyEventState::NONE,
    }
}

pub(crate) fn advance_to_main_menu(app: &mut App) {
    for _ in 0..64 {
        if app.current_screen() == ScreenId::MainMenu {
            return;
        }
        app.advance_startup();
    }
    panic!("startup did not reach main menu");
}

pub(crate) fn advance_to_first_time_menu(app: &mut App) {
    for _ in 0..16 {
        if app.current_screen() == ScreenId::FirstTimeMenu {
            return;
        }
        app.advance_startup();
    }
    panic!("startup did not reach first-time menu");
}

pub(crate) fn submit_fleet_menu_prompt(app: &mut App, fleet_number: Option<u16>) {
    if let Some(fleet_number) = fleet_number {
        submit_fleet_menu_prompt_value(app, &fleet_number.to_string());
        return;
    }
    assert_eq!(
        apply_action(&mut *app, Action::Fleet(FleetAction::SubmitMenuPrompt)),
        AppOutcome::Continue
    );
}

pub(crate) fn submit_fleet_menu_prompt_value(app: &mut App, value: &str) {
    let starting_mode = app.fleet.menu_prompt_mode;
    for ch in value.chars() {
        assert_eq!(
            apply_action(
                &mut *app,
                Action::Fleet(FleetAction::AppendMenuPromptChar(ch))
            ),
            AppOutcome::Continue
        );
    }
    if starting_mode == Some(nc_game::domains::fleet::state::FleetMenuPromptMode::ChangeField)
        && value.chars().count() == 1
    {
        return;
    }
    assert_eq!(
        apply_action(&mut *app, Action::Fleet(FleetAction::SubmitMenuPrompt)),
        AppOutcome::Continue
    );
}

pub(crate) fn submit_planet_transport_prompt(app: &mut App, value: Option<&str>) {
    if let Some(value) = value {
        submit_planet_transport_prompt_value(app, value);
        return;
    }
    assert_eq!(
        apply_action(
            &mut *app,
            Action::Planet(PlanetAction::SubmitTransportPrompt)
        ),
        AppOutcome::Continue
    );
}

pub(crate) fn submit_planet_transport_prompt_value(app: &mut App, value: &str) {
    for ch in value.chars() {
        assert_eq!(
            apply_action(
                &mut *app,
                Action::Planet(PlanetAction::AppendTransportPromptChar(ch))
            ),
            AppOutcome::Continue
        );
    }
    assert_eq!(
        apply_action(
            &mut *app,
            Action::Planet(PlanetAction::SubmitTransportPrompt)
        ),
        AppOutcome::Continue
    );
}

pub(crate) fn open_review_from_fleet_menu(app: &mut App, fleet_number: Option<u16>) {
    assert_eq!(
        apply_action(app, Action::Fleet(FleetAction::OpenReviewPrompt)),
        AppOutcome::Continue
    );
    submit_fleet_menu_prompt(app, fleet_number);
}

pub(crate) fn open_order_mission_picker_from_fleet_menu(app: &mut App, fleet_number: Option<u16>) {
    assert_eq!(
        apply_action(app, Action::Fleet(FleetAction::OpenOrder)),
        AppOutcome::Continue
    );
    submit_fleet_menu_prompt(app, fleet_number);
    assert_eq!(app.current_screen(), ScreenId::FleetMissionPicker);
}

pub(crate) fn open_change_value_prompt_from_fleet_menu(
    app: &mut App,
    fleet_number: Option<u16>,
    field: char,
) {
    assert_eq!(
        apply_action(app, Action::Fleet(FleetAction::OpenChangePrompt)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FleetMenu);
    submit_fleet_menu_prompt(app, fleet_number);
    assert_eq!(app.current_screen(), ScreenId::FleetMenu);
    submit_fleet_menu_prompt_value(app, &field.to_string());
    assert_eq!(app.current_screen(), ScreenId::FleetMenu);
}

pub(crate) fn open_change_field_prompt_from_fleet_menu(app: &mut App, fleet_number: Option<u16>) {
    assert_eq!(
        apply_action(app, Action::Fleet(FleetAction::OpenChangePrompt)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FleetMenu);
    submit_fleet_menu_prompt(app, fleet_number);
    assert_eq!(app.current_screen(), ScreenId::FleetMenu);
}

pub(crate) fn open_eta_from_fleet_menu(app: &mut App, fleet_number: Option<u16>) {
    assert_eq!(
        apply_action(app, Action::Fleet(FleetAction::OpenEta)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FleetMenu);
    submit_fleet_menu_prompt(app, fleet_number);
    assert_eq!(app.current_screen(), ScreenId::FleetEta);
}

pub(crate) fn open_detach_from_fleet_menu(app: &mut App, fleet_number: Option<u16>) {
    assert_eq!(
        apply_action(app, Action::Fleet(FleetAction::OpenDetach)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FleetMenu);
    submit_fleet_menu_prompt(app, fleet_number);
    assert_eq!(app.current_screen(), ScreenId::FleetDetach);
}

pub(crate) fn enter_detach_input(app: &mut App, input: &str) {
    for ch in input.chars() {
        assert_eq!(
            apply_action(app, Action::Fleet(FleetAction::AppendDetachChar(ch))),
            AppOutcome::Continue
        );
    }
}

pub(crate) fn submit_detach(app: &mut App) {
    assert_eq!(
        apply_action(app, Action::Fleet(FleetAction::SubmitDetach)),
        AppOutcome::Continue
    );
}

pub(crate) fn enter_fleet_order_target(app: &mut App, coords: [u8; 2]) {
    for ch in format!("{:02}", coords[0]).chars() {
        assert_eq!(
            apply_action(app, Action::Fleet(FleetAction::AppendOrderChar(ch))),
            AppOutcome::Continue
        );
    }
    assert_eq!(
        apply_action(app, Action::Fleet(FleetAction::SubmitOrder)),
        AppOutcome::Continue
    );
    for ch in format!("{:02}", coords[1]).chars() {
        assert_eq!(
            apply_action(app, Action::Fleet(FleetAction::AppendOrderChar(ch))),
            AppOutcome::Continue
        );
    }
    assert_eq!(
        apply_action(app, Action::Fleet(FleetAction::SubmitOrder)),
        AppOutcome::Continue
    );
}

pub(crate) fn confirm_fleet_order(app: &mut App, confirm: bool) {
    if !confirm {
        assert_eq!(
            apply_action(app, Action::Fleet(FleetAction::AppendOrderChar('N'))),
            AppOutcome::Continue
        );
    }
    assert_eq!(
        apply_action(app, Action::Fleet(FleetAction::SubmitOrder)),
        AppOutcome::Continue
    );
}

pub(crate) fn enter_fleet_group_order_target(app: &mut App, coords: [u8; 2]) {
    for ch in format!("{:02}", coords[0]).chars() {
        assert_eq!(
            apply_action(app, Action::Fleet(FleetAction::AppendGroupOrderChar(ch))),
            AppOutcome::Continue
        );
    }
    assert_eq!(
        apply_action(&mut *app, Action::Fleet(FleetAction::SubmitGroupOrder)),
        AppOutcome::Continue
    );
    for ch in format!("{:02}", coords[1]).chars() {
        assert_eq!(
            apply_action(app, Action::Fleet(FleetAction::AppendGroupOrderChar(ch))),
            AppOutcome::Continue
        );
    }
    assert_eq!(
        apply_action(&mut *app, Action::Fleet(FleetAction::SubmitGroupOrder)),
        AppOutcome::Continue
    );
}

pub(crate) fn confirm_fleet_group_order(app: &mut App, confirm: bool) {
    if !confirm {
        assert_eq!(
            apply_action(app, Action::Fleet(FleetAction::AppendGroupOrderChar('N'))),
            AppOutcome::Continue
        );
    }
    assert_eq!(
        apply_action(&mut *app, Action::Fleet(FleetAction::SubmitGroupOrder)),
        AppOutcome::Continue
    );
}

/// Finish a staged detach, including its explicit confirmation.
pub(crate) fn confirm_detach(app: &mut App) {
    if app.fleet.detach_mode != nc_game::screen::FleetDetachMode::Confirming {
        enter_detach_input(app, "d");
        submit_detach(app);
    }
    if app.fleet.detach_mode == nc_game::screen::FleetDetachMode::Confirming {
        submit_detach(app);
    }
}
