mod common;

use common::order_trace::{
    FleetTurnExpectation, TARGET_PLANET_IDX, TRACE_SPEED, TRACE_START, TRACE_TARGET, TraceCase,
    TraceRun, arm_new_order, build_trace_baseline, clear_fleet_for_probe, configure_cruiser_fleet,
    configured_delayed_hostile_arrival_state, no_turn_mutation, run_trace_case,
};
use nc_data::{
    BaseDat, BaseRecord, CoreGameData, FleetOrderValidationError, InvalidPlayerStateEvent, Mission,
    MissionOutcome, MissionRetargetEvent, Order,
};

const RETREAT_TARGET: [u8; 2] = [15, 8];
const OFF_TARGET_START: [u8; 2] = [20, 25];

#[test]
fn retarget_traces_preserve_speed_until_new_arrival() {
    let cases = [TraceCase {
        name: "seek-home retarget",
        setup: build_seek_home_retarget_trace,
        fleet_idx: 0,
        turns_to_run: 3,
        expectations: &[
            FleetTurnExpectation {
                turn: 0,
                coords: TRACE_START,
                target: TRACE_TARGET,
                order: Order::SeekHome,
                speed: TRACE_SPEED,
                ready_flag: None,
            },
            FleetTurnExpectation {
                turn: 1,
                coords: [10, 8],
                target: TRACE_TARGET,
                order: Order::SeekHome,
                speed: TRACE_SPEED,
                ready_flag: None,
            },
            FleetTurnExpectation {
                turn: 2,
                coords: [13, 8],
                target: RETREAT_TARGET,
                order: Order::SeekHome,
                speed: TRACE_SPEED,
                ready_flag: None,
            },
            FleetTurnExpectation {
                turn: 3,
                coords: RETREAT_TARGET,
                target: RETREAT_TARGET,
                order: Order::HoldPosition,
                speed: 0,
                ready_flag: Some(0x80),
            },
        ],
        before_turn: invalidate_seek_home_target_before_turn,
        check: check_seek_home_retarget_trace,
    }];

    for case in cases {
        run_trace_case(case);
    }
}

