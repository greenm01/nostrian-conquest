//! Crippled hulls through the fleet operations the player has (t066, t093):
//! detaching and transferring ships keep every counter. A plain quantity
//! takes healthy ships first; `n!` takes exactly `n` crippled ones; among
//! crippled hulls, this pass's newly crippled go first, then the most turns
//! left.

use std::path::PathBuf;

use nc_compat::load_classic_core_files;
use nc_data::{
    Coords, CoreGameData, CrippledHulls, CrippledShips, Fleet, FleetDetachResult,
    FleetDetachSelection, GameStateMutationError, Order, ShipCategory, crippled_left_to_stage,
    fleet_notation, recompute_max_speed, resolve_selection, set_armies_aboard, set_mission_target,
    set_troop_transports,
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

/// `count` destroyers, `crippled` of them required to be crippled (`n!`).
fn crippled_destroyers(count: u16, crippled: u16) -> FleetDetachSelection {
    let mut selection = FleetDetachSelection::default();
    selection.stage(ShipCategory::Destroyers, count, false);
    selection.required_crippled[ShipCategory::Destroyers.index()] = crippled;
    selection
}

fn new_fleet_of(game_data: &CoreGameData, result: &FleetDetachResult) -> Fleet {
    game_data.fleets[result.new_fleet_record_index_1_based - 1].clone()
}

#[test]
fn a_plain_detachment_takes_healthy_hulls_first_and_leaves_the_cripples() {
    let mut game_data = setup();
    let before = total(&game_data);
    let result = game_data
        .detach_ships_to_new_fleet(1, 1, destroyers(2), None, 6)
        .expect("detach");
    let new_fleet = new_fleet_of(&game_data, &result);
    assert_eq!(new_fleet.ships.destroyers, 2);
    assert!(new_fleet.crippled.destroyers.is_empty());
    assert_eq!(fleet_notation(&new_fleet), "DD2");
    let donor = &game_data.fleets[result.donor_fleet_record_index_1_based - 1];
    assert_eq!(donor.ships.destroyers, 3);
    assert_eq!(donor.crippled.destroyers, hulls(1, 1, 1));
    assert_eq!(fleet_notation(donor), "DD3!3 TT2");
    assert_eq!(total(&game_data), before, "nothing reset, sped up, or lost");
}

#[test]
fn a_larger_plain_detachment_then_takes_cripples_this_pass_first_and_longest_first() {
    let mut game_data = setup();
    let result = game_data
        .detach_ships_to_new_fleet(1, 1, destroyers(4), None, 6)
        .expect("detach");
    let new_fleet = new_fleet_of(&game_data, &result);
    // Both healthy hulls and two cripples: this pass's, then the one with
    // two turns left. The hull one pass from whole stays.
    assert_eq!(new_fleet.ships.destroyers, 4);
    assert_eq!(new_fleet.crippled.destroyers, hulls(0, 1, 1));
    assert_eq!(fleet_notation(&new_fleet), "DD4!2");
    let donor = &game_data.fleets[result.donor_fleet_record_index_1_based - 1];
    assert_eq!(donor.ships.destroyers, 1);
    assert_eq!(donor.crippled.destroyers, hulls(1, 0, 0));
}

#[test]
fn n_bang_takes_exactly_n_crippled_hulls_and_never_healthy_ones() {
    let mut game_data = setup();
    let before = total(&game_data);
    let result = game_data
        .detach_ships_to_new_fleet(1, 1, crippled_destroyers(2, 2), None, 6)
        .expect("detach");
    let new_fleet = new_fleet_of(&game_data, &result);
    assert_eq!(new_fleet.ships.destroyers, 2);
    assert_eq!(new_fleet.crippled.destroyers, hulls(0, 1, 1));
    assert_eq!(fleet_notation(&new_fleet), "DD2!2");
    let donor = &game_data.fleets[result.donor_fleet_record_index_1_based - 1];
    assert_eq!(donor.crippled.destroyers, hulls(1, 0, 0));
    assert_eq!(fleet_notation(donor), "DD3!1 TT2");
    assert_eq!(total(&game_data), before);
}

#[test]
fn a_crippled_request_beyond_the_crippled_hulls_changes_nothing() {
    let mut game_data = setup();
    let before = game_data.clone();
    // Three crippled, so `4!` asks for one that does not exist; the two
    // healthy hulls are not substituted.
    let err = game_data
        .detach_ships_to_new_fleet(1, 1, crippled_destroyers(4, 4), None, 6)
        .expect_err("only three destroyers are crippled");
    assert!(matches!(
        err,
        GameStateMutationError::FleetDetachCrippledExceedsAvailable {
            requested: 4,
            available: 3,
            ..
        }
    ));
    assert_eq!(
        err.to_string(),
        "fleet 1 has only 3 crippled DD available, requested 4"
    );
    assert_eq!(game_data, before);

    let err = game_data
        .transfer_ships_between_fleets(1, 1, 2, crippled_destroyers(4, 4))
        .expect_err("a transfer is held to the same rule");
    assert!(matches!(
        err,
        GameStateMutationError::FleetDetachCrippledExceedsAvailable { .. }
    ));
    assert_eq!(game_data, before);
}

#[test]
fn plain_and_crippled_quantities_stage_in_any_order_to_the_same_ships() {
    let mut game_data = setup();
    let mut first = FleetDetachSelection::default();
    first.stage(ShipCategory::Destroyers, 2, false);
    first.stage(ShipCategory::Destroyers, 1, true);
    let mut second = FleetDetachSelection::default();
    second.stage(ShipCategory::Destroyers, 1, true);
    second.stage(ShipCategory::Destroyers, 2, false);
    assert_eq!(first, second);
    // Three destroyers, one of them crippled: two healthy and the hull
    // that was crippled this pass.
    let result = game_data
        .detach_ships_to_new_fleet(1, 1, first, None, 6)
        .expect("detach");
    let new_fleet = new_fleet_of(&game_data, &result);
    assert_eq!(fleet_notation(&new_fleet), "DD3!1");
    assert_eq!(new_fleet.crippled.destroyers, hulls(0, 0, 1));
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
        .transfer_ships_between_fleets(1, 1, 2, crippled_destroyers(2, 2))
        .expect("transfer");
    // The two crippled hulls with the most time go: this pass's and the one
    // with two turns left. The host keeps its own hull and gains both.
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

#[test]
fn a_transfer_takes_healthy_hulls_first_and_n_bang_takes_cripples() {
    let mut game_data = setup();
    game_data
        .transfer_ships_between_fleets(1, 1, 2, destroyers(2))
        .expect("plain transfer");
    assert_eq!(game_data.fleets[0].crippled.destroyers, hulls(1, 1, 1));
    assert!(game_data.fleets[1].crippled.destroyers.is_empty());
    assert_eq!(fleet_notation(&game_data.fleets[1]), "DD3 TT1");

    game_data
        .transfer_ships_between_fleets(1, 1, 2, crippled_destroyers(1, 1))
        .expect("crippled transfer");
    assert_eq!(game_data.fleets[0].crippled.destroyers, hulls(1, 1, 0));
    assert_eq!(game_data.fleets[1].crippled.destroyers, hulls(0, 0, 1));
    assert_eq!(fleet_notation(&game_data.fleets[1]), "DD4!1 TT1");
}

/// Fleet #1: five healthy destroyers and four transports, two loaded (two
/// armies aboard) and two empty. Two transports are crippled, one with a
/// turn left and one with two. The loaded transports come first in the
/// split, so both crippled ones are loaded.
fn transport_setup() -> CoreGameData {
    let mut game_data = setup();
    game_data.fleets[0].crippled = CrippledShips::default();
    set_ships(&mut game_data.fleets[0], 5, 4);
    set_armies_aboard(&mut game_data.fleets[0], 2);
    game_data.fleets[0].crippled.transports = hulls(1, 1, 0);
    game_data
}

fn transports(loaded: u16, empty: u16) -> FleetDetachSelection {
    FleetDetachSelection {
        full_transports: loaded,
        empty_transports: empty,
        ..FleetDetachSelection::default()
    }
}

/// Every crippled hull, every ship, and every army in the game.
fn tally(game_data: &CoreGameData) -> (CrippledShips, u32, u32, u32) {
    let ships = game_data
        .fleets
        .iter()
        .map(|fleet| nc_data::starship_count(fleet))
        .sum();
    let armies = game_data
        .fleets
        .iter()
        .map(|fleet| u32::from(fleet.ships.loaded_transports))
        .sum();
    let transports = game_data
        .fleets
        .iter()
        .map(|fleet| u32::from(nc_data::troop_transports(fleet)))
        .sum();
    (total(game_data), ships, armies, transports)
}

#[test]
fn empty_transports_leave_the_crippled_loaded_ones_and_their_armies_alone() {
    let mut game_data = transport_setup();
    assert_eq!(fleet_notation(&game_data.fleets[0]), "DD5 TT*2!2 TT2");
    let before = tally(&game_data);
    let result = game_data
        .detach_ships_to_new_fleet(1, 1, transports(0, 2), None, 0)
        .expect("detach");
    let new_fleet = new_fleet_of(&game_data, &result);
    assert_eq!(fleet_notation(&new_fleet), "TT2");
    assert_eq!(new_fleet.ships.loaded_transports, 0);
    assert!(new_fleet.crippled.transports.is_empty());
    let donor = &game_data.fleets[result.donor_fleet_record_index_1_based - 1];
    assert_eq!(fleet_notation(donor), "DD5 TT*2!2");
    assert_eq!(tally(&game_data), before);
}

#[test]
fn a_plain_loaded_quantity_takes_crippled_loaded_transports_when_none_are_healthy() {
    let mut game_data = transport_setup();
    let before = tally(&game_data);
    let result = game_data
        .detach_ships_to_new_fleet(1, 1, transports(1, 0), None, 0)
        .expect("detach");
    let new_fleet = new_fleet_of(&game_data, &result);
    // Both loaded transports are crippled, so the one that goes is the one
    // with two turns left, and its army goes with it.
    assert_eq!(fleet_notation(&new_fleet), "TT*1!1");
    assert_eq!(new_fleet.crippled.transports, hulls(0, 1, 0));
    assert_eq!(new_fleet.ships.loaded_transports, 1);
    let donor = &game_data.fleets[result.donor_fleet_record_index_1_based - 1];
    assert_eq!(fleet_notation(donor), "DD5 TT*1!1 TT2");
    assert_eq!(donor.crippled.transports, hulls(1, 0, 0));
    assert_eq!(tally(&game_data), before);
}

#[test]
fn n_bang_on_empty_transports_is_rejected_when_only_loaded_ones_are_crippled() {
    let mut game_data = transport_setup();
    let before = game_data.clone();
    let mut selection = transports(0, 1);
    selection.required_crippled[ShipCategory::EmptyTransports.index()] = 1;
    let err = game_data
        .detach_ships_to_new_fleet(1, 1, selection, None, 0)
        .expect_err("no empty transport is crippled");
    assert_eq!(
        err.to_string(),
        "fleet 1 has only 0 crippled TT available, requested 1"
    );
    assert_eq!(game_data, before);
}

#[test]
fn n_bang_on_loaded_transports_moves_the_crippled_ones_with_their_armies() {
    let mut game_data = transport_setup();
    let before = tally(&game_data);
    let mut selection = transports(2, 0);
    selection.required_crippled[ShipCategory::FullTransports.index()] = 2;
    game_data
        .transfer_ships_between_fleets(1, 1, 2, selection)
        .expect("transfer");
    let host = &game_data.fleets[1];
    assert_eq!(fleet_notation(host), "DD1 TT*2!2 TT1");
    assert_eq!(host.crippled.transports, hulls(1, 1, 0));
    assert_eq!(host.ships.loaded_transports, 2);
    let donor = &game_data.fleets[0];
    assert_eq!(fleet_notation(donor), "DD5 TT2");
    assert!(donor.crippled.transports.is_empty());
    assert_eq!(tally(&game_data), before);
}

#[test]
fn moving_both_transport_kinds_never_duplicates_or_loses_a_crippled_hull() {
    let mut game_data = transport_setup();
    let before = tally(&game_data);
    // One loaded and one empty: the loaded one is crippled, the empty is not.
    let result = game_data
        .detach_ships_to_new_fleet(1, 1, transports(1, 1), None, 0)
        .expect("detach");
    let new_fleet = new_fleet_of(&game_data, &result);
    assert_eq!(fleet_notation(&new_fleet), "TT*1!1 TT1");
    let donor = &game_data.fleets[result.donor_fleet_record_index_1_based - 1];
    assert_eq!(fleet_notation(donor), "DD5 TT*1!1 TT1");
    assert_eq!(tally(&game_data), before);
}

#[test]
fn the_preview_resolves_the_same_ships_the_move_takes() {
    let mut game_data = setup();
    let mut selection = FleetDetachSelection::default();
    selection.stage(ShipCategory::Destroyers, 4, false);
    selection.stage(ShipCategory::EmptyTransports, 1, false);
    let preview = resolve_selection(&game_data.fleets[0], &selection).expect("resolves");
    assert_eq!(preview.notation(), "DD4!2 TT1");
    let result = game_data
        .detach_ships_to_new_fleet(1, 1, selection, None, 6)
        .expect("detach");
    let new_fleet = new_fleet_of(&game_data, &result);
    assert_eq!(fleet_notation(&new_fleet), preview.notation());
    assert_eq!(new_fleet.crippled, preview.moved);
    let donor = &game_data.fleets[result.donor_fleet_record_index_1_based - 1];
    assert_eq!(donor.crippled, preview.remaining);
}

#[test]
fn nothing_taken_reads_as_none_and_a_plain_quantity_has_no_crippled_mark() {
    let fleet = setup().fleets[0].clone();
    let preview = resolve_selection(&fleet, &FleetDetachSelection::default()).expect("resolves");
    assert_eq!(preview.notation(), "none");
    assert_eq!(
        resolve_selection(&fleet, &destroyers(2))
            .expect("resolves")
            .notation(),
        "DD2"
    );
    assert_eq!(
        resolve_selection(&fleet, &crippled_destroyers(3, 3))
            .expect("resolves")
            .notation(),
        "DD3!3"
    );
}

#[test]
fn crippled_left_to_stage_counts_what_the_staged_ships_leave() {
    let fleet = setup().fleets[0].clone();
    let none = FleetDetachSelection::default();
    assert_eq!(
        crippled_left_to_stage(&fleet, &none, ShipCategory::Destroyers),
        3
    );
    // Two healthy hulls staged: all three cripples remain.
    assert_eq!(
        crippled_left_to_stage(&fleet, &destroyers(2), ShipCategory::Destroyers),
        3
    );
    // Four staged: the two healthy hulls and two cripples.
    assert_eq!(
        crippled_left_to_stage(&fleet, &destroyers(4), ShipCategory::Destroyers),
        1
    );
    assert_eq!(
        crippled_left_to_stage(&fleet, &crippled_destroyers(2, 2), ShipCategory::Destroyers),
        1
    );
    assert_eq!(
        crippled_left_to_stage(&fleet, &none, ShipCategory::Cruisers),
        0
    );
}
