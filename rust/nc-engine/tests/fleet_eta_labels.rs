use nc_data::{Order, Real48};
use nc_engine::{FleetEtaEstimate, fleet_list_eta_label, fleet_target_eta_estimate};

/// A fleet in deep space in its home sector, on `order` against that sector.
fn fleet_in_target_sector(order: Order, target_depth: u8) -> nc_data::CoreGameData {
    let mut game_data = nc_engine::build_seeded_new_game(4, 3000, 1515).expect("seeded game");
    let fleet = &mut game_data.fleets[0];
    fleet.target = fleet.sector;
    fleet.depth = Real48::from_int(0);
    fleet.order = order;
    fleet.target_depth = target_depth;
    fleet.speed = fleet.max_speed;
    game_data
}

#[test]
fn a_world_mission_in_the_sector_still_has_to_enter_the_system() {
    let game_data = fleet_in_target_sector(Order::ViewWorld, 1);
    assert_eq!(fleet_list_eta_label(&game_data, 0), "1");
    let target = game_data.fleets[0].target.to_array();
    assert_eq!(
        fleet_target_eta_estimate(&game_data, 1, Order::ViewWorld.code(), target),
        FleetEtaEstimate::Years(1)
    );
}

#[test]
fn a_move_in_the_sector_has_arrived() {
    let game_data = fleet_in_target_sector(Order::MoveOnly, 0);
    assert_eq!(fleet_list_eta_label(&game_data, 0), "0");
}

#[test]
fn every_moving_order_counts_years_to_a_distant_target() {
    for code in 1..=15 {
        let order = Order::from_code(code);
        let mut game_data = fleet_in_target_sector(order, u8::from(order.targets_world()));
        let fleet = &mut game_data.fleets[0];
        let [x, y] = fleet.sector.to_array();
        fleet.target = nc_data::Coords::from_array([x, if y > 6 { y - 6 } else { y + 6 }]);
        let label = fleet_list_eta_label(&game_data, 0);
        assert!(
            label.parse::<u16>().is_ok_and(|years| years > 1),
            "{order:?}: {label}"
        );
    }
}
