use super::*;

fn press(app: &mut App, code: KeyCode) {
    let action = app.handle_key(KeyEvent::new(code, KeyModifiers::NONE));
    assert_eq!(apply_action(app, action), AppOutcome::Continue);
}

#[test]
fn last_stage_confirms_loaded_crippled_transport_and_enter_transfers() {
    let (mut app, root) = transfer_app(|data| {
        twelve_cruisers(data);
        arrange(&mut data.fleets[0], 1, 0, 1, 0);
        data.fleets[0].crippled = Default::default();
        data.fleets[0].crippled.loaded_transports = hulls(0, 1, 0);
    });
    for code in [
        KeyCode::Char('t'),
        KeyCode::Char('t'),
        KeyCode::Char('*'),
        KeyCode::Enter,
        KeyCode::Char('1'),
        KeyCode::Char('!'),
        KeyCode::Enter,
    ] {
        press(&mut app, code);
    }
    let terminal = screen(&mut app);
    assert!(terminal.line(14).contains("Ships: DD2"));
    assert!(terminal.line(15).trim().is_empty());
    assert!(
        terminal
            .line(16)
            .contains("Transfer these ships? [Y]/N <Q> ->")
    );
    assert!(terminal.line(17).trim().is_empty());
    assert!(terminal.line(18).contains("Staged to Transfer: TT*1!1"));
    // CaptureTerminal::line(19) aliases the legacy command row.
    assert!(terminal.lines[19].contains("Remaining on Source: SC1"));
    assert!(
        terminal
            .line(20)
            .contains("Destination After Transfer: DD2 TT*1!1")
    );
    assert!(
        !terminal
            .lines
            .iter()
            .any(|line| line.contains("<T> Transfer") || line.contains("Enter a ship code"))
    );
    let before = latest_runtime_state(&root).game_data;
    assert_eq!(before.fleets[0].ships.loaded_transports, 1);
    press(&mut app, KeyCode::Enter);
    let after = latest_runtime_state(&root).game_data;
    assert_eq!(after.fleets.len(), before.fleets.len());
    assert_eq!(nc_data::fleet_notation(&after.fleets[0]), "SC1");
    assert_eq!(nc_data::fleet_notation(&after.fleets[1]), "DD2 TT*1!1");
    assert_eq!(after.fleets[1].crippled.loaded_transports, hulls(0, 1, 0));
}

#[test]
fn early_confirmation_declines_clears_and_cancels_without_moving_ships() {
    let (mut app, root) = transfer_app(twelve_cruisers);
    press(&mut app, KeyCode::Enter);
    assert_eq!(
        app.fleet.transfer_status.as_deref(),
        Some("Stage at least one ship before transferring.")
    );
    transfer_stage(&mut app, "ca", "3!");
    press(&mut app, KeyCode::Char('t'));
    press(&mut app, KeyCode::Enter);
    assert!(line_containing(&screen(&mut app), "Transfer these ships?").contains("[Y]/N"));
    press(&mut app, KeyCode::Char('n'));
    assert!(line_containing(&screen(&mut app), "Class <SC,CA>").contains("<Q>"));
    assert!(line_containing(&screen(&mut app), "Staged to Transfer:").contains("CA3!3"));
    press(&mut app, KeyCode::Char('x'));
    assert!(line_containing(&screen(&mut app), "Staged to Transfer:").contains("none"));
    transfer_stage(&mut app, "ca", "2");
    press(&mut app, KeyCode::Enter);
    press(&mut app, KeyCode::Char('q'));
    assert_ne!(app.current_screen(), ScreenId::FleetTransfer);
    let after = latest_runtime_state(&root).game_data;
    assert_eq!(nc_data::fleet_notation(&after.fleets[0]), "SC1 CA12!3");
    assert_eq!(nc_data::fleet_notation(&after.fleets[1]), "DD2");
}

#[test]
fn y_accepts_an_early_transfer_confirmation() {
    let (mut app, root) = transfer_app(twelve_cruisers);
    transfer_stage(&mut app, "ca", "1");
    press(&mut app, KeyCode::Enter);
    press(&mut app, KeyCode::Char('y'));
    let after = latest_runtime_state(&root).game_data;
    assert_eq!(nc_data::fleet_notation(&after.fleets[0]), "SC1 CA11!3");
    assert_eq!(nc_data::fleet_notation(&after.fleets[1]), "CA1 DD2");
}
