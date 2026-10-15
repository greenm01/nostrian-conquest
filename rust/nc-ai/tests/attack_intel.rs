//! What a seat's own attacks reveal (t178): after a bombardment or an
//! assault, the attacker's memory holds the target's batteries and armies
//! as its report states them, as a human player would read them.

#[allow(dead_code)]
mod common;

use common::seated_game;
use nc_ai::{Weights, executor_turn};

const PLAYERS: u8 = 4;

#[test]
fn an_attacker_knows_what_its_attack_reports_state() {
    let mut checked = 0;
    for map_seed in 0..4 {
        let mut game = seated_game(PLAYERS, map_seed);
        let start_year = game.play.built.game_data.conquest.game_year;
        for _ in 0..60 {
            for empire in 1..=PLAYERS {
                executor_turn(&mut game, empire, &Weights::TUNED, start_year);
            }
            let events = game.run_maintenance().expect("maintenance should run");
            let reports = events
                .bombard_events
                .iter()
                .map(|e| {
                    (
                        e.attacker_empire,
                        e.planet_idx,
                        e.defender_batteries_initial
                            .saturating_sub(e.defender_battery_losses),
                        e.defender_armies_initial
                            .saturating_sub(e.defender_army_losses),
                    )
                })
                .chain(events.assault_report_events.iter().map(|e| {
                    (
                        e.attacker_empire,
                        e.planet_idx,
                        e.defender_batteries_initial
                            .saturating_sub(e.defender_battery_losses),
                        e.defender_armies_initial
                            .saturating_sub(e.defender_army_losses_softening)
                            .saturating_sub(e.defender_army_losses),
                    )
                }));
            let game_data = &game.play.built.game_data;
            for (attacker, planet_idx, batteries, armies) in reports {
                if attacker == 0 || game_data.planets[planet_idx].owner.0 == attacker {
                    continue;
                }
                let memory = &game.seats[usize::from(attacker) - 1];
                // A world attacked twice in one year keeps the later report.
                let attacks = events
                    .bombard_events
                    .iter()
                    .filter(|e| e.attacker_empire == attacker && e.planet_idx == planet_idx)
                    .count()
                    + events
                        .assault_report_events
                        .iter()
                        .filter(|e| e.attacker_empire == attacker && e.planet_idx == planet_idx)
                        .count();
                if attacks > 1 {
                    continue;
                }
                let intel = &memory.planet_intel[&(planet_idx + 1)];
                assert_eq!(intel.known_ground_batteries, Some(batteries));
                assert_eq!(intel.known_armies, Some(armies));
                checked += 1;
            }
            if game.play.winner_state.winner_empire.is_some() {
                break;
            }
        }
    }
    assert!(checked > 0, "some attack should be reported");
}
