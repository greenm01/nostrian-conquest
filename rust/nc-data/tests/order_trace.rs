mod common;

use common::order_trace::{
    FleetTurnExpectation, TARGET_PLANET_IDX, TRACE_SPEED, TRACE_START, TRACE_TARGET, TraceCase,
    TraceRun, assert_mission_event, build_blitz_world_trace, build_bombard_world_trace,
    build_colonize_world_trace, build_guard_blockade_world_trace, build_guard_starbase_trace,
    build_invade_world_trace, build_join_chase_trace, build_join_merge_trace,
    build_move_only_trace, build_patrol_sector_trace, build_rendezvous_merge_trace,
    build_rendezvous_travel_trace, build_salvage_trace, build_scout_sector_trace,
    build_scout_system_trace, build_seek_home_trace, build_view_world_trace, no_turn_mutation,
    run_trace_case,
};
use nc_data::{Mission, MissionOutcome, Order};

#[test]
fn one_shot_order_traces_monitor_speed_and_resolution() {
    let cases = [
        TraceCase {
            name: "move-only",
            setup: build_move_only_trace,
            fleet_idx: 0,
            turns_to_run: 2,
            expectations: &[
                FleetTurnExpectation {
                    turn: 0,
                    coords: TRACE_START,
                    target: TRACE_TARGET,
                    order: Order::MoveOnly,
                    speed: TRACE_SPEED,
                    ready_flag: None,
                },
                FleetTurnExpectation {
                    turn: 1,
                    coords: [10, 8],
                    target: TRACE_TARGET,
                    order: Order::MoveOnly,
                    speed: TRACE_SPEED,
                    ready_flag: None,
                },
                FleetTurnExpectation {
                    turn: 2,
                    coords: TRACE_TARGET,
                    target: TRACE_TARGET,
                    order: Order::HoldPosition,
                    speed: 0,
                    ready_flag: Some(0x00),
                },
            ],
            before_turn: no_turn_mutation,
            check: check_move_only_trace,
        },
        TraceCase {
            name: "colonize-world",
            setup: build_colonize_world_trace,
            fleet_idx: 0,
            turns_to_run: 2,
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
                    target: TRACE_TARGET,
                    order: Order::ColonizeWorld,
                    speed: TRACE_SPEED,
                    ready_flag: None,
                },
                FleetTurnExpectation {
                    turn: 2,
                    coords: TRACE_TARGET,
                    target: TRACE_TARGET,
                    order: Order::HoldPosition,
                    speed: 0,
                    ready_flag: Some(0x81),
                },
            ],
            before_turn: no_turn_mutation,
            check: check_colonize_world_trace,
        },
        TraceCase {
            name: "view-world",
            setup: build_view_world_trace,
            fleet_idx: 0,
            turns_to_run: 2,
            expectations: &[
                FleetTurnExpectation {
                    turn: 0,
                    coords: TRACE_START,
                    target: TRACE_TARGET,
                    order: Order::ViewWorld,
                    speed: TRACE_SPEED,
                    ready_flag: None,
                },
                FleetTurnExpectation {
                    turn: 1,
                    coords: [10, 8],
                    target: TRACE_TARGET,
                    order: Order::ViewWorld,
                    speed: TRACE_SPEED,
                    ready_flag: None,
                },
                FleetTurnExpectation {
                    turn: 2,
                    coords: TRACE_TARGET,
                    target: TRACE_TARGET,
                    order: Order::HoldPosition,
                    speed: 0,
                    ready_flag: Some(0x00),
                },
            ],
            before_turn: no_turn_mutation,
            check: check_view_world_trace,
        },
        TraceCase {
            name: "seek-home",
            setup: build_seek_home_trace,
            fleet_idx: 0,
            turns_to_run: 2,
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
                    coords: TRACE_TARGET,
                    target: TRACE_TARGET,
                    order: Order::HoldPosition,
                    speed: 0,
                    ready_flag: Some(0x81),
                },
            ],
            before_turn: no_turn_mutation,
            check: check_seek_home_trace,
        },
        TraceCase {
            name: "salvage",
            setup: build_salvage_trace,
            fleet_idx: 0,
            turns_to_run: 2,
            expectations: &[
                FleetTurnExpectation {
                    turn: 0,
                    coords: TRACE_START,
                    target: TRACE_TARGET,
                    order: Order::Salvage,
                    speed: TRACE_SPEED,
                    ready_flag: None,
                },
                FleetTurnExpectation {
                    turn: 1,
                    coords: [10, 8],
                    target: TRACE_TARGET,
                    order: Order::Salvage,
                    speed: TRACE_SPEED,
                    ready_flag: None,
                },
            ],
            before_turn: no_turn_mutation,
            check: check_salvage_trace,
        },
    ];

    for case in cases {
        run_trace_case(case);
    }
}

