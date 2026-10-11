//! Fleet battle effectiveness, carried hits, ROE checks, escape, and
//! pursuit.
//!
//! Inline rather than in tests/: maintenance rolls seeded dice and carried
//! hits never leave the battle, so these tests script the dice through the
//! battle's private seam to pin the arithmetic exactly. Every scripted roll
//! below 9 means no critical hits.

use std::collections::HashMap;
use std::path::Path;

use nc_compat::load_classic_core_files;
use nc_data::{
    Coords, CoreGameData, EncounterDispositionEvent, EncounterDispositionReason, Order,
    set_armies_aboard, set_troop_transports, troop_transports,
};

use super::exchange::{
    BattleDice, COMBAT_GUARDRAIL_MAX_ROUNDS, COMBAT_KIND_PURSUIT, apply_hits_to_fleet,
    fleet_die_modifier, fleet_volley,
};
use super::fleet_battle::{BattleSide, FleetBattle, apply_pending_hits, fight_encounter_with_dice};
use super::state::{FleetCombatState, IDX_BB, IDX_CA, IDX_DD, IDX_SB, IDX_TT, new_task_force};

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

/// Scripted dice: `d10` by combat kind (its low byte), round, and the acting
/// and target empires; `pursuit` by round, pursuer, and fleeing empire.
struct Scripted<'a> {
    d10: &'a dyn Fn(u64, u32, u8, u8) -> u8,
    pursuit: &'a dyn Fn(u32, u8, u8) -> (f64, f64),
}

impl BattleDice for Scripted<'_> {
    fn d10(&self, kind: u64, round: u32, acting_empire: u8, target_empire: u8) -> u8 {
        (self.d10)(kind & 0xff, round, acting_empire, target_empire)
    }

    fn pursuit(&self, _kind: u64, round: u32, pursuer: u8, fleeing: u8) -> (f64, f64) {
        (self.pursuit)(round, pursuer, fleeing)
    }
}

/// Draws that catch whatever the speeds: the arc's sine is 1 and the roll
/// is 0.
const CAUGHT: (f64, f64) = (0.5, 0.0);
/// Draws that never catch a side the pursuer is not faster than: the arc's
/// sine is 0.
const ESCAPED: (f64, f64) = (0.0, 0.5);

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
    fleet.escape = None;
}

/// Fight one battle in `DEEP_SPACE` between two fleet records, each a side
/// of its owner, under `round_cap` rounds.
fn fight_capped(
    game_data: &mut CoreGameData,
    dice: &dyn BattleDice,
    round_cap: u32,
    battle_key: u64,
    attacker: usize,
    defender: usize,
) -> FleetBattle {
    let side = |fleet: usize| BattleSide {
        empire: game_data.fleets[fleet].owner.0,
        fleets: vec![fleet],
        bases: Vec::new(),
    };
    let (attacker, defender) = (side(attacker), side(defender));
    fight_encounter_with_dice(
        game_data, dice, round_cap, battle_key, DEEP_SPACE, &attacker, &defender,
    )
}

fn fight(
    game_data: &mut CoreGameData,
    dice: &dyn BattleDice,
    battle_key: u64,
    attacker: usize,
    defender: usize,
) -> FleetBattle {
    fight_capped(
        game_data,
        dice,
        COMBAT_GUARDRAIL_MAX_ROUNDS,
        battle_key,
        attacker,
        defender,
    )
}

fn transports(game_data: &CoreGameData, idx: usize) -> u16 {
    troop_transports(&game_data.fleets[idx])
}

