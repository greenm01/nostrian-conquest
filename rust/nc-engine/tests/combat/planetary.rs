//! Planetary combat after combat decisions 4 and 5: the landing battle on
//! the shared table, the battery shield over the stardock, and failed
//! assaults that hold orbit. The exact arithmetic is pinned by the inline
//! tests in `src/maint/combat/assault_tests.rs`; these run the whole year.

use nc_data::{
    Coords, CoreGameData, MaintenanceEvents, Mission, MissionOutcome, Order, Real48,
    set_fleet_at_rest, troop_transports,
};
use nc_engine::run_maintenance_turn_with_context_and_seed;

use super::support::{
    SYSTEM, SYSTEM_PLANET_IDX, Ships, diplomacy, empty_board, mark_arrived, place_fleet,
    place_starbase,
};

/// Empire 1 fleet record 0 sits in orbit of `SYSTEM` (owned by empire 2) on
/// an assault order that executes in tick 1, ROE 10.
fn assault_on_system(order: Order, ships: Ships, armies: u8, batteries: u8) -> CoreGameData {
    let mut game_data = empty_board(armies, batteries);
    place_fleet(&mut game_data, 0, 1, SYSTEM, order, SYSTEM, ships, 10);
    mark_arrived(&mut game_data, 0);
    game_data
}

/// A year among neutral empires on campaign seed `seed`.
fn year(game_data: &CoreGameData, seed: u64) -> (CoreGameData, MaintenanceEvents) {
    let mut game_data = game_data.clone();
    let events =
        run_maintenance_turn_with_context_and_seed(&mut game_data, seed, &[], &diplomacy(&[]))
            .expect("maintenance should succeed");
    (game_data, events)
}

/// `loaded` armies in as many transports and one destroyer, the warship an
/// invasion needs. Its softening fire never reaches 10 hits, so it destroys
/// no army, and a world without batteries never fires on it.
fn landing_force(loaded: u16) -> Ships {
    Ships {
        dd: 1,
        tt: loaded,
        armies: loaded,
        ..Ships::default()
    }
}

/// Fleet 0 holds in orbit of the target world at rest: Hold, not a blockade
/// or a retreat.
fn holds_in_orbit(game_data: &CoreGameData) -> bool {
    let fleet = &game_data.fleets[0];
    fleet.order == Order::HoldPosition
        && fleet.speed == 0
        && fleet.depth == Real48::ONE
        && fleet.sector.to_array() == SYSTEM
}

#[test]
fn a_lone_army_cannot_scratch_a_defending_garrison() {
    // The defenders fire first. Five armies deal at least five quarters in
    // round 1, which kills the single attacker before it fires, whatever
    // the dice say. Under the old simultaneous round the attacker's volley
    // killed defenders on many rolls.
    for order in [Order::InvadeWorld, Order::BlitzWorld] {
        let game_data = assault_on_system(order, landing_force(1), 5, 0);
        for seed in 0..40 {
            let (after, events) = year(&game_data, seed);
            let assault = &events.assault_report_events[0];
            assert_eq!(assault.outcome, MissionOutcome::Failed, "{order:?} {seed}");
            assert_eq!(assault.defender_army_losses, 0, "{order:?} {seed}");
            assert_eq!(assault.attacker_army_losses, 1, "{order:?} {seed}");
            assert_eq!(after.planets[SYSTEM_PLANET_IDX].armies, 5);
            assert_eq!(after.planets[SYSTEM_PLANET_IDX].owner.0, 2);
        }
    }
}

/// (attackers, defenders, the model's capture rate): combat decision 4's
/// exact table, `tools/combat-model/ground_calibration.py`.
const MODEL_CAPTURE: [(u16, u8, f64); 3] =
    [(10, 10, 0.19938), (15, 10, 0.56107), (20, 10, 0.91761)];

