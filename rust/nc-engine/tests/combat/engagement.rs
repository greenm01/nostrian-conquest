//! Who fights whom: the 1992 engagement rule. Declared enemies are
//! intercepted wherever they meet (ROE permitting); other foreign fleets are
//! attacked only when they enter one of your solar systems.

use nc_data::{MaintenanceEvents, MissionOutcome, Order};
use nc_engine::run_maintenance_turn_with_context;

use super::support::{
    DEEP_SPACE, SYSTEM, Ships, add_starbase, diplomacy, empty_board, place_fleet,
};

const INTRUDER: usize = 0;
const GUARD: usize = 4;

/// Empire 2 owns `SYSTEM`, has a starbase there, and escorts it with a
/// Guard Starbase cruiser. Empire 1 places one fleet in the same sector.
fn guarded_system_with_visitor(
    order: Order,
    visitor: Ships,
    visitor_roe: u8,
    guard_roe: u8,
) -> nc_data::CoreGameData {
    let mut game_data = empty_board(4, 0);
    add_starbase(&mut game_data, 2, SYSTEM);
    place_fleet(
        &mut game_data,
        GUARD,
        2,
        SYSTEM,
        Order::GuardStarbase,
        SYSTEM,
        Ships {
            ca: 1,
            ..Ships::default()
        },
        guard_roe,
    );
    let target = if order == Order::MoveOnly {
        [16, 13]
    } else {
        SYSTEM
    };
    place_fleet(
        &mut game_data,
        INTRUDER,
        1,
        SYSTEM,
        order,
        target,
        visitor,
        visitor_roe,
    );
    game_data
}

fn destroyer() -> Ships {
    Ships {
        dd: 1,
        ..Ships::default()
    }
}

fn scout() -> Ships {
    Ships {
        sc: 1,
        ..Ships::default()
    }
}

fn run(game_data: &mut nc_data::CoreGameData, enemy_pairs: &[(u8, u8)]) -> MaintenanceEvents {
    run_maintenance_turn_with_context(game_data, &[], &diplomacy(enemy_pairs))
        .expect("maintenance should succeed")
}

#[test]
fn enemy_transit_fleet_is_intercepted_at_a_guarded_system() {
    let mut game_data = guarded_system_with_visitor(Order::MoveOnly, destroyer(), 10, 10);

    let events = run(&mut game_data, &[(1, 2)]);

    assert!(
        events
            .fleet_battle_events
            .iter()
            .any(|event| event.reporting_empire_raw == 2 && event.enemy_empires_raw == [1]),
        "a declared enemy passing a guarded system is intercepted: {:?}",
        events.fleet_battle_events
    );
}

#[test]
fn low_roe_guard_holds_formation_against_passing_unarmed_enemy() {
    let mut game_data = guarded_system_with_visitor(Order::MoveOnly, scout(), 0, 0);

    let events = run(&mut game_data, &[(1, 2)]);

    assert!(events.fleet_battle_events.is_empty());
    let visitor = &game_data.fleets.records[INTRUDER];
    assert_eq!(visitor.scout_count(), 1);
    assert_eq!(visitor.standing_order_kind(), Order::MoveOnly);
    let guard = &game_data.fleets.records[GUARD];
    assert_eq!(guard.standing_order_kind(), Order::GuardStarbase);
    assert!(
        events
            .scout_contact_events
            .iter()
            .any(|event| event.viewer_empire_raw == 2 && event.target_empire_raw == 1)
    );
}

#[test]
fn guard_with_roe_breaks_formation_against_passing_unarmed_enemy() {
    let mut game_data = guarded_system_with_visitor(Order::MoveOnly, scout(), 0, 1);

    let events = run(&mut game_data, &[(1, 2)]);

    assert!(
        events
            .fleet_battle_events
            .iter()
            .any(|event| event.reporting_empire_raw == 2),
        "ROE 1 engages a defenseless enemy"
    );
}

#[test]
fn neutral_transit_fleet_is_not_attacked_at_a_guarded_system() {
    let mut game_data = guarded_system_with_visitor(Order::MoveOnly, destroyer(), 10, 10);

    let events = run(&mut game_data, &[]);

    assert!(
        events.fleet_battle_events.is_empty(),
        "a neutral fleet in the sector's deep space is not an intrusion: {:?}",
        events.fleet_battle_events
    );
    let visitor = &game_data.fleets.records[INTRUDER];
    assert_eq!(visitor.destroyer_count(), 1);
    assert_eq!(visitor.standing_order_kind(), Order::MoveOnly);
    assert!(
        events
            .scout_contact_events
            .iter()
            .any(|event| event.viewer_empire_raw == 2 && event.target_empire_raw == 1),
        "contact is still reported"
    );
}

