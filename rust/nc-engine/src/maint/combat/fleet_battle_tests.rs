//! Fleet battle effectiveness and carried hits.
//!
//! Inline rather than in tests/: maintenance rolls seeded dice and carried
//! hits never leave the battle, so these tests script the dice through the
//! battle's private seam to pin the arithmetic exactly. Every scripted roll
//! below 9 means no critical hits.

use std::path::Path;

use nc_compat::load_classic_core_files;
use nc_data::{
    Coords, CoreGameData, Order, set_armies_aboard, set_troop_transports, troop_transports,
};

use super::exchange::{COMBAT_KIND_WITHDRAWAL, fleet_die_modifier, fleet_volley};
use super::fleet_battle::{BattleSide, fight_encounter_with_rolls};
use super::state::{FleetCombatState, IDX_BB, IDX_CA, IDX_DD, IDX_SB};

/// Empty deep-space sector.
const DEEP_SPACE: [u8; 2] = [8, 8];

#[derive(Clone, Copy)]
struct Ships {
    dd: u16,
    ca: u16,
    bb: u16,
    tt: u16,
}

const NONE: Ships = Ships {
    dd: 0,
    ca: 0,
    bb: 0,
    tt: 0,
};

fn board() -> CoreGameData {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures")
        .join("ecmaint-post")
        .join("v1.5");
    load_classic_core_files(&dir).expect("fixture should load")
}

/// Make fleet record `idx` a fleet of `owner` holding in `DEEP_SPACE`.
fn place(game_data: &mut CoreGameData, idx: usize, owner: u8, ships: Ships, roe: u8) {
    let fleet = &mut game_data.fleets[idx];
    fleet.owner.0 = owner;
    fleet.sector = Coords::from_array(DEEP_SPACE);
    fleet.order = Order::HoldPosition;
    fleet.target = Coords::from_array(DEEP_SPACE);
    fleet.speed = 0;
    fleet.ships.destroyers = ships.dd;
    fleet.ships.cruisers = ships.ca;
    fleet.ships.battleships = ships.bb;
    fleet.ships.scouts = 0;
    set_troop_transports(fleet, ships.tt);
    set_armies_aboard(fleet, 0);
    fleet.ships.etacs = 0;
    fleet.rules_of_engagement = roe;
}

/// Fight one battle in `DEEP_SPACE` between two fleet records, each a side
/// of its owner.
fn fight(
    game_data: &mut CoreGameData,
    rolls: &dyn Fn(u64, u32, u8, u8) -> u8,
    battle_key: u64,
    attacker: usize,
    defender: usize,
) {
    let side = |fleet: usize| BattleSide {
        empire: game_data.fleets[fleet].owner.0,
        fleets: vec![fleet],
        bases: Vec::new(),
    };
    let (attacker, defender) = (side(attacker), side(defender));
    fight_encounter_with_rolls(
        game_data, rolls, battle_key, DEEP_SPACE, &attacker, &defender,
    );
}

fn transports(game_data: &CoreGameData, idx: usize) -> u16 {
    troop_transports(&game_data.fleets[idx])
}

#[test]
fn fleet_volley_reads_the_flat_table() {
    let hits = |roll, modifier| fleet_volley(roll, modifier, 100).hits;
    let unmodified: Vec<u32> = (0..10).map(|roll| hits(roll, 0)).collect();
    assert_eq!(unmodified, [25, 25, 25, 50, 50, 50, 100, 100, 100, 100]);
    // A die modifier moves the roll along the table, never past 9.
    assert_eq!(hits(2, 1), 50);
    assert_eq!(hits(4, 2), 100);
    assert_eq!(hits(8, 2), 100);
    // Hits round up: a lone destroyer's quarter of 10 AS is 3.
    assert_eq!(fleet_volley(0, 0, 10).hits, 3);
    // Only an unmodified 9 is a critical.
    assert!(fleet_volley(9, 0, 100).critical);
    assert!(!fleet_volley(7, 2, 100).critical);
    assert!(!fleet_volley(9, 0, 0).critical);
}

