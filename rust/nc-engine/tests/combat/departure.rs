//! Leaving a blockaded world: the 1992 player guide has fleets attack other
//! players' fleets that "try to enter or leave a planet that your fleet is
//! blockading". Fleets ordered out of the world's orbit, including ships just
//! launched from its stardock, meet the blockade before they move.

use nc_data::{
    CommissionResult, CoreGameData, DiplomaticRelation, MaintenanceEvents, Order,
    fleet_motion_state::reset_motion_state_for_new_orders,
};
use nc_engine::run_maintenance_turn_with_context;

use super::support::{
    SYSTEM, SYSTEM_PLANET_IDX, Ships, add_starbase, diplomacy, empty_board, place_fleet,
};

const LEAVER: usize = 0;
const GUARD: usize = 4;
const BLOCKADER: usize = 8;
const AWAY: [u8; 2] = [20, 13];

fn cruisers(ca: u16) -> Ships {
    Ships {
        ca,
        ..Ships::default()
    }
}

/// Empire 3 blockades `SYSTEM`, the world empire 2 owns.
fn blockaded_system() -> CoreGameData {
    let mut game_data = empty_board(4, 0);
    place_fleet(
        &mut game_data,
        BLOCKADER,
        3,
        SYSTEM,
        Order::GuardBlockadeWorld,
        SYSTEM,
        cruisers(2),
        10,
    );
    game_data
}

/// Empire 1's fleet sits at `SYSTEM` and is ordered away this turn.
fn order_leaver_away(game_data: &mut CoreGameData) {
    place_fleet(
        game_data,
        LEAVER,
        1,
        SYSTEM,
        Order::MoveOnly,
        AWAY,
        Ships {
            dd: 1,
            ..Ships::default()
        },
        10,
    );
    let fleet = &mut game_data.fleets.records[LEAVER];
    reset_motion_state_for_new_orders(fleet);
    fleet.set_current_speed(3);
}

fn run(game_data: &mut CoreGameData) -> MaintenanceEvents {
    run_maintenance_turn_with_context(game_data, &[], &diplomacy(&[]))
        .expect("maintenance should succeed")
}

fn fought(events: &MaintenanceEvents, reporter: u8, enemy: u8) -> bool {
    events.fleet_battle_events.iter().any(|event| {
        event.reporting_empire_raw == reporter
            && event.coords == SYSTEM
            && event.enemy_empires_raw == [enemy]
    })
}

#[test]
fn neutral_fleet_leaving_a_blockaded_world_is_engaged() {
    let mut game_data = blockaded_system();
    order_leaver_away(&mut game_data);

    let events = run(&mut game_data);

    assert!(
        fought(&events, 3, 1) && fought(&events, 1, 3),
        "the blockade attacks a neutral fleet leaving its world: {:?}",
        events.fleet_battle_events
    );
    assert_eq!(
        game_data.stored_diplomatic_relation(1, 3),
        Some(DiplomaticRelation::Enemy),
        "the interception escalates diplomacy"
    );
}

#[test]
fn fleet_leaving_an_unblockaded_world_is_not_engaged() {
    let mut game_data = empty_board(4, 0);
    add_starbase(&mut game_data, 2, SYSTEM);
    place_fleet(
        &mut game_data,
        GUARD,
        2,
        SYSTEM,
        Order::GuardStarbase,
        SYSTEM,
        cruisers(1),
        10,
    );
    order_leaver_away(&mut game_data);

    let events = run(&mut game_data);

    assert!(
        events.fleet_battle_events.is_empty(),
        "leaving a guarded but unblockaded world is not an intrusion: {:?}",
        events.fleet_battle_events
    );
    let leaver = &game_data.fleets.records[LEAVER];
    assert_eq!(leaver.destroyer_count(), 1);
    assert_ne!(leaver.current_location_coords_raw(), SYSTEM);
}

#[test]
fn ships_launched_from_a_blockaded_stardock_are_intercepted() {
    let mut game_data = blockaded_system();
    let planet = &mut game_data.planets.records[SYSTEM_PLANET_IDX];
    planet.set_stardock_kind_raw(0, 1);
    planet.set_stardock_count_raw(0, 2);
    let CommissionResult::Fleet {
        fleet_record_index_1_based: launched,
    } = game_data
        .commission_planet_stardock_slots(2, SYSTEM_PLANET_IDX + 1, &[0])
        .expect("stardock destroyers commission")
    else {
        panic!("destroyers commission into a fleet");
    };
    let speed = game_data.fleets.records[launched - 1].max_speed();
    game_data
        .set_fleet_order(launched, speed, Order::MoveOnly.to_raw(), AWAY, None, None)
        .expect("the launched fleet takes a move order");

    let events = run(&mut game_data);

    assert!(
        fought(&events, 3, 2) && fought(&events, 2, 3),
        "the blockade intercepts ships launching from its world's stardock: {:?}",
        events.fleet_battle_events
    );
}
