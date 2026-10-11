//! Damage that lasts (combat decision 3): a battle's cripples stay crippled
//! into every later battle, finishing fire takes the hulls closest to
//! recovery last, and the write-back spreads a side's change over its
//! fleets and starbases.
//!
//! Inline for the same reason as `fleet_battle_tests`: the scripted dice
//! pin the arithmetic exactly. Every scripted roll is 6, so each volley
//! deals its side's whole AS, and a one-round cap ends a battle after that
//! exchange.

use nc_data::{CrippledHulls, Starbase};

use super::assault::bombard_attack_as;
use super::damage::{Holder, apply_class_change, write_back_bases};
use super::fleet_battle::{BattleSide, fight_encounter_with_dice};
use super::fleet_battle_tests::{
    CAUGHT, DEEP_SPACE, NONE, Scripted, Ships, board, fight_capped, place,
};
use super::state::{
    FleetCombatState, IDX_BB, IDX_CA, IDX_DD, IDX_SB, base_attack_strength, warship_attack_strength,
};

fn hulls(turns_1: u32, turns_2: u32, fresh: u32) -> CrippledHulls {
    CrippledHulls {
        turns: [turns_1, turns_2],
        fresh,
    }
}

fn full_strength_rolls() -> Scripted<'static> {
    Scripted {
        d10: &|_, _, _, _| 6,
        pursuit: &|_, _, _| CAUGHT,
    }
}

#[test]
fn a_battle_leaves_its_cripples_crippled_for_the_next_battle() {
    let mut game_data = board();
    place(&mut game_data, 0, 1, Ships { ca: 2, ..NONE }, 10);
    place(&mut game_data, 4, 2, Ships { ca: 2, ..NONE }, 10);
    let dice = full_strength_rolls();

    // Two cruisers deal 60 to the other pair: both of its cruisers are
    // crippled (DS 30 each), and nothing restores them after the battle.
    fight_capped(&mut game_data, &dice, 1, 1, 0, 4);
    for idx in [0, 4] {
        let fleet = &game_data.fleets[idx];
        assert_eq!(fleet.ships.cruisers, 2, "fleet {idx}");
        assert_eq!(fleet.crippled.cruisers, hulls(0, 0, 2), "fleet {idx}");
    }

    // The next battle starts from that damage: two crippled cruisers fire
    // 30 (half their AS), which finishes one of the other side's. Fought
    // from whole ships, the second battle would have crippled two again and
    // destroyed none.
    fight_capped(&mut game_data, &dice, 1, 2, 0, 4);
    for idx in [0, 4] {
        let fleet = &game_data.fleets[idx];
        assert_eq!(fleet.ships.cruisers, 1, "fleet {idx}");
        assert_eq!(fleet.crippled.cruisers, hulls(0, 0, 1), "fleet {idx}");
    }
}

#[test]
fn finishing_fire_takes_this_pass_first_then_the_most_turns_left() {
    let finished = |before: CrippledHulls, ships: u16| {
        let mut game_data = board();
        place(&mut game_data, 0, 1, Ships { ca: 1, ..NONE }, 10);
        place(&mut game_data, 4, 2, Ships { ca: ships, ..NONE }, 10);
        game_data.fleets[4].crippled.cruisers = before;
        // One nominal cruiser deals 30: exactly one finishing step on a
        // fleet of crippled cruisers.
        fight_capped(&mut game_data, &full_strength_rolls(), 1, 1, 0, 4);
        let fleet = &game_data.fleets[4];
        (fleet.ships.cruisers, fleet.crippled.cruisers)
    };
    assert_eq!(finished(hulls(1, 1, 1), 3), (2, hulls(1, 1, 0)));
    assert_eq!(finished(hulls(1, 1, 0), 2), (1, hulls(1, 0, 0)));
    assert_eq!(finished(hulls(2, 0, 0), 2), (1, hulls(1, 0, 0)));
}

