//! The yearly orders pass for rogue empires against classic ECMAINT
//! (`24a3:25f7`, `24a3:28fe`, `24a3:0dbc`; t070, and t094 for the escort
//! release at a fleet delete, unarmed guards, and the worldless rogue).
//!
//! Each case names the sector, depth, speed, and ownership it was probed
//! with on the move fixture and the orders classic gave (2026-10-11, DOSBox-X;
//! Rust agreed on every record). Empire 2 is the rogue empire: its home world
//! (4,13) has production 100, and it also owns (12,12) at production 2. Its
//! other three fleets are parked in deep space on Hold, spread out so the
//! merge pass finds nothing to merge.

mod common;

use common::move_scenario::*;
use nc_data::{
    Coords, CoreGameData, DiplomaticRelation, EscapeEpisode, Fleet, GameRng, Order, PlayerMode,
    RNG_TAG_ORDERS, Real48, set_diplomatic_relation_toward, set_empire_name, set_player_handle,
    stored_diplomatic_relation,
};
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

/// Empty every rogue fleet but `keep`; maintenance culls the empty ones
/// before anything moves, so they cannot fly home and join the others.
fn keep_only(game_data: &mut CoreGameData, keep: &[usize]) {
    for n in (0..4).filter(|n| !keep.contains(n)) {
        set_ships(rogue(game_data, n), 0, 0, 0, 0, 0);
    }
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

#[test]
fn a_rogue_guard_starbase_fleet_merged_away_lets_go_of_its_base() {
    // Classic merge probe: fleet 1 on Guard/Blockade and fleet 2, the escort
    // of the home base, share the home world. Fleet 2 merges into fleet 1
    // (`2895:4a6b`) and its delete frees the base (`2895:3d0c`); the merged
    // fleet, at rest on Guard/Blockade, then takes the base. After the year:
    // one fleet of two destroyers on Guard Starbase, the base its escort.
    let mut game_data = rogue_scenario();
    for (n, order) in [(0, Order::GuardBlockadeWorld), (1, Order::GuardStarbase)] {
        let fleet = rogue(&mut game_data, n);
        set_ships(fleet, 0, 0, 1, 0, 0);
        set_fleet(fleet, ROGUE_HOME, Real48::ONE, order, ROGUE_HOME, 1, 0, 10);
    }
    keep_only(&mut game_data, &[0, 1]);
    rogue(&mut game_data, 1).mission_target = 1;
    let escort = rogue(&mut game_data, 1).fleet_number.0;
    let base = add_base(&mut game_data, 2, ROGUE_HOME, Real48::ONE, 1, escort);

    run(&mut game_data);

    let fleets: Vec<&Fleet> = game_data
        .fleets
        .iter()
        .filter(|fleet| fleet.owner.0 == 2)
        .collect();
    assert_eq!(fleets.len(), 1);
    let fleet = fleets[0];
    assert_ne!(fleet.fleet_number.0, escort);
    assert_eq!(fleet.ships.destroyers, 2);
    assert_eq!(fleet.order, Order::GuardStarbase);
    assert_eq!(fleet.mission_target, 1);
    assert_eq!(game_data.bases[base].escort, fleet.fleet_number);
}

#[test]
fn a_rogue_fleet_without_warships_on_guard_blockade_takes_a_base_and_keeps_its_order() {
    // Classic ETAC probe, three years: the ECMAINT-played empires' orders
    // are not checked, so the ETAC-only fleet takes the base on Guard
    // Starbase with ROE 0 (a fleet without warships); every later year the
    // pass frees the base again (`24a3:26d1`) and the fleet keeps its order.
    let mut game_data = rogue_scenario();
    let fleet = rogue(&mut game_data, 1);
    set_ships(fleet, 0, 0, 0, 0, 1);
    set_fleet(
        fleet,
        ROGUE_HOME,
        Real48::ONE,
        Order::GuardBlockadeWorld,
        ROGUE_HOME,
        1,
        0,
        6,
    );
    keep_only(&mut game_data, &[1]);
    let base = add_base(&mut game_data, 2, ROGUE_HOME, Real48::ONE, 1, 0);

    run(&mut game_data);

    let fleet = rogue(&mut game_data, 0).clone();
    assert_eq!(fleet.order, Order::GuardStarbase);
    assert_eq!(fleet.mission_target, 1);
    assert_eq!(fleet.rules_of_engagement, 0);
    assert_eq!(game_data.bases[base].escort, fleet.fleet_number);

    for _ in 0..2 {
        run(&mut game_data);
        let fleet = rogue(&mut game_data, 0);
        assert_eq!(fleet.order, Order::GuardStarbase);
        assert_eq!(fleet.mission_target, 1);
        assert_eq!(game_data.bases[base].escort.0, 0);
    }
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

// ── Merges ──────────────────────────────────────────────────────────────
//
// The merge pass (`24a3:216f`, probed 2026-10-11 under DOSBox-X on the move
// fixture with Rust agreeing on every record) joins co-located fleets only in
// orbit of the playing empire's own worlds, before the empire's orders.

/// Fleet `n` as `destroyers` destroyers on Hold at the center of `sector`.
fn hold_destroyers(
    game_data: &mut CoreGameData,
    n: usize,
    sector: [u8; 2],
    depth: Real48,
    destroyers: u16,
) {
    let fleet = rogue(game_data, n);
    set_ships(fleet, 0, 0, destroyers, 0, 0);
    let target_depth = u8::from(depth == Real48::ONE);
    set_fleet(
        fleet,
        sector,
        depth,
        Order::HoldPosition,
        sector,
        target_depth,
        0,
        3,
    );
}

/// Empire 2's fleets.
fn rogue_fleets(game_data: &CoreGameData) -> Vec<&Fleet> {
    game_data
        .fleets
        .iter()
        .filter(|fleet| fleet.owner.0 == 2)
        .collect()
}

/// The fleet of empire 2 with this number, if it is still there.
fn rogue_fleet_numbered(game_data: &CoreGameData, number: u16) -> Option<&Fleet> {
    rogue_fleets(game_data)
        .into_iter()
        .find(|fleet| fleet.fleet_number.0 == number)
}

#[test]
fn rogue_fleets_in_orbit_of_their_own_world_merge_into_the_first_whatever_their_orders() {
    // A Move fleet under way, a Hold fleet, and an ETAC on Guard/Blockade
    // over the home world became one fleet, the first; the fourth fleet,
    // elsewhere, stayed its own.
    let mut game_data = rogue_scenario();
    let mover = rogue(&mut game_data, 0);
    set_ships(mover, 0, 1, 0, 0, 0);
    set_fleet(
        mover,
        ROGUE_HOME,
        Real48::ONE,
        Order::MoveOnly,
        [8, 13],
        0,
        3,
        3,
    );
    hold_destroyers(&mut game_data, 1, ROGUE_HOME, Real48::ONE, 1);
    let etac = rogue(&mut game_data, 2);
    set_ships(etac, 0, 0, 0, 0, 1);
    set_fleet(
        etac,
        ROGUE_HOME,
        Real48::ONE,
        Order::GuardBlockadeWorld,
        ROGUE_HOME,
        1,
        0,
        0,
    );
    let first = rogue(&mut game_data, 0).fleet_number.0;

    run(&mut game_data);

    assert_eq!(rogue_fleets(&game_data).len(), 2);
    let merged = rogue_fleet_numbered(&game_data, first).expect("the first fleet hosts");
    let ships = &merged.ships;
    assert_eq!((ships.cruisers, ships.destroyers, ships.etacs), (1, 1, 1));
}

#[test]
fn rogue_fleets_elsewhere_do_not_merge() {
    // Two fleets over a world the rogue does not own, in deep space, or in
    // the deep layer of its home sector stayed apart.
    for (sector, depth) in [
        (UNOWNED_WORLD, Real48::ONE),
        ([8, 13], Real48::ZERO),
        (ROGUE_HOME, Real48::ZERO),
    ] {
        let mut game_data = rogue_scenario();
        hold_destroyers(&mut game_data, 0, sector, depth, 1);
        hold_destroyers(&mut game_data, 1, sector, depth, 1);

        run(&mut game_data);

        assert_eq!(rogue_fleets(&game_data).len(), 4, "{sector:?} at {depth:?}");
    }
}

#[test]
fn two_rogue_fleets_merge_only_up_to_3000_ships() {
    // 1500 and 1500 destroyers merged; 1500 and 1501 stayed apart.
    for (second, merged) in [(1500, true), (1501, false)] {
        let mut game_data = rogue_scenario();
        hold_destroyers(&mut game_data, 0, ROGUE_HOME, Real48::ONE, 1500);
        hold_destroyers(&mut game_data, 1, ROGUE_HOME, Real48::ONE, second);

        run(&mut game_data);

        assert_eq!(
            rogue_fleets(&game_data).len(),
            if merged { 3 } else { 4 },
            "{second}"
        );
    }
}

/// A Guard Starbase fleet of `destroyers` on `base_id`, and Hold destroyers
/// beside it, over `sector`; `base_target_depth` 0 puts the base about to lift.
fn guard_and_hold_pair(
    game_data: &mut CoreGameData,
    guard: usize,
    hold: usize,
    sector: [u8; 2],
    base_id: u16,
    base_target_depth: u8,
) {
    let fleet = rogue(game_data, guard);
    set_ships(fleet, 0, 0, 1, 0, 0);
    set_fleet(
        fleet,
        sector,
        Real48::ONE,
        Order::GuardStarbase,
        sector,
        1,
        0,
        10,
    );
    fleet.mission_target = base_id;
    let escort = fleet.fleet_number.0;
    hold_destroyers(game_data, hold, sector, Real48::ONE, 1);
    add_base(game_data, 2, sector, Real48::ONE, base_target_depth, escort);
}

#[test]
fn a_rogue_guard_starbase_fleet_hosts_over_a_base_at_rest_but_is_taken_in_over_a_lifting_one() {
    // Home: the guard's base rests, so the guard hosts and takes the Hold
    // fleet in. (12,12): the base is about to lift, so the guard cannot host
    // and the Hold fleet takes it in.
    let mut game_data = rogue_scenario();
    guard_and_hold_pair(&mut game_data, 0, 1, ROGUE_HOME, 1, 1);
    guard_and_hold_pair(&mut game_data, 2, 3, NEAR_WORLD, 2, 0);
    let numbers: Vec<u16> = (0..4)
        .map(|n| rogue(&mut game_data, n).fleet_number.0)
        .collect();

    run(&mut game_data);

    assert_eq!(rogue_fleets(&game_data).len(), 2);
    let host = rogue_fleet_numbered(&game_data, numbers[0]).expect("the resting guard hosts");
    assert_eq!(host.ships.destroyers, 2);
    let host = rogue_fleet_numbered(&game_data, numbers[3]).expect("the Hold fleet hosts");
    assert_eq!(host.ships.destroyers, 2);
}

#[test]
fn a_rogue_guard_starbase_fleet_that_names_no_base_never_hosts() {
    // Owner-approved deviation: a guard with no base cannot host, even
    // when classic's stale flag would permit it. The Hold fleet hosts.
    let mut game_data = rogue_scenario();
    let guard = rogue(&mut game_data, 0);
    set_ships(guard, 0, 0, 1, 0, 0);
    set_fleet(
        guard,
        ROGUE_HOME,
        Real48::ONE,
        Order::GuardStarbase,
        ROGUE_HOME,
        1,
        0,
        10,
    );
    guard.mission_target = 9;
    hold_destroyers(&mut game_data, 1, ROGUE_HOME, Real48::ONE, 1);
    let first = rogue(&mut game_data, 0).fleet_number.0;
    let hold = rogue(&mut game_data, 1).fleet_number.0;
    let first_record = rogue(&mut game_data, 0).record;
    let hold_record = rogue(&mut game_data, 1).record;
    rogue(&mut game_data, 0).next = hold_record;
    rogue(&mut game_data, 1).prev = first_record;
    game_data.players[1].fleets_first = first_record;
    game_data.players[1].fleets_last = hold_record;
    keep_only(&mut game_data, &[0, 1]);

    run(&mut game_data);

    assert!(rogue_fleet_numbered(&game_data, first).is_none());
    let host = rogue_fleet_numbered(&game_data, hold).expect("the Hold fleet hosts");
    assert_eq!(host.ships.destroyers, 2);
    assert_eq!(game_data.players[1].fleets_first, host.record);
    assert_eq!(game_data.players[1].fleets_last, host.record);
    assert_eq!(host.prev.0, 0);
    assert_eq!(host.next.0, 0);

    // The result is independent of an earlier guard over a lifting base:
    // neither guard can host, and the Hold fleet takes both in.
    let mut game_data = rogue_scenario();
    guard_and_hold_pair(&mut game_data, 0, 2, ROGUE_HOME, 1, 0);
    let nameless = rogue(&mut game_data, 1);
    set_ships(nameless, 0, 0, 1, 0, 0);
    set_fleet(
        nameless,
        ROGUE_HOME,
        Real48::ONE,
        Order::GuardStarbase,
        ROGUE_HOME,
        1,
        0,
        10,
    );
    nameless.mission_target = 9;
    let numbers: Vec<u16> = (0..3)
        .map(|n| rogue(&mut game_data, n).fleet_number.0)
        .collect();

    run(&mut game_data);

    assert!(rogue_fleet_numbered(&game_data, numbers[0]).is_none());
    assert!(rogue_fleet_numbered(&game_data, numbers[1]).is_none());
    let host = rogue_fleet_numbered(&game_data, numbers[2]).expect("the Hold fleet hosts");
    assert_eq!(host.ships.destroyers, 3);
}

/// Empire 2 as a human empire on autopilot, owning the same two worlds.
fn autopilot_scenario() -> CoreGameData {
    let mut game_data = rogue_scenario();
    game_data.players[1].mode = PlayerMode::Human;
    game_data.players[1].autopilot = true;
    game_data
}

#[test]
fn an_autopilot_merges_hold_and_guard_blockade_fleets_at_rest_in_orbit_of_its_worlds() {
    // Two Hold fleets and a Guard/Blockade fleet at rest over the home world
    // became the first; a Move fleet over (12,12) with a Hold fleet, and a
    // Guard/Blockade fleet that is under way, did not merge.
    let mut game_data = autopilot_scenario();
    hold_destroyers(&mut game_data, 0, ROGUE_HOME, Real48::ONE, 1);
    hold_destroyers(&mut game_data, 1, ROGUE_HOME, Real48::ONE, 1);
    let guard = rogue(&mut game_data, 2);
    set_ships(guard, 0, 1, 0, 0, 0);
    set_fleet(
        guard,
        ROGUE_HOME,
        Real48::ONE,
        Order::GuardBlockadeWorld,
        ROGUE_HOME,
        1,
        0,
        6,
    );
    let first = rogue(&mut game_data, 0).fleet_number.0;

    run(&mut game_data);

    assert_eq!(rogue_fleets(&game_data).len(), 2);
    let merged = rogue_fleet_numbered(&game_data, first).expect("the first fleet hosts");
    assert_eq!((merged.ships.cruisers, merged.ships.destroyers), (1, 2));
}

#[test]
fn an_autopilot_leaves_other_orders_and_fleets_under_way_unmerged() {
    // Over (12,12): a Move fleet at rest beside a Hold fleet, and over the
    // home world a Guard/Blockade fleet at rest beside one that is setting
    // out. Neither pair merged.
    let mut game_data = autopilot_scenario();
    let mover = rogue(&mut game_data, 0);
    set_ships(mover, 0, 0, 1, 0, 0);
    set_fleet(
        mover,
        NEAR_WORLD,
        Real48::ONE,
        Order::MoveOnly,
        NEAR_WORLD,
        1,
        0,
        3,
    );
    hold_destroyers(&mut game_data, 1, NEAR_WORLD, Real48::ONE, 1);
    let at_rest = rogue(&mut game_data, 2);
    set_ships(at_rest, 0, 1, 0, 0, 0);
    set_fleet(
        at_rest,
        ROGUE_HOME,
        Real48::ONE,
        Order::GuardBlockadeWorld,
        ROGUE_HOME,
        1,
        0,
        6,
    );
    let setting_out = rogue(&mut game_data, 3);
    set_ships(setting_out, 0, 0, 1, 0, 0);
    set_fleet(
        setting_out,
        ROGUE_HOME,
        Real48::ONE,
        Order::GuardBlockadeWorld,
        NEAR_WORLD,
        1,
        2,
        6,
    );

    run(&mut game_data);

    assert_eq!(rogue_fleets(&game_data).len(), 4);
}

#[test]
fn an_autopilot_guard_starbase_fleet_hosts_but_is_never_taken_in() {
    // Home: a Hold fleet and then a guard over a base at rest; the guard
    // hosts, as the second fleet, and takes the first in. (12,12): a guard
    // whose base is about to lift, then a Hold fleet; the guard cannot host
    // and cannot be taken in, so they stay apart.
    let mut game_data = autopilot_scenario();
    guard_and_hold_pair(&mut game_data, 1, 0, ROGUE_HOME, 1, 1);
    guard_and_hold_pair(&mut game_data, 2, 3, NEAR_WORLD, 2, 0);
    let numbers: Vec<u16> = (0..4)
        .map(|n| rogue(&mut game_data, n).fleet_number.0)
        .collect();

    run(&mut game_data);

    assert_eq!(rogue_fleets(&game_data).len(), 3);
    assert!(rogue_fleet_numbered(&game_data, numbers[0]).is_none());
    let host = rogue_fleet_numbered(&game_data, numbers[1]).expect("the guard hosts");
    assert_eq!(host.ships.destroyers, 2);
    assert!(rogue_fleet_numbered(&game_data, numbers[2]).is_some());
    assert!(rogue_fleet_numbered(&game_data, numbers[3]).is_some());
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

#[test]
fn a_rogue_with_no_world_marks_every_human_empire_its_enemy() {
    // Classic, with and without a fleet (`24a3:0c2a`): the rogue's stance
    // toward empire 1, the human, is Enemy after the year; empires 3 and 4,
    // in civil disorder, stay neutral.
    for armed in [true, false] {
        let mut game_data = worldless_scenario();
        if armed {
            armed_idle(&mut game_data, 0, [8, 13]);
        } else {
            game_data.fleets.retain(|fleet| fleet.owner.0 != 2);
            game_data.players[1].fleets_first.0 = 0;
            game_data.players[1].fleets_last.0 = 0;
        }

        run(&mut game_data);

        let rogue = &game_data.players[1];
        assert_eq!(rogue.mode, PlayerMode::Rogue);
        assert_eq!(
            stored_diplomatic_relation(rogue, 1),
            Some(DiplomaticRelation::Enemy)
        );
        for other in [3, 4] {
            assert_eq!(
                stored_diplomatic_relation(rogue, other),
                Some(DiplomaticRelation::Neutral),
                "armed {armed}, empire {other}"
            );
        }
    }
}

#[test]
fn a_rogue_that_loses_its_last_world_goes_on_patrolling() {
    // Both worlds fall during the year the saved count (PLAYER 0x50) still
    // reads 2. Classic leaves the empire rogue; the next year's pass finds a
    // count of 0 and sends its warships on patrol. Rust used to turn the
    // empire into civil disorder at the end of the first year.
    let mut game_data = rogue_scenario();
    armed_idle(&mut game_data, 0, [8, 13]);
    planet_at(&mut game_data, ROGUE_HOME).owner.0 = 0;
    planet_at(&mut game_data, NEAR_WORLD).owner.0 = 0;

    run(&mut game_data);

    assert_eq!(game_data.players[1].mode, PlayerMode::Rogue);
    assert_eq!(game_data.players[1].planet_count, 0);
    assert_eq!(rogue(&mut game_data, 0).order, Order::HoldPosition);

    run(&mut game_data);

    assert_eq!(game_data.players[1].mode, PlayerMode::Rogue);
    let fleet = rogue(&mut game_data, 0);
    assert_eq!(fleet.order, Order::PatrolSector);
    assert_eq!(fleet.rules_of_engagement, 10);
}

// ── Civil disorder ──────────────────────────────────────────────────────
//
// The orders pass starts with `24a3:0c2a`: a civil-disorder empire with no
// world (probed 2026-10-11, three years; Rust agreed on every record but the
// random patrol targets) becomes a rogue. Its name "In Civil Disorder"
// becomes "Rogues" and any other name stays, its handle goes blank, and its
// stances stay; the pass then plays it as a rogue with no world.

/// Empire 3 is in civil disorder with no world: its home world (6,5) is
/// unowned and the saved count is 0. It has a cruiser on Hold in deep space,
/// an ETAC fleet, a scout fleet, and a starbase; it was a player, so it has
/// a handle, and it is at war with empire 4.
fn planetless_civil_disorder_scenario() -> CoreGameData {
    let mut game_data = scenario();
    planet_at(&mut game_data, [6, 5]).owner.0 = 0;
    game_data.players[2].planet_count = 0;
    set_player_handle(&mut game_data.players[2], "OLDPLAYER");
    set_diplomatic_relation_toward(&mut game_data.players[2], 4, DiplomaticRelation::Enemy);
    // A cruiser, an ETAC, a scout, and an empty fleet that is culled.
    for (n, (cruisers, scouts, etacs)) in [(1, 0, 0), (0, 0, 1), (0, 1, 0), (0, 0, 0)]
        .into_iter()
        .enumerate()
    {
        let fleet = &mut game_data.fleets[8 + n];
        set_ships(fleet, 0, cruisers, 0, scouts, etacs);
        let sector = [8 + n as u8, 8];
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
    add_base(&mut game_data, 3, [6, 5], Real48::ONE, 1, 0);
    game_data
}

#[test]
fn a_civil_disorder_empire_with_no_world_becomes_a_rogue_that_patrols() {
    let mut game_data = planetless_civil_disorder_scenario();

    run(&mut game_data);

    let player = &game_data.players[2];
    assert_eq!(player.mode, PlayerMode::Rogue);
    assert_eq!(player.empire_name, "Rogues");
    assert!(player.handle.trim().is_empty());
    // The stance it had stays; every human empire becomes an enemy.
    assert_eq!(
        stored_diplomatic_relation(player, 4),
        Some(DiplomaticRelation::Enemy)
    );
    assert_eq!(
        stored_diplomatic_relation(player, 1),
        Some(DiplomaticRelation::Enemy)
    );
    assert_eq!(
        stored_diplomatic_relation(player, 2),
        Some(DiplomaticRelation::Neutral)
    );
    // The warship patrols; the unarmed fleets and the starbase are gone.
    let fleets: Vec<&Fleet> = game_data
        .fleets
        .iter()
        .filter(|fleet| fleet.owner.0 == 3)
        .collect();
    assert_eq!(fleets.len(), 1);
    assert_eq!(fleets[0].ships.cruisers, 1);
    assert_eq!(fleets[0].order, Order::PatrolSector);
    assert_eq!(fleets[0].rules_of_engagement, 10);
    assert!(
        game_data
            .bases
            .iter()
            .all(|base| base.owner.0 != 3 || base.base_id.0 == 0)
    );
}

#[test]
fn a_civil_disorder_empire_with_a_world_stays_in_civil_disorder() {
    // Empires 2 and 4 own their home worlds.
    let mut game_data = planetless_civil_disorder_scenario();

    run(&mut game_data);

    for empire in [1usize, 3] {
        let player = &game_data.players[empire];
        assert_eq!(
            player.mode,
            PlayerMode::CivilDisorder,
            "empire {}",
            empire + 1
        );
        assert_eq!(player.empire_name, "In Civil Disorder");
    }
}

#[test]
fn a_planetless_civil_disorder_empire_keeps_a_name_of_its_own_and_with_no_fleet_its_base() {
    let mut game_data = planetless_civil_disorder_scenario();
    set_empire_name(&mut game_data.players[2], "Fallen Empire");
    game_data.fleets.retain(|fleet| fleet.owner.0 != 3);
    game_data.players[2].fleets_first.0 = 0;
    game_data.players[2].fleets_last.0 = 0;

    run(&mut game_data);

    let player = &game_data.players[2];
    assert_eq!(player.mode, PlayerMode::Rogue);
    assert_eq!(player.empire_name, "Fallen Empire");
    assert!(player.handle.trim().is_empty());
    assert!(
        game_data
            .bases
            .iter()
            .any(|base| base.owner.0 == 3 && base.base_id.0 == 1)
    );
}

// ── Escape episodes ─────────────────────────────────────────────────────

fn escaping(sector: [u8; 2], layer: u8) -> Option<EscapeEpisode> {
    Some(EscapeEpisode {
        sector: Coords::from_array(sector),
        layer,
        pursuers: 1,
    })
}

#[test]
fn an_order_from_the_rogue_pass_ends_an_escape_episode() {
    // A player's order ends an escape (`Fleet::end_escape`); so does each
    // order the pass gives, whether it changes the fleet's order (the Move
    // fleet is sent home), only restates it (the cruiser in orbit stays on
    // Guard/Blockade), or ends on the starbase step (the third fleet takes
    // the base).
    let mut game_data = rogue_scenario();
    let fleet = rogue(&mut game_data, 0);
    set_ships(fleet, 0, 1, 0, 0, 0);
    set_fleet(
        fleet,
        [8, 13],
        Real48::ZERO,
        Order::MoveOnly,
        [8, 13],
        0,
        0,
        6,
    );
    for (n, sector) in [(1, ROGUE_HOME), (2, NEAR_WORLD)] {
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
            10,
        );
    }
    add_base(&mut game_data, 2, NEAR_WORLD, Real48::ONE, 1, 0);
    for n in 0..3 {
        let sector = rogue(&mut game_data, n).sector.to_array();
        let layer = u8::from(n > 0);
        rogue(&mut game_data, n).escape = escaping(sector, layer);
    }
    run(&mut game_data);

    for n in 0..3 {
        assert_eq!(rogue(&mut game_data, n).escape, None, "fleet {n}");
    }
}

#[test]
fn a_rogue_patrol_order_ends_an_escape_episode() {
    // With no speed to set out on, the fleet stays in its sector, so only
    // the order can end the episode.
    let mut game_data = worldless_scenario();
    armed_idle(&mut game_data, 0, [8, 13]);
    rogue(&mut game_data, 0).max_speed = 0;
    rogue(&mut game_data, 0).escape = escaping([8, 13], 0);

    run(&mut game_data);

    let fleet = rogue(&mut game_data, 0);
    assert_eq!(fleet.order, Order::PatrolSector);
    assert_eq!(fleet.sector.to_array(), [8, 13]);
    assert_eq!(fleet.escape, None);
}
