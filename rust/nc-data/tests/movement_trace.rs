use nc_data::{BaseDat, BaseRecord, GameStateBuilder, Order};
use nc_engine::{FleetEtaEstimate, estimate_fleet_eta, run_maintenance_turn};
use std::path::Path;

#[derive(Clone, Copy)]
struct MovementCase {
    name: &'static str,
    speed: u8,
    start: [u8; 2],
    target: [u8; 2],
    expected_initial_eta: u16,
    expected_positions: &'static [[u8; 2]],
}

#[derive(Clone, Copy)]
enum PersistentProbeSetup {
    Patrol,
    GuardStarbase,
    GuardBlockade,
}

#[derive(Clone, Copy)]
struct PersistentMovementCase {
    name: &'static str,
    order: Order,
    start: [u8; 2],
    target: [u8; 2],
    expected_positions: &'static [[u8; 2]],
    setup: PersistentProbeSetup,
}

#[test]
fn move_only_traces_match_current_classic_oracle_cases() {
    let cases = [
        MovementCase {
            name: "speed3-horizontal",
            speed: 3,
            start: [10, 10],
            target: [16, 10],
            expected_initial_eta: 3,
            expected_positions: &[[10, 10], [12, 10], [15, 10], [16, 10]],
        },
        MovementCase {
            name: "speed3-diagonal",
            speed: 3,
            start: [10, 10],
            target: [16, 16],
            expected_initial_eta: 3,
            expected_positions: &[[10, 10], [11, 11], [14, 14], [16, 16], [16, 16]],
        },
        MovementCase {
            name: "speed6-diagonal",
            speed: 6,
            start: [10, 10],
            target: [16, 16],
            expected_initial_eta: 2,
            expected_positions: &[[10, 10], [14, 14], [16, 16]],
        },
        MovementCase {
            name: "speed1-diagonal",
            speed: 1,
            start: [10, 10],
            target: [13, 13],
            expected_initial_eta: 5,
            expected_positions: &[[10, 10], [10, 10], [11, 11], [11, 11], [12, 12], [13, 13]],
        },
        MovementCase {
            name: "speed3-shallow",
            speed: 3,
            start: [10, 10],
            target: [16, 12],
            expected_initial_eta: 3,
            expected_positions: &[[10, 10], [12, 11], [15, 12], [16, 12]],
        },
        MovementCase {
            name: "speed3-steep",
            speed: 3,
            start: [10, 10],
            target: [12, 16],
            expected_initial_eta: 3,
            expected_positions: &[[10, 10], [11, 12], [12, 15], [12, 16]],
        },
    ];

    for case in cases {
        let mut game_data = build_move_only_probe(case.start, case.target, case.speed);
        assert_eq!(
            estimate_fleet_eta(&game_data, 0),
            FleetEtaEstimate::Years(case.expected_initial_eta),
            "{} initial eta",
            case.name
        );

        for (turn, expected_coords) in case.expected_positions.iter().enumerate() {
            let fleet = &game_data.fleets.records[0];
            assert_eq!(
                fleet.current_location_coords_raw(),
                *expected_coords,
                "{} turn {} position",
                case.name,
                turn
            );

            if turn + 1 < case.expected_positions.len() {
                run_maintenance_turn(&mut game_data)
                    .unwrap_or_else(|err| panic!("{} maintenance failed: {err}", case.name));
            }
        }
    }
}

/// Classic ECMAINT completes a move order in the year the fleet first shows
/// in the target sector, with the order entered as ECGAME writes it (mission
/// aux bytes clear). Years are from the 2026-10-10 oracle probes.
#[test]
fn move_orders_complete_in_the_classic_oracle_year() {
    let cases: [(u8, [u8; 2], u16); 15] = [
        (1, [13, 13], 5),
        (1, [12, 12], 4),
        (1, [14, 14], 6),
        (1, [13, 11], 4),
        (1, [13, 10], 4),
        (2, [13, 13], 3),
        (2, [16, 16], 5),
        (2, [14, 12], 3),
        (2, [14, 10], 3),
        (3, [13, 13], 2),
        (3, [16, 16], 3),
        (3, [16, 10], 3),
        (4, [16, 16], 3),
        (5, [16, 16], 2),
        (6, [16, 16], 2),
    ];
    for (speed, target, classic_year) in cases {
        let mut game_data = build_move_only_probe([10, 10], target, speed);
        assert_eq!(
            estimate_fleet_eta(&game_data, 0),
            FleetEtaEstimate::Years(classic_year),
            "speed {speed} to {target:?} ETA"
        );
        let mut year = 0;
        while game_data.fleets.records[0].standing_order_kind() == Order::MoveOnly {
            year += 1;
            assert!(year <= 10, "speed {speed} to {target:?} never completed");
            run_maintenance_turn(&mut game_data).expect("maintenance should succeed");
        }
        assert_eq!(
            year, classic_year,
            "speed {speed} to {target:?} completion year"
        );
        assert_eq!(
            game_data.fleets.records[0].current_location_coords_raw(),
            target
        );
    }
}

