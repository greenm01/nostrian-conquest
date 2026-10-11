//! Every Y/N confirmation acts on the default its brackets show: Enter does
//! what the bracketed letter says, and Esc always declines.

use super::*;

fn render_fleet_lines(app: &DashApp) -> Vec<String> {
    let mut buffer = PlayfieldBuffer::new(
        app.geometry.width(),
        app.geometry.height(),
        crate::dashboard::theme::body_style(),
    );
    fleet_list::draw(&mut buffer, app, dashboard_layout(app).widgets.center_map);
    (0..buffer.height())
        .map(|row| buffer.plain_line(row))
        .collect()
}

fn fleet_shows(app: &DashApp, needle: &str) -> bool {
    render_fleet_lines(app)
        .iter()
        .any(|line| line.contains(needle))
}

fn delivered_report(year: u16) -> ReportBlockRow {
    ReportBlockRow {
        viewer_empire_id: 1,
        block_index: 0,
        decoded_text: format!("Stardate: 05/{year}\nFleet contact report"),
        raw_bytes: None,
        recipient_deleted: false,
    }
}

#[test]
fn inbox_delete_prompt_is_yes_by_default_and_escape_declines() {
    let mut app = dash_app();
    let year = app.game_data.conquest.game_year;
    app.report_block_rows = vec![delivered_report(year - 1)];
    app.overlay = ActiveOverlay::Inbox;

    app.handle_key(key(KeyCode::Char('d')));
    assert!(app.inbox_overlay.delete_confirm);
    let lines = render_dashboard_lines(&app);
    assert!(
        lines
            .iter()
            .any(|line| line.contains("Delete this item? [Y]/N"))
    );
    assert!(!lines.iter().any(|line| line.contains("Y/[N]")));

    app.handle_key(key(KeyCode::Esc));
    assert!(!app.inbox_overlay.delete_confirm);
    assert!(!app.report_block_rows[0].recipient_deleted);

    app.handle_key(key(KeyCode::Char('d')));
    app.handle_key(key(KeyCode::Enter));
    assert!(!app.inbox_overlay.delete_confirm);
    assert!(app.report_block_rows[0].recipient_deleted);
}

#[test]
fn planet_list_mass_commission_prompt_is_yes_by_default_and_escape_declines() {
    let mut app = dash_app();
    app.overlay = ActiveOverlay::PlanetList;
    let selected_record = selected_planet_record_index(&app);
    clear_stardock(&mut app, selected_record);
    let other_record = seed_extra_owned_planet_with_stardock(&mut app);
    select_planet_row_by_record(&mut app, selected_record);

    app.handle_key(key(KeyCode::Char('a')));
    assert_eq!(
        app.planet_overlay.prompt_mode,
        PlanetOverlayPromptMode::MassCommissionConfirm
    );
    assert!(
        render_planet_footer_line(&app, "Mass commission?").contains("Mass commission? [Y]/N ->")
    );

    app.handle_key(key(KeyCode::Esc));
    assert_eq!(
        app.planet_overlay.prompt_mode,
        PlanetOverlayPromptMode::None
    );
    assert_eq!(app.game_data.planets[other_record - 1].stardock[0], 1);

    app.handle_key(key(KeyCode::Char('a')));
    app.handle_key(key(KeyCode::Enter));
    assert_eq!(
        app.planet_overlay.prompt_mode,
        PlanetOverlayPromptMode::None
    );
    assert!(
        app.planet_overlay
            .footer_notice
            .as_deref()
            .unwrap_or_default()
            .contains("Mass commissioned")
    );
    assert_eq!(app.game_data.planets[other_record - 1].stardock[0], 0);
}

#[test]
fn owned_planet_mass_commission_prompt_is_yes_by_default_and_escape_declines() {
    let mut app = dash_app();
    clear_owned_stardocks(&mut app);
    let record = seed_extra_owned_planet_with_stardock(&mut app);
    app.open_owned_planet_popup(record);

    app.handle_key(key(KeyCode::Char('a')));
    assert_eq!(
        app.owned_planet_popup.mode,
        OwnedPlanetPopupMode::MassCommissionConfirm
    );
    assert!(
        render_owned_planet_popup_line(&app, "Mass commission?")
            .contains("Mass commission? [Y]/N ->")
    );

    app.handle_key(key(KeyCode::Esc));
    assert_eq!(app.owned_planet_popup.mode, OwnedPlanetPopupMode::Browse);
    assert_eq!(app.game_data.planets[record - 1].stardock[0], 1);

    app.handle_key(key(KeyCode::Char('a')));
    app.handle_key(key(KeyCode::Enter));
    assert_eq!(
        app.owned_planet_popup.mode,
        OwnedPlanetPopupMode::MassCommissionReport
    );
    assert_eq!(app.game_data.planets[record - 1].stardock[0], 0);
}

