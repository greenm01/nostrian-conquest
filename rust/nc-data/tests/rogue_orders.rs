//! The yearly orders pass for rogue empires against classic ECMAINT
//! (`24a3:25f7`, `24a3:28fe`, `24a3:0dbc`; t070).
//!
//! Each case names the sector, depth, speed, and ownership it was probed
//! with on the move fixture and the orders classic gave (2026-10-11, DOSBox-X;
//! Rust agreed on every record). Empire 2 is the rogue empire: its home world
//! (4,13) has production 100, and it also owns (12,12) at production 2. Its
//! other three fleets are parked in deep space on Hold, spread out so the
//! rogue fleet consolidation finds nothing to merge.

mod common;

use common::move_scenario::*;
use nc_data::{Coords, CoreGameData, Fleet, GameRng, Order, PlayerMode, RNG_TAG_ORDERS, Real48};
use nc_engine::run_maintenance_turn_with_seed;

const ROGUE_HOME: [u8; 2] = [4, 13];
/// An unowned world next to (12,12).
const UNOWNED_WORLD: [u8; 2] = [12, 11];
const PARKING: [[u8; 2]; 4] = [[2, 2], [2, 4], [2, 6], [2, 8]];

/// Fleet `n` (0 to 3) of the rogue empire.
fn rogue(game_data: &mut CoreGameData, n: usize) -> &mut Fleet {
    &mut game_data.fleets[4 + n]
}

fn park(game_data: &mut CoreGameData, n: usize) {
    let fleet = rogue(game_data, n);
    set_ships(fleet, 0, 0, 1, 0, 0);
    set_fleet(
        fleet,
        PARKING[n],
        Real48::ZERO,
        Order::HoldPosition,
        PARKING[n],
        0,
        0,
        6,
    );
}

#[allow(clippy::too_many_arguments)]
fn set_fleet(
    fleet: &mut Fleet,
    sector: [u8; 2],
    depth: Real48,
    order: Order,
    target: [u8; 2],
    target_depth: u8,
    speed: u8,
    roe: u8,
) {
    fleet.sector = Coords::from_array(sector);
    fleet.x_fraction = Real48::HALF;
    fleet.y_fraction = Real48::HALF;
    fleet.depth = depth;
    fleet.order = order;
    fleet.target = Coords::from_array(target);
    fleet.target_depth = target_depth;
    fleet.speed = speed;
    fleet.rules_of_engagement = roe;
    fleet.mission_target = 0;
}

/// Empire 2 is rogue and owns (4,13) and (12,12), with its fleets parked.
fn rogue_scenario() -> CoreGameData {
    let mut game_data = scenario();
    game_data.players[1].mode = PlayerMode::Rogue;
    give_world(&mut game_data, NEAR_WORLD, 2);
    set_production(&mut game_data, NEAR_WORLD, 2);
    game_data.players[1].planet_count = 2;
    for n in 0..4 {
        park(&mut game_data, n);
    }
    game_data
}

/// As `rogue_scenario`, with the rogue's home world and (12,12) given up
/// and no world left.
fn worldless_scenario() -> CoreGameData {
    let mut game_data = rogue_scenario();
    planet_at(&mut game_data, ROGUE_HOME).owner.0 = 0;
    planet_at(&mut game_data, NEAR_WORLD).owner.0 = 0;
    game_data.players[1].planet_count = 0;
    game_data
}

// ── Fleets ──────────────────────────────────────────────────────────────

#[test]
fn a_rogue_fleet_on_any_order_heads_for_the_best_world_at_roe_0() {
    // Classic: after the year the Move fleet is at (5,13) on Seek Home.
    let mut game_data = rogue_scenario();
    let fleet = rogue(&mut game_data, 0);
    set_ships(fleet, 0, 1, 0, 0, 1);
    set_fleet(
        fleet,
        [8, 13],
        Real48::ZERO,
        Order::MoveOnly,
        [8, 13],
        0,
        3,
        6,
    );

    run(&mut game_data);

    let fleet = rogue(&mut game_data, 0);
    assert_eq!(fleet.order, Order::SeekHome);
    assert_eq!(fleet.target.to_array(), ROGUE_HOME);
    assert_eq!(fleet.target_depth, 1);
    assert_eq!(fleet.speed, 3);
    assert_eq!(fleet.rules_of_engagement, 0);
    assert_eq!(fleet.sector.to_array(), [5, 13]);
}

