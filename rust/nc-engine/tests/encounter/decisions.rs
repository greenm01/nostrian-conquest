//! The decision table (`1b05:1277`, `1b05:1d11`), strength, ROE, and the
//! observer and visibility factors, checked case by case.

use nc_data::GameRng;
use nc_engine::maint::encounter::rules::{
    FleetFacts, Mode, Situation, Verdict, base_factor, decide_vs_base, decide_vs_fleet,
    fleet_posture, fleet_visibility, order, roe_permits, strength,
};

const HERE: [u8; 2] = [15, 13];

/// A fleet at rest on its order in `HERE`'s system (layer 1).
fn at_world(owner: u8, order: u8) -> FleetFacts {
    FleetFacts {
        owner,
        order,
        sector: HERE,
        layer: 1,
        target: HERE,
        target_layer: 1,
        max_speed: 3,
        cruisers: 2,
        ..FleetFacts::default()
    }
}

fn situation(world_owner: Option<u8>, declared_enemy: bool) -> Situation {
    Situation {
        world_owner,
        declared_enemy,
        target_awareness: 0,
    }
}

/// Decide with a target that cannot get away (it is stopped, so the
/// intercept always succeeds).
fn verdict(o: FleetFacts, t: FleetFacts, situation: Situation, mode: Mode) -> Verdict {
    decide_vs_fleet(&mut GameRng::from_seed(1), &o, &t, situation, mode)
}

#[test]
fn neutral_fleet_in_an_owned_system_is_attacked_by_any_willing_fleet() {
    let defender = at_world(2, order::HOLD).roe_set(1);
    for visitor_order in [
        order::HOLD,
        order::VIEW,
        order::COLONIZE,
        order::GUARD_BLOCKADE,
        order::BOMBARD,
    ] {
        let visitor = at_world(1, visitor_order);
        assert_eq!(
            verdict(defender, visitor, situation(Some(2), false), Mode::Identify),
            Verdict::Engaged,
            "order {visitor_order}"
        );
    }
}

#[test]
fn neutral_fleet_elsewhere_is_ignored() {
    let observer = at_world(2, order::PATROL).roe_set(10);
    let visitor = at_world(1, order::HOLD);
    // Someone else's world, an unowned world, and a worldless sector.
    for world_owner in [Some(3), Some(0), None] {
        assert_eq!(
            verdict(
                observer,
                visitor,
                situation(world_owner, false),
                Mode::Identify
            ),
            Verdict::Ignored
        );
    }
    // Deep space over the observer's own world is not its system.
    let mut deep = observer;
    deep.layer = 0;
    let mut deep_visitor = visitor;
    deep_visitor.layer = 0;
    assert_eq!(
        verdict(deep, deep_visitor, situation(None, false), Mode::Identify),
        Verdict::Ignored
    );
}

#[test]
fn only_the_observers_own_declaration_makes_an_enemy() {
    let observer = at_world(2, order::HOLD).roe_set(6);
    let mut visitor = at_world(1, order::HOLD);
    visitor.layer = 0;
    let mut deep_observer = observer;
    deep_observer.layer = 0;
    assert_eq!(
        verdict(
            deep_observer,
            visitor,
            situation(None, true),
            Mode::Identify
        ),
        Verdict::Engaged
    );
    // The visitor's empire declaring the observer's an enemy is not enough.
    assert_eq!(
        verdict(
            deep_observer,
            visitor,
            situation(None, false),
            Mode::Identify
        ),
        Verdict::Ignored
    );
}

#[test]
fn identification_commits_any_roe_but_rechecks_need_the_odds() {
    let mut observer = at_world(2, order::HOLD).roe_set(2);
    observer.layer = 0;
    let mut enemy = at_world(1, order::HOLD);
    enemy.layer = 0;
    enemy.battleships = 10;
    let enemy_situation = situation(None, true);
    assert_eq!(
        verdict(observer, enemy, enemy_situation, Mode::Identify),
        Verdict::Engaged
    );
    assert_eq!(
        verdict(observer, enemy, enemy_situation, Mode::Recheck),
        Verdict::Avoided
    );
    observer.roe = 0;
    assert_eq!(
        verdict(observer, enemy, enemy_situation, Mode::Identify),
        Verdict::Avoided
    );
}