#[test]
fn capability_invalidation_traces_abort_to_seek_home_or_hold() {
    let cases = [
        TraceCase {
            name: "scout-sector missing scout",
            setup: build_missing_scout_sector_trace,
            fleet_idx: 0,
            turns_to_run: 1,
            expectations: &[
                FleetTurnExpectation {
                    turn: 0,
                    coords: TRACE_START,
                    target: TRACE_TARGET,
                    order: Order::ScoutSector,
                    speed: TRACE_SPEED,
                    ready_flag: None,
                },
                FleetTurnExpectation {
                    turn: 1,
                    coords: [10, 8],
                    target: RETREAT_TARGET,
                    order: Order::SeekHome,
                    speed: TRACE_SPEED,
                    ready_flag: None,
                },
            ],
            before_turn: no_turn_mutation,
            check: check_missing_scout_sector_trace,
        },
        TraceCase {
            name: "scout-solar-system missing scout",
            setup: build_missing_scout_system_trace,
            fleet_idx: 0,
            turns_to_run: 1,
            expectations: &[
                FleetTurnExpectation {
                    turn: 0,
                    coords: TRACE_START,
                    target: TRACE_TARGET,
                    order: Order::ScoutSolarSystem,
                    speed: TRACE_SPEED,
                    ready_flag: None,
                },
                FleetTurnExpectation {
                    turn: 1,
                    coords: [10, 8],
                    target: RETREAT_TARGET,
                    order: Order::SeekHome,
                    speed: TRACE_SPEED,
                    ready_flag: None,
                },
            ],
            before_turn: no_turn_mutation,
            check: check_missing_scout_system_trace,
        },
        TraceCase {
            name: "colonize-world missing etac",
            setup: build_missing_etac_trace,
            fleet_idx: 0,
            turns_to_run: 1,
            expectations: &[
                FleetTurnExpectation {
                    turn: 0,
                    coords: TRACE_START,
                    target: TRACE_TARGET,
                    order: Order::ColonizeWorld,
                    speed: TRACE_SPEED,
                    ready_flag: None,
                },
                FleetTurnExpectation {
                    turn: 1,
                    coords: [10, 8],
                    target: RETREAT_TARGET,
                    order: Order::SeekHome,
                    speed: TRACE_SPEED,
                    ready_flag: None,
                },
            ],
            before_turn: no_turn_mutation,
            check: check_missing_etac_trace,
        },
        TraceCase {
            name: "guard-starbase missing combat ships",
            setup: build_missing_guard_combat_trace,
            fleet_idx: 0,
            turns_to_run: 1,
            expectations: &[
                FleetTurnExpectation {
                    turn: 0,
                    coords: TRACE_START,
                    target: TRACE_TARGET,
                    order: Order::GuardStarbase,
                    speed: TRACE_SPEED,
                    ready_flag: None,
                },
                FleetTurnExpectation {
                    turn: 1,
                    coords: [10, 8],
                    target: RETREAT_TARGET,
                    order: Order::SeekHome,
                    speed: TRACE_SPEED,
                    ready_flag: None,
                },
            ],
            before_turn: no_turn_mutation,
            check: check_missing_guard_combat_trace,
        },
        TraceCase {
            name: "guard-blockade-world missing combat ships",
            setup: build_missing_blockade_combat_trace,
            fleet_idx: 0,
            turns_to_run: 1,
            expectations: &[
                FleetTurnExpectation {
                    turn: 0,
                    coords: TRACE_START,
                    target: TRACE_TARGET,
                    order: Order::GuardBlockadeWorld,
                    speed: TRACE_SPEED,
                    ready_flag: None,
                },
                FleetTurnExpectation {
                    turn: 1,
                    coords: [10, 8],
                    target: RETREAT_TARGET,
                    order: Order::SeekHome,
                    speed: TRACE_SPEED,
                    ready_flag: None,
                },
            ],
            before_turn: no_turn_mutation,
            check: check_missing_blockade_combat_trace,
        },
        TraceCase {
            name: "bombard-world missing combat ships",
            setup: build_missing_bombard_combat_trace,
            fleet_idx: 0,
            turns_to_run: 1,
            expectations: &[
                FleetTurnExpectation {
                    turn: 0,
                    coords: TRACE_START,
                    target: TRACE_TARGET,
                    order: Order::BombardWorld,
                    speed: TRACE_SPEED,
                    ready_flag: None,
                },
                FleetTurnExpectation {
                    turn: 1,
                    coords: [10, 8],
                    target: RETREAT_TARGET,
                    order: Order::SeekHome,
                    speed: TRACE_SPEED,
                    ready_flag: None,
                },
            ],
            before_turn: no_turn_mutation,
            check: check_missing_bombard_combat_trace,
        },
        TraceCase {
            name: "invade-world missing loaded troops",
            setup: build_missing_invade_troops_trace,
            fleet_idx: 0,
            turns_to_run: 1,
            expectations: &[
                FleetTurnExpectation {
                    turn: 0,
                    coords: TRACE_START,
                    target: TRACE_TARGET,
                    order: Order::InvadeWorld,
                    speed: TRACE_SPEED,
                    ready_flag: None,
                },
                FleetTurnExpectation {
                    turn: 1,
                    coords: [10, 8],
                    target: RETREAT_TARGET,
                    order: Order::SeekHome,
                    speed: TRACE_SPEED,
                    ready_flag: None,
                },
            ],
            before_turn: no_turn_mutation,
            check: check_missing_invade_troops_trace,
        },
        TraceCase {
            name: "blitz-world missing loaded troops",
            setup: build_missing_blitz_troops_trace,
            fleet_idx: 0,
            turns_to_run: 1,
            expectations: &[
                FleetTurnExpectation {
                    turn: 0,
                    coords: TRACE_START,
                    target: TRACE_TARGET,
                    order: Order::BlitzWorld,
                    speed: TRACE_SPEED,
                    ready_flag: None,
                },
                FleetTurnExpectation {
                    turn: 1,
                    coords: [10, 8],
                    target: RETREAT_TARGET,
                    order: Order::SeekHome,
                    speed: TRACE_SPEED,
                    ready_flag: None,
                },
            ],
            before_turn: no_turn_mutation,
            check: check_missing_blitz_troops_trace,
        },
        TraceCase {
            name: "blitz-world missing loaded troops with no retreat target",
            setup: build_missing_blitz_troops_no_retreat_trace,
            fleet_idx: 0,
            turns_to_run: 1,
            expectations: &[
                FleetTurnExpectation {
                    turn: 0,
                    coords: TRACE_START,
                    target: TRACE_TARGET,
                    order: Order::BlitzWorld,
                    speed: TRACE_SPEED,
                    ready_flag: None,
                },
                FleetTurnExpectation {
                    turn: 1,
                    coords: TRACE_START,
                    target: TRACE_START,
                    order: Order::HoldPosition,
                    speed: 0,
                    ready_flag: Some(0x81),
                },
            ],
            before_turn: no_turn_mutation,
            check: check_missing_blitz_troops_no_retreat_trace,
        },
    ];

    for case in cases {
        run_trace_case(case);
    }
}