#[test]
fn finishing_fire_spreads_over_a_sides_fleets_by_turns_left() {
    let mut game_data = board();
    place(&mut game_data, 0, 1, Ships { ca: 1, ..NONE }, 10);
    place(&mut game_data, 4, 2, Ships { ca: 1, ..NONE }, 10);
    place(&mut game_data, 8, 2, Ships { ca: 1, ..NONE }, 10);
    game_data.fleets[4].crippled.cruisers = hulls(1, 0, 0);
    game_data.fleets[8].crippled.cruisers = hulls(0, 1, 0);
    let attacker = BattleSide {
        empire: 1,
        fleets: vec![0],
        bases: Vec::new(),
    };
    let defender = BattleSide {
        empire: 2,
        fleets: vec![4, 8],
        bases: Vec::new(),
    };

    fight_encounter_with_dice(
        &mut game_data,
        &full_strength_rolls(),
        1,
        1,
        DEEP_SPACE,
        &attacker,
        &defender,
    );

    // The hull with two turns left is the one finished, though fleet 4 is
    // earlier in the side; the other keeps its single turn.
    assert_eq!(game_data.fleets[8].ships.cruisers, 0);
    assert_eq!(game_data.fleets[4].ships.cruisers, 1);
    assert_eq!(game_data.fleets[4].crippled.cruisers, hulls(1, 0, 0));
    assert!(game_data.fleets[8].crippled.cruisers.is_empty());
}

#[test]
fn a_surviving_cripple_keeps_its_count() {
    let mut game_data = board();
    place(&mut game_data, 0, 1, Ships { dd: 1, ..NONE }, 10);
    place(&mut game_data, 4, 2, Ships { bb: 2, ..NONE }, 10);
    game_data.fleets[4].crippled.battleships = hulls(0, 1, 0);
    // A lone destroyer's 10 hits buy no step against a battleship.
    fight_capped(&mut game_data, &full_strength_rolls(), 1, 1, 0, 4);
    let fleet = &game_data.fleets[4];
    assert_eq!(fleet.ships.battleships, 2);
    assert_eq!(fleet.crippled.battleships, hulls(0, 1, 0));
}

fn holders(parts: &[(u32, CrippledHulls)]) -> Vec<Holder> {
    parts
        .iter()
        .map(|&(hulls, crippled)| Holder { hulls, crippled })
        .collect()
}

fn state(parts: &[(u32, CrippledHulls)]) -> (u32, CrippledHulls) {
    let mut total = CrippledHulls::default();
    for (_, crippled) in parts {
        total.add(crippled);
    }
    (parts.iter().map(|part| part.0).sum(), total)
}

#[test]
fn newly_crippled_hulls_go_to_the_first_nominal_hulls() {
    let mut side = holders(&[(2, hulls(0, 1, 0)), (3, CrippledHulls::default())]);
    // Two hulls were crippled this battle.
    apply_class_change(&mut side, (5, &hulls(0, 1, 0)), (5, &hulls(0, 1, 2)));
    assert_eq!(side[0].crippled, hulls(0, 1, 1));
    assert_eq!(side[1].crippled, hulls(0, 0, 1));
    assert_eq!(side[0].hulls + side[1].hulls, 5);
}

#[test]
fn destroyed_nominal_hulls_leave_the_first_holders_that_have_any() {
    let mut side = holders(&[(1, hulls(1, 0, 0)), (2, CrippledHulls::default())]);
    apply_class_change(&mut side, (3, &hulls(1, 0, 0)), (1, &hulls(1, 0, 0)));
    assert_eq!((side[0].hulls, side[1].hulls), (1, 0));
    assert_eq!(side[0].crippled, hulls(1, 0, 0));
}

#[test]
fn a_hull_crippled_and_finished_in_one_battle_counts_as_a_nominal_one_destroyed() {
    // Three nominal hulls; the battle crippled two and finished one of them.
    let mut side = holders(&[(3, CrippledHulls::default())]);
    apply_class_change(
        &mut side,
        (3, &CrippledHulls::default()),
        (2, &hulls(0, 0, 1)),
    );
    assert_eq!(side[0].hulls, 2);
    assert_eq!(side[0].crippled, hulls(0, 0, 1));
}

