//! ECGAME's detach, transfer, and commission (t042): what they do to ROE
//! (FLEETS.DAT 0x25) and to the fleet chain. The expected values come from
//! ECGAME 1.51 runs on the same setups.

use std::path::PathBuf;

use nc_data::{CoreGameData, FleetDetachSelection, FleetRecord, Order};
use nc_engine::run_maintenance_turn;

fn load_fixture(name: &str) -> CoreGameData {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures")
        .join(name)
        .join("v1.5");
    CoreGameData::load(&dir).unwrap_or_else(|e| panic!("load fixture {name}: {e}"))
}

fn set_ships(fleet: &mut FleetRecord, cruisers: u16, destroyers: u16, etacs: u16, roe: u8) {
    fleet.set_battleship_count(0);
    fleet.set_cruiser_count(cruisers);
    fleet.set_destroyer_count(destroyers);
    fleet.set_troop_transport_count(0);
    fleet.set_army_count(0);
    fleet.set_scout_count(0);
    fleet.set_etac_count(etacs);
    fleet.recompute_max_speed_from_composition();
    fleet.set_current_speed(0);
    fleet.set_current_location_coords_raw([16, 13]);
    fleet.set_standing_order_kind(Order::HoldPosition);
    fleet.set_standing_order_target_coords_raw([16, 13]);
    fleet.set_mission_target(0, 0);
    fleet.set_rules_of_engagement(roe);
}

/// Empire 1's four fleets at home: #1 and #2 a CA and an ETAC at ROE 6, #3
/// two DDs at ROE 0, #4 an ETAC at ROE 5.
fn probe_setup() -> CoreGameData {
    let mut game_data = load_fixture("ecmaint-post");
    let fleets = &mut game_data.fleets.records;
    set_ships(&mut fleets[0], 1, 0, 1, 6);
    set_ships(&mut fleets[1], 1, 0, 1, 6);
    set_ships(&mut fleets[2], 0, 2, 0, 0);
    set_ships(&mut fleets[3], 0, 0, 1, 5);
    game_data
}

fn ships(cruisers: u16, destroyers: u16) -> FleetDetachSelection {
    FleetDetachSelection {
        cruisers,
        destroyers,
        ..FleetDetachSelection::default()
    }
}

/// (record, fleet number, id, next, previous) for each empire-1 record.
fn chain(game_data: &CoreGameData) -> Vec<(usize, u16, u16, u16, u16)> {
    game_data
        .fleets
        .records
        .iter()
        .enumerate()
        .filter(|(_, fleet)| fleet.owner_empire_raw() == 1)
        .map(|(idx, fleet)| {
            (
                idx + 1,
                fleet.local_slot_word_raw(),
                fleet.fleet_id(),
                fleet.next_fleet_id(),
                fleet.previous_fleet_id(),
            )
        })
        .collect()
}

#[test]
fn a_donor_left_without_warships_drops_to_roe_0() {
    let mut game_data = probe_setup();
    let result = game_data
        .detach_ships_to_new_fleet(1, 1, ships(1, 0), None, 6)
        .expect("detach");
    assert_eq!(game_data.fleets.records[0].rules_of_engagement(), 0);
    let new_fleet = &game_data.fleets.records[result.new_fleet_record_index_1_based - 1];
    assert_eq!(new_fleet.rules_of_engagement(), 6);
}

#[test]
fn a_donor_that_keeps_a_warship_keeps_its_roe() {
    let mut game_data = probe_setup();
    let result = game_data
        .detach_ships_to_new_fleet(1, 3, ships(0, 1), None, 0)
        .expect("detach");
    // Fleet #3 keeps ROE 0, and the new combat fleet takes the ROE 0 asked for.
    assert_eq!(game_data.fleets.records[2].rules_of_engagement(), 0);
    let new_fleet = &game_data.fleets.records[result.new_fleet_record_index_1_based - 1];
    assert_eq!(new_fleet.rules_of_engagement(), 0);
}

#[test]
fn a_new_fleet_without_warships_gets_roe_0() {
    let mut game_data = probe_setup();
    let etac = FleetDetachSelection {
        etacs: 1,
        ..FleetDetachSelection::default()
    };
    let result = game_data
        .detach_ships_to_new_fleet(1, 1, etac, None, 6)
        .expect("detach");
    let new_fleet = &game_data.fleets.records[result.new_fleet_record_index_1_based - 1];
    assert_eq!(new_fleet.rules_of_engagement(), 0);
    assert_eq!(game_data.fleets.records[0].rules_of_engagement(), 6);
}

#[test]
fn a_transfer_clears_the_donor_roe_and_leaves_the_host_roe() {
    let mut game_data = probe_setup();
    game_data
        .transfer_ships_between_fleets(1, 2, 4, ships(1, 0))
        .expect("transfer");
    assert_eq!(game_data.fleets.records[1].rules_of_engagement(), 0);
    let host = &game_data.fleets.records[3];
    assert_eq!(host.cruiser_count(), 1);
    assert_eq!(host.rules_of_engagement(), 5);
}

