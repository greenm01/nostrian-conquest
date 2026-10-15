//! Late-game habits of one seat against three tuned executors, the seat
//! rotated game by game: the exported agent, or with `--executor` the tuned
//! executor alone, for comparison.
//!
//! ```text
//! cargo run --release -p nc-ai-onnx --example late_game -- MODEL.onnx \
//!     [--executor] [--games 256] [--years 60] [--first-map 100000]
//! ```
//!
//! Reports, per game, the seat's invasions before and after year 30, its
//! fleet-years on Bombard with no troops aboard, the longest run of years it
//! bombarded one world, its troop transports at years 30 and 45 and how many
//! of them were at home, and its worlds at their peak and at the end.

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::thread;

use nc_ai::{Families, Weights, commission_submission, executor_turn, match_game, personality};
use nc_ai_onnx::NetworkSeat;
use nc_data::Order;

const PLAYERS: u8 = 4;

#[derive(Default, Clone, Copy)]
struct Habits {
    win: f32,
    early_invasions: u32,
    late_invasions: u32,
    troopless_bombard_years: u32,
    longest_bombard_run: u32,
    transports_30: u32,
    home_transports_30: u32,
    transports_45: u32,
    home_transports_45: u32,
    peak_worlds: u32,
    final_worlds: u32,
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let arg = |name: &str| {
        args.iter()
            .position(|arg| arg == name)
            .map(|idx| args[idx + 1].clone())
    };
    let model = PathBuf::from(&args[0]);
    let executor_only = args.iter().any(|arg| arg == "--executor");
    let games: u64 = arg("--games").map_or(256, |n| n.parse().expect("--games"));
    let years: u16 = arg("--years").map_or(60, |n| n.parse().expect("--years"));
    let first: u64 = arg("--first-map").map_or(100000, |n| n.parse().expect("--first-map"));
    let threads = thread::available_parallelism().map_or(1, |n| n.get()) as u64;
    let results: Vec<Habits> = thread::scope(|scope| {
        let workers: Vec<_> = (0..threads)
            .map(|worker| {
                let model = &model;
                scope.spawn(move || {
                    let network = (!executor_only).then(|| {
                        NetworkSeat::load(
                            model,
                            Families {
                                strike: true,
                                ..Families::NONE
                            },
                            personality("land-grab").expect("built in"),
                        )
                        .expect("the model loads")
                    });
                    (first..first + games)
                        .skip(worker as usize)
                        .step_by(threads as usize)
                        .map(|map| play(network.as_ref(), map, years))
                        .collect::<Vec<_>>()
                })
            })
            .collect();
        workers
            .into_iter()
            .flat_map(|worker| worker.join().expect("worker should finish"))
            .collect()
    });
    let n = results.len() as f64;
    let mean = |get: fn(&Habits) -> u32| results.iter().map(|h| f64::from(get(h))).sum::<f64>() / n;
    println!(
        "{} seat, {games} games: win share {:.3}",
        if executor_only { "executor" } else { "network" },
        results.iter().map(|h| f64::from(h.win)).sum::<f64>() / n
    );
    println!(
        "  invasions per game: {:.2} before year 30, {:.2} after",
        mean(|h| h.early_invasions),
        mean(|h| h.late_invasions)
    );
    println!(
        "  fleet-years on Bombard with no troops aboard: {:.1}; longest run on one world {:.1} years",
        mean(|h| h.troopless_bombard_years),
        mean(|h| h.longest_bombard_run)
    );
    println!(
        "  transports at year 30: {:.1} ({:.1} at home); at year 45: {:.1} ({:.1} at home)",
        mean(|h| h.transports_30),
        mean(|h| h.home_transports_30),
        mean(|h| h.transports_45),
        mean(|h| h.home_transports_45)
    );
    println!(
        "  worlds: peak {:.1}, end {:.1}",
        mean(|h| h.peak_worlds),
        mean(|h| h.final_worlds)
    );
}

