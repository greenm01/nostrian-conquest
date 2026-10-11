//! Automatic ship recovery (combat decision 3, RT 2 for every class):
//! timing, the starbase's extra turn and who may not have it, and damage
//! that outlives a battle.

use nc_data::{
    CoreGameData, CrippledHulls, DiplomacyOverride, MaintenanceEvents, Order, PlayerMode,
    RecoveredUnit, ShipsRecoveredEvent, set_fleet_at_rest, set_mission_target,
};
use nc_engine::run_maintenance_turn_with_context_and_seed;

use super::support::{
    DEEP_SPACE, SYSTEM, SYSTEM_PLANET_IDX, Ships, diplomacy, empty_board, place_fleet,
    place_starbase,
};

/// Empire 2 owns `SYSTEM`; its fleets here are records 4 (the host) and 5.
const HOST: usize = 4;
const JOINER: usize = 5;

fn hulls(turns_1: u32, turns_2: u32, fresh: u32) -> CrippledHulls {
    CrippledHulls {
        turns: [turns_1, turns_2],
        fresh,
    }
}

fn run(
    game_data: &mut CoreGameData,
    seed: u64,
    overrides: &[DiplomacyOverride],
) -> MaintenanceEvents {
    run_maintenance_turn_with_context_and_seed(game_data, seed, &[], overrides)
        .expect("maintenance should succeed")
}

fn run_neutral(game_data: &mut CoreGameData) -> MaintenanceEvents {
    run(game_data, 0, &diplomacy(&[]))
}

/// Empire 2, a human player, holds `SYSTEM` with three cruisers of fleet
/// record `HOST` resting in orbit on Hold, ROE 0, and `crippled` among
/// them. No other empire's units are near.
fn resting_host(crippled: CrippledHulls) -> CoreGameData {
    let mut game_data = empty_board(0, 0);
    game_data.players[1].mode = PlayerMode::Human;
    game_data.players[1].autopilot = false;
    rest_in_orbit(&mut game_data, HOST, 3);
    game_data.fleets[HOST].crippled.cruisers = crippled;
    game_data
}

fn rest_in_orbit(game_data: &mut CoreGameData, idx: usize, cruisers: u16) {
    place_fleet(
        game_data,
        idx,
        2,
        SYSTEM,
        Order::HoldPosition,
        SYSTEM,
        Ships {
            ca: cruisers,
            ..Ships::default()
        },
        0,
    );
    set_fleet_at_rest(&mut game_data.fleets[idx]);
}

fn recovered_by(events: &MaintenanceEvents, fleet_number: u16) -> Option<ShipsRecoveredEvent> {
    events
        .ship_recovery_events
        .iter()
        .copied()
        .find(|event| event.unit == RecoveredUnit::Fleet { fleet_number })
}

fn host_number(game_data: &CoreGameData) -> u16 {
    game_data.fleets[HOST].fleet_number.0
}

/// One hull crippled during pass N, then passes N, N+1, N+2 under the cohort
/// model's rules: `(whole again, hulls recovered)` after each. The model
/// is `tools/combat-model/recovery.py`; these rows are its `rt` 2 timelines
/// in `nc_recovery_timing.json`.
const MODEL_WITHOUT_BASE: [(bool, u32); 3] = [(false, 0), (false, 0), (true, 1)];
const MODEL_WITH_BASE: [(bool, u32); 3] = [(false, 0), (true, 1), (true, 0)];

fn timeline(with_base: bool) -> Vec<(bool, u32)> {
    // A hull crippled during pass N is a "this pass" hull when the pass begins.
    let mut game_data = resting_host(hulls(0, 0, 1));
    if with_base {
        place_starbase(&mut game_data, 2, SYSTEM);
    }
    let number = host_number(&game_data);
    (0..3)
        .map(|_| {
            let events = run_neutral(&mut game_data);
            let recovered = recovered_by(&events, number).map_or(0, |event| event.hulls);
            (game_data.fleets[HOST].crippled.is_empty(), recovered)
        })
        .collect()
}

