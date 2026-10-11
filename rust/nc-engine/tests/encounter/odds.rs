//! The random rules against their formulas: many seeded trials with
//! binomial tolerances.

use nc_data::GameRng;
use nc_engine::maint::encounter::rules::{
    FleetFacts, Situation, Verdict, decide_vs_fleet, intercept, order, roll, sighting_percent,
    strength,
};

use super::support::assert_rate;

const TRIALS: usize = 20_000;

fn fleet(owner: u8, max_speed: u8, speed: u8) -> FleetFacts {
    FleetFacts {
        owner,
        order: order::HOLD,
        sector: [8, 8],
        target: [8, 8],
        speed,
        max_speed,
        roe: 6,
        cruisers: 1,
        ..FleetFacts::default()
    }
}

/// Attacker and target alone on their sides, the target's empire aware of
/// the attacker at `awareness`.
fn meeting(attacker: &FleetFacts, target: &FleetFacts, awareness: u8) -> Situation {
    Situation {
        world_owner: None,
        declared_enemy: true,
        target_awareness: awareness,
        own_strength: strength(attacker),
        enemy_strength: strength(target),
    }
}

/// `E[min(1, r sin(pi u))]` for `u` uniform in [0, 1).
fn mean_capped_sine(ratio: f64) -> f64 {
    let n = 100_000;
    (0..n)
        .map(|i| (ratio * (std::f64::consts::PI * (i as f64 + 0.5) / n as f64).sin()).min(1.0))
        .sum::<f64>()
        / n as f64
}

#[test]
fn sighting_rolls_succeed_at_the_formula_rate() {
    let mut rng = GameRng::from_seed(35);
    // Awareness adds 10 points per level on top of visibility x factor.
    for (awareness, visibility, factor) in [
        (0, 0.1053, 10.0),
        (0, 0.745, 5.0),
        (1, 0.5266, 1.0),
        (3, 0.91, 1.0),
    ] {
        let percent = sighting_percent(awareness, visibility, factor);
        let hits = (0..TRIALS).filter(|_| roll(&mut rng, percent)).count();
        assert_rate(
            &format!("sighting at {percent:.3}%"),
            hits,
            TRIALS,
            percent / 100.0,
            4.5,
        );
    }
}

/// The spec's intercept probe: target max speed 6 moving at 1, unaware of
/// the attacker. Classic measured 0.651, 0.852, 0.976, and 1.000.
#[test]
fn intercept_odds_follow_the_speed_ratio() {
    let mut rng = GameRng::from_seed(36);
    let target = fleet(2, 6, 1);
    for (attacker_max, classic) in [(1u8, 0.651), (2, 0.852), (6, 0.976), (7, 1.0)] {
        let attacker = fleet(1, attacker_max, 0);
        let expected = if attacker_max > 6 {
            1.0
        } else {
            mean_capped_sine(f64::from(attacker_max))
        };
        let hits = (0..TRIALS)
            .filter(|_| intercept(&mut rng, &attacker, &target, meeting(&attacker, &target, 0)))
            .count();
        eprintln!(
            "attacker max {attacker_max}: expected {expected:.3} classic {classic:.3} rust {:.3}",
            hits as f64 / TRIALS as f64
        );
        assert_rate(
            &format!("intercept, attacker max {attacker_max}"),
            hits,
            TRIALS,
            expected,
            4.5,
        );
    }
}

/// A target that already knows the attacker notices it only with
/// `100 - 33 * awareness` percent; otherwise it stays to fight only if its
/// own ROE accepts the odds.
#[test]
fn an_aware_target_rarely_runs() {
    let mut rng = GameRng::from_seed(37);
    let attacker = fleet(1, 1, 0);
    let mut target = fleet(2, 6, 6);
    let run_odds = mean_capped_sine(1.0 / 6.0);
    for (awareness, target_roe) in [(1u8, 0u8), (3, 0), (3, 10)] {
        target.roe = target_roe;
        let notice = (100.0 - 33.0 * f64::from(awareness)) / 100.0;
        let stays = if target_roe == 10 { 1.0 } else { 0.0 };
        let expected = notice * run_odds + (1.0 - notice) * stays;
        let hits = (0..TRIALS)
            .filter(|_| {
                intercept(
                    &mut rng,
                    &attacker,
                    &target,
                    meeting(&attacker, &target, awareness),
                )
            })
            .count();
        assert_rate(
            &format!("intercept with awareness {awareness}, target ROE {target_roe}"),
            hits,
            TRIALS,
            expected,
            4.5,
        );
    }
}

/// A decision against an identified enemy attacks only when the strength
/// test passes, and then at the intercept rate; a failed test never
/// attacks.
#[test]
fn decisions_attack_at_the_strength_test_times_the_intercept_rate() {
    let mut rng = GameRng::from_seed(38);
    let mut target = fleet(2, 6, 3);
    let observer = fleet(1, 3, 0);
    // ROE 6 against an equal cruiser passes; against 2 cruisers it fails.
    for (target_cruisers, expected) in [(1u16, mean_capped_sine(1.0)), (2, 0.0)] {
        target.cruisers = target_cruisers;
        let enemy = meeting(&observer, &target, 0);
        let attacks = (0..TRIALS)
            .filter(|_| decide_vs_fleet(&mut rng, &observer, &target, enemy) == Verdict::Engaged)
            .count();
        assert_rate(
            &format!("recheck against {target_cruisers} CA"),
            attacks,
            TRIALS,
            expected,
            4.5,
        );
    }
}