#[test]
fn a_rogue_fleet_weighs_distance_by_production() {
    // From orbit of the unowned (12,11), (12,12) is 3.0 away at production 2
    // and the home world 10.2 away at production 100.
    let mut game_data = rogue_scenario();
    let fleet = rogue(&mut game_data, 3);
    set_ships(fleet, 0, 0, 1, 0, 0);
    set_fleet(
        fleet,
        UNOWNED_WORLD,
        Real48::ONE,
        Order::GuardBlockadeWorld,
        UNOWNED_WORLD,
        1,
        0,
        6,
    );

    run(&mut game_data);

    let fleet = rogue(&mut game_data, 3);
    assert_eq!(fleet.order, Order::SeekHome);
    assert_eq!(fleet.target.to_array(), ROGUE_HOME);
    assert_eq!(fleet.speed, 6);
    assert_eq!(fleet.rules_of_engagement, 0);
    assert_eq!(fleet.sector.to_array(), [7, 12]);
}

#[test]
fn a_rogue_guard_blockade_fleet_at_home_keeps_its_order_and_takes_roe_10() {
    let mut game_data = rogue_scenario();
    let fleet = rogue(&mut game_data, 1);
    set_ships(fleet, 0, 1, 0, 0, 0);
    set_fleet(
        fleet,
        ROGUE_HOME,
        Real48::ONE,
        Order::GuardBlockadeWorld,
        ROGUE_HOME,
        1,
        0,
        3,
    );

    run(&mut game_data);

    let fleet = rogue(&mut game_data, 1);
    assert_eq!(fleet.order, Order::GuardBlockadeWorld);
    assert_eq!(fleet.speed, 0);
    assert_eq!(fleet.rules_of_engagement, 10);
}

#[test]
fn rogue_warships_in_orbit_of_an_owned_world_guard_it_whatever_they_were_doing() {
    // A cruiser leaving home on a Move order, and one on patrol over
    // (12,12), both stop on Guard/Blockade at ROE 10.
    let mut game_data = rogue_scenario();
    for (n, sector, order, target, speed) in [
        (0, ROGUE_HOME, Order::MoveOnly, [8, 13], 3),
        (1, NEAR_WORLD, Order::PatrolSector, [12, 15], 2),
    ] {
        let fleet = rogue(&mut game_data, n);
        set_ships(fleet, 0, 1, 0, 0, 0);
        set_fleet(fleet, sector, Real48::ONE, order, target, 0, speed, 6);
    }

    run(&mut game_data);

    for (n, world) in [(0, ROGUE_HOME), (1, NEAR_WORLD)] {
        let fleet = rogue(&mut game_data, n);
        assert_eq!(fleet.order, Order::GuardBlockadeWorld, "fleet {n}");
        assert_eq!(fleet.target.to_array(), world);
        assert_eq!(fleet.target_depth, 1);
        assert_eq!(fleet.speed, 0);
        assert_eq!(fleet.rules_of_engagement, 10);
    }
}

#[test]
fn a_rogue_fleet_without_warships_in_orbit_keeps_its_order() {
    // A lone ETAC on Hold over (12,12): only its target is set.
    let mut game_data = rogue_scenario();
    let fleet = rogue(&mut game_data, 2);
    set_ships(fleet, 0, 0, 0, 0, 1);
    set_fleet(
        fleet,
        NEAR_WORLD,
        Real48::ONE,
        Order::HoldPosition,
        NEAR_WORLD,
        0,
        0,
        0,
    );

    run(&mut game_data);

    let fleet = rogue(&mut game_data, 2);
    assert_eq!(fleet.order, Order::HoldPosition);
    assert_eq!(fleet.target.to_array(), NEAR_WORLD);
    assert_eq!(fleet.target_depth, 1);
}

// ── Guard Starbase ──────────────────────────────────────────────────────