#[test]
fn stale_off_target_hostile_traces_rearm_without_executing() {
    let cases = [
        TraceCase {
            name: "bombard-world stale off-target",
            setup: build_stale_off_target_bombard_trace,
            fleet_idx: 0,
            turns_to_run: 1,
            expectations: &[
                FleetTurnExpectation {
                    turn: 0,
                    coords: OFF_TARGET_START,
                    target: [25, 25],
                    order: Order::BombardWorld,
                    speed: 0,
                    ready_flag: Some(0x80),
                },
                FleetTurnExpectation {
                    turn: 1,
                    coords: [22, 25],
                    target: [25, 25],
                    order: Order::BombardWorld,
                    speed: TRACE_SPEED,
                    ready_flag: Some(0x00),
                },
            ],
            before_turn: no_turn_mutation,
            check: check_stale_off_target_bombard_trace,
        },
        TraceCase {
            name: "invade-world stale off-target",
            setup: build_stale_off_target_invade_trace,
            fleet_idx: 0,
            turns_to_run: 1,
            expectations: &[
                FleetTurnExpectation {
                    turn: 0,
                    coords: OFF_TARGET_START,
                    target: [25, 25],
                    order: Order::InvadeWorld,
                    speed: 0,
                    ready_flag: Some(0x80),
                },
                FleetTurnExpectation {
                    turn: 1,
                    coords: [22, 25],
                    target: [25, 25],
                    order: Order::InvadeWorld,
                    speed: TRACE_SPEED,
                    ready_flag: Some(0x00),
                },
            ],
            before_turn: no_turn_mutation,
            check: check_stale_off_target_invade_trace,
        },
        TraceCase {
            name: "blitz-world stale off-target",
            setup: build_stale_off_target_blitz_trace,
            fleet_idx: 0,
            turns_to_run: 1,
            expectations: &[
                FleetTurnExpectation {
                    turn: 0,
                    coords: OFF_TARGET_START,
                    target: [25, 25],
                    order: Order::BlitzWorld,
                    speed: 0,
                    ready_flag: Some(0x80),
                },
                FleetTurnExpectation {
                    turn: 1,
                    coords: [22, 25],
                    target: [25, 25],
                    order: Order::BlitzWorld,
                    speed: TRACE_SPEED,
                    ready_flag: Some(0x00),
                },
            ],
            before_turn: no_turn_mutation,
            check: check_stale_off_target_blitz_trace,
        },
    ];

    for case in cases {
        run_trace_case(case);
    }
}

fn build_seek_home_retarget_trace() -> CoreGameData {
    let mut game_data = build_trace_baseline();
    clear_all_planet_ownership(&mut game_data);
    set_world_owner(&mut game_data, TARGET_PLANET_IDX, TRACE_TARGET, 1);
    set_world_owner(&mut game_data, TARGET_PLANET_IDX + 1, RETREAT_TARGET, 1);

    let fleet = &mut game_data.fleets.records[0];
    configure_cruiser_fleet(fleet);
    fleet.set_max_speed(TRACE_SPEED);
    arm_new_order(
        fleet,
        Order::SeekHome,
        TRACE_START,
        TRACE_TARGET,
        TRACE_SPEED,
    );

    game_data
}