#[test]
fn whole_year_captures_follow_the_model() {
    // The escort softens nothing, and there is no battery and no landing
    // fire: the landing is the whole fight. Each trial is a year on
    // its own campaign seed. A rate is within 4.5 binomial standard
    // deviations of the model. The old rule took 10 against 10 about 8% of
    // the time and 15 or 20 against 10 every time.
    const TRIALS: u64 = 400;
    let mut next_seed = 0..;
    for order in [Order::InvadeWorld, Order::BlitzWorld] {
        for (attackers, defenders, model) in MODEL_CAPTURE {
            let game_data = assault_on_system(order, landing_force(attackers), defenders, 0);
            let first_seed = TRIALS * next_seed.next().unwrap_or(0);
            let mut captures = 0;
            for seed in first_seed..first_seed + TRIALS {
                let (after, events) = year(&game_data, seed);
                let assault = &events.assault_report_events[0];
                let planet = &after.planets[SYSTEM_PLANET_IDX];
                if planet.owner.0 == 1 {
                    captures += 1;
                    // The survivors are the garrison, whole armies only.
                    let survivors = u32::from(attackers) - assault.attacker_army_losses;
                    assert_eq!(u32::from(planet.armies), survivors);
                    assert_eq!(assault.outcome, MissionOutcome::Succeeded);
                } else {
                    assert_eq!(
                        u32::from(planet.armies),
                        u32::from(defenders) - u32::from(assault.defender_army_losses)
                    );
                    assert_eq!(assault.attacker_army_losses, u32::from(attackers));
                    assert_eq!(assault.outcome, MissionOutcome::Failed);
                }
            }
            let n = TRIALS as f64;
            let sd = (n * model * (1.0 - model)).sqrt();
            eprintln!(
                "{order:?} {attackers} v {defenders}: {captures} of {TRIALS}, model {:.1}",
                n * model
            );
            assert!(
                (f64::from(captures) - n * model).abs() <= 4.5 * sd,
                "{order:?} {attackers} v {defenders}: {captures} of {TRIALS}, model {:.1}",
                n * model
            );
        }
    }
}

// Combat decision 5.

#[test]
fn a_failed_landing_leaves_the_fleet_on_hold_in_orbit() {
    // Two armies land on twenty. Softening takes at most ten of them, so
    // the landing fails; the escort, never fired on by a world without
    // batteries, and the emptied transports stay.
    let ships = Ships {
        ca: 2,
        tt: 2,
        armies: 2,
        ..Ships::default()
    };
    let game_data = assault_on_system(Order::InvadeWorld, ships, 20, 0);
    for seed in 0..20 {
        let (after, events) = year(&game_data, seed);

        let assault = &events.assault_report_events[0];
        assert_eq!(assault.kind, Mission::InvadeWorld);
        assert_eq!(assault.outcome, MissionOutcome::Failed, "seed {seed}");
        assert!(events.ownership_change_events.is_empty(), "seed {seed}");
        assert!(
            events.mission_events.iter().any(|event| {
                event.fleet_idx == 0
                    && event.kind == Mission::InvadeWorld
                    && event.outcome == MissionOutcome::Failed
            }),
            "seed {seed}"
        );
        assert!(holds_in_orbit(&after), "seed {seed}");
        let fleet = &after.fleets[0];
        assert_eq!(fleet.ships.cruisers, 2, "seed {seed}");
        assert_eq!(troop_transports(fleet), 2, "seed {seed}");
        assert_eq!(fleet.ships.loaded_transports, 0, "seed {seed}");
        assert_eq!(after.planets[SYSTEM_PLANET_IDX].owner.0, 2);
    }
}

#[test]
fn an_invasion_the_batteries_block_keeps_its_order_and_its_armies() {
    // One battleship cannot clear eight two-step batteries in a year, and
    // their full-strength fire sinks it. A thousand scouts take the rest of
    // the return fire, so the loaded transports survive, armies aboard, and
    // the fleet keeps its Invade order to suppress again next year (t202).
    let ships = Ships {
        bb: 1,
        sc: 1000,
        tt: 10,
        armies: 10,
        ..Ships::default()
    };
    let game_data = assault_on_system(Order::InvadeWorld, ships, 10, 8);
    for seed in 0..20 {
        let (after, events) = year(&game_data, seed);

        let assault = &events.assault_report_events[0];
        assert_eq!(assault.outcome, MissionOutcome::Aborted, "seed {seed}");
        assert_eq!(assault.attacker_army_losses, 0, "seed {seed}");
        assert!(events.ownership_change_events.is_empty(), "seed {seed}");
        let fleet = &after.fleets[0];
        assert_eq!(fleet.order, Order::InvadeWorld, "seed {seed}");
        assert_eq!(fleet.sector.to_array(), SYSTEM, "seed {seed}");
        assert_eq!(fleet.ships.battleships, 0, "seed {seed}");
        assert_eq!(fleet.ships.loaded_transports, 10, "seed {seed}");
        let planet = &after.planets[SYSTEM_PLANET_IDX];
        assert_eq!(planet.owner.0, 2);
        assert!(planet.ground_batteries >= 2, "seed {seed}");
        assert_eq!(planet.armies, 10, "no softening before a landing");
    }
}