/// The probed starbase cases: a rogue cruiser on Guard/Blockade over each of
/// (4,13), (12,12), and the rogue's third world (12,11), with a base under
/// each: base 1 already escorted by another fleet, base 2 about to lift
/// (target depth 0), and base 3 at rest but flagged as moving.
fn starbase_scenario() -> CoreGameData {
    let mut game_data = rogue_scenario();
    give_world(&mut game_data, UNOWNED_WORLD, 2);
    set_production(&mut game_data, UNOWNED_WORLD, 2);
    game_data.players[1].planet_count = 3;
    for (n, sector) in [ROGUE_HOME, NEAR_WORLD, UNOWNED_WORLD]
        .into_iter()
        .enumerate()
    {
        let fleet = rogue(&mut game_data, n);
        set_ships(fleet, 0, 1, 0, 0, 0);
        set_fleet(
            fleet,
            sector,
            Real48::ONE,
            Order::GuardBlockadeWorld,
            sector,
            1,
            0,
            6,
        );
    }
    let third = rogue(&mut game_data, 2).fleet_number.0;
    add_base(&mut game_data, 2, ROGUE_HOME, Real48::ONE, 1, third);
    add_base(&mut game_data, 2, NEAR_WORLD, Real48::ONE, 0, 0);
    let moving = add_base(&mut game_data, 2, UNOWNED_WORLD, Real48::ONE, 1, 0);
    game_data.bases[moving].moving = true;
    game_data
}

#[test]
fn a_rogue_guard_blockade_fleet_takes_an_unescorted_base_at_rest() {
    // The moving flag does not matter, only the position and target.
    let mut game_data = starbase_scenario();
    let base_id = game_data.bases[2].base_id.0;

    run(&mut game_data);

    let fleet = rogue(&mut game_data, 2).clone();
    assert_eq!(fleet.order, Order::GuardStarbase);
    assert_eq!(fleet.mission_target, base_id);
    assert_eq!(fleet.rules_of_engagement, 10);
    assert_eq!(game_data.bases[2].escort.0, fleet.fleet_number.0);
}

#[test]
fn a_rogue_guard_blockade_fleet_leaves_an_escorted_base_alone() {
    let mut game_data = starbase_scenario();

    run(&mut game_data);

    let fleet = rogue(&mut game_data, 0).clone();
    assert_eq!(fleet.order, Order::GuardBlockadeWorld);
    assert_ne!(game_data.bases[0].escort.0, fleet.fleet_number.0);
}

#[test]
fn a_rogue_guard_blockade_fleet_leaves_a_base_that_is_about_to_lift_alone() {
    // The fleet's step sees the base before the base's own orders turn it
    // back to orbit, so the base is not at rest yet.
    let mut game_data = starbase_scenario();

    run(&mut game_data);

    assert_eq!(rogue(&mut game_data, 1).order, Order::GuardBlockadeWorld);
    assert_eq!(game_data.bases[1].escort.0, 0);
    assert_eq!(game_data.bases[1].target_depth, 1);
}

#[test]
fn a_rogue_guard_starbase_fleet_that_is_re_ordered_lets_go_of_its_base() {
    // The base hangs over a world the rogue does not own, with fleet 2 as
    // its escort. The fleet is sent home and the base's escort word cleared
    // (`2895:30f4`); classic leaves the fleet's 0x23 alone.
    let mut game_data = rogue_scenario();
    let fleet = rogue(&mut game_data, 1);
    set_ships(fleet, 0, 1, 0, 0, 0);
    set_fleet(
        fleet,
        UNOWNED_WORLD,
        Real48::ONE,
        Order::GuardStarbase,
        UNOWNED_WORLD,
        1,
        0,
        6,
    );
    fleet.mission_target = 1;
    let number = fleet.fleet_number.0;
    add_base(&mut game_data, 2, UNOWNED_WORLD, Real48::ONE, 1, number);

    run(&mut game_data);

    let fleet = rogue(&mut game_data, 1);
    assert_eq!(fleet.order, Order::SeekHome);
    assert_eq!(fleet.target.to_array(), ROGUE_HOME);
    assert_eq!(fleet.mission_target, 1);
    assert_eq!(game_data.bases[0].escort.0, 0);
}

#[test]
fn releasing_an_escort_clears_only_the_words_of_the_fleet_that_guards() {
    let mut game_data = rogue_scenario();
    let number = rogue(&mut game_data, 0).fleet_number.0;
    set_fleet(
        rogue(&mut game_data, 0),
        NEAR_WORLD,
        Real48::ONE,
        Order::GuardStarbase,
        NEAR_WORLD,
        1,
        0,
        6,
    );
    // The fleet's base, another base over the same world in deep space, one
    // elsewhere, and one escorted by another fleet.
    let here = add_base(&mut game_data, 2, NEAR_WORLD, Real48::ONE, 1, number);
    let below = add_base(&mut game_data, 2, NEAR_WORLD, Real48::ZERO, 0, number);
    let away = add_base(&mut game_data, 2, ROGUE_HOME, Real48::ONE, 1, number);
    let other = add_base(&mut game_data, 2, NEAR_WORLD, Real48::ONE, 1, number + 1);

    game_data.release_starbase_escort(4);

    assert_eq!(game_data.bases[here].escort.0, 0);
    assert_eq!(game_data.bases[below].escort.0, number);
    assert_eq!(game_data.bases[away].escort.0, number);
    assert_eq!(game_data.bases[other].escort.0, number + 1);

    // On any other order the fleet holds on to nothing.
    rogue(&mut game_data, 0).order = Order::GuardBlockadeWorld;
    game_data.bases[here].escort.0 = number;
    game_data.release_starbase_escort(4);
    assert_eq!(game_data.bases[here].escort.0, number);
}

