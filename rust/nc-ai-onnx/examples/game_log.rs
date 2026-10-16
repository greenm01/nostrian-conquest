//! Play one game and log what the network seat does each year (t165): the
//! exported agent in one seat against three tuned executors.
//!
//! ```text
//! cargo run --release -p nc-ai-onnx --example game_log -- MODEL.onnx \
//!     [--map 100000] [--seat 1] [--years 60] [--style land-grab]
//! ```

use std::collections::BTreeMap;
use std::path::PathBuf;

use nc_ai::{Families, Weights, commission_submission, executor_turn, match_game, personality};
use nc_ai_onnx::NetworkSeat;
use nc_data::{DiplomaticRelation, FleetTurnAction, Order, PlanetTurnAction};
use nc_engine::BUILD_UNITS;

const PLAYERS: u8 = 4;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let arg = |name: &str| {
        args.iter()
            .position(|arg| arg == name)
            .map(|idx| args[idx + 1].clone())
    };
    let model = PathBuf::from(&args[0]);
    let map: u64 = arg("--map").map_or(100000, |n| n.parse().expect("--map"));
    let seat: u8 = arg("--seat").map_or(1, |n| n.parse().expect("--seat"));
    let years: u16 = arg("--years").map_or(60, |n| n.parse().expect("--years"));
    let style = arg("--style").unwrap_or_else(|| "land-grab".to_string());
    let network = NetworkSeat::load(
        &model,
        Families {
            strike: true,
            ..Families::NONE
        },
        personality(&style).expect("a known personality"),
    )
    .expect("the model loads");

    let mut game = match_game(map, map, &[false; PLAYERS as usize], years);
    let start = game.play.built.game_data.conquest.game_year;
    println!("map {map}: the network plays empire {seat} ({style}); empires 1-4 below\n");
    for _ in 0..years {
        if let Some(commission) = commission_submission(&game.play.built.game_data, seat) {
            game.submit_turn(&commission)
                .expect("commissioning submits");
        }
        let mut notes = Vec::new();
        for empire in 1..=PLAYERS {
            if empire != seat {
                executor_turn(&mut game, empire, &Weights::TUNED, start);
                continue;
            }
            let game_data = &game.play.built.game_data;
            let turn = network
                .turn(game_data, &game.seats[usize::from(seat) - 1], 0.0, 0)
                .expect("the network runs");
            // Builds, by kind.
            let mut builds: BTreeMap<String, u32> = BTreeMap::new();
            for action in turn.planets.iter().flat_map(|p| &p.actions) {
                if let PlanetTurnAction::Build {
                    points_remaining,
                    kind,
                } = action
                    && let Some(unit) = BUILD_UNITS
                        .iter()
                        .find(|unit| nc_engine::production_item_kind_code(unit.kind) == *kind)
                {
                    *builds.entry(format!("{:?}", unit.kind)).or_default() +=
                        points_remaining / unit.cost.max(1);
                }
            }
            // Fleet orders, by mission, noting deep space and detaches.
            let mut orders: BTreeMap<String, u32> = BTreeMap::new();
            let mut detaches = 0;
            for block in &turn.fleets {
                for action in &block.actions {
                    match action {
                        FleetTurnAction::Order {
                            order_code, target, ..
                        } => {
                            let order = Order::from_code(*order_code);
                            let deep = target.is_some_and(|sector| {
                                !game_data
                                    .planets
                                    .iter()
                                    .any(|planet| planet.sector.to_array() == sector)
                            });
                            let name =
                                format!("{order:?}{}", if deep { " (deep space)" } else { "" });
                            *orders.entry(name).or_default() += 1;
                        }
                        FleetTurnAction::Detach { .. } => detaches += 1,
                        _ => {}
                    }
                }
            }
            if let Some(tax) = turn.tax_rate {
                notes.push(format!("tax {tax}%"));
            }
            for directive in &turn.diplomacy {
                notes.push(format!(
                    "declares empire {} {}",
                    directive.to_empire,
                    if directive.relation == DiplomaticRelation::Enemy {
                        "an enemy"
                    } else {
                        "neutral"
                    }
                ));
            }
            if !builds.is_empty() {
                notes.push(format!("builds {}", list(&builds)));
            }
            if !orders.is_empty() {
                notes.push(format!("orders {}", list(&orders)));
            }
            if detaches > 0 {
                notes.push(format!("detaches {detaches}"));
            }
            game.submit_turn(&turn).expect("the network's turn submits");
        }
        let owners: Vec<u8> = game
            .play
            .built
            .game_data
            .planets
            .iter()
            .map(|planet| planet.owner.0)
            .collect();
        let events = game.run_maintenance().expect("maintenance runs");
        let game_data = &game.play.built.game_data;
        let year = game_data.conquest.game_year - start;
        let name = |idx: usize| format!("world {}", idx + 1);
        for e in events
            .bombard_events
            .iter()
            .filter(|e| e.attacker_empire == seat)
        {
            notes.push(format!("bombards {}", name(e.planet_idx)));
        }
        for e in events
            .assault_report_events
            .iter()
            .filter(|e| e.attacker_empire == seat)
        {
            notes.push(format!("assaults {}", name(e.planet_idx)));
        }
        // Captures and losses, from each world's owner before and after.
        for (idx, (planet, &before)) in game_data.planets.iter().zip(&owners).enumerate() {
            let after = planet.owner.0;
            if after == seat && before != seat {
                notes.push(format!("takes {} from {}", name(idx), owner(before)));
            } else if before == seat && after != seat {
                notes.push(format!("loses {} to {}", name(idx), owner(after)));
            }
        }
        let battles: Vec<_> = events
            .fleet_battle_events
            .iter()
            .filter(|b| b.reporting_empire == seat)
            .collect();
        if !battles.is_empty() {
            let won = battles.iter().filter(|b| b.held_field).count();
            notes.push(format!(
                "fleet battles {} (held the field {won})",
                battles.len()
            ));
        }
        let worlds: Vec<usize> = (1..=PLAYERS)
            .map(|e| game_data.planets.iter().filter(|p| p.owner.0 == e).count())
            .collect();
        let fleet = |e: u8| {
            game_data
                .fleets
                .iter()
                .filter(|f| f.owner.0 == e)
                .map(|f| {
                    u32::from(f.ships.destroyers)
                        + 3 * u32::from(f.ships.cruisers)
                        + 9 * u32::from(f.ships.battleships)
                })
                .sum::<u32>()
        };
        let strength: Vec<u32> = (1..=PLAYERS).map(fleet).collect();
        println!(
            "year {year:2}  worlds {worlds:?}  warship strength {strength:?}\n    {}",
            notes.join("; ")
        );
        if let Some(winner) = game.play.winner_state.winner_empire {
            println!("\nempire {winner} wins in year {year}");
            break;
        }
    }
}

fn owner(empire: u8) -> String {
    if empire == 0 {
        "nobody".to_string()
    } else {
        format!("empire {empire}")
    }
}

fn list(counts: &BTreeMap<String, u32>) -> String {
    counts
        .iter()
        .map(|(name, count)| format!("{count} {name}"))
        .collect::<Vec<_>>()
        .join(", ")
}