fn invalidate_seek_home_target_before_turn(turn: usize, game_data: &mut CoreGameData) {
    if turn == 2 {
        let target = &mut game_data.planets.records[TARGET_PLANET_IDX];
        target.set_owner_empire_slot_raw(2);
        target.set_ownership_status_raw(2);
    }
}

fn check_seek_home_retarget_trace(run: &TraceRun) {
    assert!(run.events[1].mission_retarget_events.iter().any(|event| {
        matches!(
            event,
            MissionRetargetEvent::Retargeted {
                fleet_idx,
                mission,
                current_coords,
                previous_target_coords,
                new_target_coords,
                ..
            } if *fleet_idx == 0
                && *mission == Mission::SeekHome
                && *current_coords == [10, 8]
                && *previous_target_coords == TRACE_TARGET
                && *new_target_coords == RETREAT_TARGET
        )
    }));
    assert!(run.events[2].mission_events.iter().any(|event| {
        event.fleet_idx == 0
            && event.kind == Mission::SeekHome
            && event.outcome == MissionOutcome::Succeeded
            && event.location_coords == Some(RETREAT_TARGET)
    }));
}

fn build_missing_scout_sector_trace() -> CoreGameData {
    build_capability_invalidated_trace(Order::ScoutSector, MissingCapability::Scout, true)
}

fn check_missing_scout_sector_trace(run: &TraceRun) {
    check_invalidated_to_seek_home(
        run,
        Order::ScoutSector,
        FleetOrderValidationError::MissingScoutShip,
    );
}

fn build_missing_scout_system_trace() -> CoreGameData {
    build_capability_invalidated_trace(Order::ScoutSolarSystem, MissingCapability::Scout, true)
}

fn check_missing_scout_system_trace(run: &TraceRun) {
    check_invalidated_to_seek_home(
        run,
        Order::ScoutSolarSystem,
        FleetOrderValidationError::MissingScoutShip,
    );
}

fn build_missing_etac_trace() -> CoreGameData {
    build_capability_invalidated_trace(Order::ColonizeWorld, MissingCapability::Etac, true)
}

fn check_missing_etac_trace(run: &TraceRun) {
    check_invalidated_to_seek_home(
        run,
        Order::ColonizeWorld,
        FleetOrderValidationError::MissingEtac,
    );
}

fn build_missing_guard_combat_trace() -> CoreGameData {
    build_capability_invalidated_trace(Order::GuardStarbase, MissingCapability::Combat, true)
}

fn check_missing_guard_combat_trace(run: &TraceRun) {
    check_invalidated_to_seek_home(
        run,
        Order::GuardStarbase,
        FleetOrderValidationError::MissingCombatShips,
    );
}

fn build_missing_blockade_combat_trace() -> CoreGameData {
    build_capability_invalidated_trace(Order::GuardBlockadeWorld, MissingCapability::Combat, true)
}

fn check_missing_blockade_combat_trace(run: &TraceRun) {
    check_invalidated_to_seek_home(
        run,
        Order::GuardBlockadeWorld,
        FleetOrderValidationError::MissingCombatShips,
    );
}

fn build_missing_bombard_combat_trace() -> CoreGameData {
    build_capability_invalidated_trace(Order::BombardWorld, MissingCapability::Combat, true)
}

fn check_missing_bombard_combat_trace(run: &TraceRun) {
    check_invalidated_to_seek_home(
        run,
        Order::BombardWorld,
        FleetOrderValidationError::MissingCombatShips,
    );
}

fn build_missing_invade_troops_trace() -> CoreGameData {
    build_capability_invalidated_trace(Order::InvadeWorld, MissingCapability::LoadedTroops, true)
}

fn check_missing_invade_troops_trace(run: &TraceRun) {
    check_invalidated_to_seek_home(
        run,
        Order::InvadeWorld,
        FleetOrderValidationError::MissingLoadedTroopTransports,
    );
}

fn build_missing_blitz_troops_trace() -> CoreGameData {
    build_capability_invalidated_trace(Order::BlitzWorld, MissingCapability::LoadedTroops, true)
}

