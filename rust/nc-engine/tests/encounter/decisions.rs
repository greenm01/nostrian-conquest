//! The decision table (`1b05:1277`, `1b05:1d11`), strength, ROE, and the
//! observer and visibility factors, checked case by case. Strength is NC's
//! one yardstick (combat decision 2): attack strength, not classic's
//! 21/7/2 weights, and the decision weighs the sides as they would fight
//! (combat decision 6): here one fleet a side unless a test assembles more.

use nc_data::{CrippledHulls, GameRng};
use nc_engine::maint::encounter::rules::{
    BASE_STRENGTH, FleetFacts, Situation, Verdict, admitted, base_factor, decide_vs_base,
    decide_vs_fleet, fleet_posture, fleet_visibility, order, roe_permits, strength,
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

/// The situation of observer `o` meeting target `t` with nobody else on
/// either side.
fn situation(
    o: &FleetFacts,
    t: &FleetFacts,
    world_owner: Option<u8>,
    declared_enemy: bool,
) -> Situation {
    Situation {
        world_owner,
        declared_enemy,
        target_awareness: 0,
        own_strength: strength(o),
        enemy_strength: strength(t),
    }
}

/// Decide, one fleet a side, with a target that cannot get away (it is
/// stopped, so the intercept always succeeds).
fn verdict(o: FleetFacts, t: FleetFacts, world_owner: Option<u8>, declared_enemy: bool) -> Verdict {
    let situation = situation(&o, &t, world_owner, declared_enemy);
    decide_vs_fleet(&mut GameRng::from_seed(1), &o, &t, situation)
}

/// Two cruisers a side: the defender's ROE 6 accepts even odds. (It was
/// ROE 1 under classic's first-identification bypass, gone with combat
/// decision 6: first contact obeys ROE.)
#[test]
fn neutral_fleet_in_an_owned_system_is_attacked_by_any_willing_fleet() {
    let defender = at_world(2, order::HOLD).roe_set(6);
    for visitor_order in [
        order::HOLD,
        order::VIEW,
        order::COLONIZE,
        order::GUARD_BLOCKADE,
        order::BOMBARD,
    ] {
        let visitor = at_world(1, visitor_order);
        assert_eq!(
            verdict(defender, visitor, Some(2), false),
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
            verdict(observer, visitor, world_owner, false),
            Verdict::Ignored
        );
    }
    // Deep space over the observer's own world is not its system.
    let mut deep = observer;
    deep.layer = 0;
    let mut deep_visitor = visitor;
    deep_visitor.layer = 0;
    assert_eq!(verdict(deep, deep_visitor, None, false), Verdict::Ignored);
}

#[test]
fn only_the_observers_own_declaration_makes_an_enemy() {
    let observer = at_world(2, order::HOLD).roe_set(6);
    let mut visitor = at_world(1, order::HOLD);
    visitor.layer = 0;
    let mut deep_observer = observer;
    deep_observer.layer = 0;
    assert_eq!(
        verdict(deep_observer, visitor, None, true),
        Verdict::Engaged
    );
    // The visitor's empire declaring the observer's an enemy is not enough.
    assert_eq!(
        verdict(deep_observer, visitor, None, false),
        Verdict::Ignored
    );
}

/// Combat decision 6: first contact obeys ROE. Classic let any ROE above 0
/// attack the first time it identified an enemy; here a cautious fleet
/// declines a stronger enemy whenever it meets it, and ROE 0 never attacks.
#[test]
fn first_contact_obeys_roe() {
    let mut observer = at_world(2, order::HOLD).roe_set(2);
    observer.layer = 0;
    let mut enemy = at_world(1, order::HOLD);
    enemy.layer = 0;
    enemy.battleships = 10;
    assert_eq!(verdict(observer, enemy, None, true), Verdict::Avoided);
    enemy.battleships = 0;
    enemy.cruisers = 0;
    assert_eq!(verdict(observer, enemy, None, true), Verdict::Engaged);
    observer.roe = 0;
    assert_eq!(verdict(observer, enemy, None, true), Verdict::Avoided);
}

/// The decision weighs the sides as they would fight: two cruiser pairs
/// (66 each) at ROE 6 would each decline four cruisers (132) alone, and
/// take them on assembled (132 against 132). A fleet without warships
/// never volunteers, whatever its ROE.
#[test]
fn the_decision_weighs_the_assembled_sides() {
    let mut observer = at_world(2, order::HOLD).roe_set(6);
    observer.layer = 0;
    let mut enemy = at_world(1, order::HOLD);
    enemy.layer = 0;
    enemy.cruisers = 4;
    assert_eq!(verdict(observer, enemy, None, true), Verdict::Avoided);
    let mut assembled = situation(&observer, &enemy, None, true);
    assembled.own_strength = 132;
    assert_eq!(
        decide_vs_fleet(&mut GameRng::from_seed(1), &observer, &enemy, assembled),
        Verdict::Engaged
    );
    let mut unarmed = observer;
    unarmed.cruisers = 0;
    unarmed.roe = 10;
    assert_eq!(
        decide_vs_fleet(&mut GameRng::from_seed(1), &unarmed, &enemy, assembled),
        Verdict::Avoided
    );
}

/// Admission (combat decision 6): each volunteer tests its own ROE on the
/// volunteers' assembled strength, recomputed until none refuses. The
/// model's cases (`participation_review.py`): two 180s at ROE 6 take on
/// 300 together; with one of them at ROE 0 the other is left alone and
/// refuses; two 180s at ROE 6 refuse 390; ROE 8 joins ROE 6.
#[test]
fn volunteers_are_admitted_on_their_assembled_strength() {
    assert_eq!(admitted(&[(6, 180), (6, 180)], 300), [true, true]);
    assert_eq!(admitted(&[(6, 180), (0, 180)], 300), [false, false]);
    assert_eq!(admitted(&[(6, 300), (0, 30)], 300), [true, false]);
    assert_eq!(admitted(&[(6, 180), (6, 180)], 390), [false, false]);
    assert_eq!(admitted(&[(6, 180), (8, 180)], 300), [true, true]);
    // A refusal is for good: once the ROE 6 fleet drops (280 against 300),
    // the ROE 8 fleet stays alone (180 against 300, within 2:1), and a
    // fleet without warships is never a volunteer.
    assert_eq!(
        admitted(&[(6, 100), (8, 180), (10, 0)], 300),
        [false, true, false]
    );
    // Without it the ROE 7 fleet would have dropped too (300 against 180
    // is beyond 3:2), though it accepted the odds with the ROE 6 fleet in.
    assert_eq!(admitted(&[(6, 100), (7, 180)], 300), [false, false]);
}

#[test]
fn blockade_attacks_assaults_and_colonizers_of_an_unowned_world() {
    let blockade = at_world(3, order::GUARD_BLOCKADE).roe_set(6);
    for assault in [order::BOMBARD, order::INVADE, order::BLITZ] {
        assert_eq!(
            verdict(blockade, at_world(1, assault), Some(2), false),
            Verdict::Engaged
        );
    }
    let colonizer = at_world(1, order::COLONIZE);
    assert_eq!(
        verdict(blockade, colonizer, Some(0), false),
        Verdict::Engaged
    );
    // Colonizing an owned world, and other neutral orders, are ignored.
    assert_eq!(
        verdict(blockade, colonizer, Some(2), false),
        Verdict::Ignored
    );
    assert_eq!(
        verdict(blockade, at_world(1, order::HOLD), Some(2), false),
        Verdict::Ignored
    );
    // A colonizer in a worldless sector has no unowned world to race for.
    assert_eq!(verdict(blockade, colonizer, None, false), Verdict::Ignored);
}

/// Two cruisers a side: ROE 6 takes the race, ROE 0 avoids it. (The racer
/// was ROE 1 under classic's first-identification bypass, gone with combat
/// decision 6.)
#[test]
fn colonizers_race_for_an_unowned_world() {
    let first = at_world(1, order::COLONIZE).roe_set(6);
    let second = at_world(2, order::COLONIZE);
    assert_eq!(verdict(first, second, Some(0), false), Verdict::Engaged);
    assert_eq!(
        verdict(first.roe_set(0), second, Some(0), false),
        Verdict::Avoided
    );
}

#[test]
fn assault_fleet_blocked_by_a_guard_fights_or_aborts() {
    let guard = at_world(2, order::GUARD_BLOCKADE);
    let bomber = at_world(1, order::BOMBARD);
    assert_eq!(
        verdict(bomber.roe_set(6), guard, Some(2), false),
        Verdict::Engaged
    );
    assert_eq!(verdict(bomber, guard, Some(2), false), Verdict::Aborted);
    // A guard ordered elsewhere is not guarding this world.
    let mut leaving = guard;
    leaving.target = [1, 1];
    assert_eq!(verdict(bomber, leaving, Some(2), false), Verdict::Ignored);
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
    let enemy = situation(&observer, &runner, None, true);
    let mut rng = GameRng::from_seed(9);
    observer.max_speed = 7;
    assert!(
        (0..200)
            .all(|_| { decide_vs_fleet(&mut rng, &observer, &runner, enemy) == Verdict::Engaged })
    );
    observer.max_speed = 1;
    let escapes = (0..200)
        .filter(|_| decide_vs_fleet(&mut rng, &observer, &runner, enemy) == Verdict::Escaped)
        .count();
    assert!(escapes > 0);
}

/// `decide_vs_base` takes the sides as they would fight: the observer's
/// volunteers and the base with its guard and its owner's other fleets.
#[test]
fn starbases_are_attacked_only_as_an_assault_target_or_a_declared_enemy() {
    let mut bomber = at_world(1, order::BOMBARD).roe_set(6);
    bomber.battleships = 2;
    let own = strength(&bomber);
    assert_eq!(
        decide_vs_base(&bomber, own, BASE_STRENGTH, false),
        Verdict::Engaged
    );
    // The base protects the target world: its guard counts too.
    let guarded = BASE_STRENGTH + strength(&at_world(2, order::GUARD_STARBASE)) + 100;
    assert_eq!(
        decide_vs_base(&bomber, own, guarded, false),
        Verdict::Aborted
    );
    let hold = at_world(1, order::HOLD).roe_set(10);
    let own = strength(&hold);
    assert_eq!(
        decide_vs_base(&hold, own, BASE_STRENGTH, false),
        Verdict::Ignored
    );
    assert_eq!(
        decide_vs_base(&hold, own, BASE_STRENGTH, true),
        Verdict::Engaged
    );
    // Against a declared enemy classic weighed the base alone (30, its
    // guard ignored); combat decision 6 weighs the base's side as it would
    // fight, so a lone base (100) is taken on and a guarded one declined.
    let cautious = at_world(1, order::HOLD).roe_set(6);
    let mut strong = cautious;
    strong.battleships = 2;
    assert_eq!(
        decide_vs_base(&cautious, strength(&cautious), BASE_STRENGTH, true),
        Verdict::Avoided
    );
    assert_eq!(
        decide_vs_base(&strong, strength(&strong), BASE_STRENGTH, true),
        Verdict::Engaged
    );
    assert_eq!(
        decide_vs_base(&strong, strength(&strong), 1000, true),
        Verdict::Avoided
    );
    // A fleet without warships never attacks a base, whatever its ROE.
    let mut unarmed = hold;
    unarmed.cruisers = 0;
    assert_eq!(
        decide_vs_base(&unarmed, 0, BASE_STRENGTH, true),
        Verdict::Avoided
    );
}

/// Decision 2's yardstick: each warship at its attack strength, nothing for
/// a guard at rest, and a starbase at 100.
#[test]
fn strength_is_attack_strength() {
    let mut fleet = at_world(1, order::HOLD);
    fleet.battleships = 1;
    fleet.cruisers = 2;
    fleet.destroyers = 3;
    fleet.scouts = 5;
    fleet.transports = 4;
    fleet.etacs = 1;
    assert_eq!(strength(&fleet), 90 + 66 + 36);
    fleet.order = order::GUARD_STARBASE;
    assert_eq!(strength(&fleet), 192);
    assert_eq!(BASE_STRENGTH, 100);
}

/// Decision 3 meets decision 2: a crippled warship counts at its crippled
/// attack strength (half), in the decision to fight as in the battle.
#[test]
fn a_crippled_warship_counts_at_half_its_attack_strength() {
    let mut fleet = at_world(1, order::HOLD);
    fleet.battleships = 2;
    fleet.cruisers = 2;
    fleet.destroyers = 3;
    assert_eq!(strength(&fleet), 180 + 66 + 36);
    let hurt = |turns_2: u32| CrippledHulls {
        turns: [0, turns_2],
        fresh: 0,
    };
    fleet.crippled.battleships = hurt(1);
    fleet.crippled.cruisers = hurt(2);
    fleet.crippled.destroyers = CrippledHulls {
        turns: [1, 0],
        fresh: 1,
    };
    // Battleships 90 + 45, cruisers 2 x 16, destroyers 12 + 2 x 6.
    assert_eq!(strength(&fleet), 135 + 32 + 24);
}

/// Three cruisers (99) take on three cruisers at ROE 6; with two of the
/// observer's crippled (65) they decline.
#[test]
fn a_crippled_fleet_declines_the_fight_its_whole_strength_would_take() {
    let mut observer = at_world(1, order::HOLD).roe_set(6);
    observer.layer = 0;
    observer.cruisers = 3;
    let mut target = at_world(2, order::HOLD);
    target.layer = 0;
    target.cruisers = 3;
    let rechecks = |observer: FleetFacts| verdict(observer, target, None, true);
    assert_eq!(rechecks(observer), Verdict::Engaged);
    observer.crippled.cruisers.turns[0] = 2;
    assert_eq!(rechecks(observer), Verdict::Avoided);
}

/// Where the yardstick changes who agrees to fight: three destroyers meet
/// a cruiser at ROE 6 (36 against 33; classic's weights gave 6 against 7),
/// and four cruisers take on a lone starbase (132 against 100; classic gave
/// 28 against 30).
#[test]
fn the_yardstick_lets_three_destroyers_fight_a_cruiser_and_four_cruisers_a_base() {
    let mut destroyers = at_world(1, order::HOLD).roe_set(6);
    destroyers.layer = 0;
    destroyers.cruisers = 0;
    destroyers.destroyers = 3;
    let mut cruiser = at_world(2, order::HOLD);
    cruiser.layer = 0;
    cruiser.cruisers = 1;
    assert_eq!(verdict(destroyers, cruiser, None, true), Verdict::Engaged);
    let mut cruisers = at_world(1, order::BOMBARD).roe_set(6);
    cruisers.cruisers = 4;
    let own = strength(&cruisers);
    assert_eq!(
        decide_vs_base(&cruisers, own, BASE_STRENGTH, false),
        Verdict::Engaged
    );
    assert_eq!(
        decide_vs_base(&cruisers, own, BASE_STRENGTH, true),
        Verdict::Engaged
    );
}

#[test]
fn roe_thresholds_follow_classic() {
    let cases: &[(u8, u32, u32, bool)] = &[
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
