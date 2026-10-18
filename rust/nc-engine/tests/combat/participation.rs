//! Who takes part in a battle (combat decision 6): an empire's fleets in
//! the sector and layer fight together, volunteers are admitted on the
//! assembled odds, a refused trigger is cancelled, starbases only defend,
//! three empires still make two-empire battles, and first contact obeys
//! ROE. Whole maintenance years over many seeds; the joined-force odds are
//! checked against `tools/combat-model/participation_review.py`.

use nc_data::{
    CoreGameData, DiplomacyOverride, EncounterDispositionEvent, EncounterDispositionReason,
    MaintenanceEvents, Order, Real48, recompute_max_speed, set_mission_target,
};
use nc_engine::run_maintenance_turn_with_context_and_seed;

use super::support::{DEEP_SPACE, Ships, diplomacy, empty_board, place_fleet, place_starbase};

const COST_DD: u32 = 5;
const COST_CA: u32 = 15;

fn run(
    game_data: &mut CoreGameData,
    seed: u64,
    overrides: &[DiplomacyOverride],
) -> MaintenanceEvents {
    run_maintenance_turn_with_context_and_seed(game_data, seed, &[], overrides)
        .expect("maintenance should succeed")
}

/// Fleet record `idx` of `owner` on `order` at the centre of `DEEP_SPACE`,
/// rated by its ships.
fn deep_space_fleet(
    game_data: &mut CoreGameData,
    idx: usize,
    owner: u8,
    order: Order,
    ships: Ships,
    roe: u8,
) {
    place_fleet(
        game_data, idx, owner, DEEP_SPACE, order, DEEP_SPACE, ships, roe,
    );
    let fleet = &mut game_data.fleets[idx];
    fleet.x_fraction = Real48::HALF;
    fleet.y_fraction = Real48::HALF;
    fleet.depth = Real48::ZERO;
    set_mission_target(fleet, 0, 0);
    recompute_max_speed(fleet);
}

fn warships(game_data: &CoreGameData, empire: u8) -> (u32, u32) {
    game_data
        .fleets
        .iter()
        .filter(|fleet| fleet.owner.0 == empire && fleet.sector.to_array() == DEEP_SPACE)
        .fold((0, 0), |(dd, ca), fleet| {
            (
                dd + u32::from(fleet.ships.destroyers),
                ca + u32::from(fleet.ships.cruisers),
            )
        })
}

fn battles_at(events: &MaintenanceEvents, empire: u8) -> usize {
    events
        .fleet_battle_events
        .iter()
        .filter(|event| event.reporting_empire == empire && event.coords == DEEP_SPACE)
        .count()
}

/// Binomial check with one count of slack.
fn assert_rate(label: &str, hits: usize, trials: usize, p: f64, sigmas: f64) {
    let expected = p * trials as f64;
    let sd = (trials as f64 * p * (1.0 - p)).sqrt();
    assert!(
        (hits as f64 - expected).abs() <= sigmas * sd + 1.0,
        "{label}: {hits} of {trials}, expected {expected:.1} (p = {p:.4}, sd {sd:.1})"
    );
}

/// The model's joined-force controls (`participation_review.py`, 20,000
/// trials, ROE 10 both sides, no catch, t200 hulls under the engine-t206
/// rules): two fleets of `half` against
/// `opponent`, patrolling the same empty sector at war. Returns the
/// attackers' win rate over the years with a battle and their mean loss in
/// build points.
fn joined_odds(half: Ships, opponent: Ships, seeds: std::ops::Range<u64>) -> (f64, f64, usize) {
    let mut scenario = empty_board(0, 0);
    deep_space_fleet(&mut scenario, 0, 1, Order::PatrolSector, half, 10);
    deep_space_fleet(&mut scenario, 1, 1, Order::PatrolSector, half, 10);
    deep_space_fleet(&mut scenario, 4, 2, Order::PatrolSector, opponent, 10);
    let overrides = diplomacy(&[(1, 2)]);
    let (mut fought, mut wins, mut lost) = (0usize, 0usize, 0u64);
    for seed in seeds {
        let mut game_data = scenario.clone();
        let events = run(&mut game_data, seed, &overrides);
        if battles_at(&events, 1) == 0 {
            continue;
        }
        fought += 1;
        let (dd, ca) = warships(&game_data, 1);
        let (enemy_dd, enemy_ca) = warships(&game_data, 2);
        wins += usize::from(enemy_dd + enemy_ca == 0 && dd + ca > 0);
        let initial_dd = 2 * u32::from(half.dd);
        let initial_ca = 2 * u32::from(half.ca);
        lost += u64::from((initial_dd - dd) * COST_DD + (initial_ca - ca) * COST_CA);
    }
    (
        wins as f64 / fought as f64,
        lost as f64 / fought as f64,
        fought,
    )
}