#[test]
fn a_fleet_beaten_in_orbit_takes_no_battery_volley_on_its_way_out() {
    // The base and the destroyer beside it beat the bomber at the orbital
    // gate and it retreats (see `a_beaten_attacker_retreats_repelled_by_the_
    // orbital_defence`). Ten batteries on the world change nothing about
    // that: the gate is the orbit's fight, and the surviving fleet leaves
    // exactly as it does from a world with none.
    let bare = |batteries| {
        let mut game_data = assault_on_system(
            Order::BombardWorld,
            Ships {
                dd: 1,
                sc: 1200,
                ..Ships::default()
            },
            0,
            batteries,
        );
        place_starbase(&mut game_data, 2, SYSTEM);
        place_fleet(
            &mut game_data,
            4,
            2,
            SYSTEM,
            Order::HoldPosition,
            SYSTEM,
            Ships {
                dd: 1,
                ..Ships::default()
            },
            0,
        );
        set_fleet_at_rest(&mut game_data.fleets[4]);
        // The bomber's home is far enough that it is still under way at
        // year end.
        game_data.planets[14].sector = Coords::from_array([3, 13]);
        game_data
    };
    let (undefended, fortified) = (bare(0), bare(10));
    for seed in 0..4 {
        let (without, _) = year(&undefended, seed);
        let (with, events) = year(&fortified, seed);

        assert!(events.bombard_events.is_empty(), "seed {seed}");
        assert!(without.fleets[0].ships.scouts > 0, "seed {seed}");
        assert_eq!(
            with.fleets[0].ships, without.fleets[0].ships,
            "seed {seed}: the batteries fired on the retreating fleet"
        );
        assert_eq!(with.fleets[0].order, Order::SeekHome, "seed {seed}");
        assert_eq!(with.planets[SYSTEM_PLANET_IDX].ground_batteries, 10);
    }
}

#[test]
fn a_battery_shields_the_docked_ships_a_lone_destroyer_cannot_reach() {
    // A destroyer's bombardment never exceeds 6 hits, short of the 20 that
    // destroy the battery, so the battery stands through all three rounds
    // and the three docked destroyers (DS 5, one lost per 5 hits under the
    // old order) are untouched on every roll.
    let mut game_data = assault_on_system(
        Order::BombardWorld,
        Ships {
            dd: 1,
            ..Ships::default()
        },
        0,
        1,
    );
    game_data.planets[SYSTEM_PLANET_IDX].stardock.destroyers = 3;
    for seed in 0..40 {
        let (after, events) = year(&game_data, seed);

        let bombard = &events.bombard_events[0];
        assert_eq!(bombard.defender_battery_losses, 0, "seed {seed}");
        assert_eq!(bombard.stardock_items_destroyed, 0, "seed {seed}");
        assert_eq!(
            after.planets[SYSTEM_PLANET_IDX].stardock.destroyers, 3,
            "seed {seed}"
        );
    }
}

#[test]
fn the_dock_is_exposed_once_the_batteries_are_gone() {
    // Four battleships (AS 540) clear one battery in round 1 on any roll
    // and its excess hits expire; the exchange of round 2 begins with no
    // battery standing and destroys the docked battleship (DS 100).
    let mut game_data = assault_on_system(
        Order::BombardWorld,
        Ships {
            bb: 4,
            ..Ships::default()
        },
        10,
        1,
    );
    game_data.planets[SYSTEM_PLANET_IDX].stardock.battleships = 1;
    for seed in 0..10 {
        let (after, events) = year(&game_data, seed);

        let bombard = &events.bombard_events[0];
        assert_eq!(bombard.defender_battery_losses, 1, "seed {seed}");
        assert_eq!(bombard.stardock_items_destroyed, 1, "seed {seed}");
        assert_eq!(bombard.docked_losses.battleships, 1, "seed {seed}");
        assert_eq!(after.planets[SYSTEM_PLANET_IDX].stardock.battleships, 0);
    }
}

