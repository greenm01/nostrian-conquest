//! The landing battle and the battery shield.
//!
//! Inline rather than in tests/: maintenance rolls seeded dice, so these
//! tests script the landing battle's dice and call the planet-damage
//! functions directly to pin the arithmetic exactly. Scripted rolls: 0 is
//! 25% of the side's armies, 3 is 50%, 6 is 100%. The expected values come
//! from `tools/combat-model/ground_calibration.py` (`Shared(EOTS_X4, 4,
//! first=True)`), the model combat decision 4 was measured with.

use nc_data::{Planet, Real48};

use super::assault::{
    apply_planet_bombardment_damage, resolve_landing_battle, resolve_seeded_landing,
};
use super::exchange::{COMBAT_KIND_BLITZ_GROUND, COMBAT_KIND_GROUND, landing_volley};

const ATTACKER: u8 = 1;
const DEFENDER: u8 = 2;

/// Who fires in which round, in the order the battle asked for the dice.
type Volleys = Vec<(u32, u8)>;

/// A landing battle with every side's dice from `dice(round, acting_empire)`.
fn landing(attackers: u32, defenders: u32, dice: impl Fn(u32, u8) -> u8) -> ((u32, u32), Volleys) {
    let mut volleys = Volleys::new();
    let mut roll = |round, acting_empire, _target_empire| {
        volleys.push((round, acting_empire));
        dice(round, acting_empire)
    };
    let survivors = resolve_landing_battle(ATTACKER, DEFENDER, attackers, defenders, &mut roll);
    (survivors, volleys)
}

#[test]
fn the_landing_volley_reads_the_flat_table_in_quarter_armies() {
    let quarters: Vec<u32> = (0..10).map(|roll| landing_volley(roll, 4)).collect();
    assert_eq!(quarters, [4, 4, 4, 8, 8, 8, 16, 16, 16, 16]);
    // No critical: a 9 is a full volley and nothing more.
    assert_eq!(landing_volley(9, 1), 4);
}

#[test]
fn the_defenders_fire_first_and_the_survivors_reply() {
    // Three armies a side, every roll 3 (50%): two quarters an army.
    let ((attackers, defenders), volleys) = landing(3, 3, |_, _| 3);

    // Round 1: the defenders' 6 quarters kill 1 attacker and carry 2; the 2
    // attackers' 4 quarters kill 1 defender. Round 2: 2 + 4 kills 1 more
    // attacker, carrying 2; the last attacker's 2 quarters kill nobody.
    // Round 3: 2 + 4 kills the last attacker, who never fires.
    assert_eq!((attackers, defenders), (0, 2));
    assert_eq!(
        volleys,
        [
            (1, DEFENDER),
            (1, ATTACKER),
            (2, DEFENDER),
            (2, ATTACKER),
            (3, DEFENDER)
        ]
    );
}

#[test]
fn damage_short_of_an_army_carries_to_the_next_round_of_the_landing() {
    // Four armies a side, every roll 0 (25%): one quarter an army. Without
    // carry no volley after the first would ever kill an army.
    let ((attackers, defenders), volleys) = landing(4, 4, |_, _| 0);

    // Round 1: the defenders' 4 quarters kill 1 attacker; the 3 attackers'
    // 3 quarters kill nobody but are carried. Round 2: 4 more quarters kill
    // another attacker, and the 2 attackers' 2 quarters on top of the carried
    // 3 kill a defender, carrying 1. The battle runs five rounds, the
    // attackers' last volley in round 4, and the defenders win with 2 armies.
    assert_eq!((attackers, defenders), (0, 2));
    assert_eq!(volleys.len(), 9);
    assert_eq!(volleys.last(), Some(&(5, DEFENDER)));
}

#[test]
fn a_second_landing_starts_without_the_first_ones_carry() {
    // Three attackers against one defender, every roll 0. The attackers win
    // in round 2 with two quarters of damage still on them.
    let ((attackers, defenders), _) = landing(3, 1, |_, _| 0);
    assert_eq!((attackers, defenders), (3, 0));

    // That damage is lost with the battle: the next landing is fought from
    // whole armies. Had the two quarters carried, round 2 would have cost
    // the attackers one army.
    let ((attackers, defenders), _) = landing(3, 1, |_, _| 0);
    assert_eq!((attackers, defenders), (3, 0));
}

