//! Planetary assault: when a fleet strikes, and what each planetary hit
//! costs.

use nc_data::{CoreGameData, Mission, MissionOutcome, Order, Real48};
use nc_engine::{run_maintenance_turn, run_maintenance_turn_with_context};

use super::support::{
    SYSTEM, SYSTEM_PLANET_IDX, Ships, diplomacy, empty_board, mark_arrived, place_fleet,
};

/// Empire 1 fleet record 0 sits in orbit of `SYSTEM` (owned by empire 2) on
/// an assault order that executes this turn.
fn assault_on_system(order: Order, ships: Ships, armies: u8, batteries: u8) -> CoreGameData {
    let mut game_data = empty_board(armies, batteries);
    place_fleet(&mut game_data, 0, 1, SYSTEM, order, SYSTEM, ships, 10);
    mark_arrived(&mut game_data, 0);
    game_data
}

/// The tick a fleet diving from deep space at speed 3 reaches orbit.
const DIVE_TICK: u8 = 18;

/// Fleet record `idx` of `owner` in deep space over `SYSTEM`, diving toward
/// orbit on `order` at speed 3: it shares the system layer from tick 9 and
/// reaches depth 1.0 on `DIVE_TICK`.
fn diving_on_system(
    game_data: &mut CoreGameData,
    idx: usize,
    owner: u8,
    order: Order,
    ships: Ships,
    roe: u8,
) {
    place_fleet(game_data, idx, owner, SYSTEM, order, SYSTEM, ships, roe);
    let fleet = &mut game_data.fleets.records[idx];
    fleet.set_mission_target(1, fleet.mission_id_raw());
    fleet.set_at_rest_motion();
    fleet.set_depth(Real48::ZERO);
    fleet.set_max_speed(3);
    fleet.set_current_speed(3);
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
    // Both strike on tick 1 in the year's unit order: the bomber either
    // bombarded first or found the world already its own. Either way it
    // keeps its order and its armies, in orbit.
    assert_eq!(bombard_fleet.depth(), nc_data::Real48::ONE);
    assert_eq!(game_data.fleets.records[0].army_count(), 0);
}

#[test]
fn a_bombard_fleet_in_orbit_at_the_start_of_the_year_strikes_on_tick_one() {
    let mut game_data = assault_on_system(Order::BombardWorld, cruiser(), 0, 0);

    let events = run_maintenance_turn(&mut game_data).expect("maintenance should succeed");

    assert_eq!(events.bombard_events[0].stardate_week, Some(1));
    let mission = events
        .mission_events
        .iter()
        .find(|event| {
            event.kind == Mission::BombardWorld && event.outcome == MissionOutcome::Succeeded
        })
        .expect("the bombardment is reported");
    assert_eq!(mission.stardate_week, Some(1));
}

#[test]
fn a_bombardment_resolves_in_its_tick_before_relief_arrives() {
    // Empire 1's destroyer bombards from orbit on tick 1. Empire 2's relief
    // squadron dives in from deep space, shares the orbit from tick 9, and
    // engages the bomber through the encounter loop, too late to spare the
    // world or to weaken the bombardment.
    let mut game_data = assault_on_system(
        Order::BombardWorld,
        Ships {
            dd: 1,
            ..Ships::default()
        },
        0,
        0,
    );
    diving_on_system(
        &mut game_data,
        4,
        2,
        Order::MoveOnly,
        Ships {
            bb: 5,
            ..Ships::default()
        },
        10,
    );

    let events = run_maintenance_turn_with_context(&mut game_data, &[], &diplomacy(&[(1, 2)]))
        .expect("maintenance should succeed");

    let bombard = &events.bombard_events[0];
    assert_eq!(bombard.stardate_week, Some(1));
    assert_eq!(bombard.attacker_initial.destroyers, 1);
    assert_eq!(bombard.attacker_losses.destroyers, 0);
    let battle = events
        .fleet_battle_events
        .iter()
        .find(|event| event.reporting_empire_raw == 2)
        .expect("the relief squadron engages the bomber");
    assert!(
        battle.stardate_week > bombard.stardate_week,
        "the battle comes after the strike: {:?}",
        battle.stardate_week
    );
    let bomber = &game_data.fleets.records[0];
    assert!(
        !(bomber.has_any_force() && bomber.standing_order_kind() == Order::BombardWorld),
        "five battleships end the bombardment"
    );
}

#[test]
fn assault_fleets_of_different_classes_strike_in_tick_order() {
    // Empire 1's cruiser bombards on tick 1; its transports blitz when they
    // reach orbit on tick 18. Neither waits for the other, and the blitz
    // does not go first for being a blitz.
    let mut game_data = assault_on_system(Order::BombardWorld, cruiser(), 1, 0);
    diving_on_system(
        &mut game_data,
        1,
        1,
        Order::BlitzWorld,
        Ships {
            tt: 20,
            armies: 20,
            ..Ships::default()
        },
        10,
    );

    let events = run_maintenance_turn(&mut game_data).expect("maintenance should succeed");

    assert_eq!(events.bombard_events.len(), 1);
    assert_eq!(events.bombard_events[0].stardate_week, Some(1));
    let blitz = &events.assault_report_events[0];
    assert_eq!(blitz.kind, Mission::BlitzWorld);
    assert_eq!(blitz.stardate_week, Some(DIVE_TICK));
    assert_eq!(
        events.ownership_change_events[0].stardate_week,
        Some(DIVE_TICK)
    );
    assert_eq!(
        game_data.planets.records[SYSTEM_PLANET_IDX].owner_empire_slot_raw(),
        1
    );
    assert_eq!(
        game_data.fleets.records[0].standing_order_kind(),
        Order::BombardWorld
    );
    assert_eq!(
        game_data.fleets.records[1].standing_order_kind(),
        Order::HoldPosition
    );
}

#[test]
fn a_world_captured_mid_year_is_defended_by_its_new_owner() {
    // Empire 1's invasion takes `SYSTEM` from empire 2 on tick 1. Empire 3's
    // cruiser reaches orbit on tick 18 and bombards empire 1's world.
    let mut game_data = assault_on_system(
        Order::InvadeWorld,
        Ships {
            dd: 1,
            tt: 20,
            armies: 20,
            ..Ships::default()
        },
        1,
        0,
    );
    game_data.fleets.records[0].set_rules_of_engagement(0);
    diving_on_system(&mut game_data, 8, 3, Order::BombardWorld, cruiser(), 0);

    let events = run_maintenance_turn_with_context(&mut game_data, &[], &diplomacy(&[]))
        .expect("maintenance should succeed");

    let capture = &events.ownership_change_events[0];
    assert_eq!(capture.stardate_week, Some(1));
    assert_eq!(capture.new_owner_empire_raw, 1);
    let bombard = &events.bombard_events[0];
    assert_eq!(bombard.stardate_week, Some(DIVE_TICK));
    assert_eq!(bombard.attacker_empire_raw, 3);
    assert_eq!(bombard.defender_empire_raw, 1);
    assert_eq!(
        game_data.planets.records[SYSTEM_PLANET_IDX].owner_empire_slot_raw(),
        1
    );
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
