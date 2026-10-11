//! A Guard Starbase fleet lets go of its base when a battle takes it off
//! its order, as ECMAINT's does at the scatter (`1b05:25ab`) and at every
//! fleet delete (`2895:3d46`; classic probes 2026-10-11: a scattered escort
//! and a destroyed one both left its base with an escort word of 0).

use nc_data::{Coords, CoreGameData, Order, Real48, recompute_max_speed, set_mission_target};
use nc_engine::run_maintenance_turn_with_context_and_seed;

use super::support::{Ships, diplomacy, empty_board, place_fleet, place_starbase};

/// Empire 1's home world, where the base rests and the escort guards it.
const HOME: [u8; 2] = [16, 13];
/// An unowned world next door, the other world empire 1 may own.
const OTHER_PLANET_IDX: usize = 11;
const OTHER: [u8; 2] = [12, 12];

/// Empire 1's record 0 guards its base over `HOME` on `roe`; the base's
/// escort word is the fleet's number. Empire 1's other fleets are parked in
/// deep space, where they take no part. Empire 2's record 4, `attackers`
/// strong on ROE 10, sits in the same orbit and is at war with empire 1.
fn guarded_home(guard: Ships, roe: u8, attackers: Ships) -> CoreGameData {
    let mut game_data = empty_board(0, 0);
    place_starbase(&mut game_data, 1, HOME);
    for idx in 1..4 {
        place_fleet(
            &mut game_data,
            idx,
            1,
            [8, 8],
            Order::HoldPosition,
            [8, 8],
            Ships {
                ca: 1,
                ..Ships::default()
            },
            0,
        );
        game_data.fleets[idx].depth = Real48::ZERO;
    }
    place_fleet(
        &mut game_data,
        0,
        1,
        HOME,
        Order::GuardStarbase,
        HOME,
        guard,
        roe,
    );
    let fleet = &mut game_data.fleets[0];
    fleet.depth = Real48::ONE;
    fleet.target_depth = 1;
    set_mission_target(fleet, 1, 1);
    recompute_max_speed(fleet);
    game_data.bases[0].escort = game_data.fleets[0].fleet_number;
    place_fleet(
        &mut game_data,
        4,
        2,
        HOME,
        Order::GuardBlockadeWorld,
        HOME,
        attackers,
        10,
    );
    recompute_max_speed(&mut game_data.fleets[4]);
    game_data
}

fn run(game_data: &mut CoreGameData, seed: u64) {
    run_maintenance_turn_with_context_and_seed(game_data, seed, &[], &diplomacy(&[(1, 2)]))
        .expect("maintenance should succeed");
}

#[test]
fn a_guard_that_leaves_a_battle_lets_go_of_its_base() {
    // The guard is on ROE 0: it fights the compulsory exchange beside its
    // base and leaves. One battleship cannot take the base in the year, and
    // the base fights on alone.
    let mut left = 0;
    for seed in 0..24u64 {
        let mut game_data = guarded_home(
            Ships {
                ca: 1,
                ..Ships::default()
            },
            0,
            Ships {
                bb: 1,
                ..Ships::default()
            },
        );
        game_data.planets[OTHER_PLANET_IDX].owner.0 = 1;
        run(&mut game_data, seed);
        let guard = &game_data.fleets[0];
        if guard.order == Order::GuardStarbase || game_data.bases[0].base_id.0 == 0 {
            continue;
        }
        left += 1;
        assert_eq!(guard.order, Order::SeekHome, "seed {seed}");
        assert_eq!(guard.target.to_array(), OTHER, "seed {seed}");
        assert_eq!(game_data.bases[0].escort.0, 0, "seed {seed}");
    }
    assert!(left > 0);
}

#[test]
fn a_guard_with_nowhere_to_go_holds_and_lets_go_of_its_base() {
    // Classic's scatter finds no other world, leaves the order alone, and
    // still releases the base; Rust's guard holds in place.
    let mut left = 0;
    for seed in 0..24u64 {
        let mut game_data = guarded_home(
            Ships {
                ca: 1,
                ..Ships::default()
            },
            0,
            Ships {
                bb: 1,
                ..Ships::default()
            },
        );
        run(&mut game_data, seed);
        let guard = &game_data.fleets[0];
        if guard.order == Order::GuardStarbase || game_data.bases[0].base_id.0 == 0 {
            continue;
        }
        left += 1;
        assert_eq!(guard.order, Order::HoldPosition, "seed {seed}");
        assert_eq!(game_data.bases[0].escort.0, 0, "seed {seed}");
    }
    assert!(left > 0);
}

#[test]
fn a_guard_destroyed_in_a_battle_lets_go_of_its_base() {
    // A lone destroyer beside the base on ROE 10 dies to a cruiser's fire
    // before the base does; the base outlasts the cruiser.
    let mut destroyed = 0;
    for seed in 0..48u64 {
        let mut game_data = guarded_home(
            Ships {
                dd: 1,
                ..Ships::default()
            },
            10,
            Ships {
                ca: 1,
                ..Ships::default()
            },
        );
        run(&mut game_data, seed);
        let guard = &game_data.fleets[0];
        let ships = &guard.ships;
        let none_left = ships.destroyers + ships.cruisers + ships.battleships == 0;
        if !none_left || game_data.bases[0].base_id.0 == 0 {
            continue;
        }
        destroyed += 1;
        assert_eq!(game_data.bases[0].escort.0, 0, "seed {seed}");
        assert_eq!(guard.sector, Coords::from_array(HOME));
    }
    assert!(destroyed > 0);
}

#[test]
fn a_guard_that_fights_on_keeps_its_base() {
    // Four cruisers on ROE 10 beat a lone destroyer: nothing frees the base
    // while the guard holds its order.
    for seed in 0..8u64 {
        let mut game_data = guarded_home(
            Ships {
                ca: 4,
                ..Ships::default()
            },
            10,
            Ships {
                dd: 1,
                ..Ships::default()
            },
        );
        run(&mut game_data, seed);
        assert_eq!(
            game_data.fleets[0].order,
            Order::GuardStarbase,
            "seed {seed}"
        );
        assert_eq!(
            game_data.bases[0].escort, game_data.fleets[0].fleet_number,
            "seed {seed}"
        );
    }
}