#[test]
fn the_attackers_fire_only_with_what_survives() {
    // Two armies a side and every roll 6 (100%): the defenders' 8 quarters
    // kill both attackers before they fire, so the defenders lose nothing.
    // The old simultaneous round would have destroyed both sides.
    let ((attackers, defenders), volleys) = landing(2, 2, |_, _| 6);
    assert_eq!((attackers, defenders), (0, 2));
    assert_eq!(volleys, [(1, DEFENDER)]);
}

#[test]
fn a_landing_ends_with_exactly_one_side_standing() {
    for attackers in 1..=8 {
        for defenders in 1..=8 {
            for attacker_roll in 0..10 {
                for defender_roll in 0..10 {
                    let ((attackers_left, defenders_left), _) =
                        landing(attackers, defenders, |_, empire| {
                            if empire == ATTACKER {
                                attacker_roll
                            } else {
                                defender_roll
                            }
                        });
                    assert!(
                        (attackers_left == 0) != (defenders_left == 0),
                        "{attackers} v {defenders}, rolls {attacker_roll}/{defender_roll}: \
                         {attackers_left} v {defenders_left}"
                    );
                    assert!(attackers_left <= attackers && defenders_left <= defenders);
                }
            }
        }
    }
}

#[test]
fn an_empty_side_ends_the_landing_without_a_roll() {
    assert_eq!(landing(0, 5, |_, _| 9), ((0, 5), Vec::new()));
    assert_eq!(landing(5, 0, |_, _| 9), ((5, 0), Vec::new()));
}

/// (attackers, defenders, capture rate, mean armies the attackers keep, mean
/// armies the defenders keep): the model's exact enumeration.
const MODEL_ROWS: [(u32, u32, f64, f64, f64); 5] = [
    (1, 1, 0.335661, 0.3357, 0.6643),
    (3, 2, 0.556289, 1.1280, 0.6858),
    (10, 10, 0.199380, 1.0737, 6.4748),
    (15, 10, 0.561074, 5.0843, 2.7230),
    (20, 10, 0.917611, 11.4259, 0.4363),
];

#[test]
fn seeded_landings_match_the_model() {
    // 20,000 campaign seeds a row through the seeded dice every landing
    // uses, invasion and blitz alike. A count is within 4.5 binomial
    // standard deviations of the model's exact rate, a mean within 4.5
    // standard errors of the model's exact mean.
    const TRIALS: u32 = 20_000;
    // Each row and kind draws its own block of seeds, so rows do not share
    // luck.
    let mut next_seed = 0..;
    for kind in [COMBAT_KIND_GROUND, COMBAT_KIND_BLITZ_GROUND] {
        for (attackers, defenders, capture, mean_attackers, mean_defenders) in MODEL_ROWS {
            let first_seed = u64::from(TRIALS) * next_seed.next().unwrap_or(0);
            let (mut captures, mut sum_a, mut sum_a2, mut sum_d, mut sum_d2) =
                (0u32, 0.0, 0.0, 0.0, 0.0);
            for seed in first_seed..first_seed + u64::from(TRIALS) {
                let (a, d) = resolve_seeded_landing(
                    seed,
                    3000,
                    [15, 13],
                    kind,
                    ATTACKER,
                    DEFENDER,
                    attackers,
                    defenders,
                );
                assert!((a == 0) != (d == 0));
                captures += u32::from(a > 0);
                sum_a += f64::from(a);
                sum_a2 += f64::from(a * a);
                sum_d += f64::from(d);
                sum_d2 += f64::from(d * d);
            }
            let n = f64::from(TRIALS);
            let label = format!("{attackers} v {defenders}, kind {kind}");
            eprintln!(
                "{label}: capture {:.4} (model {capture}), attackers left {:.4} \
                 (model {mean_attackers}), defenders left {:.4} (model {mean_defenders})",
                f64::from(captures) / f64::from(TRIALS),
                sum_a / f64::from(TRIALS),
                sum_d / f64::from(TRIALS),
            );
            let sd = (n * capture * (1.0 - capture)).sqrt();
            assert!(
                (f64::from(captures) - n * capture).abs() <= 4.5 * sd,
                "{label}: {captures} captures of {TRIALS}, model {:.1}",
                n * capture
            );
            for (sum, sum2, model, side) in [
                (sum_a, sum_a2, mean_attackers, "attackers"),
                (sum_d, sum_d2, mean_defenders, "defenders"),
            ] {
                let mean = sum / n;
                let se = ((sum2 / n - mean * mean) / n).sqrt();
                assert!(
                    (mean - model).abs() <= 4.5 * se + 5e-5,
                    "{label}: mean {side} left {mean:.4} (se {se:.4}), model {model}"
                );
            }
        }
    }
}