#[test]
fn persistent_order_traces_monitor_speed_on_arrival_and_on_station() {
    let cases = [
        TraceCase {
            name: "scout-sector",
            setup: build_scout_sector_trace,
            fleet_idx: 0,
            turns_to_run: 3,
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
                    target: TRACE_TARGET,
                    order: Order::ScoutSector,
                    speed: TRACE_SPEED,
                    ready_flag: None,
                },
                FleetTurnExpectation {
                    turn: 2,
                    coords: TRACE_TARGET,
                    target: TRACE_TARGET,
                    order: Order::ScoutSector,
                    speed: 0,
                    ready_flag: Some(0x00),
                },
                FleetTurnExpectation {
                    turn: 3,
                    coords: TRACE_TARGET,
                    target: TRACE_TARGET,
                    order: Order::ScoutSector,
                    speed: 0,
                    ready_flag: Some(0x00),
                },
            ],
            before_turn: no_turn_mutation,
            check: check_scout_sector_trace,
        },
        TraceCase {
            name: "scout-solar-system",
            setup: build_scout_system_trace,
            fleet_idx: 0,
            turns_to_run: 3,
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
                    target: TRACE_TARGET,
                    order: Order::ScoutSolarSystem,
                    speed: TRACE_SPEED,
                    ready_flag: None,
                },
                FleetTurnExpectation {
                    turn: 2,
                    coords: TRACE_TARGET,
                    target: TRACE_TARGET,
                    order: Order::ScoutSolarSystem,
                    speed: 0,
                    ready_flag: Some(0x81),
                },
                FleetTurnExpectation {
                    turn: 3,
                    coords: TRACE_TARGET,
                    target: TRACE_TARGET,
                    order: Order::ScoutSolarSystem,
                    speed: 0,
                    ready_flag: Some(0x81),
                },
            ],
            before_turn: no_turn_mutation,
            check: check_scout_system_trace,
        },
        TraceCase {
            name: "patrol-sector",
            setup: build_patrol_sector_trace,
            fleet_idx: 0,
            turns_to_run: 3,
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
                    coords: [10, 8],
                    target: TRACE_TARGET,
                    order: Order::PatrolSector,
                    speed: TRACE_SPEED,
                    ready_flag: None,
                },
                FleetTurnExpectation {
                    turn: 2,
                    coords: TRACE_TARGET,
                    target: TRACE_TARGET,
                    order: Order::PatrolSector,
                    speed: 0,
                    ready_flag: Some(0x00),
                },
                FleetTurnExpectation {
                    turn: 3,
                    coords: TRACE_TARGET,
                    target: TRACE_TARGET,
                    order: Order::PatrolSector,
                    speed: 0,
                    ready_flag: Some(0x00),
                },
            ],
            before_turn: no_turn_mutation,
            check: check_patrol_sector_trace,
        },
        TraceCase {
            name: "guard-starbase",
            setup: build_guard_starbase_trace,
            fleet_idx: 0,
            turns_to_run: 3,
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
                    target: TRACE_TARGET,
                    order: Order::GuardStarbase,
                    speed: TRACE_SPEED,
                    ready_flag: None,
                },
                FleetTurnExpectation {
                    turn: 2,
                    coords: TRACE_TARGET,
                    target: TRACE_TARGET,
                    order: Order::GuardStarbase,
                    speed: 0,
                    ready_flag: Some(0x81),
                },
                FleetTurnExpectation {
                    turn: 3,
                    coords: TRACE_TARGET,
                    target: TRACE_TARGET,
                    order: Order::GuardStarbase,
                    speed: 0,
                    ready_flag: Some(0x81),
                },
            ],
            before_turn: no_turn_mutation,
            check: check_guard_starbase_trace,
        },
        TraceCase {
            name: "guard-blockade-world",
            setup: build_guard_blockade_world_trace,
            fleet_idx: 0,
            turns_to_run: 3,
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
                    target: TRACE_TARGET,
                    order: Order::GuardBlockadeWorld,
                    speed: TRACE_SPEED,
                    ready_flag: None,
                },
                FleetTurnExpectation {
                    turn: 2,
                    coords: TRACE_TARGET,
                    target: TRACE_TARGET,
                    order: Order::GuardBlockadeWorld,
                    speed: 0,
                    ready_flag: Some(0x81),
                },
                FleetTurnExpectation {
                    turn: 3,
                    coords: TRACE_TARGET,
                    target: TRACE_TARGET,
                    order: Order::GuardBlockadeWorld,
                    speed: 0,
                    ready_flag: Some(0x81),
                },
            ],
            before_turn: no_turn_mutation,
            check: check_guard_blockade_trace,
        },
    ];

    for case in cases {
        run_trace_case(case);
    }
}

