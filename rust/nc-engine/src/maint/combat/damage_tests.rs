//! Damage that lasts (combat decision 3): a battle's cripples stay crippled
//! into every later battle, finishing fire takes the hulls closest to
//! recovery last, and the write-back spreads a side's change over its
//! fleets and starbases.
//!
//! Inline for the same reason as `fleet_battle_tests`: the scripted dice
//! pin the arithmetic exactly. Every scripted roll is 6, so each volley
//! deals its side's whole AS, and a one-round cap ends a battle after that
//! exchange.

use nc_data::{CrippledHulls, GameRng, Starbase, set_armies_aboard};

use super::assault::bombard_attack_as;
use super::damage::{Holder, apply_class_change, apportion, write_back_base, write_back_strike};
use super::fleet_battle::{BattleSide, fight_encounter_with_dice};
use super::fleet_battle_tests::{
    CAUGHT, DEEP_SPACE, NONE, Scripted, Ships, board, fight_capped, place,
};
use super::state::{
    FleetCombatState, IDX_BB, IDX_CA, IDX_DD, IDX_SB, IDX_TT, TransportGroup, Transports,
    base_attack_strength, fleet_state_from_records, warship_attack_strength,
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
        false,
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
        .map(|&(hulls, crippled)| Holder {
            hulls,
            crippled,
            loaded: false,
        })
        .collect()
}

/// No allocation below draws: every share here is untied.
fn no_draws() -> GameRng {
    GameRng::from_seed(0)
}

fn state(parts: &[(u32, CrippledHulls)]) -> (u32, CrippledHulls) {
    let mut total = CrippledHulls::default();
    for (_, crippled) in parts {
        total.add(crippled);
    }
    (parts.iter().map(|part| part.0).sum(), total)
}

/// Two hulls crippled this battle are shared by nominal hulls: one with
/// one nominal hull and one with three take 0.5 and 1.5, so the larger
/// remainder (the first holder's) is rounded up first.
#[test]
fn newly_crippled_hulls_are_shared_by_nominal_hulls() {
    let mut side = holders(&[(2, hulls(0, 1, 0)), (3, CrippledHulls::default())]);
    apply_class_change(
        &mut side,
        (5, &hulls(0, 1, 0)),
        (5, &hulls(0, 1, 2)),
        &mut no_draws(),
    );
    assert_eq!(side[0].crippled, hulls(0, 1, 1));
    assert_eq!(side[1].crippled, hulls(0, 0, 1));
    assert_eq!(side[0].hulls + side[1].hulls, 5);
}

/// Two nominal hulls destroyed leave the holders of nominal hulls: the
/// first holder's only hull is crippled, so both come from the second.
#[test]
fn destroyed_nominal_hulls_leave_the_holders_that_have_any() {
    let mut side = holders(&[(1, hulls(1, 0, 0)), (2, CrippledHulls::default())]);
    apply_class_change(
        &mut side,
        (3, &hulls(1, 0, 0)),
        (1, &hulls(1, 0, 0)),
        &mut no_draws(),
    );
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
        &mut no_draws(),
    );
    assert_eq!(side[0].hulls, 2);
    assert_eq!(side[0].crippled, hulls(0, 0, 1));
}

/// Combat decision 6: losses are shared by largest remainder. Fleets of 8
/// and 2 eligible cruisers share 3 losses as 2 and 1 (exact shares 2.4 and
/// 0.6), and no draw is made for an untied award.
#[test]
fn losses_are_shared_by_largest_remainder_without_a_draw_when_untied() {
    let mut rng = GameRng::from_seed(750_000);
    let mut untouched = rng.clone();
    assert_eq!(apportion(&[8, 2], 3, &mut rng), [2, 1]);
    assert_eq!(rng.next_u64(), untouched.next_u64(), "nothing was drawn");
    // Every share is bounded by the holder's hulls and the total is exact.
    for counts in [[0, 3, 1], [2, 2, 2], [1, 0, 0], [3, 3, 2]] {
        for loss in 0..=counts.iter().sum::<u32>() {
            let shares = apportion(&counts, loss, &mut GameRng::from_seed(1));
            assert_eq!(shares.iter().sum::<u32>(), loss, "{counts:?} {loss}");
            assert!(shares.iter().zip(&counts).all(|(s, n)| s <= n));
        }
    }
    // The 8 and 2 example at the battle level: three nominal cruisers
    // destroyed leave the fleets with 6 and 1.
    let mut side = holders(&[(8, CrippledHulls::default()), (2, CrippledHulls::default())]);
    apply_class_change(
        &mut side,
        (10, &CrippledHulls::default()),
        (7, &CrippledHulls::default()),
        &mut no_draws(),
    );
    assert_eq!((side[0].hulls, side[1].hulls), (6, 1));
}