#[test]
fn mixed_fleets_and_starbases_each_add_one_to_the_die() {
    let modifier = |classes: &[usize]| {
        let mut state = FleetCombatState::default();
        for &idx in classes {
            state.counts[idx] = 1;
        }
        fleet_die_modifier(&state)
    };
    assert_eq!(modifier(&[IDX_CA]), 0);
    assert_eq!(modifier(&[IDX_DD, IDX_CA]), 1);
    assert_eq!(modifier(&[IDX_SB]), 1);
    assert_eq!(modifier(&[IDX_DD, IDX_CA, IDX_BB, IDX_SB]), 2);
}

#[test]
fn carried_hits_buy_steps_and_end_with_the_round_cap() {
    let mut game_data = board();
    place(&mut game_data, 0, 1, Ships { dd: 1, ..NONE }, 10);
    place(&mut game_data, 4, 2, Ships { tt: 200, ..NONE }, 0);
    let low = |_, _, _, _| 0;

    // A lone destroyer fires 3 execution hits a round, and a nominal
    // transport costs 20 (2x DS). Only carry lets it kill: 64 rounds bank
    // 192 hits, which buy 9 transports, and the cap drops the other 12.
    fight(&mut game_data, &low, 1, 0, 4);
    assert_eq!(transports(&game_data, 4), 191);

    // The next battle between the same fleets in the same tick starts at
    // zero: a carried 12 would have bought a tenth transport.
    fight(&mut game_data, &low, 2, 0, 4);
    assert_eq!(transports(&game_data, 4), 182);
}

#[test]
fn suppression_and_execution_carry_separately() {
    let mut game_data = board();
    let mixed = Ships {
        dd: 1,
        ca: 1,
        ..NONE
    };
    place(&mut game_data, 0, 1, mixed, 10);
    place(&mut game_data, 4, 2, Ships { tt: 200, ..NONE }, 0);
    let low = |_, _, _, _| 0;

    // The mixed fleet's +1 keeps a roll of 0 at 25%: 10 hits a round, 8
    // suppression (the cruiser's share) and 2 execution. Over 64 rounds the
    // suppression pool cripples 51 transports at 10 hits each, and the
    // execution pool finishes 12 of them at 10 each. Pooled hits, or none
    // carried at all, would destroy a different number.
    fight(&mut game_data, &low, 1, 0, 4);
    assert_eq!(transports(&game_data, 4), 188);
}

#[test]
fn a_withdrawal_spends_carried_hits_and_the_next_battle_starts_at_zero() {
    let mut game_data = board();
    place(&mut game_data, 0, 1, Ships { bb: 2, ..NONE }, 10);
    place(&mut game_data, 4, 2, Ships { bb: 1, ..NONE }, 1);
    place(&mut game_data, 5, 2, Ships { tt: 200, ..NONE }, 0);
    // Every roll is 0 (25%) except empire 1's withdrawal exchange volley,
    // which rolls 3 (50%).
    let rolls = |kind: u64, _, acting_empire: u8, _| {
        if kind & 0xff == COMBAT_KIND_WITHDRAWAL && acting_empire == 1 {
            3
        } else {
            0
        }
    };

    // Two battleships fire 45 a round at a battleship of DS 100: 135 by
    // round 3 cripples it with 35 to spare. Its ROE 1 breaks off in round 4,
    // and the withdrawal exchange's 90 plus the carried 35 finish it. No
    // single volley reaches 100.
    fight(&mut game_data, &rolls, 1, 0, 4);
    assert_eq!(game_data.fleets[4].ships.battleships, 0);
    assert_eq!(game_data.fleets[0].ships.battleships, 2);

    // The 25 hits left over against empire 2 die with that battle. Against
    // 200 transports the battleships' 64 rounds of 45 buy 288 steps of 10:
    // 200 cripples, then 88 kills. Carrying the 25 would buy 90.
    fight(&mut game_data, &rolls, 2, 0, 5);
    assert_eq!(transports(&game_data, 5), 112);
}