// ── Starbases ───────────────────────────────────────────────────────────

#[test]
fn a_rogue_base_in_orbit_of_its_world_stays_and_one_elsewhere_sets_out() {
    // Classic: bases over (12,12) and the home world stay put; a base in deep
    // space at (9,13) and one over the unowned (12,11) head for the home
    // world.
    let mut game_data = rogue_scenario();
    let at_home = add_base(&mut game_data, 2, ROGUE_HOME, Real48::ONE, 1, 0);
    let over_world = add_base(&mut game_data, 2, NEAR_WORLD, Real48::ONE, 1, 0);
    let in_space = add_base(&mut game_data, 2, [9, 13], Real48::ZERO, 0, 0);
    let lost = add_base(&mut game_data, 2, UNOWNED_WORLD, Real48::ONE, 1, 0);

    run(&mut game_data);

    for (index, world) in [(at_home, ROGUE_HOME), (over_world, NEAR_WORLD)] {
        let base = &game_data.bases[index];
        assert_eq!(base.target.to_array(), world);
        assert_eq!(base.target_depth, 1);
        assert!(!base.moving, "base {index}");
    }
    for index in [in_space, lost] {
        let base = &game_data.bases[index];
        assert_eq!(base.target.to_array(), ROGUE_HOME, "base {index}");
        assert_eq!(base.target_depth, 1);
        assert!(base.moving, "base {index}");
    }
}

#[test]
fn a_rogue_base_measures_from_the_sector_corner() {
    // From the corner (13,12) is 1.707 from both (12,11) and (12,12), and
    // the earlier world wins; from its centre (12,12) would be nearer.
    let mut game_data = rogue_scenario();
    planet_at(&mut game_data, ROGUE_HOME).owner.0 = 0;
    give_world(&mut game_data, UNOWNED_WORLD, 2);
    set_production(&mut game_data, UNOWNED_WORLD, 2);
    let base = add_base(&mut game_data, 2, [13, 12], Real48::ZERO, 0, 0);

    run(&mut game_data);

    assert_eq!(game_data.bases[base].target.to_array(), UNOWNED_WORLD);
    assert!(game_data.bases[base].moving);
}

// ── No world ────────────────────────────────────────────────────────────

fn armed_idle(game_data: &mut CoreGameData, n: usize, sector: [u8; 2]) {
    let fleet = rogue(game_data, n);
    set_ships(fleet, 0, 1, 0, 0, 0);
    set_fleet(
        fleet,
        sector,
        Real48::ZERO,
        Order::HoldPosition,
        sector,
        0,
        0,
        3,
    );
}

#[test]
fn a_rogue_with_no_world_patrols_with_its_warships() {
    // Classic, over ten runs: ROE 10, Patrol (3), a deep-space target in
    // 1..=18 on both axes (nine sectors for each whole root of the four
    // players) other than the fleet's own sector, and the fleet's maximum
    // speed. A fleet bound elsewhere keeps its target.
    let mut game_data = worldless_scenario();
    armed_idle(&mut game_data, 0, [8, 13]);
    let bound = rogue(&mut game_data, 1);
    set_ships(bound, 0, 0, 2, 0, 0);
    set_fleet(
        bound,
        [6, 6],
        Real48::ZERO,
        Order::MoveOnly,
        [10, 10],
        0,
        3,
        6,
    );

    run_maintenance_turn_with_seed(&mut game_data, 7).unwrap();

    let idle = rogue(&mut game_data, 0);
    assert_eq!(idle.order, Order::PatrolSector);
    assert_eq!(idle.rules_of_engagement, 10);
    assert_eq!(idle.target_depth, 0);
    assert_eq!(idle.speed, idle.max_speed);
    let [x, y] = idle.target.to_array();
    assert!((1..=18).contains(&x) && (1..=18).contains(&y), "{x},{y}");
    assert_ne!([x, y], [8, 13]);
    let bound = rogue(&mut game_data, 1);
    assert_eq!(bound.order, Order::PatrolSector);
    assert_eq!(bound.rules_of_engagement, 10);
    assert_eq!(bound.target.to_array(), [10, 10]);
}