#[test]
fn persistent_standing_traces_match_current_classic_oracle_cases() {
    let cases = [
        PersistentMovementCase {
            name: "patrol-speed3-axial",
            order: Order::PatrolSector,
            start: [8, 10],
            target: [11, 10],
            expected_positions: &[[8, 10], [10, 10], [11, 10], [11, 10]],
            setup: PersistentProbeSetup::Patrol,
        },
        PersistentMovementCase {
            name: "guard-starbase-speed3-axial",
            order: Order::GuardStarbase,
            start: [8, 8],
            target: [11, 8],
            expected_positions: &[[8, 8], [10, 8], [11, 8], [11, 8]],
            setup: PersistentProbeSetup::GuardStarbase,
        },
        PersistentMovementCase {
            name: "guard-blockade-speed3-axial",
            order: Order::GuardBlockadeWorld,
            start: [8, 8],
            target: [11, 8],
            expected_positions: &[[8, 8], [10, 8], [11, 8], [11, 8]],
            setup: PersistentProbeSetup::GuardBlockade,
        },
    ];

    for case in cases {
        let mut game_data = build_persistent_probe(case);

        for (turn, expected_coords) in case.expected_positions.iter().enumerate() {
            let fleet = &game_data.fleets.records[0];
            assert_eq!(
                fleet.current_location_coords_raw(),
                *expected_coords,
                "{} turn {} position",
                case.name,
                turn
            );

            assert_eq!(
                fleet.standing_order_kind(),
                case.order,
                "{} order on turn {}",
                case.name,
                turn
            );

            let expected_speed = if turn < 2 { 3 } else { 0 };
            assert_eq!(
                fleet.current_speed(),
                expected_speed,
                "{} speed on turn {}",
                case.name,
                turn
            );

            if turn + 1 < case.expected_positions.len() {
                run_maintenance_turn(&mut game_data)
                    .unwrap_or_else(|err| panic!("{} maintenance failed: {err}", case.name));
            }
        }
    }
}

fn build_move_only_probe(start: [u8; 2], target: [u8; 2], speed: u8) -> nc_data::CoreGameData {
    let mut game_data = load_fixture("ecmaint-post");

    let fleet = &mut game_data.fleets.records[0];
    fleet.set_battleship_count(0);
    fleet.set_cruiser_count(0);
    fleet.set_destroyer_count(0);
    fleet.set_troop_transport_count(0);
    fleet.set_etac_count(0);
    fleet.set_scout_count(1);
    fleet.set_rules_of_engagement(0);
    fleet.recompute_max_speed_from_composition();
    fleet.set_current_location_coords_raw(start);
    fleet.set_mission_aux_bytes([1, 0]);

    game_data
        .set_fleet_order(1, speed, Order::MoveOnly.to_raw(), target, None, None)
        .expect("move-only order should apply");
    game_data.fleets.records[0].raw[0x19] = 0x81;

    game_data
}

fn build_persistent_probe(case: PersistentMovementCase) -> nc_data::CoreGameData {
    let mut game_data = GameStateBuilder::new()
        .with_player_count(4)
        .with_year(3000)
        .build_initialized_baseline()
        .expect("baseline should build");

    let fleet = &mut game_data.fleets.records[0];
    fleet.set_battleship_count(0);
    fleet.set_cruiser_count(1);
    fleet.set_destroyer_count(0);
    fleet.set_troop_transport_count(0);
    fleet.set_etac_count(0);
    fleet.set_scout_count(0);
    fleet.set_rules_of_engagement(0);
    fleet.recompute_max_speed_from_composition();
    fleet.set_current_location_coords_raw(case.start);
    fleet.set_current_speed(3);
    fleet.set_standing_order_kind(case.order);
    fleet.set_standing_order_target_coords_raw(case.target);
    fleet.raw[0x0d] = 0x80;
    fleet.raw[0x0f] = 0x00;
    fleet.raw[0x19] = 0x81;

    match case.setup {
        PersistentProbeSetup::Patrol => {
            fleet.set_mission_aux_bytes([1, 0]);
        }
        PersistentProbeSetup::GuardStarbase => {
            fleet.set_mission_aux_bytes([1, 1]);
            let mut base = BaseRecord::new_zeroed();
            base.set_active_flag_raw(1);
            base.set_base_id_raw(1);
            base.set_owner_empire_raw(1);
            base.set_coords_raw(case.target);
            game_data.bases = BaseDat {
                records: vec![base],
            };
        }
        PersistentProbeSetup::GuardBlockade => {
            fleet.set_mission_aux_bytes([1, 0]);
            let target_world = &mut game_data.planets.records[4];
            target_world.set_coords_raw(case.target);
            target_world.set_owner_empire_slot_raw(2);
            target_world.set_ownership_status_raw(2);
            target_world.set_planet_name("Target");
        }
    }

    game_data
}

fn load_fixture(name: &str) -> nc_data::CoreGameData {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures")
        .join(name)
        .join("v1.5");
    nc_data::CoreGameData::load(&dir)
        .unwrap_or_else(|err| panic!("failed to load fixture {name}: {err}"))
}
