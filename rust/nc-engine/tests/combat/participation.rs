//! Who takes part in a battle (ADR bd8trz18, classic's pairing): one fleet
//! against one fleet, or a base with its guard, a refused trigger is
//! cancelled, starbases only defend, three empires still make two-empire
//! battles, and first contact obeys ROE. Whole maintenance years over many
//! seeds.

use nc_data::{
    CoreGameData, DiplomacyOverride, EncounterDispositionEvent, EncounterDispositionReason,
    MaintenanceEvents, Order, Real48, recompute_max_speed, set_mission_target,
};
use nc_engine::run_maintenance_turn_with_context_and_seed;

use super::support::{DEEP_SPACE, Ships, diplomacy, empty_board, place_fleet, place_starbase};

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

fn battles_at(events: &MaintenanceEvents, empire: u8) -> usize {
    events
        .fleet_battle_events
        .iter()
        .filter(|event| event.reporting_empire == empire && event.coords == DEEP_SPACE)
        .count()
}

/// Classic's pairing (ADR bd8trz18): two 5 CA fleets of empire 1 patrolling
/// the sector of empire 2's 10 CA, all at ROE 10 and at war, never fight
/// together. Every battle sets one fleet of each side against the other.
#[test]
fn an_empires_fleets_in_one_sector_fight_one_at_a_time() {
    let five = Ships {
        ca: 5,
        ..Ships::default()
    };
    let ten = Ships {
        ca: 10,
        ..Ships::default()
    };
    let mut scenario = empty_board(0, 0);
    deep_space_fleet(&mut scenario, 0, 1, Order::PatrolSector, five, 10);
    deep_space_fleet(&mut scenario, 1, 1, Order::PatrolSector, five, 10);
    deep_space_fleet(&mut scenario, 4, 2, Order::PatrolSector, ten, 10);
    let overrides = diplomacy(&[(1, 2)]);
    let mut fought = 0;
    for seed in 0..100u64 {
        let mut game_data = scenario.clone();
        let events = run(&mut game_data, seed, &overrides);
        for battle in &events.fleet_battle_events {
            let (ours, theirs) = if battle.reporting_empire == 1 {
                (&battle.friendly_initial, &battle.enemy_initial)
            } else {
                (&battle.enemy_initial, &battle.friendly_initial)
            };
            assert!(ours.cruisers <= 5, "seed {seed}: {ours:?}");
            assert!(theirs.cruisers <= 10, "seed {seed}: {theirs:?}");
        }
        fought += battles_at(&events, 1);
    }
    assert!(fought >= 50, "{fought} battles");
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