#[test]
fn blockade_attacks_assaults_and_colonizers_of_an_unowned_world() {
    let blockade = at_world(3, order::GUARD_BLOCKADE).roe_set(6);
    for assault in [order::BOMBARD, order::INVADE, order::BLITZ] {
        assert_eq!(
            verdict(
                blockade,
                at_world(1, assault),
                situation(Some(2), false),
                Mode::Identify
            ),
            Verdict::Engaged
        );
    }
    let colonizer = at_world(1, order::COLONIZE);
    assert_eq!(
        verdict(
            blockade,
            colonizer,
            situation(Some(0), false),
            Mode::Identify
        ),
        Verdict::Engaged
    );
    // Colonizing an owned world, and other neutral orders, are ignored.
    assert_eq!(
        verdict(
            blockade,
            colonizer,
            situation(Some(2), false),
            Mode::Identify
        ),
        Verdict::Ignored
    );
    assert_eq!(
        verdict(
            blockade,
            at_world(1, order::HOLD),
            situation(Some(2), false),
            Mode::Identify
        ),
        Verdict::Ignored
    );
    // A colonizer in a worldless sector has no unowned world to race for.
    assert_eq!(
        verdict(blockade, colonizer, situation(None, false), Mode::Identify),
        Verdict::Ignored
    );
}

#[test]
fn colonizers_race_for_an_unowned_world() {
    let first = at_world(1, order::COLONIZE).roe_set(1);
    let second = at_world(2, order::COLONIZE);
    assert_eq!(
        verdict(first, second, situation(Some(0), false), Mode::Identify),
        Verdict::Engaged
    );
    assert_eq!(
        verdict(
            first.roe_set(0),
            second,
            situation(Some(0), false),
            Mode::Identify
        ),
        Verdict::Avoided
    );
}

#[test]
fn assault_fleet_blocked_by_a_guard_fights_or_aborts() {
    let guard = at_world(2, order::GUARD_BLOCKADE);
    let bomber = at_world(1, order::BOMBARD);
    assert_eq!(
        verdict(
            bomber.roe_set(6),
            guard,
            situation(Some(2), false),
            Mode::Identify
        ),
        Verdict::Engaged
    );
    assert_eq!(
        verdict(bomber, guard, situation(Some(2), false), Mode::Identify),
        Verdict::Aborted
    );
    // A guard ordered elsewhere is not guarding this world.
    let mut leaving = guard;
    leaving.target = [1, 1];
    assert_eq!(
        verdict(bomber, leaving, situation(Some(2), false), Mode::Identify),
        Verdict::Ignored
    );
}

#[test]
fn faster_attacker_always_intercepts_and_a_slower_one_may_not() {
    let mut observer = at_world(2, order::HOLD).roe_set(6);
    observer.layer = 0;
    let mut runner = at_world(1, order::HOLD);
    runner.layer = 0;
    runner.speed = 6;
    runner.max_speed = 6;
    runner.cruisers = 0;
    let enemy = situation(None, true);
    let mut rng = GameRng::from_seed(9);
    observer.max_speed = 7;
    assert!((0..200).all(|_| {
        decide_vs_fleet(&mut rng, &observer, &runner, enemy, Mode::Identify) == Verdict::Engaged
    }));
    observer.max_speed = 1;
    let escapes = (0..200)
        .filter(|_| {
            decide_vs_fleet(&mut rng, &observer, &runner, enemy, Mode::Identify) == Verdict::Escaped
        })
        .count();
    assert!(escapes > 0);
}

#[test]
fn starbases_are_attacked_only_as_an_assault_target_or_a_declared_enemy() {
    let mut bomber = at_world(1, order::BOMBARD).roe_set(6);
    bomber.battleships = 2;
    assert_eq!(decide_vs_base(&bomber, 30, false), Verdict::Engaged);
    // The base protects the target world: its guard counts too.
    let guarded = 30 + strength(&at_world(2, order::GUARD_STARBASE)) + 100;
    assert_eq!(decide_vs_base(&bomber, guarded, false), Verdict::Aborted);
    let hold = at_world(1, order::HOLD).roe_set(10);
    assert_eq!(decide_vs_base(&hold, 30, false), Verdict::Ignored);
    assert_eq!(decide_vs_base(&hold, 30, true), Verdict::Engaged);
    // A declared enemy weighs the base alone (30), not its guard.
    let cautious = at_world(1, order::HOLD).roe_set(6);
    assert_eq!(decide_vs_base(&cautious, 1000, true), Verdict::Avoided);
    let mut strong = cautious;
    strong.battleships = 2;
    assert_eq!(decide_vs_base(&strong, 1000, true), Verdict::Engaged);
}

