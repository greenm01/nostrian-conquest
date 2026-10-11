//! Regression tests: fleet indexes captured during a maintenance turn must
//! keep naming the same fleet after mid-turn fleet removals (merges, salvage,
//! civil-disorder defections).

use nc_data::{
    ColonizationResolvedEvent, CoreGameData, Fleet, GameStateBuilder, JoinMissionHostEvent,
    Mission, MissionOutcome, Order,
};
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
        player.set_autopilot_flag(0);
    }
    game_data
}

fn order_fleet(
    game_data: &mut CoreGameData,
    idx: usize,
    coords: [u8; 2],
    order: Order,
    target: [u8; 2],
    speed: u8,
) {
    let fleet = &mut game_data.fleets[idx];
    fleet.set_current_location_coords_raw(coords);
    fleet.set_standing_order_kind(order);
    fleet.set_standing_order_target_coords_raw(target);
    fleet.set_current_speed(speed);
    // As ECGAME writes the order, with the fleet in deep space.
    fleet.set_mission_target(u8::from(order.targets_world()), fleet.mission_id_raw());
    fleet.set_depth(nc_data::Real48::ZERO);
}

/// Order fleet `joiner_idx` to join the co-located fleet `host_idx` this turn.
fn join_in_place(game_data: &mut CoreGameData, joiner_idx: usize, host_idx: usize) {
    let host = &game_data.fleets[host_idx];
    let (host_number, host_coords) = (
        host.local_slot_word_raw(),
        host.current_location_coords_raw(),
    );
    order_fleet(
        game_data,
        joiner_idx,
        host_coords,
        Order::JoinAnotherFleet,
        host_coords,
        0,
    );
    game_data.fleets[joiner_idx].set_join_host_fleet_id_raw(host_number);
}

fn place_world(game_data: &mut CoreGameData, planet_idx: usize, coords: [u8; 2], owner: u8) {
    let planet = &mut game_data.planets[planet_idx];
    planet.set_coords_raw(coords);
    if owner != 0 {
        planet.set_owner_empire_slot_raw(owner);
        planet.set_ownership_status_raw(2);
        planet.set_planet_name("Target");
        planet.set_army_count_raw(1);
        planet.set_ground_batteries_raw(1);
    }
}

fn fleet_by_number(game_data: &CoreGameData, owner: u8, number: u16) -> &Fleet {
    game_data
        .fleets
        .iter()
        .find(|fleet| fleet.owner_empire_raw() == owner && fleet.local_slot_word_raw() == number)
        .unwrap_or_else(|| panic!("empire {owner} fleet {number} should exist"))
}

fn fleet_identity(game_data: &CoreGameData, fleet_idx: usize) -> (u8, u16) {
    let fleet = &game_data.fleets[fleet_idx];
    (fleet.owner_empire_raw(), fleet.local_slot_word_raw())
}

#[test]
fn colonization_follows_colonizer_shifted_by_join_merge() {
    let mut game_data = baseline();
    let world = [8, 8];
    let planet_idx = 10;
    place_world(&mut game_data, planet_idx, world, 0);
    // Fleet 2 (index 1) colonizes the world it orbits while fleet 1 (index 0)
    // joins it there, so the merge removes the record below the colonizer.
    order_fleet(&mut game_data, 1, world, Order::ColonizeWorld, world, 0);
    join_in_place(&mut game_data, 0, 1);

    let events = run_maintenance_turn(&mut game_data).expect("maintenance should succeed");

    assert_eq!(events.fleet_merge_events.len(), 1);
    assert!(
        matches!(
            events.colonization_events.as_slice(),
            [ColonizationResolvedEvent::Succeeded { planet_idx: idx, .. }] if *idx == planet_idx
        ),
        "colonization should succeed: {:?}",
        events.colonization_events
    );
    assert_eq!(game_data.planets[planet_idx].owner_empire_slot_raw(), 1);
}

#[test]
fn view_world_completion_follows_viewer_shifted_by_join_merge() {
    let mut game_data = baseline();
    let viewed = [6, 4];
    place_world(&mut game_data, 11, viewed, 0);
    join_in_place(&mut game_data, 0, 1);
    order_fleet(&mut game_data, 2, HOME, Order::ViewWorld, viewed, 6);
    order_fleet(&mut game_data, 3, HOME, Order::MoveOnly, [4, 16], 1);

    let events = run_maintenance_turn(&mut game_data).expect("maintenance should succeed");

    let view_events = events
        .mission_events
        .iter()
        .filter(|event| event.kind == Mission::ViewWorld)
        .collect::<Vec<_>>();
    assert_eq!(view_events.len(), 1, "one view report: {view_events:?}");
    assert_eq!(view_events[0].outcome, MissionOutcome::Succeeded);
    assert_eq!(fleet_identity(&game_data, view_events[0].fleet_idx), (1, 3));
    assert_eq!(
        fleet_by_number(&game_data, 1, 3).standing_order_kind(),
        Order::HoldPosition
    );
    let mover = fleet_by_number(&game_data, 1, 4);
    assert_eq!(mover.standing_order_kind(), Order::MoveOnly);
    assert_eq!(mover.current_speed(), 1);
}