/// `tools/combat-model/collateral_t202.py` (`assault_collateral.py` under the
/// t202 rules, ships and batteries firing at full strength, each round's hits
/// pouring down the target order), case `BB4-B2`: the
/// exact expected losses of one bombardment turn of four battleships against
/// two two-step batteries, ten armies, and
/// a docked battleship, in cost points (BB 45), batteries destroyed, armies,
/// and points of production.
const SHIELD_MODEL: [(&str, f64); 5] = [
    ("ship cost lost", 13.5),
    ("dock cost lost", 45.0),
    ("batteries lost", 2.0),
    ("armies lost", 10.0),
    ("production lost", 94.142),
];

#[test]
fn a_bombardment_turn_costs_what_the_shield_model_says() {
    // Each trial is a year on its own campaign seed. A mean is within 4.5
    // standard errors of the model's exact expectation.
    const TRIALS: u64 = 1000;
    let mut game_data = assault_on_system(
        Order::BombardWorld,
        Ships {
            bb: 4,
            ..Ships::default()
        },
        10,
        2,
    );
    let planet = &mut game_data.planets[SYSTEM_PLANET_IDX];
    planet.stored_goods = Real48::from_int(500);
    planet.stardock.battleships = 1;

    let mut samples = vec![Vec::new(); SHIELD_MODEL.len()];
    for seed in 0..TRIALS {
        let (_, events) = year(&game_data, seed);
        assert_eq!(events.bombard_events.len(), 1, "seed {seed}");
        let bombard = &events.bombard_events[0];
        let observed = [
            45 * bombard.attacker_losses.battleships,
            45 * bombard.docked_losses.battleships,
            u32::from(bombard.defender_battery_losses),
            u32::from(bombard.defender_army_losses),
            u32::from(bombard.factories_destroyed),
        ];
        for (sample, value) in samples.iter_mut().zip(observed) {
            sample.push(f64::from(value));
        }
    }
    for (sample, (label, model)) in samples.iter().zip(SHIELD_MODEL) {
        let n = TRIALS as f64;
        let mean = sample.iter().sum::<f64>() / n;
        let variance = sample.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / (n - 1.0);
        let se = (variance / n).sqrt();
        eprintln!("{label}: engine {mean:.3} (se {se:.3}), model {model}");
        assert!(
            (mean - model).abs() <= 4.5 * se,
            "{label}: engine {mean:.3} (se {se:.3}), model {model}"
        );
    }
}

// Economy decision 5: bombardment destroys 25 goods per point of industry.

#[test]
fn an_unopposed_bombardment_destroys_25_goods_and_one_factory_per_point() {
    // One ship bombards a world with nothing in orbit, no battery, and no
    // garrison: all three rounds reach the economy. On the overwhelming
    // column of the bombardment table each round's point count is
    // `floor(hits / 10)`; exact enumeration over the rolls
    // (`collateral_t202.py`) gives a cruiser (bombardment AS 33) 15.9 points
    // and a battleship (90) 45.6, and 25 goods a point makes a cruiser raid
    // about 400 goods and a battleship raid about 1,140.
    const TRIALS: u64 = 2000;
    for (name, ships, mean_points) in [
        (
            "cruiser",
            Ships {
                ca: 1,
                ..Ships::default()
            },
            15.9,
        ),
        (
            "battleship",
            Ships {
                bb: 1,
                ..Ships::default()
            },
            45.6,
        ),
    ] {
        let mut game_data = assault_on_system(Order::BombardWorld, ships, 0, 0);
        game_data.planets[SYSTEM_PLANET_IDX].stored_goods = Real48::from_int(5000);
        let mut points = Vec::new();
        for seed in 0..TRIALS {
            let (_, events) = year(&game_data, seed);
            let bombard = &events.bombard_events[0];
            assert_eq!(
                bombard.stored_goods_destroyed,
                25 * u32::from(bombard.factories_destroyed),
                "{name}, seed {seed}"
            );
            points.push(f64::from(bombard.factories_destroyed));
        }
        let n = TRIALS as f64;
        let mean = points.iter().sum::<f64>() / n;
        let variance = points.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / (n - 1.0);
        let se = (variance / n).sqrt();
        eprintln!("{name}: {mean:.3} points (se {se:.3}), model {mean_points}");
        assert!(
            (mean - mean_points).abs() <= 4.5 * se,
            "{name}: {mean:.3} points (se {se:.3}), model {mean_points}"
        );
    }
}