#[test]
fn strength_counts_warships_and_a_guard_at_rest() {
    let mut fleet = at_world(1, order::HOLD);
    fleet.battleships = 1;
    fleet.cruisers = 2;
    fleet.destroyers = 3;
    fleet.scouts = 5;
    fleet.transports = 4;
    fleet.etacs = 1;
    assert_eq!(strength(&fleet), 21 + 14 + 6);
    fleet.order = order::GUARD_STARBASE;
    assert_eq!(strength(&fleet), 41 + 30);
    fleet.speed = 1;
    assert_eq!(strength(&fleet), 41);
}

#[test]
fn roe_thresholds_follow_classic() {
    let cases: &[(u8, u16, u16, bool)] = &[
        (0, 100, 0, false),
        (1, 100, 0, true),
        (1, 100, 1, false),
        (2, 40, 10, true),
        (2, 39, 10, false),
        (3, 30, 10, true),
        (4, 20, 10, true),
        (4, 19, 10, false),
        (5, 15, 10, true),
        (6, 10, 10, true),
        (6, 9, 10, false),
        (7, 10, 15, true),
        (7, 10, 16, false),
        (7, 0, 0, false),
        (8, 10, 20, true),
        (9, 10, 30, true),
        (9, 10, 31, false),
        (10, 0, 1000, true),
        (11, 0, 1000, true),
    ];
    for &(roe, own, enemy, expected) in cases {
        assert_eq!(
            roe_permits(roe, own, enemy),
            expected,
            "ROE {roe}: {own} vs {enemy}"
        );
    }
}

#[test]
fn observer_factors_follow_the_order_at_rest() {
    let factor = |order, speed: u8, at_target: bool, roe: u8| {
        let mut fleet = at_world(1, order);
        fleet.speed = speed;
        fleet.roe = roe;
        if !at_target {
            fleet.target = [1, 1];
        }
        let posture = fleet_posture(&fleet);
        (posture.factor, posture.closing)
    };
    assert_eq!(factor(order::PATROL, 0, true, 0), (10.0, true));
    assert_eq!(factor(order::SCOUT_SECTOR, 0, true, 0), (5.0, true));
    assert_eq!(factor(order::GUARD_BLOCKADE, 0, true, 0), (5.0, true));
    assert_eq!(factor(order::GUARD_STARBASE, 0, true, 9), (1.0, false));
    assert_eq!(factor(order::HOLD, 0, true, 0), (1.0, false));
    assert_eq!(factor(order::HOLD, 0, true, 1), (1.0, true));
    // A patrol still under way scans at factor 1.
    assert_eq!(factor(order::PATROL, 3, false, 0), (1.0, false));
    assert_eq!(base_factor(1), 25.0);
    assert_eq!(base_factor(0), 30.0);
}

#[test]
fn visibility_follows_size_depth_scouts_order_and_speed() {
    let close = |a: f64, b: f64| (a - b).abs() < 1e-9;
    let mut fleet = at_world(1, order::HOLD);
    fleet.cruisers = 1;
    fleet.layer = 0;
    fleet.target_layer = 0;
    assert!(close(fleet_visibility(&fleet), 30f64.sqrt() / 52.0));
    fleet.layer = 1;
    fleet.target_layer = 1;
    assert!(close(fleet_visibility(&fleet), 5.0 * 30f64.sqrt() / 52.0));
    fleet.scouts = 3;
    assert!(close(
        fleet_visibility(&fleet),
        (5.0 * 30f64.sqrt() + 3.0) / 52.0
    ));
    fleet.order = order::BOMBARD;
    assert!(close(
        fleet_visibility(&fleet),
        20.0 * (5.0 * 30f64.sqrt() + 3.0) / 52.0
    ));
    fleet.target = [1, 1];
    fleet.speed = 5;
    assert!(close(
        fleet_visibility(&fleet),
        1.5 * (5.0 * 30f64.sqrt() + 3.0) / 52.0
    ));
}

trait RoeSet {
    fn roe_set(self, roe: u8) -> Self;
}

impl RoeSet for FleetFacts {
    fn roe_set(mut self, roe: u8) -> Self {
        self.roe = roe;
        self
    }
}