#[test]
fn hostile_order_traces_monitor_speed_through_delay_and_execution() {
    let cases = [
        TraceCase {
            name: "bombard-world",
            setup: build_bombard_world_trace,
            fleet_idx: 0,
            turns_to_run: 2,
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
                    target: TRACE_TARGET,
                    order: Order::BombardWorld,
                    speed: TRACE_SPEED,
                    ready_flag: None,
                },
                FleetTurnExpectation {
                    turn: 2,
                    coords: TRACE_TARGET,
                    target: TRACE_TARGET,
                    order: Order::BombardWorld,
                    speed: 3,
                    ready_flag: Some(0x80),
                },
            ],
            before_turn: no_turn_mutation,
            check: check_bombard_world_trace,
        },
        TraceCase {
            name: "invade-world",
            setup: build_invade_world_trace,
            fleet_idx: 0,
            turns_to_run: 3,
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
                    target: TRACE_TARGET,
                    order: Order::InvadeWorld,
                    speed: TRACE_SPEED,
                    ready_flag: None,
                },
                FleetTurnExpectation {
                    turn: 2,
                    coords: TRACE_TARGET,
                    target: TRACE_TARGET,
                    order: Order::HoldPosition,
                    speed: 0,
                    ready_flag: Some(0x81),
                },
                FleetTurnExpectation {
                    turn: 3,
                    coords: TRACE_TARGET,
                    target: TRACE_TARGET,
                    order: Order::HoldPosition,
                    speed: 0,
                    ready_flag: Some(0x81),
                },
            ],
            before_turn: no_turn_mutation,
            check: check_invade_world_trace,
        },
        TraceCase {
            name: "blitz-world",
            setup: build_blitz_world_trace,
            fleet_idx: 0,
            turns_to_run: 3,
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
                    target: TRACE_TARGET,
                    order: Order::BlitzWorld,
                    speed: TRACE_SPEED,
                    ready_flag: None,
                },
                FleetTurnExpectation {
                    turn: 2,
                    coords: TRACE_TARGET,
                    target: TRACE_TARGET,
                    order: Order::HoldPosition,
                    speed: 0,
                    ready_flag: Some(0x81),
                },
                FleetTurnExpectation {
                    turn: 3,
                    coords: TRACE_TARGET,
                    target: TRACE_TARGET,
                    order: Order::HoldPosition,
                    speed: 0,
                    ready_flag: Some(0x81),
                },
            ],
            before_turn: no_turn_mutation,
            check: check_blitz_world_trace,
        },
    ];

    for case in cases {
        run_trace_case(case);
    }
}