#[test]
fn a_rogue_with_no_world_draws_its_targets_from_the_campaign_seed() {
    let targets = |seed| {
        let mut game_data = worldless_scenario();
        armed_idle(&mut game_data, 0, [8, 13]);
        run_maintenance_turn_with_seed(&mut game_data, seed).unwrap();
        rogue(&mut game_data, 0).target.to_array()
    };
    assert_eq!(targets(11), targets(11));
    let drawn: std::collections::HashSet<_> = (1..=6).map(targets).collect();
    assert!(drawn.len() > 1, "{drawn:?}");
}

#[test]
fn a_rogue_with_no_world_redraws_a_target_that_is_its_own_sector() {
    // The draw is two values below 18 each; the fleet sits on the first
    // draw of the seed, so it must take the next.
    let seed = 3;
    // The pass draws in the year it is about to play.
    let year = u64::from(worldless_scenario().conquest.game_year) + 1;
    let mut rng = GameRng::from_context(seed, RNG_TAG_ORDERS, &[year, 2]);
    let first = [rng.below(18) as u8 + 1, rng.below(18) as u8 + 1];
    let second = [rng.below(18) as u8 + 1, rng.below(18) as u8 + 1];
    assert_ne!(first, second);

    let mut game_data = worldless_scenario();
    armed_idle(&mut game_data, 0, first);
    run_maintenance_turn_with_seed(&mut game_data, seed).unwrap();

    assert_eq!(rogue(&mut game_data, 0).target.to_array(), second);
}

#[test]
fn a_rogue_with_no_world_disbands_unarmed_fleets_and_starbases() {
    let mut game_data = worldless_scenario();
    armed_idle(&mut game_data, 0, [8, 13]);
    // Fleet 2 is an ETAC, fleet 3 a scout.
    for (n, scouts, etacs) in [(1, 0, 1), (2, 1, 0)] {
        let fleet = rogue(&mut game_data, n);
        set_ships(fleet, 0, 0, 0, scouts, etacs);
    }
    add_base(&mut game_data, 2, ROGUE_HOME, Real48::ONE, 1, 0);
    let fleets = game_data.fleets.len();

    run(&mut game_data);

    assert_eq!(game_data.fleets.len(), fleets - 2);
    let numbers: Vec<u16> = game_data
        .fleets
        .iter()
        .filter(|fleet| fleet.owner.0 == 2)
        .map(|fleet| fleet.fleet_number.0)
        .collect();
    assert_eq!(numbers, [1, 4]);
    assert_eq!(game_data.bases[0].base_id.0, 0);
    assert_eq!(game_data.players[1].bases_first.0, 0);
}

#[test]
fn a_rogue_with_no_fleet_keeps_its_starbases() {
    // Empire 4 has no fleet: classic's patrol pass returns before it reaches
    // the bases. (The fixture's last four records are empire 4's.)
    let mut game_data = scenario();
    game_data.fleets.truncate(12);
    game_data.players[3].fleets_first.0 = 0;
    game_data.players[3].fleets_last.0 = 0;
    game_data.players[3].mode = PlayerMode::Rogue;
    game_data.players[3].planet_count = 0;
    planet_at(&mut game_data, [13, 5]).owner.0 = 0;
    add_base(&mut game_data, 4, [13, 5], Real48::ONE, 1, 0);

    run(&mut game_data);

    assert_eq!(game_data.bases[0].base_id.0, 1);
}

#[test]
fn the_rogue_planet_count_is_the_one_the_last_run_saved() {
    // Empire 2 owns its home world, but PLAYER 0x50 says none: classic takes
    // the patrol branch and disbands the ETAC fleet.
    let mut game_data = rogue_scenario();
    game_data.players[1].planet_count = 0;
    let fleet = rogue(&mut game_data, 1);
    set_ships(fleet, 0, 0, 0, 0, 1);

    run(&mut game_data);

    assert!(
        game_data
            .fleets
            .iter()
            .all(|fleet| fleet.owner.0 != 2 || fleet.ships.etacs == 0)
    );
}