#[test]
fn neutral_scout_entering_the_system_is_attacked() {
    let mut game_data = guarded_system_with_visitor(Order::ScoutSolarSystem, scout(), 0, 10);

    let events = run(&mut game_data, &[]);

    assert!(
        events
            .fleet_battle_events
            .iter()
            .any(|event| event.reporting_empire_raw == 2 && event.enemy_empires_raw == [1]),
        "Scout Solar System enters the owner's system: {:?}",
        events.fleet_battle_events
    );
}

#[test]
fn neutral_view_world_fleet_is_not_attacked() {
    let mut game_data = guarded_system_with_visitor(Order::ViewWorld, destroyer(), 10, 10);

    let events = run(&mut game_data, &[]);

    assert!(
        events.fleet_battle_events.is_empty(),
        "View World scans from the system edge: {:?}",
        events.fleet_battle_events
    );
    assert_eq!(game_data.fleets.records[INTRUDER].destroyer_count(), 1);
}

#[test]
fn neutral_bombard_fleet_is_attacked() {
    let mut game_data = guarded_system_with_visitor(Order::BombardWorld, destroyer(), 0, 0);

    let events = run(&mut game_data, &[]);

    assert!(
        events
            .fleet_battle_events
            .iter()
            .any(|event| event.reporting_empire_raw == 2 && event.enemy_empires_raw == [1]),
        "an assault order at an owned world is a hostile intrusion: {:?}",
        events.fleet_battle_events
    );
}

#[test]
fn neutral_bystander_neither_wins_nor_is_reported_in_anothers_battle() {
    let mut game_data = empty_board(4, 0);
    place_fleet(
        &mut game_data,
        0,
        1,
        DEEP_SPACE,
        Order::MoveOnly,
        [9, 8],
        Ships {
            ca: 3,
            ..Ships::default()
        },
        10,
    );
    place_fleet(
        &mut game_data,
        4,
        2,
        DEEP_SPACE,
        Order::MoveOnly,
        [7, 8],
        destroyer(),
        10,
    );
    place_fleet(
        &mut game_data,
        8,
        3,
        DEEP_SPACE,
        Order::MoveOnly,
        [8, 9],
        Ships {
            bb: 2,
            ..Ships::default()
        },
        10,
    );

    let events = run(&mut game_data, &[(1, 2)]);

    assert!(
        events
            .fleet_battle_events
            .iter()
            .all(|event| event.reporting_empire_raw != 3),
        "the neutral bystander did not fight: {:?}",
        events.fleet_battle_events
    );
    let victor = events
        .fleet_battle_events
        .iter()
        .find(|event| event.reporting_empire_raw == 1)
        .expect("empire 1 fought");
    assert!(victor.held_field, "the real victor holds the field");
    assert_eq!(victor.enemy_empires_raw, vec![2]);

    let victor_fleet = &game_data.fleets.records[0];
    assert_eq!(victor_fleet.standing_order_kind(), Order::MoveOnly);
    let bystander = &game_data.fleets.records[8];
    assert_eq!(bystander.standing_order_kind(), Order::MoveOnly);
    assert_eq!(bystander.battleship_count(), 2);
    assert!(
        !events
            .mission_events
            .iter()
            .any(|event| event.owner_empire_raw != 2 && event.outcome == MissionOutcome::Aborted)
    );
}

#[test]
fn unarmed_enemy_contact_does_not_abort_missions() {
    let mut game_data = empty_board(4, 0);
    place_fleet(
        &mut game_data,
        0,
        1,
        DEEP_SPACE,
        Order::ScoutSector,
        DEEP_SPACE,
        scout(),
        0,
    );
    place_fleet(
        &mut game_data,
        4,
        2,
        DEEP_SPACE,
        Order::MoveOnly,
        [7, 8],
        Ships {
            tt: 2,
            ..Ships::default()
        },
        0,
    );

    let events = run(&mut game_data, &[(1, 2)]);

    assert!(events.fleet_battle_events.is_empty());
    assert!(
        !events
            .mission_events
            .iter()
            .any(|event| event.outcome == MissionOutcome::Aborted),
        "{:?}",
        events.mission_events
    );
    assert_eq!(
        game_data.fleets.records[0].standing_order_kind(),
        Order::ScoutSector
    );
    assert_eq!(
        game_data.fleets.records[4].standing_order_kind(),
        Order::MoveOnly
    );
    assert!(
        events
            .scout_contact_events
            .iter()
            .any(|event| event.viewer_empire_raw == 1 && event.target_empire_raw == 2)
    );
}