#[test]
fn merge_and_pursuit_orders_trace_speed_and_completion() {
    let cases = [
        TraceCase {
            name: "join-another-fleet chase",
            setup: build_join_chase_trace,
            fleet_idx: 1,
            turns_to_run: 2,
            expectations: &[
                FleetTurnExpectation {
                    turn: 0,
                    coords: [4, 10],
                    target: [10, 10],
                    order: Order::JoinAnotherFleet,
                    speed: TRACE_SPEED,
                    ready_flag: None,
                },
                FleetTurnExpectation {
                    turn: 1,
                    coords: [6, 10],
                    target: [10, 10],
                    order: Order::JoinAnotherFleet,
                    speed: TRACE_SPEED,
                    ready_flag: None,
                },
                FleetTurnExpectation {
                    turn: 2,
                    coords: [9, 10],
                    target: [12, 10],
                    order: Order::JoinAnotherFleet,
                    speed: TRACE_SPEED,
                    ready_flag: None,
                },
            ],
            before_turn: no_turn_mutation,
            check: check_join_chase_trace,
        },
        TraceCase {
            name: "join-another-fleet merge",
            setup: build_join_merge_trace,
            fleet_idx: 1,
            turns_to_run: 1,
            expectations: &[FleetTurnExpectation {
                turn: 0,
                coords: [10, 10],
                target: [10, 10],
                order: Order::JoinAnotherFleet,
                speed: 0,
                ready_flag: Some(0x81),
            }],
            before_turn: no_turn_mutation,
            check: check_join_merge_trace,
        },
        TraceCase {
            name: "rendezvous-sector travel",
            setup: build_rendezvous_travel_trace,
            fleet_idx: 0,
            turns_to_run: 3,
            expectations: &[
                FleetTurnExpectation {
                    turn: 0,
                    coords: TRACE_START,
                    target: TRACE_TARGET,
                    order: Order::RendezvousSector,
                    speed: TRACE_SPEED,
                    ready_flag: None,
                },
                FleetTurnExpectation {
                    turn: 1,
                    coords: [10, 8],
                    target: TRACE_TARGET,
                    order: Order::RendezvousSector,
                    speed: TRACE_SPEED,
                    ready_flag: None,
                },
                FleetTurnExpectation {
                    turn: 2,
                    coords: TRACE_TARGET,
                    target: TRACE_TARGET,
                    order: Order::RendezvousSector,
                    speed: 0,
                    ready_flag: Some(0x00),
                },
                FleetTurnExpectation {
                    turn: 3,
                    coords: TRACE_TARGET,
                    target: TRACE_TARGET,
                    order: Order::RendezvousSector,
                    speed: 0,
                    ready_flag: Some(0x00),
                },
            ],
            before_turn: no_turn_mutation,
            check: check_rendezvous_travel_trace,
        },
        TraceCase {
            name: "rendezvous-sector merge",
            setup: build_rendezvous_merge_trace,
            fleet_idx: 1,
            turns_to_run: 1,
            expectations: &[FleetTurnExpectation {
                turn: 0,
                coords: TRACE_TARGET,
                target: TRACE_TARGET,
                order: Order::RendezvousSector,
                speed: 0,
                ready_flag: Some(0x81),
            }],
            before_turn: no_turn_mutation,
            check: check_rendezvous_merge_trace,
        },
    ];

    for case in cases {
        run_trace_case(case);
    }
}

fn check_move_only_trace(run: &TraceRun) {
    assert_mission_event(
        &run.events[1],
        0,
        Mission::MoveOnly,
        MissionOutcome::Succeeded,
    );
}

fn check_colonize_world_trace(run: &TraceRun) {
    assert_mission_event(
        &run.events[1],
        0,
        Mission::ColonizeWorld,
        MissionOutcome::Succeeded,
    );
    assert_eq!(
        run.states[2].planets.records[TARGET_PLANET_IDX].owner_empire_slot_raw(),
        1
    );
}

fn check_view_world_trace(run: &TraceRun) {
    assert_mission_event(
        &run.events[1],
        0,
        Mission::ViewWorld,
        MissionOutcome::Succeeded,
    );
    assert!(run.events[1].planet_intel_events.iter().any(|event| {
        event.planet_idx == TARGET_PLANET_IDX && event.source_fleet_idx == Some(0)
    }));
}

fn check_seek_home_trace(run: &TraceRun) {
    assert_mission_event(
        &run.events[1],
        0,
        Mission::SeekHome,
        MissionOutcome::Succeeded,
    );
}

fn check_salvage_trace(run: &TraceRun) {
    assert_eq!(
        run.states[2].fleets.records.len(),
        run.states[1].fleets.records.len() - 1,
        "salvage should remove the fleet on arrival"
    );
    assert!(!run.events[1].salvage_events.is_empty());
}

fn check_scout_sector_trace(run: &TraceRun) {
    assert_mission_event(
        &run.events[1],
        0,
        Mission::ScoutSector,
        MissionOutcome::Arrived,
    );
    assert_mission_event(
        &run.events[2],
        0,
        Mission::ScoutSector,
        MissionOutcome::Succeeded,
    );
}

