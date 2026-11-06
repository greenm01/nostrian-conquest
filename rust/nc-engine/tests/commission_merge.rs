use nc_data::{GameStateBuilder, Order};
use nc_engine::{CommissionMerge, commission_merge_hosts};

fn baseline() -> nc_data::CoreGameData {
    GameStateBuilder::new()
        .with_player_count(4)
        .with_year(3000)
        .build_initialized_baseline()
        .expect("baseline should build")
}

// Empire 1 starts with Fleets #1-#4 on Guard/Blockade in orbit of its home
// world; record 4 stands in for a fleet just commissioned there.

#[test]
fn a_new_fleet_merges_into_the_lowest_numbered_fleet_at_rest_in_orbit() {
    let mut game_data = baseline();
    game_data.fleets[0].order = Order::MoveOnly;
    game_data.fleets[0].speed = 3;

    assert_eq!(
        commission_merge_hosts(&game_data, &[4]),
        vec![CommissionMerge {
            fleet_record_index_1_based: 4,
            fleet_number: 4,
            host_record_index_1_based: 2,
            host_fleet_number: 2,
        }]
    );
}

#[test]
fn new_fleets_never_host_each_other() {
    let mut game_data = baseline();
    for fleet in &mut game_data.fleets[..2] {
        fleet.order = Order::MoveOnly;
        fleet.speed = 3;
    }

    assert!(commission_merge_hosts(&game_data, &[3, 4]).is_empty());
}

#[test]
fn a_host_never_takes_in_past_3000_ships() {
    let mut game_data = baseline();
    game_data.fleets[0].ships.destroyers = 2999;

    let merges = commission_merge_hosts(&game_data, &[4]);
    assert_eq!(merges[0].host_fleet_number, 2);
}

#[test]
fn a_fleet_outside_orbit_does_not_host() {
    let mut game_data = baseline();
    for fleet in &mut game_data.fleets[..3] {
        fleet.depth = nc_data::Real48::ZERO;
    }

    assert!(commission_merge_hosts(&game_data, &[4]).is_empty());
}

#[test]
fn a_guard_fleet_on_station_hosts_at_any_speed() {
    let mut game_data = baseline();
    // Fleet #1 flew in on Guard/Blockade and kept its travel speed.
    game_data.fleets[0].speed = 5;
    game_data.fleets[0].target = game_data.fleets[0].sector;

    let merges = commission_merge_hosts(&game_data, &[4]);
    assert_eq!(merges[0].host_fleet_number, 1);
}

#[test]
fn a_guard_fleet_passing_through_on_its_way_elsewhere_does_not_host() {
    let mut game_data = baseline();
    game_data.fleets[0].speed = 5;
    game_data.fleets[0].target = nc_data::Coords::new(1, 1);

    let merges = commission_merge_hosts(&game_data, &[4]);
    assert_eq!(merges[0].host_fleet_number, 2);
}

#[test]
fn a_hold_fleet_still_moving_does_not_host() {
    let mut game_data = baseline();
    game_data.fleets[0].order = Order::HoldPosition;
    game_data.fleets[0].speed = 3;

    let merges = commission_merge_hosts(&game_data, &[4]);
    assert_eq!(merges[0].host_fleet_number, 2);
}
