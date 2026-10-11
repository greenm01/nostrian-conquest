//! Crippled hulls through the fleet operations the player has (t066):
//! detaching and transferring ships keep every counter, taking the crippled
//! hulls of a class first and the ones with the most turns left before the
//! rest, so the fleet that stays is closest to full strength.

use std::path::PathBuf;

use nc_compat::load_classic_core_files;
use nc_data::{
    Coords, CoreGameData, CrippledHulls, CrippledShips, Fleet, FleetDetachSelection, Order,
    recompute_max_speed, set_armies_aboard, set_mission_target, set_troop_transports,
};

fn hulls(turns_1: u32, turns_2: u32, fresh: u32) -> CrippledHulls {
    CrippledHulls {
        turns: [turns_1, turns_2],
        fresh,
    }
}

fn load_fixture(name: &str) -> CoreGameData {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures")
        .join(name)
        .join("v1.5");
    load_classic_core_files(&dir).unwrap_or_else(|e| panic!("load fixture {name}: {e}"))
}

fn set_ships(fleet: &mut Fleet, destroyers: u16, transports: u16) {
    fleet.ships.battleships = 0;
    fleet.ships.cruisers = 0;
    fleet.ships.destroyers = destroyers;
    set_troop_transports(fleet, transports);
    set_armies_aboard(fleet, 0);
    fleet.ships.scouts = 0;
    fleet.ships.etacs = 0;
    recompute_max_speed(fleet);
    fleet.speed = 0;
    fleet.sector = Coords::new(16, 13);
    fleet.order = Order::HoldPosition;
    fleet.target = Coords::new(16, 13);
    set_mission_target(fleet, 0, 0);
}

/// Empire 1's fleet #1 holds five destroyers, three of them crippled: one
/// this pass, one with two turns left, one with one. Fleet #2 holds a
/// destroyer and a transport.
fn setup() -> CoreGameData {
    let mut game_data = load_fixture("ecmaint-post");
    set_ships(&mut game_data.fleets[0], 5, 2);
    set_ships(&mut game_data.fleets[1], 1, 1);
    game_data.fleets[0].crippled.destroyers = hulls(1, 1, 1);
    game_data
}

fn destroyers(count: u16) -> FleetDetachSelection {
    FleetDetachSelection {
        destroyers: count,
        ..FleetDetachSelection::default()
    }
}

fn total(game_data: &CoreGameData) -> CrippledShips {
    let mut sum = CrippledShips::default();
    for fleet in &game_data.fleets {
        sum.add(&fleet.crippled);
    }
    sum
}

#[test]
fn a_detachment_takes_the_crippled_hulls_first_this_pass_and_longest_first() {
    let mut game_data = setup();
    let before = total(&game_data);
    let result = game_data
        .detach_ships_to_new_fleet(1, 1, destroyers(2), None, 6)
        .expect("detach");
    let new_fleet = &game_data.fleets[result.new_fleet_record_index_1_based - 1];
    assert_eq!(new_fleet.ships.destroyers, 2);
    assert_eq!(new_fleet.crippled.destroyers, hulls(0, 1, 1));
    let donor = &game_data.fleets[result.donor_fleet_record_index_1_based - 1];
    assert_eq!(donor.ships.destroyers, 3);
    assert_eq!(donor.crippled.destroyers, hulls(1, 0, 0));
    assert_eq!(total(&game_data), before, "nothing reset, sped up, or lost");
}

#[test]
fn a_larger_detachment_takes_every_cripple_and_then_nominal_hulls() {
    let mut game_data = setup();
    let result = game_data
        .detach_ships_to_new_fleet(1, 1, destroyers(4), None, 6)
        .expect("detach");
    let new_fleet = &game_data.fleets[result.new_fleet_record_index_1_based - 1];
    assert_eq!(new_fleet.crippled.destroyers, hulls(1, 1, 1));
    assert_eq!(new_fleet.ships.destroyers, 4);
    let donor = &game_data.fleets[result.donor_fleet_record_index_1_based - 1];
    assert_eq!(donor.ships.destroyers, 1);
    assert!(donor.crippled.destroyers.is_empty());
}

#[test]
fn a_detachment_of_other_classes_leaves_the_cripples_alone() {
    let mut game_data = setup();
    let transports = FleetDetachSelection {
        empty_transports: 1,
        ..FleetDetachSelection::default()
    };
    game_data
        .detach_ships_to_new_fleet(1, 1, transports, None, 0)
        .expect("detach");
    assert_eq!(game_data.fleets[0].crippled.destroyers, hulls(1, 1, 1));
}

#[test]
fn full_and_empty_transports_share_one_crippled_class() {
    let mut game_data = setup();
    game_data.fleets[0].crippled.transports = hulls(0, 2, 0);
    set_armies_aboard(&mut game_data.fleets[0], 1);
    let selection = FleetDetachSelection {
        full_transports: 1,
        empty_transports: 1,
        ..FleetDetachSelection::default()
    };
    let before = total(&game_data);
    // Moving every transport but leaving destroyers behind is allowed.
    let result = game_data
        .detach_ships_to_new_fleet(1, 1, selection, None, 0)
        .expect("detach");
    let new_fleet = &game_data.fleets[result.new_fleet_record_index_1_based - 1];
    assert_eq!(new_fleet.crippled.transports, hulls(0, 2, 0));
    assert_eq!(total(&game_data), before);
}

#[test]
fn a_transfer_adds_the_counts_to_the_host() {
    let mut game_data = setup();
    game_data.fleets[1].crippled.destroyers = hulls(0, 0, 1);
    let before = total(&game_data);
    game_data
        .transfer_ships_between_fleets(1, 1, 2, destroyers(2))
        .expect("transfer");
    // The two hulls with the most time go: this pass's and the one with two
    // turns left. The host keeps its own hull and gains both.
    assert_eq!(game_data.fleets[0].crippled.destroyers, hulls(1, 0, 0));
    assert_eq!(game_data.fleets[1].crippled.destroyers, hulls(0, 1, 2));
    assert_eq!(game_data.fleets[1].ships.destroyers, 3);
    assert_eq!(total(&game_data), before);
}

#[test]
fn splitting_and_transferring_back_restores_every_counter() {
    let mut game_data = setup();
    let original = game_data.fleets[0].crippled;
    let selection = FleetDetachSelection {
        destroyers: 3,
        empty_transports: 1,
        ..FleetDetachSelection::default()
    };
    let result = game_data
        .detach_ships_to_new_fleet(1, 1, selection, None, 6)
        .expect("detach");
    let donor_number = game_data.fleets[result.donor_fleet_record_index_1_based - 1]
        .fleet_number
        .0;
    let new_number = game_data.fleets[result.new_fleet_record_index_1_based - 1]
        .fleet_number
        .0;
    let index_of = |game_data: &CoreGameData, number: u16| {
        game_data
            .fleets
            .iter()
            .position(|fleet| fleet.owner.0 == 1 && fleet.fleet_number.0 == number)
            .expect("fleet")
            + 1
    };
    let (donor, new) = (
        index_of(&game_data, donor_number),
        index_of(&game_data, new_number),
    );
    game_data
        .transfer_ships_between_fleets(1, new, donor, destroyers(3))
        .expect("transfer back");
    let rejoined = &game_data.fleets[index_of(&game_data, donor_number) - 1];
    assert_eq!(rejoined.crippled, original);
    assert_eq!(rejoined.ships.destroyers, 5);
}