fn check_missing_blitz_troops_trace(run: &TraceRun) {
    check_invalidated_to_seek_home(
        run,
        Order::BlitzWorld,
        FleetOrderValidationError::MissingLoadedTroopTransports,
    );
}

fn build_missing_blitz_troops_no_retreat_trace() -> CoreGameData {
    let mut game_data = build_capability_invalidated_trace(
        Order::BlitzWorld,
        MissingCapability::LoadedTroops,
        false,
    );
    game_data.fleets.records[0].set_owner_empire_raw(9);
    game_data
}

fn check_missing_blitz_troops_no_retreat_trace(run: &TraceRun) {
    assert_invalid_player_reason(
        &run.events[0].invalid_player_state_events,
        FleetOrderValidationError::MissingLoadedTroopTransports,
    );
    assert!(
        run.events[0]
            .mission_events
            .iter()
            .all(|event| event.kind != Mission::BlitzWorld),
        "invalid blitz fleet should not emit mission events"
    );
}

fn build_stale_off_target_bombard_trace() -> CoreGameData {
    build_stale_off_target_hostile_trace(Order::BombardWorld, (0, 0, 1, 0, 0, 0, 0))
}

fn check_stale_off_target_bombard_trace(run: &TraceRun) {
    check_stale_off_target_hostile_trace(run, Mission::BombardWorld);
}

fn build_stale_off_target_invade_trace() -> CoreGameData {
    build_stale_off_target_hostile_trace(Order::InvadeWorld, (0, 0, 1, 2, 6, 0, 0))
}

fn check_stale_off_target_invade_trace(run: &TraceRun) {
    check_stale_off_target_hostile_trace(run, Mission::InvadeWorld);
}

fn build_stale_off_target_blitz_trace() -> CoreGameData {
    build_stale_off_target_hostile_trace(Order::BlitzWorld, (0, 0, 1, 4, 8, 0, 0))
}

fn check_stale_off_target_blitz_trace(run: &TraceRun) {
    check_stale_off_target_hostile_trace(run, Mission::BlitzWorld);
}

#[derive(Clone, Copy)]
enum MissingCapability {
    Scout,
    Etac,
    Combat,
    LoadedTroops,
}

fn build_capability_invalidated_trace(
    order: Order,
    missing: MissingCapability,
    with_retreat_target: bool,
) -> CoreGameData {
    let mut game_data = build_trace_baseline();
    clear_all_planet_ownership(&mut game_data);
    set_target_world_for_order(&mut game_data, order);
    if with_retreat_target {
        set_world_owner(&mut game_data, TARGET_PLANET_IDX + 1, RETREAT_TARGET, 1);
    }
    if order == Order::GuardStarbase {
        let mut base = BaseRecord::new_zeroed();
        base.set_active_flag_raw(1);
        base.set_base_id_raw(1);
        base.set_owner_empire_raw(1);
        base.set_coords_raw(TRACE_TARGET);
        game_data.bases = BaseDat {
            records: vec![base],
        };
    }

    let fleet = &mut game_data.fleets.records[0];
    clear_fleet_for_probe(fleet);
    match missing {
        MissingCapability::Scout => {
            fleet.set_cruiser_count(1);
        }
        MissingCapability::Etac => {
            fleet.set_cruiser_count(1);
        }
        MissingCapability::Combat => {
            fleet.set_scout_count(1);
        }
        MissingCapability::LoadedTroops => {
            fleet.set_destroyer_count(1);
            fleet.set_troop_transport_count(2);
            fleet.set_army_count(0);
        }
    }
    fleet.recompute_max_speed_from_composition();
    fleet.set_max_speed(TRACE_SPEED);
    arm_new_order(fleet, order, TRACE_START, TRACE_TARGET, TRACE_SPEED);
    if order == Order::GuardStarbase {
        fleet.set_mission_aux_bytes([1, 1]);
    }

    game_data
}