/// A tie at the cutoff is drawn from the battle's seeded stream, without
/// replacement, so either fleet can take the loss, the same seed replays
/// the same draw, and roster order gives no priority.
#[test]
fn a_cutoff_tie_is_drawn_from_the_seeded_stream() {
    let draws: Vec<Vec<u32>> = (0..64u64)
        .map(|seed| apportion(&[1, 1], 1, &mut GameRng::from_seed(seed)))
        .collect();
    assert!(draws.contains(&vec![1, 0]) && draws.contains(&vec![0, 1]));
    for seed in 0..8u64 {
        let (mut first, mut second) = (GameRng::from_seed(seed), GameRng::from_seed(seed));
        let replay: Vec<Vec<u32>> = (0..5)
            .map(|_| apportion(&[1, 1, 1], 2, &mut first))
            .collect();
        for shares in &replay {
            assert_eq!(shares.iter().sum::<u32>(), 2);
            assert!(shares.iter().all(|&share| share <= 1));
        }
        assert_eq!(
            replay,
            (0..5)
                .map(|_| apportion(&[1, 1, 1], 2, &mut second))
                .collect::<Vec<_>>()
        );
    }
}

/// A fleet's transports are two holders, its loaded and its empty
/// subgroup, each with its own crippled cohorts.
fn cargo(loaded: (u32, CrippledHulls), empty: (u32, CrippledHulls)) -> Transports {
    Transports {
        loaded: TransportGroup {
            hulls: loaded.0,
            crippled: loaded.1,
        },
        empty: TransportGroup {
            hulls: empty.0,
            crippled: empty.1,
        },
    }
}

/// Transport losses fall on loaded transports first across the side: six
/// loaded in one fleet and four empty in another lose four loaded ones to
/// four losses, as one fleet of ten would; the empty ones are untouched.
#[test]
fn transport_losses_fall_on_loaded_transports_first_across_the_side() {
    let whole = CrippledHulls::default();
    let mut side = Vec::new();
    side.extend(cargo((6, whole), (0, whole)).holders());
    side.extend(cargo((0, whole), (4, whole)).holders());
    apply_class_change(&mut side, (10, &whole), (6, &whole), &mut no_draws());
    let loaded_fleet = Transports::from_holders(&side[0], &side[1]);
    let empty_fleet = Transports::from_holders(&side[2], &side[3]);
    assert_eq!(loaded_fleet, cargo((2, whole), (0, whole)));
    assert_eq!(empty_fleet, cargo((0, whole), (4, whole)));
}

/// Each cargo subgroup keeps its own crippled counter: crippling and
/// finishing strike the loaded subgroup first, and nothing is counted twice
/// or moves between subgroups. One fleet holds two loaded and two empty
/// transports, another two empty; the side's four nominal transports are
/// crippled, then two of its cripples are finished.
#[test]
fn transport_damage_is_written_to_the_struck_subgroups_own_counter() {
    let whole = CrippledHulls::default();
    let mut side = Vec::new();
    side.extend(cargo((2, whole), (2, whole)).holders());
    side.extend(cargo((0, whole), (2, whole)).holders());
    // Four crippled: the two loaded first, then the empty ones shared by
    // their holders (2 and 2 share two losses as 1 and 1).
    apply_class_change(
        &mut side,
        (6, &whole),
        (6, &hulls(0, 0, 4)),
        &mut no_draws(),
    );
    assert_eq!(
        Transports::from_holders(&side[0], &side[1]),
        cargo((2, hulls(0, 0, 2)), (2, hulls(0, 0, 1)))
    );
    assert_eq!(
        Transports::from_holders(&side[2], &side[3]),
        cargo((0, whole), (2, hulls(0, 0, 1)))
    );
    // Two of the cripples finished: the loaded ones go before any empty
    // one, so the empty subgroups keep their cripple each.
    apply_class_change(
        &mut side,
        (6, &hulls(0, 0, 4)),
        (4, &hulls(0, 0, 2)),
        &mut no_draws(),
    );
    assert_eq!(
        Transports::from_holders(&side[0], &side[1]),
        cargo((0, whole), (2, hulls(0, 0, 1)))
    );
    assert_eq!(
        Transports::from_holders(&side[2], &side[3]),
        cargo((0, whole), (2, hulls(0, 0, 1)))
    );
}