#[test]
fn scout_order_restored_on_scout_shifted_by_join_merge() {
    let mut game_data = baseline();
    join_in_place(&mut game_data, 0, 1);
    order_fleet(&mut game_data, 2, HOME, Order::ScoutSector, [6, 4], 6);
    game_data.fleets[2].set_scout_count(1);
    order_fleet(&mut game_data, 3, HOME, Order::MoveOnly, [4, 16], 1);

    run_maintenance_turn(&mut game_data).expect("maintenance should succeed");

    assert_eq!(
        fleet_by_number(&game_data, 1, 3).standing_order_kind(),
        Order::ScoutSector
    );
    assert_eq!(
        fleet_by_number(&game_data, 1, 4).standing_order_kind(),
        Order::MoveOnly
    );
}

#[test]
fn move_report_names_mover_shifted_by_join_merge() {
    let mut game_data = baseline();
    join_in_place(&mut game_data, 1, 0);
    order_fleet(&mut game_data, 3, HOME, Order::MoveOnly, [6, 4], 6);

    let events = run_maintenance_turn(&mut game_data).expect("maintenance should succeed");

    let moves = events
        .mission_events
        .iter()
        .filter(|event| event.kind == Mission::MoveOnly)
        .collect::<Vec<_>>();
    assert_eq!(moves.len(), 1, "one move report: {moves:?}");
    assert_eq!(fleet_identity(&game_data, moves[0].fleet_idx), (1, 4));
}

/// Fleet 4 (index 3) bombards an enemy world it settled over last turn
/// (bombarding keeps it striking every year) while, optionally, fleet 1
/// (index 0) joins fleet 2 and fleet 3 (index 2) reaches a second enemy world
/// this turn.
fn bombard_after_arrival(with_join: bool) -> usize {
    let mut game_data = baseline();
    let target = [6, 6];
    let second_target = [8, 4];
    place_world(&mut game_data, 10, target, 2);
    place_world(&mut game_data, 11, second_target, 2);
    order_fleet(&mut game_data, 3, [5, 6], Order::BombardWorld, target, 6);
    game_data.fleets[3].set_cruiser_count(5);
    let arrival = run_maintenance_turn(&mut game_data).expect("arrival turn should succeed");
    assert_eq!(arrival.bombard_events.len(), 1, "bomber strikes on arrival");
    assert_eq!(
        game_data.fleets[3].standing_order_kind(),
        Order::BombardWorld,
        "bomber keeps bombarding"
    );

    order_fleet(
        &mut game_data,
        2,
        [7, 4],
        Order::BombardWorld,
        second_target,
        6,
    );
    game_data.fleets[2].set_cruiser_count(5);
    if with_join {
        join_in_place(&mut game_data, 0, 1);
    }
    let events = run_maintenance_turn(&mut game_data).expect("bombard turn should succeed");
    events
        .bombard_events
        .iter()
        .filter(|event| event.attacker_fleet_number == Some(4))
        .count()
}

#[test]
fn ready_bombard_survives_join_merge_and_other_hostile_arrival() {
    assert_eq!(bombard_after_arrival(false), 1, "control");
    assert_eq!(bombard_after_arrival(true), 1, "with join merge");
}

#[test]
fn join_chase_keeps_host_when_unrelated_join_merge_compacts_ids() {
    let mut game_data = baseline();
    let chased = [4, 9];
    // Fleet 2 joins fleet 1 at home; fleet 4 is still chasing fleet 3.
    join_in_place(&mut game_data, 1, 0);
    order_fleet(&mut game_data, 2, chased, Order::HoldPosition, chased, 0);
    order_fleet(
        &mut game_data,
        3,
        [4, 16],
        Order::JoinAnotherFleet,
        chased,
        1,
    );
    let host_number = game_data.fleets[2].local_slot_word_raw();
    game_data.fleets[3].set_join_host_fleet_id_raw(host_number);

    let events = run_maintenance_turn(&mut game_data).expect("maintenance should succeed");

    assert!(
        !events
            .join_host_events
            .iter()
            .any(|event| matches!(event, JoinMissionHostEvent::Retargeted { .. })),
        "no retarget expected: {:?}",
        events.join_host_events
    );
    let chaser = fleet_by_number(&game_data, 1, 4);
    // The host's record id compacted from 3 to 2; its fleet number did not.
    assert_eq!(fleet_by_number(&game_data, 1, 3).fleet_id_word_raw(), 2);
    assert_eq!(chaser.join_host_fleet_id_raw(), 3);
    assert_eq!(chaser.standing_order_target_coords_raw(), chased);
}

