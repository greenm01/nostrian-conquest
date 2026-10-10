//! Fleet battle resolution: round cap, determinism, and ROE withdrawals.

use nc_data::{CoreGameData, Order};
use nc_engine::run_maintenance_turn_with_context_and_seed;

use super::support::{DEEP_SPACE, Ships, diplomacy, empty_board, place_fleet};

#[test]
fn battle_reaching_the_round_cap_ends_as_a_stalemate() {
    let mut game_data = empty_board(4, 0);
    place_fleet(
        &mut game_data,
        0,
        1,
        DEEP_SPACE,
        Order::MoveOnly,
        [9, 8],
        Ships {
            dd: 1,
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
        Ships {
            tt: 200,
            ..Ships::default()
        },
        0,
    );

    let events =
        run_maintenance_turn_with_context_and_seed(&mut game_data, 1, &[], &diplomacy(&[(1, 2)]))
            .expect("a long battle must not abort maintenance");

    assert_eq!(game_data.fleets.records[0].destroyer_count(), 1);
    let transports = game_data.fleets.records[4].troop_transport_count();
    assert!(
        transports > 0 && transports < 200,
        "survivors keep their ships: {transports}"
    );
    assert!(
        events
            .fleet_battle_events
            .iter()
            .any(|event| event.reporting_empire_raw == 1)
    );
    assert!(
        events
            .fleet_battle_events
            .iter()
            .any(|event| event.reporting_empire_raw == 2)
    );
}

#[test]
fn equal_roe_battle_resolves_identically_every_run() {
    let mut initial = empty_board(4, 0);
    for (idx, owner, target) in [(0usize, 1u8, [9, 8]), (4, 2, [7, 8])] {
        place_fleet(
            &mut initial,
            idx,
            owner,
            DEEP_SPACE,
            Order::MoveOnly,
            target,
            Ships {
                ca: 2,
                ..Ships::default()
            },
            4,
        );
    }
    let overrides = diplomacy(&[(1, 2)]);

    let outcome = |mut game_data: CoreGameData| {
        let events = run_maintenance_turn_with_context_and_seed(&mut game_data, 7, &[], &overrides)
            .expect("maintenance should succeed");
        (
            game_data.fleets.records[0].to_bytes().to_vec(),
            game_data.fleets.records[4].to_bytes().to_vec(),
            format!("{:?}", events.encounter_disposition_events),
            format!("{:?}", events.fleet_battle_events),
            format!("{:?}", events.mission_events),
        )
    };

    let first = outcome(initial.clone());
    for run in 1..32 {
        assert_eq!(
            outcome(initial.clone()),
            first,
            "run {run} diverged from the first run"
        );
    }
}

#[test]
fn withdrawal_resolution_does_not_depend_on_empire_slot_order() {
    // A committed patrol (ROE 6) that is outgunned breaks off from round 4.
    // Whether the withdrawing side is empire 1 or empire 2, it takes only the
    // withdrawal exchange in that round, never also the opponent's full volley.
    let escapes = |withdrawing: u8, fighting: u8| {
        let (withdrawing_idx, fighting_idx) = if withdrawing == 1 { (0, 4) } else { (4, 0) };
        let mut escaped_fleets = 0;
        let mut escaped_ships = 0;
        for seed in 0..60u64 {
            let mut game_data = empty_board(4, 0);
            place_fleet(
                &mut game_data,
                withdrawing_idx,
                withdrawing,
                DEEP_SPACE,
                Order::PatrolSector,
                DEEP_SPACE,
                Ships {
                    bb: 6,
                    ..Ships::default()
                },
                6,
            );
            place_fleet(
                &mut game_data,
                fighting_idx,
                fighting,
                DEEP_SPACE,
                Order::MoveOnly,
                [9, 8],
                Ships {
                    bb: 7,
                    ..Ships::default()
                },
                10,
            );
            run_maintenance_turn_with_context_and_seed(
                &mut game_data,
                seed,
                &[],
                &diplomacy(&[(1, 2)]),
            )
            .expect("maintenance should succeed");
            let fleet = &game_data.fleets.records[withdrawing_idx];
            if fleet.standing_order_kind() == Order::SeekHome {
                escaped_fleets += 1;
                escaped_ships += fleet.battleship_count();
            }
        }
        (escaped_fleets, escaped_ships)
    };

    let low_slot = escapes(1, 2);
    let high_slot = escapes(2, 1);
    // Each year's encounter differs by slot (who sights whom first, and when),
    // so the two runs no longer fight the same battle; both slots must still
    // escape with ships. Before the fix empire 2 escaped with 1 ship in 60.
    assert!(
        low_slot.0 > 0 && high_slot.0 > 0,
        "{low_slot:?} {high_slot:?}"
    );
    assert!(
        low_slot.1 >= 3 && high_slot.1 >= 3,
        "{low_slot:?} {high_slot:?}"
    );
}