#[test]
fn rt_two_hulls_crippled_in_pass_n_recover_when_pass_n_plus_two_ends() {
    assert_eq!(timeline(false), MODEL_WITHOUT_BASE);
}

#[test]
fn a_starbase_brings_hulls_back_a_pass_early_but_never_this_passs_damage() {
    assert_eq!(timeline(true), MODEL_WITH_BASE);
}

#[test]
fn counts_step_down_in_place_and_this_passs_hulls_enter_at_rt() {
    let mut game_data = resting_host(hulls(0, 1, 2));
    game_data.fleets[HOST].crippled.cruisers.turns[0] = 1;
    run_neutral(&mut game_data);
    // One hull had a turn left and recovered; the one with two has one left;
    // the two crippled during the pass enter at RT.
    assert_eq!(game_data.fleets[HOST].crippled.cruisers, hulls(1, 2, 0));
}

#[test]
fn assisted_recovery_is_reported_only_when_it_helped() {
    let mut game_data = resting_host(hulls(1, 2, 0));
    place_starbase(&mut game_data, 2, SYSTEM);
    let events = run_neutral(&mut game_data);
    let number = host_number(&game_data);
    let event = recovered_by(&events, number).expect("ships recovered");
    // The hull with one turn left recovers either way; the two with two
    // turns left come back a pass early.
    assert_eq!((event.hulls, event.early), (3, 2));
    assert_eq!(event.coords, SYSTEM);
    assert!(event.in_system);
    assert!(game_data.fleets[HOST].crippled.is_empty());

    let mut plain = resting_host(hulls(1, 2, 0));
    let events = run_neutral(&mut plain);
    let event = recovered_by(&events, number).expect("ships recovered");
    assert_eq!((event.hulls, event.early), (1, 0));
    assert_eq!(plain.fleets[HOST].crippled.cruisers, hulls(2, 0, 0));
}

/// What a pass with hulls of two turns left leaves, given `setup`: how
/// many recovered, and what remains crippled.
fn two_turn_hulls_after(setup: impl FnOnce(&mut CoreGameData)) -> (u32, CrippledHulls) {
    let mut game_data = resting_host(hulls(0, 2, 0));
    place_starbase(&mut game_data, 2, SYSTEM);
    setup(&mut game_data);
    let number = host_number(&game_data);
    let events = run_neutral(&mut game_data);
    (
        recovered_by(&events, number).map_or(0, |event| event.hulls),
        game_data.fleets[HOST].crippled.cruisers,
    )
}

#[test]
fn the_extra_turn_needs_an_owned_world_and_an_owned_resting_base() {
    let helped = (2, hulls(0, 0, 0));
    let unhelped = (0, hulls(2, 0, 0));
    assert_eq!(two_turn_hulls_after(|_| {}), helped);
    // The base belongs to someone else.
    assert_eq!(two_turn_hulls_after(|g| g.bases[0].owner.0 = 3), unhelped);
    // The world belongs to someone else.
    assert_eq!(
        two_turn_hulls_after(|g| g.planets[SYSTEM_PLANET_IDX].owner.0 = 3),
        unhelped
    );
    // The base is not at rest in orbit: it is under way.
    assert_eq!(
        two_turn_hulls_after(|g| {
            g.bases[0].moving = true;
            g.bases[0].target = nc_data::Coords::from_array(DEEP_SPACE);
            g.bases[0].target_depth = 0;
        }),
        unhelped
    );
    // The fleet is in deep space over the world, not in its orbit.
    assert_eq!(
        two_turn_hulls_after(|g| g.fleets[HOST].depth = nc_data::Real48::ZERO),
        unhelped
    );
    // There is no base at all.
    assert_eq!(two_turn_hulls_after(|g| g.bases.clear()), unhelped);
}

#[test]
fn a_fleet_that_leaves_orbit_recovers_on_the_ordinary_schedule() {
    let (recovered, remaining) = two_turn_hulls_after(|g| {
        let fleet = &mut g.fleets[HOST];
        fleet.order = Order::MoveOnly;
        fleet.target = nc_data::Coords::from_array(DEEP_SPACE);
        set_mission_target(fleet, 0, 0);
        fleet.speed = fleet.max_speed;
    });
    assert_eq!((recovered, remaining), (0, hulls(2, 0, 0)));
}