#[test]
fn join_resolves_by_fleet_number_when_host_number_differs_from_record_id() {
    let mut game_data = baseline();
    // Empire 2's fleets are records 5..8: fleet numbers 1..4, record ids 5..8.
    let host_idx = game_data
        .fleets
        .iter()
        .position(|fleet| fleet.owner_empire_raw() == 2 && fleet.local_slot_word_raw() == 1)
        .expect("empire 2 fleet 1");
    let joiner_idx = host_idx + 1;
    assert_eq!(fleet_identity(&game_data, joiner_idx), (2, 2));
    assert_ne!(
        game_data.fleets[host_idx].fleet_id_word_raw(),
        1,
        "the host's record id must differ from its fleet number"
    );
    let host_cruisers = game_data.fleets[host_idx].cruiser_count();
    let joiner_cruisers = game_data.fleets[joiner_idx].cruiser_count();
    let empire_1_fleet_1_cruisers = fleet_by_number(&game_data, 1, 1).cruiser_count();
    join_in_place(&mut game_data, joiner_idx, host_idx);
    assert_eq!(game_data.fleets[joiner_idx].join_host_fleet_id_raw(), 1);

    let events = run_maintenance_turn(&mut game_data).expect("maintenance should succeed");

    assert!(
        events.fleet_merge_events.iter().any(|event| {
            event.kind == Mission::JoinAnotherFleet
                && event.owner_empire_raw == 2
                && event.host_fleet_number == 1
                && event.absorbed_fleet_number == 2
        }),
        "{:?}",
        events.fleet_merge_events
    );
    assert_eq!(
        fleet_by_number(&game_data, 2, 1).cruiser_count(),
        host_cruisers + joiner_cruisers
    );
    assert!(
        !game_data
            .fleets
            .iter()
            .any(|fleet| fleet.owner_empire_raw() == 2 && fleet.local_slot_word_raw() == 2)
    );
    // Empire 1's fleet 1 (record id 1) was not the host.
    assert_eq!(
        fleet_by_number(&game_data, 1, 1).cruiser_count(),
        empire_1_fleet_1_cruisers
    );
}

#[test]
fn move_report_names_mover_shifted_by_civil_disorder_defection() {
    let mut game_data = baseline();
    for planet in &mut game_data.planets {
        if planet.owner_empire_slot_raw() == 1 {
            planet.set_owner_empire_slot_raw(0);
            planet.set_ownership_status_raw(0);
        }
    }
    for fleet in &mut game_data.fleets {
        if fleet.owner_empire_raw() == 1 {
            fleet.set_etac_count(0);
        }
    }
    run_maintenance_turn(&mut game_data).expect("collapse turn should succeed");

    let mover_idx = game_data
        .fleets
        .iter()
        .position(|fleet| fleet.owner_empire_raw() == 2 && fleet.local_slot_word_raw() == 1)
        .expect("empire 2 fleet 1");
    order_fleet(
        &mut game_data,
        mover_idx,
        [14, 4],
        Order::MoveOnly,
        [12, 4],
        6,
    );
    let events = run_maintenance_turn(&mut game_data).expect("defection turn should succeed");

    assert_eq!(events.fleet_defection_events.len(), 1);
    let moves = events
        .mission_events
        .iter()
        .filter(|event| event.kind == Mission::MoveOnly && event.owner_empire_raw == 2)
        .collect::<Vec<_>>();
    assert_eq!(moves.len(), 1, "one move report: {moves:?}");
    assert_eq!(fleet_identity(&game_data, moves[0].fleet_idx), (2, 1));
}

#[test]
fn arrival_report_week_is_the_tick_the_order_resolves() {
    let mut game_data = baseline();
    order_fleet(&mut game_data, 3, HOME, Order::MoveOnly, [6, 4], 3);

    let events = run_maintenance_turn(&mut game_data).expect("maintenance should succeed");

    let arrival = events
        .mission_events
        .iter()
        .find(|event| event.kind == Mission::MoveOnly)
        .expect("move report");
    // The report carries the tick the Move resolved, as classic's do: the
    // fleet enters (6,4) on tick 26.
    assert_eq!(arrival.stardate_week, Some(26));
}