fn check_scout_system_trace(run: &TraceRun) {
    assert_mission_event(
        &run.events[1],
        0,
        Mission::ScoutSolarSystem,
        MissionOutcome::Succeeded,
    );
    assert_mission_event(
        &run.events[2],
        0,
        Mission::ScoutSolarSystem,
        MissionOutcome::Succeeded,
    );
    assert!(run.events[1].planet_intel_events.iter().any(|event| {
        event.planet_idx == TARGET_PLANET_IDX && event.source_fleet_idx == Some(0)
    }));
    assert!(run.events[2].planet_intel_events.iter().any(|event| {
        event.planet_idx == TARGET_PLANET_IDX && event.source_fleet_idx == Some(0)
    }));
}

fn check_patrol_sector_trace(run: &TraceRun) {
    assert_mission_event(
        &run.events[1],
        0,
        Mission::PatrolSector,
        MissionOutcome::Arrived,
    );
}

fn check_guard_starbase_trace(run: &TraceRun) {
    assert_mission_event(
        &run.events[1],
        0,
        Mission::GuardStarbase,
        MissionOutcome::Arrived,
    );
}

fn check_guard_blockade_trace(run: &TraceRun) {
    assert_mission_event(
        &run.events[1],
        0,
        Mission::GuardBlockadeWorld,
        MissionOutcome::Arrived,
    );
}

fn check_bombard_world_trace(run: &TraceRun) {
    assert_mission_event(
        &run.events[1],
        0,
        Mission::BombardWorld,
        MissionOutcome::Arrived,
    );
    // It strikes in the year it settles in orbit.
    assert!(!run.events[1].bombard_events.is_empty());
}

fn check_invade_world_trace(run: &TraceRun) {
    assert_mission_event(
        &run.events[1],
        0,
        Mission::InvadeWorld,
        MissionOutcome::Arrived,
    );
    // It strikes in the year it settles in orbit.
    assert_mission_event(
        &run.events[1],
        0,
        Mission::InvadeWorld,
        MissionOutcome::Succeeded,
    );
    assert!(!run.events[1].assault_report_events.is_empty());
    assert_eq!(
        run.states[2].planets.records[TARGET_PLANET_IDX].owner_empire_slot_raw(),
        1
    );
}

fn check_blitz_world_trace(run: &TraceRun) {
    assert_mission_event(
        &run.events[1],
        0,
        Mission::BlitzWorld,
        MissionOutcome::Arrived,
    );
    // It strikes in the year it settles in orbit.
    assert_mission_event(
        &run.events[1],
        0,
        Mission::BlitzWorld,
        MissionOutcome::Succeeded,
    );
    assert!(!run.events[1].assault_report_events.is_empty());
    assert_eq!(
        run.states[2].planets.records[TARGET_PLANET_IDX].owner_empire_slot_raw(),
        1
    );
}

fn check_join_chase_trace(run: &TraceRun) {
    let host = &run.states[1].fleets.records[0];
    assert_eq!(host.current_location_coords_raw(), [12, 10]);
    let host_after_second = &run.states[2].fleets.records[0];
    assert_eq!(host_after_second.current_location_coords_raw(), [14, 10]);
}

fn check_join_merge_trace(run: &TraceRun) {
    assert!(
        run.events[0]
            .fleet_merge_events
            .iter()
            .any(|event| { event.kind == Mission::JoinAnotherFleet && !event.survivor_side })
    );
    assert_eq!(
        run.states[1].fleets.records[0].standing_order_kind(),
        Order::HoldPosition
    );
}

fn check_rendezvous_travel_trace(run: &TraceRun) {
    assert_mission_event(
        &run.events[1],
        0,
        Mission::RendezvousSector,
        MissionOutcome::Arrived,
    );
}

fn check_rendezvous_merge_trace(run: &TraceRun) {
    assert!(
        run.events[0]
            .fleet_merge_events
            .iter()
            .any(|event| { event.kind == Mission::RendezvousSector && !event.survivor_side })
    );
    let survivor = &run.states[1].fleets.records[0];
    assert_eq!(survivor.standing_order_kind(), Order::RendezvousSector);
    assert_eq!(survivor.standing_order_target_coords_raw(), TRACE_TARGET);
    assert_eq!(survivor.max_speed(), 3);
}