/// Two 5 CA fleets against 10 CA: the model wins 49.7% joined (93.0 build
/// points lost) against 7.7% one at a time (145.8).
#[test]
fn two_five_cruiser_fleets_against_ten_fight_together() {
    let half = Ships {
        ca: 5,
        ..Ships::default()
    };
    let ten = Ships {
        ca: 10,
        ..Ships::default()
    };
    let (win, lost, fought) = joined_odds(half, ten, 0..600);
    eprintln!(
        "two 5 CA vs 10 CA: win {win:.3}, lost {lost:.1} over {fought} battles (model joined 0.497, 93.0; sequential 0.077, 145.8)"
    );
    assert!(fought >= 500, "{fought} battles");
    assert_rate(
        "two 5 CA vs 10 CA",
        (win * fought as f64) as usize,
        fought,
        0.4975,
        4.0,
    );
    assert!(win > 0.3, "far from the one-at-a-time 8%");
    assert!((lost - 93.0).abs() < 15.0, "lost {lost:.1}");
}

/// Two 6 CA fleets against 10 CA: 76.6% joined (64.7) against 38.4% one at
/// a time (150.9).
#[test]
fn two_six_cruiser_fleets_against_ten_fight_together() {
    let half = Ships {
        ca: 6,
        ..Ships::default()
    };
    let ten = Ships {
        ca: 10,
        ..Ships::default()
    };
    let (win, lost, fought) = joined_odds(half, ten, 0..600);
    eprintln!(
        "two 6 CA vs 10 CA: win {win:.3}, lost {lost:.1} over {fought} battles (model joined 0.766, 64.7; sequential 0.384, 150.9)"
    );
    assert!(fought >= 500, "{fought} battles");
    assert_rate(
        "two 6 CA vs 10 CA",
        (win * fought as f64) as usize,
        fought,
        0.7661,
        4.0,
    );
    assert!(win > 0.6, "far from the one-at-a-time 38%");
    assert!((lost - 64.7).abs() < 15.0, "lost {lost:.1}");
}

/// Two fleets of 5 DD and 5 CA against 12 CA: 76.0% joined (73.4) against
/// 37.7% one at a time (168.3) under the t206 rules, where the cruisers
/// cover the destroyers and the destroyers finish cripples at half price
/// (60.9% / 106.2 before them).
#[test]
fn two_mixed_fleets_against_twelve_cruisers_fight_together() {
    let half = Ships {
        dd: 5,
        ca: 5,
        ..Ships::default()
    };
    let twelve = Ships {
        ca: 12,
        ..Ships::default()
    };
    let (win, lost, fought) = joined_odds(half, twelve, 0..600);
    eprintln!(
        "two (5 DD + 5 CA) vs 12 CA: win {win:.3}, lost {lost:.1} over {fought} battles (model joined 0.760, 73.4; sequential 0.377, 168.3)"
    );
    assert!(fought >= 500, "{fought} battles");
    assert_rate(
        "two mixed vs 12 CA",
        (win * fought as f64) as usize,
        fought,
        0.7601,
        4.0,
    );
    assert!(win > 0.6, "far from the one-at-a-time 38%");
    assert!((lost - 73.4).abs() < 15.0, "lost {lost:.1}");
}

/// A cautious fleet no longer attacks a stronger enemy because it
/// identified it first: a lone cruiser at ROE 2 meeting three cruisers
/// declines every time and reports it, and nobody fights.
#[test]
fn first_contact_obeys_roe() {
    let mut scenario = empty_board(0, 0);
    deep_space_fleet(
        &mut scenario,
        0,
        1,
        Order::PatrolSector,
        Ships {
            ca: 1,
            ..Ships::default()
        },
        2,
    );
    deep_space_fleet(
        &mut scenario,
        4,
        2,
        Order::HoldPosition,
        Ships {
            ca: 3,
            ..Ships::default()
        },
        0,
    );
    let overrides = diplomacy(&[(1, 2)]);
    let mut declined = 0;
    for seed in 0..200u64 {
        let mut game_data = scenario.clone();
        let events = run(&mut game_data, seed, &overrides);
        assert!(events.fleet_battle_events.is_empty(), "seed {seed}");
        declined += events
            .encounter_disposition_events
            .iter()
            .filter(|event| {
                matches!(
                    event,
                    EncounterDispositionEvent::NoEngagement {
                        owner_empire: 1,
                        reason: EncounterDispositionReason::RoeDeclined,
                        ..
                    }
                )
            })
            .count();
    }
    assert!(declined > 100, "{declined} declined identifications");
}