#[test]
fn scorch_prompts_are_no_by_default_and_escape_declines() {
    let mut app = dash_app();
    let record = selected_planet_record_index(&app);
    app.open_owned_planet_popup(record);

    let modes = [
        OwnedPlanetPopupMode::ScorchConfirm1,
        OwnedPlanetPopupMode::ScorchConfirm2,
        OwnedPlanetPopupMode::ScorchConfirm3,
    ];
    for mode in modes {
        for decline in [KeyCode::Enter, KeyCode::Esc, KeyCode::Char('n')] {
            app.open_owned_planet_scorch_confirm();
            app.owned_planet_popup.mode = mode;
            assert!(
                render_owned_planet_popup_line(&app, "Y/[N]").contains("Y/[N] ->"),
                "{mode:?} reads Y/[N]"
            );

            app.handle_key(key(decline));

            assert_eq!(app.owned_planet_popup.mode, OwnedPlanetPopupMode::Browse);
            assert!(!app.game_data.planets[record - 1].scorch_earth);
        }
    }
}

#[test]
fn quit_to_lobby_prompt_is_no_by_default_and_escape_declines() {
    let mut app = dash_app();

    for decline in [KeyCode::Enter, KeyCode::Esc, KeyCode::Char('n')] {
        app.dispatch_key_event(KeyEvent::new(KeyCode::Char('q'), KeyModifiers::ALT));
        assert_eq!(app.popup, ActivePopup::QuitConfirm);
        assert!(render_dashboard_line(&app, "Quit to Lobby?").contains("Quit to Lobby? Y/[N]"));

        app.dispatch_key_event(key(decline));

        assert_eq!(app.popup, ActivePopup::None);
        assert_eq!(app.take_exit_request(), None);
    }
}

#[test]
fn message_send_and_discard_prompts_are_no_by_default_and_escape_declines() {
    let mut app = dash_app();
    open_inbox_message_body_editor(&mut app, "Hi", "Move");

    for (alt_key, action, label) in [
        ('s', InboxMessageConfirmAction::Send, "Send Message? Y/[N]"),
        (
            'x',
            InboxMessageConfirmAction::Discard,
            "Discard Message? Y/[N]",
        ),
    ] {
        for decline in [KeyCode::Enter, KeyCode::Esc, KeyCode::Char('n')] {
            app.handle_key(KeyEvent::new(KeyCode::Char(alt_key), KeyModifiers::ALT));
            assert_eq!(app.popup, ActivePopup::InboxMessageConfirm { action });
            assert!(render_dashboard_line(&app, label).contains(label));

            app.handle_key(key(decline));

            assert_eq!(app.popup, ActivePopup::None);
            assert_eq!(app.inbox_overlay.prompt_mode, InboxPromptMode::ComposeBody);
            assert_eq!(app.inbox_overlay.compose_body, "Move");
            assert!(app.hosted_turn_draft.as_ref().unwrap().messages.is_empty());
        }
    }
}

/// The inbox's footer-line send prompt (`ComposeConfirm`) is no longer opened
/// by any key, but its handler still follows the label.
#[test]
fn compose_confirm_prompt_is_no_by_default_and_escape_declines() {
    let mut app = dash_app();
    open_inbox_message_body_editor(&mut app, "Hi", "Move");

    for decline in [KeyCode::Enter, KeyCode::Esc, KeyCode::Char('n')] {
        app.inbox_overlay.prompt_mode = InboxPromptMode::ComposeConfirm;

        app.handle_key(key(decline));

        assert_eq!(app.inbox_overlay.prompt_mode, InboxPromptMode::ComposeBody);
        assert!(app.hosted_turn_draft.as_ref().unwrap().messages.is_empty());
    }

    app.inbox_overlay.prompt_mode = InboxPromptMode::ComposeConfirm;
    app.handle_key(key(KeyCode::Char('y')));
    assert_eq!(app.hosted_turn_draft.as_ref().unwrap().messages.len(), 1);
}

#[test]
fn checked_merge_prompt_is_yes_by_default_and_escape_declines() {
    let mut app = dash_app_with_store();
    app.overlay = ActiveOverlay::FleetList;
    let records = select_first_two_fleet_rows(&mut app);

    app.handle_key(key(KeyCode::Char('m')));
    assert_eq!(
        app.fleet_overlay.prompt_mode,
        FleetOverlayPromptMode::MergeConfirm
    );
    assert!(fleet_shows(&app, "Merge checked fleets? [Y]/N <ESC> ->"));

    app.handle_key(key(KeyCode::Esc));
    assert_eq!(app.fleet_overlay.prompt_mode, FleetOverlayPromptMode::None);
    assert!(
        records
            .iter()
            .all(|record| app.game_data.fleets[*record - 1].order != Order::JoinAnotherFleet)
    );

    app.handle_key(key(KeyCode::Char('m')));
    app.handle_key(key(KeyCode::Enter));
    assert_eq!(app.fleet_overlay.prompt_mode, FleetOverlayPromptMode::None);
    assert!(
        records
            .iter()
            .any(|record| app.game_data.fleets[*record - 1].order == Order::JoinAnotherFleet)
    );
}

