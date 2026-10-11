//! Planetary combat after combat decision 4: the landing battle on the
//! shared table. The exact arithmetic is pinned by the inline tests in
//! `src/maint/combat/assault_tests.rs`; these run the whole year.

use nc_data::{CoreGameData, MaintenanceEvents, MissionOutcome, Order};
use nc_engine::run_maintenance_turn_with_context_and_seed;

use super::support::{
    SYSTEM, SYSTEM_PLANET_IDX, Ships, diplomacy, empty_board, mark_arrived, place_fleet,
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
