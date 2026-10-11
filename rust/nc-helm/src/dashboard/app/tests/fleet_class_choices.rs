use super::*;

#[test]
fn transfer_class_prompt_tracks_available_donor_classes() {
    let mut app = dash_app();
    app.fleet_overlay.transfer_donor_record_index_1_based = Some(1);
    app.fleet_overlay.transfer_mode =
        crate::dashboard::app::state::FleetOverlayTransferMode::ChoosingClass;
    let fleet = &mut app.game_data.fleets[0];
    fleet.ships = Default::default();
    fleet.ships.battleships = 1;
    fleet.ships.destroyers = 1;
    assert_eq!(app.fleet_transfer_prompt_and_default().0, "Class <BB,DD> ");
    app.fleet_overlay.transfer_selection.battleships = 1;
    assert_eq!(app.fleet_transfer_prompt_and_default().0, "Class <DD> ");
    app.fleet_overlay.transfer_selection.destroyers = 1;
    assert_eq!(
        app.fleet_transfer_prompt_and_default().0,
        "No more ships to stage "
    );
    app.fleet_overlay.transfer_selection = Default::default();
    assert_eq!(app.fleet_transfer_prompt_and_default().0, "Class <BB,DD> ");
}