/// How fleet `idx` left the battle, from its disposition event: the reason
/// and whether it was caught in a pursuit exchange.
fn departure(battle: &FleetBattle, idx: usize) -> Option<(EncounterDispositionReason, bool)> {
    battle
        .events
        .encounter_disposition_events
        .iter()
        .find_map(|event| match *event {
            EncounterDispositionEvent::Retreated {
                fleet_idx, reason, ..
            } if fleet_idx == idx => Some((reason, false)),
            EncounterDispositionEvent::PursuitFire {
                fleet_idx, reason, ..
            } if fleet_idx == idx => Some((reason, true)),
            _ => None,
        })
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

/// Decision 7: a destroyer's DS is 8, so each of its damage steps costs 8
/// and execution fire pays 16 to destroy a nominal one.
#[test]
fn a_destroyer_costs_eight_hits_a_step() {
    let mut state = FleetCombatState::default();
    state.counts[IDX_DD] = 2;
    let carried = apply_hits_to_fleet(&mut state, 15, 0, 0);
    assert_eq!(state.crippled[IDX_DD], 1, "15 suppression hits cripple one");
    assert_eq!(carried.suppression, 7);
    let carried = apply_hits_to_fleet(&mut state, 0, 16, 0);
    assert_eq!(
        (state.counts[IDX_DD], state.crippled[IDX_DD]),
        (1, 0),
        "execution fire finishes the crippled one for 8 and cannot afford the nominal one"
    );
    assert_eq!(carried.execution, 8);
    let carried = apply_hits_to_fleet(&mut state, 0, 16, 0);
    assert_eq!(
        state.counts[IDX_DD], 0,
        "16 execution hits destroy a nominal destroyer"
    );
    assert_eq!(carried.execution, 0);
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
fn carried_hits_buy_damage_steps() {
    let mut game_data = board();
    place(&mut game_data, 0, 1, Ships { bb: 2, ..NONE }, 10);
    place(&mut game_data, 4, 2, Ships { bb: 2, ..NONE }, 10);
    let low = |_, _, _, _| 0;
    let dice = Scripted {
        d10: &low,
        pursuit: &|_, _, _| CAUGHT,
    };

    // Two battleships fire 45 a round at battleships of DS 100: no single
    // volley buys a step, but the carried hits cripple one in round 3 and
    // the other in round 5 and finish them in rounds 7 and 9, on both
    // sides alike.
    fight(&mut game_data, &dice, 1, 0, 4);
    assert_eq!(game_data.fleets[0].ships.battleships, 0);
    assert_eq!(game_data.fleets[4].ships.battleships, 0);
}

#[test]
fn suppression_and_execution_carry_separately() {
    let game_data = {
        let mut game_data = board();
        place(&mut game_data, 4, 2, Ships { tt: 1, ..NONE }, 0);
        game_data
    };
    let pools = |carried_suppression: u32, suppression: u32, execution: u32| {
        let mut forces = vec![new_task_force(&game_data, 2, DEEP_SPACE, vec![4], 0)];
        forces[0].carried_hits.suppression = carried_suppression;
        apply_pending_hits(
            &mut forces,
            &HashMap::from([(2, suppression)]),
            &HashMap::from([(2, execution)]),
            &HashMap::new(),
        );
        (forces[0].state.crippled[IDX_TT], forces[0].carried_hits)
    };
    // Seven carried suppression hits and three new ones cripple a transport
    // of DS 10; three new execution hits buy nothing with them, and each
    // pool keeps its own remainder.
    let (crippled, carried) = pools(7, 3, 0);
    assert_eq!(
        (crippled, carried.suppression, carried.execution),
        (1, 0, 0)
    );
    let (crippled, carried) = pools(7, 0, 3);
    assert_eq!(
        (crippled, carried.suppression, carried.execution),
        (0, 7, 3)
    );
}

#[test]
fn a_fleet_checks_its_roe_after_the_first_exchange() {
    let mut game_data = board();
    place(&mut game_data, 0, 1, Ships { ca: 2, ..NONE }, 6);
    place(&mut game_data, 4, 2, Ships { ca: 1, ..NONE }, 6);
    let low = |_, _, _, _| 0;
    let dice = Scripted {
        d10: &low,
        pursuit: &|_, _, _| ESCAPED,
    };

    // Both sides are on ROE 6. After the compulsory exchange the lone
    // cruiser (30 against 60) breaks off and, uncaught, seeks home; the
    // pair (60 against 30) stays and holds the field. Rolls of 0 buy no
    // damage step, so one exchange is all that happens.
    let battle = fight(&mut game_data, &dice, 1, 0, 4);
    assert_eq!(
        departure(&battle, 4),
        Some((EncounterDispositionReason::RoeWithdrawal, false))
    );
    assert_eq!(departure(&battle, 0), None);
    assert_eq!(battle.held_by, Some(1));
    assert_eq!(game_data.fleets[4].order, Order::SeekHome);
    assert_eq!(game_data.fleets[0].order, Order::HoldPosition);
    assert_eq!(game_data.fleets[4].ships.cruisers, 1);
    assert_eq!(game_data.fleets[0].ships.cruisers, 2);
}

#[test]
fn unarmed_survivors_escape() {
    // Transports attacked by a cruiser leave after the first exchange.
    let mut game_data = board();
    place(&mut game_data, 0, 1, Ships { ca: 1, ..NONE }, 10);
    place(&mut game_data, 4, 2, Ships { tt: 5, ..NONE }, 0);
    let low = |_, _, _, _| 0;
    let dice = Scripted {
        d10: &low,
        pursuit: &|_, _, _| ESCAPED,
    };
    let battle = fight(&mut game_data, &dice, 1, 0, 4);
    assert_eq!(
        departure(&battle, 4),
        Some((EncounterDispositionReason::UnarmedEscape, false))
    );
    assert_eq!(game_data.fleets[4].order, Order::SeekHome);
    assert_eq!(transports(&game_data, 4), 5);

    // So does a fleet that has just lost its last armed ship: a battleship's
    // 23 hits cripple and finish its destroyer in the first exchange.
    let mut game_data = board();
    place(&mut game_data, 0, 1, Ships { bb: 1, ..NONE }, 10);
    place(
        &mut game_data,
        4,
        2,
        Ships {
            dd: 1,
            tt: 5,
            ..NONE
        },
        10,
    );
    let battle = fight(&mut game_data, &dice, 1, 0, 4);
    assert_eq!(game_data.fleets[4].ships.destroyers, 0);
    assert_eq!(
        departure(&battle, 4),
        Some((EncounterDispositionReason::UnarmedEscape, false))
    );
    assert_eq!(transports(&game_data, 4), 5);
    assert_eq!(game_data.fleets[4].order, Order::SeekHome);
}

#[test]
fn a_faster_pursuer_catches_for_certain_and_an_equal_one_needs_the_roll() {
    let high = |_, _, _, _| 6;

    // A destroyer (speed 6) chasing transports (speed 5) catches them even
    // on draws that would fail the roll, and the pursuit exchange's 10 hits
    // join the first exchange's carried 10 to destroy one.
    let mut game_data = board();
    place(&mut game_data, 0, 1, Ships { dd: 1, ..NONE }, 10);
    place(&mut game_data, 4, 2, Ships { tt: 5, ..NONE }, 0);
    let dice = Scripted {
        d10: &high,
        pursuit: &|_, _, _| ESCAPED,
    };
    let battle = fight(&mut game_data, &dice, 1, 0, 4);
    assert_eq!(
        departure(&battle, 4),
        Some((EncounterDispositionReason::UnarmedEscape, true))
    );
    assert_eq!(transports(&game_data, 4), 4);

    // A cruiser (speed 5) chasing transports (speed 5) catches them only
    // when the roll says so.
    for (draws, caught, left) in [(ESCAPED, false, 5), (CAUGHT, true, 4)] {
        let mut game_data = board();
        place(&mut game_data, 0, 1, Ships { ca: 1, ..NONE }, 10);
        place(&mut game_data, 4, 2, Ships { tt: 5, ..NONE }, 0);
        let dice = Scripted {
            d10: &high,
            pursuit: &move |_, _, _| draws,
        };
        let battle = fight(&mut game_data, &dice, 1, 0, 4);
        assert_eq!(
            departure(&battle, 4),
            Some((EncounterDispositionReason::UnarmedEscape, caught))
        );
        // 30 suppression hits cripple three transports; a pursuit's 30 more
        // cripple the other two and finish one.
        assert_eq!(transports(&game_data, 4), left);
        assert_eq!(game_data.fleets[4].order, Order::SeekHome);
    }
}

#[test]
fn nobody_pursues_when_both_sides_break_off() {
    let mut game_data = board();
    place(&mut game_data, 0, 1, Ships { ca: 1, ..NONE }, 1);
    place(&mut game_data, 4, 2, Ships { ca: 1, ..NONE }, 1);
    let low = |_, _, _, _| 0;
    let dice = Scripted {
        d10: &low,
        pursuit: &|_, _, _| panic!("no pursuit when both sides break off"),
    };
    // ROE 1 accepts only a defenceless enemy: both cruisers break off after
    // the first exchange, and both seek home unpursued.
    let battle = fight(&mut game_data, &dice, 1, 0, 4);
    assert_eq!(
        departure(&battle, 0),
        Some((EncounterDispositionReason::RoeWithdrawal, false))
    );
    assert_eq!(
        departure(&battle, 4),
        Some((EncounterDispositionReason::RoeWithdrawal, false))
    );
    assert_eq!(battle.held_by, None);
    assert_eq!(game_data.fleets[0].order, Order::SeekHome);
    assert_eq!(game_data.fleets[4].order, Order::SeekHome);
}

#[test]
fn carry_is_kept_in_the_immediate_pursuit_and_a_later_pursuit_starts_at_zero() {
    let mut game_data = board();
    place(&mut game_data, 0, 1, Ships { dd: 1, ..NONE }, 10);
    place(&mut game_data, 4, 2, Ships { tt: 1, ..NONE }, 0);
    // The destroyer rolls 6 (10 hits) in an ordinary exchange and 0 (3) in
    // a pursuit exchange.
    let rolls = |kind: u64, _, _, _| if kind == COMBAT_KIND_PURSUIT { 0 } else { 6 };
    let dice = Scripted {
        d10: &rolls,
        pursuit: &|_, _, _| CAUGHT,
    };

    // The transport escapes and is caught: 10 and 3 execution hits carry to
    // 13, short of the 20 that destroy it.
    let battle = fight(&mut game_data, &dice, 1, 0, 4);
    assert_eq!(
        departure(&battle, 4),
        Some((EncounterDispositionReason::UnarmedEscape, true))
    );
    assert_eq!(transports(&game_data, 4), 1);
    let escape = game_data.fleets[4]
        .escape
        .expect("the transport is escaping");
    assert_eq!(escape.sector.to_array(), DEEP_SPACE);
    assert!(escape.pursued_by(1));

    // The destroyer catches it again on a later tick: a new encounter that
    // starts at zero, so its 10 hits are not the 23 that would have killed,
    // and the one pursuit exchange of this escape has been had.
    let battle = fight(&mut game_data, &dice, 2, 0, 4);
    assert_eq!(
        departure(&battle, 4),
        Some((EncounterDispositionReason::UnarmedEscape, false))
    );
    assert_eq!(transports(&game_data, 4), 1);
    assert_eq!(game_data.fleets[4].order, Order::SeekHome);
}

#[test]
fn each_opposing_empire_gets_one_exchange_with_an_escaping_fleet() {
    let mut game_data = board();
    place(&mut game_data, 0, 1, Ships { dd: 1, ..NONE }, 10);
    place(&mut game_data, 4, 2, Ships { tt: 3, ..NONE }, 0);
    place(&mut game_data, 8, 3, Ships { dd: 1, ..NONE }, 10);
    let low = |_, _, _, _| 0;
    let dice = Scripted {
        d10: &low,
        pursuit: &|_, _, _| CAUGHT,
    };

    // Empire 1 catches the escaping transports once. Empire 3's attack on
    // them while they escape is its own pursuit: one exchange, and when they
    // break off again it gets no second. Empire 1's second attack is a
    // pursuit it has already had, so no pursuit exchange follows it either.
    let battle = fight(&mut game_data, &dice, 1, 0, 4);
    assert_eq!(departure(&battle, 4).map(|(_, caught)| caught), Some(true));
    let battle = fight(&mut game_data, &dice, 2, 8, 4);
    assert_eq!(departure(&battle, 4).map(|(_, caught)| caught), Some(false));
    let battle = fight(&mut game_data, &dice, 3, 0, 4);
    assert_eq!(departure(&battle, 4).map(|(_, caught)| caught), Some(false));
    let escape = game_data.fleets[4].escape.expect("still escaping");
    assert!(escape.pursued_by(1) && escape.pursued_by(3));
    assert!(!escape.pursued_by(2));
}

#[test]
fn the_round_cap_ends_the_battle_unresolved() {
    let mut game_data = board();
    place(&mut game_data, 0, 1, Ships { dd: 2, ..NONE }, 10);
    place(&mut game_data, 4, 2, Ships { dd: 2, ..NONE }, 10);
    let low = |_, _, _, _| 0;
    let dice = Scripted {
        d10: &low,
        pursuit: &|_, _, _| panic!("nobody breaks off"),
    };

    // Two destroyers a side fire 5 a round at destroyers that cost 16 to
    // destroy: under a cap of two rounds nothing dies, and the battle ends
    // with nobody holding the field, nobody retreating, and both fleets as
    // they were apart from the fight itself.
    let battle = fight_capped(&mut game_data, &dice, 2, 1, 0, 4);
    assert_eq!(battle.held_by, None);
    assert!(battle.events.encounter_disposition_events.is_empty());
    assert!(battle.events.mission_events.is_empty());
    assert_eq!(battle.events.fleet_battle_events.len(), 2);
    assert!(
        battle
            .events
            .fleet_battle_events
            .iter()
            .all(|event| !event.held_field)
    );
    for idx in [0, 4] {
        assert_eq!(game_data.fleets[idx].ships.destroyers, 2);
        assert_eq!(game_data.fleets[idx].order, Order::HoldPosition);
        assert_eq!(game_data.fleets[idx].escape, None);
    }
    // The ordinary cap is 64 rounds: the same fight then runs to the
    // destroyers' mutual destruction.
    let battle = fight(&mut game_data, &dice, 2, 0, 4);
    assert_eq!(battle.held_by, None);
    assert_eq!(game_data.fleets[0].ships.destroyers, 0);
    assert_eq!(game_data.fleets[4].ships.destroyers, 0);
}