/// A refused trigger is cancelled. Empire 1's holding pair of cruisers at
/// ROE 6 (fleet 0) refuses ten cruisers even with its ROE 10 companion
/// (fleet 1, a scout group of two cruisers) counted: 120 against 300. Its
/// sighting starts no battle, however willing the companion; the companion
/// attacks only when its own sighting and intercept bring it to the enemy,
/// and then alone, since fleet 0 still refuses.
#[test]
fn a_refused_trigger_is_cancelled_and_another_fleet_needs_its_own_contact() {
    let mut scenario = empty_board(0, 0);
    deep_space_fleet(
        &mut scenario,
        0,
        1,
        Order::HoldPosition,
        Ships {
            ca: 2,
            ..Ships::default()
        },
        6,
    );
    deep_space_fleet(
        &mut scenario,
        1,
        1,
        Order::ScoutSector,
        Ships {
            ca: 2,
            sc: 1,
            ..Ships::default()
        },
        10,
    );
    deep_space_fleet(
        &mut scenario,
        4,
        2,
        Order::HoldPosition,
        Ships {
            ca: 10,
            ..Ships::default()
        },
        0,
    );
    let overrides = diplomacy(&[(1, 2)]);
    let (mut refused, mut attacked) = (0, 0);
    for seed in 0..300u64 {
        let mut game_data = scenario.clone();
        let events = run(&mut game_data, seed, &overrides);
        let refusals: Vec<u8> = events
            .encounter_disposition_events
            .iter()
            .filter_map(|event| match event {
                EncounterDispositionEvent::NoEngagement {
                    fleet_idx: 0,
                    reason: EncounterDispositionReason::RoeDeclined,
                    stardate_week,
                    ..
                } => *stardate_week,
                _ => None,
            })
            .collect();
        refused += usize::from(!refusals.is_empty());
        for event in &events.fleet_battle_events {
            if event.reporting_empire != 1 {
                continue;
            }
            attacked += 1;
            assert_eq!(
                event.reporting_fleet_number,
                Some(game_data.fleets[1].fleet_number.0 as u8),
                "seed {seed}"
            );
            assert_eq!(
                event.friendly_initial.cruisers, 2,
                "seed {seed}: the companion attacks alone"
            );
            assert_eq!(event.friendly_initial.scouts, 1, "seed {seed}");
            assert!(
                !refusals.contains(&event.stardate_week.expect("dated")),
                "seed {seed}: a refusal started a battle"
            );
        }
    }
    assert!(refused > 20, "{refused} years with a refusal");
    assert!(attacked > 20, "{attacked} battles by the companion");
}

/// Starbases never volunteer: empire 1's base in deep space and its holding
/// cruiser pair at ROE 10 meet an enemy cruiser on ROE 0. The pair attacks;
/// the base is never on the attacking side, though it shares the sector.
#[test]
fn starbases_never_volunteer_to_attack() {
    let mut scenario = empty_board(0, 0);
    place_starbase(&mut scenario, 1, DEEP_SPACE);
    scenario.bases[0].depth = Real48::ZERO;
    scenario.bases[0].target_depth = 0;
    deep_space_fleet(
        &mut scenario,
        0,
        1,
        Order::PatrolSector,
        Ships {
            ca: 2,
            ..Ships::default()
        },
        10,
    );
    deep_space_fleet(
        &mut scenario,
        4,
        2,
        Order::HoldPosition,
        Ships {
            ca: 1,
            ..Ships::default()
        },
        0,
    );
    let overrides = diplomacy(&[(1, 2)]);
    let mut battles = 0;
    for seed in 0..100u64 {
        let mut game_data = scenario.clone();
        let events = run(&mut game_data, seed, &overrides);
        for event in &events.fleet_battle_events {
            if event.reporting_empire == 1 {
                battles += 1;
                assert_eq!(event.friendly_initial_starbases, 0, "seed {seed}");
                assert_eq!(event.friendly_initial.cruisers, 2, "seed {seed}");
            }
        }
    }
    assert!(battles > 50, "{battles} battles");
}

/// Three empires at war in one sector fight two-empire battles: every
/// report names one enemy empire, and a side's ships are its empire's only.
#[test]
fn three_empires_in_a_sector_fight_two_at_a_time() {
    let mut scenario = empty_board(0, 0);
    for (idx, owner) in [(0, 1), (4, 2), (8, 3)] {
        deep_space_fleet(
            &mut scenario,
            idx,
            owner,
            Order::PatrolSector,
            Ships {
                ca: 2,
                ..Ships::default()
            },
            10,
        );
    }
    let overrides = diplomacy(&[(1, 2), (1, 3), (2, 3)]);
    let mut battles = 0;
    for seed in 0..100u64 {
        let mut game_data = scenario.clone();
        let events = run(&mut game_data, seed, &overrides);
        for event in &events.fleet_battle_events {
            battles += 1;
            assert_eq!(event.enemy_empires.len(), 1, "seed {seed}: {event:?}");
            assert_ne!(event.enemy_empires[0], event.reporting_empire);
            assert!(
                event.friendly_initial.cruisers <= 2,
                "seed {seed}: {event:?}"
            );
            assert!(event.enemy_initial.cruisers <= 2, "seed {seed}: {event:?}");
        }
    }
    assert!(battles > 50, "{battles} reports");
}