#[test]
fn fleet_order_confirm_prompt_is_yes_by_default_and_escape_declines() {
    let mut app = dash_app_with_store();
    app.overlay = ActiveOverlay::FleetList;
    let ready_to_confirm = |app: &mut DashApp| {
        app.open_selected_fleet_order_flow();
        let record = app
            .selected_fleet_order_row()
            .expect("selected fleet row")
            .fleet_record_index_1_based;
        app.fleet_overlay.mission_picker_input = Order::MoveOnly.code().to_string();
        app.submit_fleet_mission_picker();
        app.fleet_overlay.order_target_x_input = "10".to_string();
        app.submit_fleet_order().expect("submit x");
        app.fleet_overlay.order_target_y_input = "10".to_string();
        app.submit_fleet_order().expect("submit y");
        assert_eq!(
            app.fleet_overlay.prompt_mode,
            FleetOverlayPromptMode::OrderConfirm
        );
        record
    };

    let record = ready_to_confirm(&mut app);
    assert!(fleet_shows(&app, "Confirm [Y]/N <ESC> ->"));
    app.handle_key(key(KeyCode::Esc));
    assert_eq!(
        app.fleet_overlay.prompt_mode,
        FleetOverlayPromptMode::OrderTargetY
    );
    assert_ne!(app.game_data.fleets[record - 1].order, Order::MoveOnly);

    let record = ready_to_confirm(&mut app);
    app.handle_key(key(KeyCode::Enter));
    assert_eq!(app.fleet_overlay.prompt_mode, FleetOverlayPromptMode::None);
    assert_eq!(app.game_data.fleets[record - 1].order, Order::MoveOnly);
    assert_eq!(app.game_data.fleets[record - 1].target.to_array(), [10, 10]);
}

#[test]
fn seek_next_closest_prompt_is_yes_by_default_and_escape_declines() {
    let (mut app, record) = seek_home_from_the_home_system(true);
    assert_eq!(
        app.fleet_overlay.prompt_mode,
        FleetOverlayPromptMode::SeekNextClosest
    );

    app.handle_key(key(KeyCode::Esc));
    assert_eq!(
        app.fleet_overlay.prompt_mode,
        FleetOverlayPromptMode::MissionPicker
    );
    assert_ne!(app.game_data.fleets[record - 1].order, Order::SeekHome);

    // Enter answers yes, which the seek-home test above follows through to the
    // order; here it only needs to take the next-closest branch.
    let (mut app, _) = seek_home_from_the_home_system(true);
    assert!(fleet_shows(
        &app,
        "seek the next closest system? [Y]/N <ESC> ->"
    ));
    app.handle_key(key(KeyCode::Enter));
    assert_eq!(
        app.fleet_overlay.prompt_mode,
        FleetOverlayPromptMode::OrderConfirm
    );
    assert_eq!(app.fleet_overlay.order_target_x_input, "08");
    assert_eq!(app.fleet_overlay.order_target_y_input, "08");
}

/// The fleet list lists no starbase rows, so nothing opens this prompt in the
/// running client; the test sets up the state a starbase row would.
#[test]
fn starbase_halt_prompt_is_yes_by_default_and_escape_declines() {
    let mut app = dash_app_with_starbase_store();
    app.overlay = ActiveOverlay::FleetList;
    let base = nc_engine::owned_starbase_targets(&app.game_data, 1)[0].base_record_index_1_based;
    let ask_to_halt = |app: &mut DashApp| {
        app.fleet_overlay.order_scope = FleetOrderScope::StarbaseMove;
        app.fleet_overlay.active_row_key = Some(FleetOverlayRowKey::Starbase(base));
        app.fleet_overlay
            .open_prompt(FleetOverlayPromptMode::StarbaseHaltConfirm);
    };
    let halted_reports = |app: &DashApp| {
        app.report_block_rows
            .iter()
            .filter(|row| row.decoded_text.contains("halted at"))
            .count()
    };

    ask_to_halt(&mut app);
    assert!(fleet_shows(&app, "Halt this starbase? [Y]/N <ESC> ->"));
    app.handle_key(key(KeyCode::Esc));
    assert_eq!(app.fleet_overlay.prompt_mode, FleetOverlayPromptMode::None);
    assert_eq!(halted_reports(&app), 0);

    ask_to_halt(&mut app);
    app.handle_key(key(KeyCode::Enter));
    assert_eq!(app.fleet_overlay.prompt_mode, FleetOverlayPromptMode::None);
    assert_eq!(halted_reports(&app), 1);
}
