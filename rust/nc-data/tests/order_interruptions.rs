mod common;

use common::order_trace::{
    FleetTurnExpectation, TARGET_PLANET_IDX, TRACE_SPEED, TRACE_START, TRACE_TARGET, TraceCase,
    TraceRun, arm_new_order, build_trace_baseline, clear_fleet_for_probe, configure_cruiser_fleet,
    configured_delayed_hostile_arrival_state, no_turn_mutation, run_trace_case,
};
use nc_data::{
    Coords, CoreGameData, FleetOrderValidationError, InvalidPlayerStateEvent, Mission,
    MissionOutcome, MissionRetargetEvent, Order, Real48, Starbase, recompute_max_speed,
    set_armies_aboard, set_mission_target, set_planet_name, set_troop_transports,
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
        TraceCase {
            name: "patrol-sector missing combat ships holds even with a home to seek",
            setup: build_missing_patrol_combat_trace,
            fleet_idx: 0,
            turns_to_run: 1,
            expectations: &[
                FleetTurnExpectation {
                    turn: 0,
                    coords: TRACE_START,
                    target: TRACE_TARGET,
                    order: Order::PatrolSector,
                    speed: TRACE_SPEED,
                    ready_flag: None,
                },
                FleetTurnExpectation {
                    turn: 1,
                    coords: TRACE_START,
                    target: TRACE_START,
                    order: Order::HoldPosition,
                    speed: 0,
                    ready_flag: None,
                },
            ],
            before_turn: no_turn_mutation,
            check: check_missing_patrol_combat_trace,
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

    let fleet = &mut game_data.fleets[0];
    configure_cruiser_fleet(fleet);
    fleet.max_speed = TRACE_SPEED;
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
        let target = &mut game_data.planets[TARGET_PLANET_IDX];
        target.owner.0 = 2;
        target.years_under_owner = 2;
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

fn build_missing_patrol_combat_trace() -> CoreGameData {
    build_capability_invalidated_trace(Order::PatrolSector, MissingCapability::Combat, true)
}

fn check_missing_patrol_combat_trace(run: &TraceRun) {
    check_invalidated_to_seek_home(
        run,
        Order::PatrolSector,
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
    game_data.fleets[0].owner.0 = 9;
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
        let mut base = Starbase::default();
        base.base_id.0 = 1;
        base.owner.0 = 1;
        base.sector = Coords::from_array(TRACE_TARGET);
        game_data.bases = vec![base];
    }

    let fleet = &mut game_data.fleets[0];
    clear_fleet_for_probe(fleet);
    match missing {
        MissingCapability::Scout => {
            fleet.ships.cruisers = 1;
        }
        MissingCapability::Etac => {
            fleet.ships.cruisers = 1;
        }
        MissingCapability::Combat => {
            fleet.ships.scouts = 1;
        }
        MissingCapability::LoadedTroops => {
            fleet.ships.destroyers = 1;
            set_troop_transports(fleet, 2);
            set_armies_aboard(fleet, 0);
        }
    }
    recompute_max_speed(fleet);
    fleet.max_speed = TRACE_SPEED;
    arm_new_order(fleet, order, TRACE_START, TRACE_TARGET, TRACE_SPEED);
    if order == Order::GuardStarbase {
        set_mission_target(fleet, 1, 1);
    }

    game_data
}

fn build_stale_off_target_hostile_trace(
    order: Order,
    ships: (u16, u16, u16, u16, u16, u16, u16),
) -> CoreGameData {
    let (mut game_data, _target_idx, _target_coords) =
        configured_delayed_hostile_arrival_state(order, ships);
    let fleet = &mut game_data.fleets[0];
    fleet.max_speed = TRACE_SPEED;
    fleet.sector = Coords::from_array(OFF_TARGET_START);
    fleet.speed = 0;
    let mut depth = fleet.depth.to_bytes();
    depth[..5].copy_from_slice(&[0x80, 0xb9, 0xff, 0xff, 0xff]);
    fleet.depth = Real48::from_bytes(depth);
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
    for planet in &mut game_data.planets {
        planet.owner.0 = 0;
        planet.years_under_owner = 0;
        planet.ground_batteries = 0;
        planet.armies = 0;
    }
}

fn set_world_owner(game_data: &mut CoreGameData, planet_idx: usize, coords: [u8; 2], owner: u8) {
    let planet = &mut game_data.planets[planet_idx];
    planet.sector = Coords::from_array(coords);
    set_planet_name(planet, "Trace");
    planet.owner.0 = owner;
    planet.years_under_owner = if owner == 0 { 0 } else { 2 };
    planet.ground_batteries = 4;
    planet.armies = 10;
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
