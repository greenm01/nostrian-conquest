//! Planetary assault: which fleets execute, and what each planetary hit costs.

use nc_data::{CoreGameData, Mission, Order};
use nc_engine::run_maintenance_turn;

use super::support::{SYSTEM, SYSTEM_PLANET_IDX, Ships, empty_board, mark_arrived, place_fleet};

/// Empire 1 fleet record 0 sits in orbit of `SYSTEM` (owned by empire 2) on
/// an assault order that executes this turn.
fn assault_on_system(order: Order, ships: Ships, armies: u8, batteries: u8) -> CoreGameData {
    let mut game_data = empty_board(armies, batteries);
    place_fleet(&mut game_data, 0, 1, SYSTEM, order, SYSTEM, ships, 10);
    mark_arrived(&mut game_data, 0);
    game_data
}

fn cruiser() -> Ships {
    Ships {
        ca: 1,
        ..Ships::default()
    }
}

#[test]
fn one_cruiser_cannot_clear_ten_batteries_in_one_round() {
    let mut game_data = assault_on_system(Order::BombardWorld, cruiser(), 0, 10);

    let events = run_maintenance_turn(&mut game_data).expect("maintenance should succeed");

    let bombard = &events.bombard_events[0];
    assert_eq!(bombard.defender_batteries_initial, 10);
    assert!(
        bombard.defender_battery_losses <= 1,
        "each battery costs 20 hits; a lone cruiser deals at most 31 per round: {}",
        bombard.defender_battery_losses
    );
}

#[test]
fn stardock_ships_absorb_their_own_defense_strength() {
    let mut game_data = assault_on_system(Order::BombardWorld, cruiser(), 0, 0);
    let planet = &mut game_data.planets.records[SYSTEM_PLANET_IDX];
    planet.set_stardock_count_raw(2, 1);

    let events = run_maintenance_turn(&mut game_data).expect("maintenance should succeed");

    let bombard = &events.bombard_events[0];
    assert_eq!(
        bombard.stardock_items_destroyed, 0,
        "a docked battleship (DS 100) outlasts a lone cruiser's per-round hits"
    );
    assert_eq!(
        game_data.planets.records[SYSTEM_PLANET_IDX].stardock_count_raw(2),
        1
    );
}

#[test]
fn breakthrough_hits_reach_industry_on_the_ten_times_scale() {
    let mut game_data = assault_on_system(
        Order::BombardWorld,
        Ships {
            dd: 1,
            ..Ships::default()
        },
        0,
        0,
    );
    let planet = &mut game_data.planets.records[SYSTEM_PLANET_IDX];
    planet.set_present_production_points(100);
    planet.set_stored_production_points(10_000);

    let events = run_maintenance_turn(&mut game_data).expect("maintenance should succeed");

    let bombard = &events.bombard_events[0];
    assert!(bombard.breakthrough);
    assert!(
        bombard.factories_destroyed <= 1,
        "a destroyer deals at most 14 hits, one factory on the 10x scale: {}",
        bombard.factories_destroyed
    );
    assert!(bombard.stored_goods_destroyed <= 100);
}

#[test]
fn invasion_softening_compares_firepower_against_army_strength() {
    let mut game_data = assault_on_system(
        Order::InvadeWorld,
        Ships {
            ca: 1,
            tt: 40,
            armies: 40,
            ..Ships::default()
        },
        20,
        0,
    );

    let events = run_maintenance_turn(&mut game_data).expect("maintenance should succeed");

    let assault = &events.assault_report_events[0];
    assert_eq!(assault.kind, Mission::InvadeWorld);
    assert!(
        assault.defender_army_losses_softening <= 3,
        "one cruiser (bombard AS 30) against 20 armies (AS 200) is disadvantaged \
         and each army costs 10 hits: {}",
        assault.defender_army_losses_softening
    );
}

#[test]
fn bombard_fleet_is_not_folded_into_a_same_turn_invasion() {
    let mut game_data = assault_on_system(
        Order::InvadeWorld,
        Ships {
            ca: 4,
            tt: 10,
            armies: 10,
            ..Ships::default()
        },
        1,
        0,
    );
    place_fleet(
        &mut game_data,
        1,
        1,
        SYSTEM,
        Order::BombardWorld,
        SYSTEM,
        Ships {
            dd: 2,
            tt: 3,
            armies: 3,
            ..Ships::default()
        },
        10,
    );
    mark_arrived(&mut game_data, 1);

    let events = run_maintenance_turn(&mut game_data).expect("maintenance should succeed");

    let invasion = &events.assault_report_events[0];
    assert_eq!(invasion.kind, Mission::InvadeWorld);
    assert_eq!(
        invasion.attacker_loaded_armies_initial, 10,
        "only the invading fleet lands its armies"
    );
    let bombard_fleet = &game_data.fleets.records[1];
    assert_eq!(bombard_fleet.army_count(), 3);
    assert_eq!(bombard_fleet.standing_order_kind(), Order::BombardWorld);
    // It keeps its pre-strike state (at rest in orbit) to strike next year.
    assert_eq!(bombard_fleet.depth(), nc_data::Real48::ONE);
    assert_eq!(game_data.fleets.records[0].army_count(), 0);
}

#[test]
fn landing_fire_pays_transport_defense_strength_on_the_ten_times_scale() {
    // Twenty loaded transports blitz a world with one battery and no garrison.
    // The battery fires 90 AS at the descending transports; on any CRT row
    // that is 90..=225 hits, and each transport costs 2x its DS of 10.
    let ships = Ships {
        tt: 20,
        armies: 20,
        ..Ships::default()
    };
    let mut game_data = assault_on_system(Order::BlitzWorld, ships, 0, 1);

    let events = run_maintenance_turn(&mut game_data).expect("maintenance should succeed");

    let blitz = &events.assault_report_events[0];
    assert_eq!(blitz.kind, Mission::BlitzWorld);
    let transport_losses = blitz.attacker_ship_losses.transports;
    assert!(
        (4..=12).contains(&transport_losses),
        "one battery should sink 4 to 12 transports, not one per battery: {transport_losses}"
    );
    assert_eq!(blitz.transport_army_losses, transport_losses);
}