#[test]
fn a_change_is_conserved_over_any_split_of_the_holders() {
    let before = [
        (3, hulls(1, 1, 0)),
        (2, hulls(0, 1, 1)),
        (4, CrippledHulls::default()),
    ];
    let (hulls_before, crippled_before) = state(&before);
    let mut after_crippled = crippled_before;
    let finished = after_crippled.take(3);
    after_crippled.cripple(2);
    let hulls_after = hulls_before - finished.total() - 1;
    let mut side = holders(&before);
    apply_class_change(
        &mut side,
        (hulls_before, &crippled_before),
        (hulls_after, &after_crippled),
    );
    let total: u32 = side.iter().map(|holder| holder.hulls).sum();
    let mut crippled = CrippledHulls::default();
    for holder in &side {
        crippled.add(&holder.crippled);
        assert!(holder.crippled.total() <= holder.hulls);
    }
    assert_eq!(total, hulls_after);
    assert_eq!(crippled, after_crippled);
}

fn base(game_data: &mut nc_data::CoreGameData, id: u16, crippled: CrippledHulls) -> usize {
    let mut record = Starbase::default();
    record.base_id.0 = id;
    record.owner.0 = 2;
    record.crippled = crippled;
    game_data.bases.push(record);
    game_data.bases.len() - 1
}

#[test]
fn a_destroyed_base_is_the_crippled_one_and_a_survivor_keeps_its_state() {
    let mut game_data = board();
    game_data.bases.clear();
    let whole = base(&mut game_data, 1, CrippledHulls::default());
    let hurt = base(&mut game_data, 2, hulls(0, 1, 0));
    let mut before = FleetCombatState::default();
    before.counts[IDX_SB] = 2;
    before.crippled[IDX_SB] = hulls(0, 1, 0);
    let mut after = FleetCombatState::default();
    after.counts[IDX_SB] = 1;

    let destroyed = write_back_bases(&mut game_data, &[whole, hurt], &before, &after);

    assert_eq!(destroyed, [2]);
    assert_eq!(game_data.bases[hurt].base_id.0, 0);
    assert_eq!(game_data.bases[whole].base_id.0, 1);
    assert!(game_data.bases[whole].crippled.is_empty());

    // A base the battle crippled keeps the hull's count.
    let mut crippled_after = FleetCombatState::default();
    crippled_after.counts[IDX_SB] = 1;
    crippled_after.crippled[IDX_SB] = hulls(0, 0, 1);
    let mut one = FleetCombatState::default();
    one.counts[IDX_SB] = 1;
    let destroyed = write_back_bases(&mut game_data, &[whole], &one, &crippled_after);
    assert!(destroyed.is_empty());
    assert_eq!(game_data.bases[whole].crippled, hulls(0, 0, 1));
}

#[test]
fn a_crippled_hull_counts_at_its_crippled_attack_strength() {
    let mut crippled = nc_data::CrippledShips::default();
    assert_eq!(warship_attack_strength(2, 2, 1, &crippled), 20 + 60 + 90);
    crippled.destroyers = hulls(0, 0, 1);
    crippled.cruisers = hulls(1, 1, 0);
    crippled.battleships = hulls(0, 1, 0);
    assert_eq!(
        warship_attack_strength(2, 2, 1, &crippled),
        10 + 5 + 15 + 15 + 45
    );

    let mut record = Starbase::default();
    assert_eq!(base_attack_strength(&record), 100);
    record.crippled = hulls(0, 1, 0);
    assert_eq!(base_attack_strength(&record), 50);
}

#[test]
fn bombardment_fire_keeps_its_crippled_rounding() {
    let mut state = FleetCombatState::default();
    state.counts[IDX_DD] = 2;
    state.counts[IDX_CA] = 2;
    state.counts[IDX_BB] = 2;
    assert_eq!(bombard_attack_as(&state), 2 * 5 + 2 * 30 + 2 * 135);
    state.crippled[IDX_DD] = hulls(1, 0, 0);
    state.crippled[IDX_CA] = hulls(0, 1, 0);
    state.crippled[IDX_BB] = hulls(0, 0, 1);
    assert_eq!(bombard_attack_as(&state), 5 + 2 + 30 + 15 + 135 + 67);
}
