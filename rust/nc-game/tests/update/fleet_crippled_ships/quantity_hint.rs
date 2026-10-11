use super::*;

const HINT: &str = "1! = 1 crippled";

fn choose(app: &mut App, detach: bool, class: &str) {
    if detach {
        enter_detach_input(app, class);
        submit_detach(app);
    } else {
        enter_transfer_input(app, class);
        submit_transfer(app);
    }
}

#[test]
fn quantity_hint_uses_existing_blank_row_only_for_available_crippled_class() {
    for detach in [true, false] {
        let (mut app, _root) = if detach {
            detach_app(twelve_cruisers)
        } else {
            transfer_app(twelve_cruisers)
        };
        let command_row = if detach { 14 } else { 18 };
        let before = screen(&mut app);
        assert!(!before.lines.iter().any(|line| line.contains(HINT)));
        choose(&mut app, detach, "ca");
        let terminal = screen(&mut app);
        assert_eq!(terminal.lines.len(), 25);
        assert!(terminal.lines.iter().all(|line| line.chars().count() <= 80));
        assert!(terminal.lines[command_row - 2].contains("Clear"));
        assert_eq!(terminal.lines[command_row - 1].trim(), HINT);
        assert!(terminal.lines[command_row].contains("CA to stage (max 12) [1] <Q> ->"));
        assert!(!terminal.lines[command_row].contains(HINT));
        assert!(terminal.lines[command_row + 1].trim().is_empty());
        assert!(terminal.lines[command_row + 2].contains("Staged"));

        // Cancel quantity entry and choose a healthy class in the same fleet.
        apply_action(
            &mut app,
            Action::Fleet(if detach {
                FleetAction::CancelDetach
            } else {
                FleetAction::CancelTransfer
            }),
        );
        choose(&mut app, detach, "sc");
        let terminal = screen(&mut app);
        assert!(terminal.lines[command_row - 1].trim().is_empty());
        assert!(!terminal.lines.iter().any(|line| line.contains(HINT)));
    }
}

#[test]
fn staging_all_crippled_hulls_removes_hint_and_confirmation_stays_compact() {
    for detach in [true, false] {
        let (mut app, _root) = if detach {
            detach_app(twelve_cruisers)
        } else {
            transfer_app(twelve_cruisers)
        };
        if detach {
            detach_stage(&mut app, "ca", "3!");
        } else {
            transfer_stage(&mut app, "ca", "3!");
        }
        choose(&mut app, detach, "ca");
        assert!(
            !screen(&mut app)
                .lines
                .iter()
                .any(|line| line.contains(HINT))
        );
        apply_action(
            &mut app,
            Action::Fleet(if detach {
                FleetAction::CancelDetach
            } else {
                FleetAction::CancelTransfer
            }),
        );
        if detach {
            submit_detach(&mut app);
        } else {
            submit_transfer(&mut app);
        }
        let terminal = screen(&mut app);
        let command_row = if detach { 12 } else { 16 };
        assert!(terminal.lines[command_row].contains("these ships? [Y]/N <Q> ->"));
        assert!(terminal.lines[command_row - 1].trim().is_empty());
        assert!(terminal.lines[command_row + 1].trim().is_empty());
        assert!(!terminal.lines.iter().any(|line| line.contains(HINT)));
        assert!(terminal.lines[command_row + 2].contains("Staged"));
        assert!(terminal.lines[command_row + 3].contains("Remaining"));
        if !detach {
            assert!(terminal.lines[command_row + 4].contains("Destination After Transfer"));
        }
    }
}