fn build_stale_off_target_hostile_trace(
    order: Order,
    ships: (u16, u16, u16, u16, u16, u16, u16),
) -> CoreGameData {
    let (mut game_data, _target_idx, _target_coords) =
        configured_delayed_hostile_arrival_state(order, ships);
    let fleet = &mut game_data.fleets.records[0];
    fleet.set_max_speed(TRACE_SPEED);
    fleet.set_current_location_coords_raw(OFF_TARGET_START);
    fleet.set_current_speed(0);
    fleet.set_tuple_c_payload_raw([0x80, 0xb9, 0xff, 0xff, 0xff]);
    game_data
}

fn check_invalidated_to_seek_home(
    run: &TraceRun,
    mission: Order,
    reason: FleetOrderValidationError,
) {
    assert_invalid_player_reason(&run.events[0].invalid_player_state_events, reason);
    assert!(
        run.events[0]
            .mission_events
            .iter()
            .all(|event| event.kind != mission_to_event(mission)),
        "invalidated fleet should not emit the original mission event"
    );
}

fn check_stale_off_target_hostile_trace(run: &TraceRun, mission: Mission) {
    assert!(
        run.events[0]
            .mission_events
            .iter()
            .all(|event| !(event.fleet_idx == 0 && event.kind == mission)),
        "stale off-target hostile order should not emit mission events"
    );
    assert!(
        run.events[0].bombard_events.is_empty(),
        "stale off-target hostile order should not bombard remotely"
    );
    assert!(
        run.events[0].assault_report_events.is_empty(),
        "stale off-target hostile order should not assault remotely"
    );
}

fn mission_to_event(order: Order) -> Mission {
    match order {
        Order::MoveOnly => Mission::MoveOnly,
        Order::PatrolSector => Mission::PatrolSector,
        Order::GuardStarbase => Mission::GuardStarbase,
        Order::GuardBlockadeWorld => Mission::GuardBlockadeWorld,
        Order::BombardWorld => Mission::BombardWorld,
        Order::InvadeWorld => Mission::InvadeWorld,
        Order::BlitzWorld => Mission::BlitzWorld,
        Order::ScoutSector => Mission::ScoutSector,
        Order::ScoutSolarSystem => Mission::ScoutSolarSystem,
        Order::ViewWorld => Mission::ViewWorld,
        Order::ColonizeWorld => Mission::ColonizeWorld,
        Order::SeekHome => Mission::SeekHome,
        Order::Salvage => Mission::Salvage,
        Order::JoinAnotherFleet => Mission::JoinAnotherFleet,
        Order::RendezvousSector => Mission::RendezvousSector,
        Order::HoldPosition | Order::Unknown(_) => unreachable!(),
    }
}

fn assert_invalid_player_reason(
    events: &[InvalidPlayerStateEvent],
    reason: FleetOrderValidationError,
) {
    assert!(events.iter().any(|event| {
        matches!(
            event,
            InvalidPlayerStateEvent::FleetMission {
                fleet_idx: 0,
                reason: event_reason,
                ..
            } if *event_reason == reason
        )
    }));
}

fn clear_all_planet_ownership(game_data: &mut CoreGameData) {
    for planet in &mut game_data.planets.records {
        planet.set_owner_empire_slot_raw(0);
        planet.set_ownership_status_raw(0);
        planet.set_ground_batteries_raw(0);
        planet.set_army_count_raw(0);
    }
}

fn set_world_owner(game_data: &mut CoreGameData, planet_idx: usize, coords: [u8; 2], owner: u8) {
    let planet = &mut game_data.planets.records[planet_idx];
    planet.set_coords_raw(coords);
    planet.set_planet_name("Trace");
    planet.set_owner_empire_slot_raw(owner);
    planet.set_ownership_status_raw(if owner == 0 { 0 } else { 2 });
    planet.set_ground_batteries_raw(4);
    planet.set_army_count_raw(10);
}

fn set_target_world_for_order(game_data: &mut CoreGameData, order: Order) {
    let owner = match order {
        Order::ColonizeWorld => 0,
        Order::GuardBlockadeWorld
        | Order::BombardWorld
        | Order::InvadeWorld
        | Order::BlitzWorld
        | Order::ViewWorld
        | Order::ScoutSolarSystem => 2,
        Order::ScoutSector | Order::GuardStarbase => 0,
        _ => 0,
    };
    set_world_owner(game_data, TARGET_PLANET_IDX, TRACE_TARGET, owner);
}
