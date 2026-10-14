//! Score the scripted executor (t154): one executor seat against three seats
//! of another policy, with the executor's seat rotated game by game.
//!
//! ```text
//! cargo run --release -p nc-ai --example executor_matches -- \
//!     [--weights baseline|tuned|classic] \
//!     [--opponent aggressor|autopilot|baseline|tuned|classic] \
//!     [--with strike_weakest|strike_by_value] \
//!     [--games 64] [--years 60] [--first-map 0]
//! ```
//!
//! `classic` is t154's tuned executor with its original build order, kept
//! to measure the land-grab doctrine against (t159). `--with` sets
//! doctrine switches for the candidate seat only.
//!
//! Games run on every core. A win is the declared winner or, without one,
//! the most production at the end (a tie shares the win).

use std::thread;

use nc_ai::{SeatPolicy, Weights, play_match};

const PLAYERS: usize = 4;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let arg = |name: &str| {
        args.iter()
            .position(|arg| arg == name)
            .map(|idx| args[idx + 1].clone())
    };
    let weights = match arg("--weights").as_deref() {
        None | Some("tuned") => Weights::TUNED,
        Some("baseline") => Weights::BASELINE,
        Some("classic") => Weights::CLASSIC,
        Some(other) => panic!("unknown weights {other}"),
    };
    let opponent = match arg("--opponent").as_deref() {
        None | Some("aggressor") => SeatPolicy::Aggressor,
        Some("autopilot") => SeatPolicy::Autopilot,
        Some("baseline") => SeatPolicy::Executor(Weights::BASELINE),
        Some("tuned") => SeatPolicy::Executor(Weights::TUNED),
        Some("classic") => SeatPolicy::Executor(Weights::CLASSIC),
        Some(other) => panic!("unknown opponent {other}"),
    };
    // Doctrine switches to turn on for the candidate seat only.
    let mut weights = weights;
    for switch in arg("--with").iter().flat_map(|names| names.split(',')) {
        match switch {
            "strike_weakest" => weights.doctrine.strike_weakest = true,
            "strike_by_value" => weights.doctrine.strike_weakest = false,
            other => panic!("unknown doctrine switch {other}"),
        }
    }
    let games: u64 = arg("--games").map_or(64, |n| n.parse().expect("--games"));
    let years: u16 = arg("--years").map_or(60, |n| n.parse().expect("--years"));
    let first: u64 = arg("--first-map").map_or(0, |n| n.parse().expect("--first-map"));

    let results = run(first..first + games, years, weights, opponent);
    let wins: f32 = results.iter().map(|(_, win, _)| win).sum();
    let share: f32 = results.iter().map(|(_, _, share)| share).sum();
    let n = results.len() as f32;
    println!(
        "{games} games, {years} years, executor vs 3x {opponent:?}: win rate {:.3}, mean production share {:.3}",
        wins / n,
        share / n
    );
}

/// (map, win share, production share) for each game. With `NC_AI_VERBOSE`
/// set, each game's end state is printed.
pub fn run(
    maps: std::ops::Range<u64>,
    years: u16,
    weights: Weights,
    opponent: SeatPolicy,
) -> Vec<(u64, f32, f32)> {
    let threads = thread::available_parallelism().map_or(1, |n| n.get()) as u64;
    let maps: Vec<u64> = maps.collect();
    let mut results = Vec::new();
    thread::scope(|scope| {
        let workers: Vec<_> = (0..threads)
            .map(|worker| {
                let maps = &maps;
                scope.spawn(move || {
                    maps.iter()
                        .skip(worker as usize)
                        .step_by(threads as usize)
                        .map(|&map| {
                            let seat = (map as usize) % PLAYERS;
                            let mut policies = [opponent; PLAYERS];
                            policies[seat] = SeatPolicy::Executor(weights);
                            let result = play_match(map, &policies, years);
                            if std::env::var_os("NC_AI_VERBOSE").is_some() {
                                eprintln!("map {map} seat {}: {result:?}", seat + 1);
                            }
                            let empire = seat as u8 + 1;
                            (
                                map,
                                result.win_share(empire),
                                result.production_share(empire),
                            )
                        })
                        .collect::<Vec<_>>()
                })
            })
            .collect();
        for worker in workers {
            results.extend(worker.join().expect("worker should finish"));
        }
    });
    results.sort_by_key(|(map, _, _)| *map);
    results
}