fn play(network: Option<&NetworkSeat>, map: u64, years: u16) -> Habits {
    let mut game = match_game(map, map, &[false; PLAYERS as usize], years);
    let start = game.play.built.game_data.conquest.game_year;
    let seat = (map % u64::from(PLAYERS)) as u8 + 1;
    let mut habits = Habits::default();
    // Consecutive years the seat has bombarded each world (0-based index).
    let mut runs: BTreeMap<usize, u32> = BTreeMap::new();
    for _ in 0..years {
        if network.is_some()
            && let Some(commission) = commission_submission(&game.play.built.game_data, seat)
        {
            game.submit_turn(&commission)
                .expect("commissioning submits");
        }
        for empire in 1..=PLAYERS {
            // With --executor the measured seat is an executor too.
            if empire != seat || network.is_none() {
                executor_turn(&mut game, empire, &Weights::TUNED, start);
                continue;
            }
            let game_data = &game.play.built.game_data;
            let turn = network
                .expect("a network seat")
                .turn(game_data, &game.seats[usize::from(seat) - 1], 0.0, 0)
                .expect("the network runs");
            game.submit_turn(&turn).expect("the network's turn submits");
        }
        let game_data = &game.play.built.game_data;
        habits.troopless_bombard_years += game_data
            .fleets
            .iter()
            .filter(|fleet| {
                fleet.owner.0 == seat
                    && fleet.order == Order::BombardWorld
                    && fleet.ships.loaded_transports == 0
            })
            .count() as u32;
        let events = game.run_maintenance().expect("maintenance runs");
        let game_data = &game.play.built.game_data;
        let year = game_data.conquest.game_year - start;
        let invasions = events
            .assault_report_events
            .iter()
            .filter(|e| e.attacker_empire == seat)
            .count() as u32;
        if year <= 30 {
            habits.early_invasions += invasions;
        } else {
            habits.late_invasions += invasions;
        }
        let bombarded: Vec<usize> = events
            .bombard_events
            .iter()
            .filter(|e| e.attacker_empire == seat)
            .map(|e| e.planet_idx)
            .collect();
        runs.retain(|world, _| bombarded.contains(world));
        for world in bombarded {
            let run = runs.entry(world).or_insert(0);
            *run += 1;
            habits.longest_bombard_run = habits.longest_bombard_run.max(*run);
        }
        // Home as the executor finds it: the named homeworld if the seat
        // still owns it, otherwise its richest world.
        let named = usize::from(game_data.players[usize::from(seat) - 1].homeworld);
        let home_sector = game_data
            .planets
            .get(named.wrapping_sub(1))
            .filter(|planet| planet.owner.0 == seat)
            .or_else(|| {
                game_data
                    .planets
                    .iter()
                    .filter(|planet| planet.owner.0 == seat)
                    .max_by_key(|planet| planet.potential_production)
            })
            .map(|planet| planet.sector);
        let (transports, at_home) = game_data
            .fleets
            .iter()
            .filter(|fleet| fleet.owner.0 == seat)
            .fold((0u32, 0u32), |(all, home), fleet| {
                let count = u32::from(fleet.ships.empty_transports + fleet.ships.loaded_transports);
                let here = if Some(fleet.sector) == home_sector {
                    count
                } else {
                    0
                };
                (all + count, home + here)
            });
        if year == 30 {
            (habits.transports_30, habits.home_transports_30) = (transports, at_home);
        }
        if year == 45 {
            (habits.transports_45, habits.home_transports_45) = (transports, at_home);
        }
        let worlds = game_data
            .planets
            .iter()
            .filter(|planet| planet.owner.0 == seat)
            .count() as u32;
        habits.peak_worlds = habits.peak_worlds.max(worlds);
        habits.final_worlds = worlds;
        if game.play.winner_state.winner_empire.is_some() {
            break;
        }
    }
    habits.win = win_share(
        &game.play.built.game_data,
        game.play.winner_state.winner_empire,
        seat,
    );
    habits
}

fn win_share(game_data: &nc_data::CoreGameData, winner: Option<u8>, empire: u8) -> f32 {
    if let Some(winner) = winner {
        return if winner == empire { 1.0 } else { 0.0 };
    }
    let mut production = vec![0u32; usize::from(game_data.conquest.player_count)];
    for planet in &game_data.planets {
        if let Some(sum) = production.get_mut(usize::from(planet.owner.0).wrapping_sub(1)) {
            *sum += u32::from(nc_data::present_production_points(planet));
        }
    }
    let best = production.iter().copied().max().unwrap_or(0);
    let leaders = production.iter().filter(|&&p| p == best).count();
    if production[usize::from(empire) - 1] == best {
        1.0 / leaders as f32
    } else {
        0.0
    }
}