#[test]
fn new_fleets_join_the_chain_end_and_leave_the_player_tail() {
    let mut game_data = probe_setup();
    game_data
        .detach_ships_to_new_fleet(1, 1, ships(1, 0), None, 6)
        .expect("first detach");
    game_data
        .detach_ships_to_new_fleet(1, 3, ships(0, 1), None, 0)
        .expect("second detach");
    assert_eq!(
        chain(&game_data),
        [
            (1, 1, 1, 2, 0),
            (2, 2, 2, 3, 1),
            (3, 3, 3, 4, 2),
            (4, 4, 4, 17, 3),
            (17, 5, 17, 18, 4),
            (18, 6, 18, 0, 17),
        ]
    );
    let player = &game_data.player.records[0];
    assert_eq!(player.fleet_chain_head_raw(), 1);
    // ECGAME writes the tail only when it deletes a fleet.
    assert_eq!(player.fleet_chain_tail_raw(), 4);
}

#[test]
fn a_new_fleet_in_a_number_gap_moves_contents_not_links() {
    let mut game_data = probe_setup();
    game_data.fleets.records[2].set_local_slot_word_raw(4);
    game_data.fleets.records[3].set_local_slot_word_raw(5);
    let result = game_data
        .detach_ships_to_new_fleet(1, 1, ships(1, 0), None, 6)
        .expect("detach");

    // New fleet #3 is appended as record 17, then the chain is sorted by
    // fleet number: contents move, the next/id/previous words stay put.
    assert_eq!(
        chain(&game_data),
        [
            (1, 1, 1, 2, 0),
            (2, 2, 2, 3, 1),
            (3, 3, 3, 4, 2),
            (4, 4, 4, 17, 3),
            (17, 5, 17, 0, 4),
        ]
    );
    assert_eq!(result.new_fleet_record_index_1_based, 3);
    assert_eq!(result.donor_fleet_record_index_1_based, 1);
    let fleets = &game_data.fleets.records;
    assert_eq!(fleets[2].cruiser_count(), 1);
    assert_eq!(fleets[3].destroyer_count(), 2);
    assert_eq!(fleets[16].etac_count(), 1);
    assert_eq!(fleets[16].rules_of_engagement(), 5);
    assert_eq!(game_data.player.records[0].fleet_chain_tail_raw(), 4);
}

#[test]
fn a_moved_donor_is_reported_at_its_new_record() {
    let mut game_data = probe_setup();
    game_data.fleets.records[2].set_local_slot_word_raw(4);
    game_data.fleets.records[3].set_local_slot_word_raw(5);
    // Detaching from fleet #4 (record 3) creates #3, which takes record 3.
    let result = game_data
        .detach_ships_to_new_fleet(1, 3, ships(0, 1), None, 6)
        .expect("detach");
    assert_eq!(result.new_fleet_record_index_1_based, 3);
    assert_eq!(result.donor_fleet_record_index_1_based, 4);
    let donor = &game_data.fleets.records[3];
    assert_eq!(donor.local_slot_word_raw(), 4);
    assert_eq!(donor.destroyer_count(), 1);
}

#[test]
fn commissioning_into_a_number_gap_sorts_the_chain_like_detach() {
    let mut game_data = probe_setup();
    game_data.fleets.records[2].set_local_slot_word_raw(4);
    game_data.fleets.records[3].set_local_slot_word_raw(5);
    let planet_idx = game_data
        .planets
        .records
        .iter()
        .position(|planet| planet.coords_raw() == [16, 13])
        .expect("home world");
    let planet = &mut game_data.planets.records[planet_idx];
    let slot = nc_data::stardock_slot_for_kind(1).expect("destroyer slot");
    planet.set_stardock_count_raw(slot, 1);

    game_data
        .commission_planet_stardock_slot(1, planet_idx + 1, slot)
        .expect("commission");

    assert_eq!(
        chain(&game_data),
        [
            (1, 1, 1, 2, 0),
            (2, 2, 2, 3, 1),
            (3, 3, 3, 4, 2),
            (4, 4, 4, 17, 3),
            (17, 5, 17, 0, 4),
        ]
    );
    let fleets = &game_data.fleets.records;
    assert_eq!(fleets[2].destroyer_count(), 1);
    assert_eq!(fleets[3].destroyer_count(), 2);
    assert_eq!(fleets[16].etac_count(), 1);
}

#[test]
fn a_maintenance_merge_leaves_the_host_roe() {
    // ECMAINT's merge (2895:4a6b) adds the ships and never writes 0x25.
    let mut game_data = probe_setup();
    for (record, cruisers, roe) in [(0, 0, 0), (1, 1, 6)] {
        let fleet = &mut game_data.fleets.records[record];
        set_ships(fleet, cruisers, 0, 1, roe);
        fleet.set_standing_order_kind(Order::RendezvousSector);
    }
    run_maintenance_turn(&mut game_data).expect("maintenance");
    let host = game_data
        .fleets
        .records
        .iter()
        .find(|fleet| fleet.owner_empire_raw() == 1 && fleet.local_slot_word_raw() == 1)
        .expect("fleet #1 survives");
    assert_eq!(host.cruiser_count(), 1);
    assert_eq!(host.etac_count(), 2);
    assert_eq!(host.rules_of_engagement(), 0);
}