// Combat decision 5: batteries shield the stardock.

/// A world with `batteries` standing, `armies`, and `battleships` docked.
fn world(batteries: u16, armies: u16, destroyers: u32, battleships: u32) -> Planet {
    let mut planet = Planet {
        ground_batteries: batteries,
        armies,
        production: Real48::from_int(100),
        stored_goods: Real48::from_int(500),
        ..Planet::default()
    };
    planet.stardock.destroyers = destroyers;
    planet.stardock.battleships = battleships;
    planet
}

#[test]
fn hits_left_after_the_last_battery_falls_pour_down_the_hierarchy() {
    let mut planet = world(2, 10, 0, 1);

    // 400 hits against two batteries: 132 cripple both and 132 destroy them.
    // The other 136 go on: 110 destroy the docked battleship (DS 110), and
    // 26 destroy two armies (10 each).
    apply_planet_bombardment_damage(&mut planet, 400);
    assert_eq!(planet.ground_batteries, 0);
    assert_eq!(planet.crippled_batteries, 0);
    assert_eq!(planet.stardock.battleships, 0);
    assert_eq!(planet.armies, 8);
}

#[test]
fn an_undefended_world_with_an_empty_dock_loses_armies_at_once() {
    let mut planet = world(0, 22, 0, 0);

    // A lone cruiser's round: 19 hits, one army and 9 to spare.
    apply_planet_bombardment_damage(&mut planet, 19);
    assert_eq!(planet.armies, 21);
}

#[test]
fn hits_short_of_the_last_battery_do_not_spill_onto_the_dock() {
    let mut planet = world(3, 0, 5, 0);

    // 131 hits cripple one battery; the other 65 expire. Spilled onto the
    // dock they would have destroyed all five docked destroyers (DS 7).
    apply_planet_bombardment_damage(&mut planet, 131);
    assert_eq!(planet.ground_batteries, 3);
    assert_eq!(planet.crippled_batteries, 1);
    assert_eq!(planet.stardock.destroyers, 5);
}

#[test]
fn batteries_are_all_crippled_before_any_is_destroyed() {
    let mut planet = world(3, 0, 0, 0);

    // 250 hits: 198 cripple all three, then 52 fall short of finishing one.
    apply_planet_bombardment_damage(&mut planet, 250);
    assert_eq!((planet.ground_batteries, planet.crippled_batteries), (3, 3));
    // Crippled batteries are finished for 66 hits each: two of three.
    apply_planet_bombardment_damage(&mut planet, 140);
    assert_eq!((planet.ground_batteries, planet.crippled_batteries), (1, 1));
}

#[test]
fn an_unshielded_dock_takes_the_hits_before_the_armies_behind_it() {
    let mut planet = world(0, 20, 0, 1);

    // The round-3 breakthrough: 120 hits, 110 for the docked battleship and
    // 10 for one army.
    apply_planet_bombardment_damage(&mut planet, 120);
    assert_eq!(planet.stardock.battleships, 0);
    assert_eq!(planet.armies, 19);
}

#[test]
fn bombardment_destroys_only_positive_goods_and_keeps_their_fraction() {
    // 20 hits reach industry as 2 points: 50 goods and 2 production.
    let mut planet = world(0, 0, 0, 0);
    planet.stored_goods = Real48::from_f64(100.75);
    planet.production = Real48::from_f64(50.25);
    apply_planet_bombardment_damage(&mut planet, 20);
    assert_eq!(planet.stored_goods.to_f64(), 50.75);
    assert_eq!(planet.production.to_f64(), 48.25);

    // More than the world holds leaves 0, not a debt.
    planet.stored_goods = Real48::from_f64(10.5);
    apply_planet_bombardment_damage(&mut planet, 20);
    assert_eq!(planet.stored_goods.to_f64(), 0.0);
}

#[test]
fn bombarding_a_world_in_debt_leaves_the_debt() {
    let mut planet = world(0, 0, 0, 0);
    planet.stored_goods = Real48::from_f64(-40.5);
    apply_planet_bombardment_damage(&mut planet, 20);
    assert_eq!(planet.stored_goods.to_f64(), -40.5);
    assert_eq!(planet.production.to_f64(), 98.0, "production still burns");
}