#[test]
fn a_fleet_that_merges_in_late_cannot_pass_its_hulls_off_as_assisted() {
    // Three hulls with two turns left rest at the base; two more hulls with
    // two turns left, in a second fleet resting beside them, join the host
    // during the pass. Only the host's own three came with the pass.
    let mut game_data = resting_host(hulls(0, 3, 0));
    place_starbase(&mut game_data, 2, SYSTEM);
    rest_in_orbit(&mut game_data, JOINER, 2);
    let joiner = &mut game_data.fleets[JOINER];
    joiner.crippled.cruisers = hulls(0, 2, 0);
    joiner.order = Order::JoinAnotherFleet;
    joiner.target_depth = 1;
    let host_number = game_data.fleets[HOST].fleet_number.0;
    set_mission_target(&mut game_data.fleets[JOINER], 1, host_number);

    let events = run_neutral(&mut game_data);

    let host = &game_data.fleets[HOST];
    assert_eq!(host.ships.cruisers, 5, "the joiner merged into the host");
    let event = recovered_by(&events, host_number).expect("the host's hulls recovered");
    assert_eq!((event.hulls, event.early), (3, 3));
    assert_eq!(host.crippled.cruisers, hulls(2, 0, 0));
}

/// Loaded and empty transports recover on their own counters, with the
/// starbase's extra turn and this pass's cohort entering at RT on each.
#[test]
fn loaded_and_empty_transports_recover_on_their_own_counters() {
    let mut game_data = resting_host(CrippledHulls::default());
    place_starbase(&mut game_data, 2, SYSTEM);
    let host = &mut game_data.fleets[HOST];
    nc_data::set_troop_transports(host, 4);
    nc_data::set_armies_aboard(host, 2);
    host.crippled.loaded_transports = hulls(1, 1, 0);
    host.crippled.empty_transports = hulls(0, 1, 1);

    let events = run_neutral(&mut game_data);

    let event = recovered_by(&events, host_number(&game_data)).expect("ships recovered");
    // The loaded hull with a turn left recovers either way; the two with
    // two turns left, one loaded and one empty, come back a pass early.
    assert_eq!((event.hulls, event.early), (3, 2));
    let host = &game_data.fleets[HOST];
    assert_eq!(
        (host.ships.loaded_transports, host.ships.empty_transports),
        (2, 2)
    );
    assert!(host.crippled.loaded_transports.is_empty());
    assert_eq!(host.crippled.empty_transports, hulls(0, 1, 0));
}

/// A merge adds each cargo state's cohorts to the host's own.
#[test]
fn a_merge_carries_each_cargo_states_own_cohorts() {
    let mut game_data = resting_host(CrippledHulls::default());
    let host = &mut game_data.fleets[HOST];
    nc_data::set_troop_transports(host, 3);
    nc_data::set_armies_aboard(host, 2);
    host.crippled.loaded_transports = hulls(0, 2, 0);
    host.crippled.empty_transports = hulls(0, 1, 0);
    rest_in_orbit(&mut game_data, JOINER, 0);
    let joiner = &mut game_data.fleets[JOINER];
    nc_data::set_troop_transports(joiner, 2);
    joiner.crippled.empty_transports = hulls(1, 1, 0);
    joiner.order = Order::JoinAnotherFleet;
    joiner.target_depth = 1;
    let host_number = game_data.fleets[HOST].fleet_number.0;
    set_mission_target(&mut game_data.fleets[JOINER], 1, host_number);

    let events = run_neutral(&mut game_data);

    let host = &game_data.fleets[HOST];
    assert_eq!(
        (host.ships.loaded_transports, host.ships.empty_transports),
        (2, 3),
        "the joiner merged into the host"
    );
    // After the merge, the pass's recovery step: the joiner's one-turn hull
    // recovers, and every other hull steps down a turn on its own counter.
    let event = recovered_by(&events, host_number).expect("a hull recovered");
    assert_eq!((event.hulls, event.early), (1, 0));
    assert_eq!(host.crippled.loaded_transports, hulls(2, 0, 0));
    assert_eq!(host.crippled.empty_transports, hulls(2, 0, 0));
}

