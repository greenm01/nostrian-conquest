//! A merged fleet keeps its host's ROE, unless the host had no warships: then
//! it takes the ROE of the warships it took in (owner rule, 2026-11-06).

use nc_data::{Coords, CoreGameData, GameStateBuilder, Order, Real48};
use nc_engine::run_maintenance_turn;

const HOME: [u8; 2] = [4, 4];

fn baseline() -> CoreGameData {
    let mut game_data = GameStateBuilder::new()
        .with_player_count(4)
        .with_year(3000)
        .with_homeworld_coords(vec![HOME, [14, 4], [4, 14], [14, 14]])
        .build_initialized_baseline()
        .expect("baseline should build");
    for player in &mut game_data.players {
        player.autopilot = false;
    }
    // Empire 1's Fleet #1 becomes transports only, on ROE 0; Fleet #2 keeps
    // its warships on ROE 6. Both wait in deep space at home.
    for (idx, roe) in [(0, 0), (1, 6)] {
        let fleet = &mut game_data.fleets[idx];
        fleet.sector = Coords::from_array(HOME);
        fleet.target = Coords::from_array(HOME);
        fleet.depth = Real48::ZERO;
        fleet.target_depth = 0;
        fleet.speed = 0;
        fleet.order = Order::HoldPosition;
        fleet.rules_of_engagement = roe;
    }
    game_data.fleets[0].ships = Default::default();
    game_data.fleets[0].ships.empty_transports = 5;
    game_data
}

fn fleet_number(game_data: &CoreGameData, number: u16) -> Option<&nc_data::Fleet> {
    game_data
        .fleets
        .iter()
        .find(|fleet| fleet.owner.0 == 1 && fleet.fleet_number.0 == number)
}

#[test]
fn warships_joining_a_transport_fleet_keep_their_roe() {
    let mut game_data = baseline();
    game_data.fleets[1].order = Order::JoinAnotherFleet;
    game_data.fleets[1].mission_target = 1;

    let events = run_maintenance_turn(&mut game_data).expect("maintenance should succeed");

    assert_eq!(events.fleet_merge_events.len(), 1);
    let merged = fleet_number(&game_data, 1).expect("Fleet #1 hosts");
    assert!(nc_data::has_combat_ships(merged));
    assert_eq!(merged.rules_of_engagement, 6);
}

#[test]
fn a_rendezvous_hosted_by_a_transport_fleet_keeps_the_warships_roe() {
    let mut game_data = baseline();
    for fleet in &mut game_data.fleets[..2] {
        fleet.order = Order::RendezvousSector;
    }

    run_maintenance_turn(&mut game_data).expect("maintenance should succeed");

    assert!(fleet_number(&game_data, 2).is_none(), "Fleet #2 merged");
    let merged = fleet_number(&game_data, 1).expect("Fleet #1 hosts");
    assert_eq!(merged.rules_of_engagement, 6);
}

#[test]
fn transports_joining_a_battle_fleet_leave_its_roe_alone() {
    let mut game_data = baseline();
    game_data.fleets[0].order = Order::JoinAnotherFleet;
    game_data.fleets[0].mission_target = 2;

    run_maintenance_turn(&mut game_data).expect("maintenance should succeed");

    let merged = fleet_number(&game_data, 2).expect("Fleet #2 hosts");
    assert_eq!(merged.rules_of_engagement, 6);
}