/// Two fleets of one side, one with two loaded transports and one with two
/// empty, meet a cruiser: its 30 hits cripple three transports. The loaded
/// fleet's two come first, then one of the empty fleet's, each on its
/// fleet's own counter for that cargo state, and the armies stay aboard.
/// The unarmed side then breaks off, and the cruiser, no faster than the
/// transports, never catches it.
#[test]
fn a_battle_writes_transport_damage_to_each_fleets_own_cargo_counter() {
    let mut game_data = board();
    place(&mut game_data, 0, 1, Ships { ca: 1, ..NONE }, 10);
    place(&mut game_data, 4, 2, Ships { tt: 2, ..NONE }, 10);
    set_armies_aboard(&mut game_data.fleets[4], 2);
    place(&mut game_data, 8, 2, Ships { tt: 2, ..NONE }, 10);
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
    let dice = Scripted {
        d10: &|_, _, _, _| 6,
        pursuit: &|_, _, _| (0.0, 0.5),
    };

    fight_encounter_with_dice(
        &mut game_data,
        &dice,
        1,
        1,
        DEEP_SPACE,
        &attacker,
        &defender,
        false,
    );

    let loaded = &game_data.fleets[4];
    assert_eq!(
        (
            loaded.ships.loaded_transports,
            loaded.ships.empty_transports
        ),
        (2, 0)
    );
    assert_eq!(loaded.crippled.loaded_transports, hulls(0, 0, 2));
    assert!(loaded.crippled.empty_transports.is_empty());
    let empty = &game_data.fleets[8];
    assert_eq!(
        (empty.ships.loaded_transports, empty.ships.empty_transports),
        (0, 2)
    );
    assert_eq!(empty.crippled.empty_transports, hulls(0, 0, 1));
    assert!(empty.crippled.loaded_transports.is_empty());
}

/// A strike's result goes on the record by the same rule. A fleet of two
/// loaded and two empty transports loses one and has one crippled: both on
/// the loaded subgroup, and the lost one takes its army.
#[test]
fn a_strike_writes_transport_damage_to_the_loaded_subgroup_first() {
    let mut game_data = board();
    place(&mut game_data, 0, 1, Ships { tt: 4, ..NONE }, 10);
    set_armies_aboard(&mut game_data.fleets[0], 2);
    let mut after = fleet_state_from_records(&game_data, &[0], &[]);
    after.counts[IDX_TT] = 3;
    after.crippled[IDX_TT] = hulls(0, 0, 1);

    write_back_strike(&mut game_data, 0, &after);

    let fleet = &game_data.fleets[0];
    assert_eq!(
        (fleet.ships.loaded_transports, fleet.ships.empty_transports),
        (1, 2)
    );
    assert_eq!(fleet.crippled.loaded_transports, hulls(0, 0, 1));
    assert!(fleet.crippled.empty_transports.is_empty());
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
        &mut GameRng::from_seed(7),
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
    let gone = FleetCombatState::default();
    let mut kept = FleetCombatState::default();
    kept.counts[IDX_SB] = 1;

    assert_eq!(write_back_base(&mut game_data, hurt, &gone), Some(2));
    assert_eq!(write_back_base(&mut game_data, whole, &kept), None);
    assert_eq!(game_data.bases[hurt].base_id.0, 0);
    assert_eq!(game_data.bases[whole].base_id.0, 1);
    assert!(game_data.bases[whole].crippled.is_empty());

    // A base the battle crippled keeps the hull's count.
    let mut crippled_after = FleetCombatState::default();
    crippled_after.counts[IDX_SB] = 1;
    crippled_after.crippled[IDX_SB] = hulls(0, 0, 1);
    assert_eq!(
        write_back_base(&mut game_data, whole, &crippled_after),
        None
    );
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