#[test]
fn a_starbase_does_not_assist_its_own_recovery() {
    let mut game_data = resting_host(CrippledHulls::default());
    place_starbase(&mut game_data, 2, SYSTEM);
    game_data.bases[0].crippled = hulls(0, 1, 0);

    let events = run_neutral(&mut game_data);
    assert_eq!(game_data.bases[0].crippled, hulls(1, 0, 0));
    assert!(events.ship_recovery_events.is_empty());

    let events = run_neutral(&mut game_data);
    assert!(game_data.bases[0].crippled.is_empty());
    let event = events.ship_recovery_events[0];
    assert_eq!(event.unit, RecoveredUnit::Starbase { base_id: 1 });
    assert_eq!((event.hulls, event.early), (1, 0));
}

#[test]
fn a_crippled_base_keeps_the_economic_bonuses() {
    let production_after_a_year = |crippled: CrippledHulls| {
        let mut game_data = resting_host(CrippledHulls::default());
        place_starbase(&mut game_data, 2, SYSTEM);
        game_data.bases[0].crippled = crippled;
        run_neutral(&mut game_data);
        let planet = &game_data.planets[SYSTEM_PLANET_IDX];
        (planet.production, planet.stored_goods)
    };
    assert_eq!(
        production_after_a_year(hulls(0, 1, 0)),
        production_after_a_year(CrippledHulls::default())
    );
}

/// Two equal cruiser fleets, empires 2 and 3, patrolling `DEEP_SPACE` on
/// ROE 10 at war.
fn cruiser_duel() -> (CoreGameData, Vec<DiplomacyOverride>) {
    let mut game_data = empty_board(0, 0);
    for (idx, owner) in [(HOST, 2u8), (8, 3)] {
        place_fleet(
            &mut game_data,
            idx,
            owner,
            DEEP_SPACE,
            Order::PatrolSector,
            DEEP_SPACE,
            Ships {
                ca: 4,
                ..Ships::default()
            },
            10,
        );
        game_data.fleets[idx].depth = nc_data::Real48::ZERO;
        set_mission_target(&mut game_data.fleets[idx], 0, 0);
    }
    (game_data, diplomacy(&[(2, 3)]))
}

#[test]
fn damage_outlives_the_battle_and_is_counted_down_by_the_next_pass() {
    let mut survivors_with_damage = 0;
    for seed in 0..24u64 {
        let (mut game_data, overrides) = cruiser_duel();
        if run(&mut game_data, seed, &overrides)
            .fleet_battle_events
            .is_empty()
        {
            continue;
        }
        let survivors: Vec<_> = game_data
            .fleets
            .iter()
            .filter(|fleet| matches!(fleet.owner.0, 2 | 3) && fleet.ships.cruisers > 0)
            .cloned()
            .collect();
        for fleet in &survivors {
            // The cripples are in recovery at RT, and never more than the
            // ships there are.
            let damage = fleet.crippled.cruisers;
            assert!(
                damage.total() <= u32::from(fleet.ships.cruisers),
                "seed {seed}"
            );
            assert_eq!((damage.turns[0], damage.fresh), (0, 0), "seed {seed}");
            survivors_with_damage += usize::from(damage.total() > 0);
        }
        if survivors.len() != 1 {
            continue;
        }
        // Nobody is left to fight: the next pass counts the damage down once.
        let survivor = &survivors[0];
        run(&mut game_data, seed + 100, &overrides);
        let now = game_data
            .fleets
            .iter()
            .find(|fleet| {
                fleet.owner == survivor.owner && fleet.fleet_number == survivor.fleet_number
            })
            .expect("the survivor is still here");
        assert_eq!(now.ships, survivor.ships, "seed {seed}");
        assert_eq!(
            now.crippled.cruisers,
            hulls(survivor.crippled.cruisers.turns[1], 0, 0),
            "seed {seed}"
        );
    }
    assert!(
        survivors_with_damage > 0,
        "some survivor should carry its damage out of the battle"
    );
}
