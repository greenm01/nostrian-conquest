//! Score an exported agent as a Rust seat (t165): one network seat against
//! three tuned executors, the seat rotated game by game.
//!
//! ```text
//! cargo run --release -p nc-ai-onnx --example seat_matches -- MODEL.onnx \
//!     [--temperature 0] [--games 64] [--years 60] [--first-map 100000]
//! ```
//!
//! Prints the win share, and the mean and slowest time the network seat took
//! for a turn. Games run on every core.

use std::path::{Path, PathBuf};
use std::thread;
use std::time::Instant;

use nc_ai::{Families, Weights, commission_submission, executor_turn, match_game};
use nc_ai_onnx::NetworkSeat;

const PLAYERS: u8 = 4;

/// One game: the network seat's win share, its seconds in turns, its
/// slowest turn, and its turn count.
struct Played {
    win: f32,
    seconds: f64,
    slowest: f64,
    turns: u32,
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let arg = |name: &str| {
        args.iter()
            .position(|arg| arg == name)
            .map(|idx| args[idx + 1].clone())
    };
    let model = PathBuf::from(&args[0]);
    let temperature: f32 = arg("--temperature").map_or(0.0, |t| t.parse().expect("--temperature"));
    let games: u64 = arg("--games").map_or(64, |n| n.parse().expect("--games"));
    let years: u16 = arg("--years").map_or(60, |n| n.parse().expect("--years"));
    let first: u64 = arg("--first-map").map_or(100000, |n| n.parse().expect("--first-map"));
    let threads = thread::available_parallelism().map_or(1, |n| n.get()) as u64;
    let results: Vec<Played> = thread::scope(|scope| {
        let workers: Vec<_> = (0..threads)
            .map(|worker| {
                let model = &model;
                scope.spawn(move || {
                    let seat = load(model);
                    (first..first + games)
                        .skip(worker as usize)
                        .step_by(threads as usize)
                        .map(|map| play(&seat, map, years, temperature))
                        .collect::<Vec<_>>()
                })
            })
            .collect();
        workers
            .into_iter()
            .flat_map(|worker| worker.join().expect("worker should finish"))
            .collect()
    });
    let n = results.len() as f32;
    let seconds: f64 = results.iter().map(|r| r.seconds).sum();
    let turns: u32 = results.iter().map(|r| r.turns).sum();
    println!(
        "{games} games, {years} years, temperature {temperature}: win share {:.3}; \
         network turn mean {:.3} s, slowest {:.3} s",
        results.iter().map(|r| r.win).sum::<f32>() / n,
        seconds / f64::from(turns),
        results.iter().map(|r| r.slowest).fold(0.0, f64::max),
    );
}

fn load(model: &Path) -> NetworkSeat {
    let strike = Families {
        strike: true,
        ..Families::NONE
    };
    NetworkSeat::load(model, strike, Weights::TUNED).expect("the model loads")
}

fn play(seat: &NetworkSeat, map: u64, years: u16, temperature: f32) -> Played {
    let mut game = match_game(map, map, &[false; PLAYERS as usize], years);
    let start_year = game.play.built.game_data.conquest.game_year;
    let learner = (map % u64::from(PLAYERS)) as u8 + 1;
    let mut played = Played {
        win: 0.0,
        seconds: 0.0,
        slowest: 0.0,
        turns: 0,
    };
    for _ in 0..years {
        // The network seat commissions at the start of the year and the
        // executors at their own turns, as the batch environment has them.
        if let Some(commission) = commission_submission(&game.play.built.game_data, learner) {
            game.submit_turn(&commission)
                .expect("commissioning submits");
        }
        for empire in 1..=PLAYERS {
            if empire != learner {
                executor_turn(&mut game, empire, &Weights::TUNED, start_year);
                continue;
            }
            let started = Instant::now();
            let year = u64::from(game.play.built.game_data.conquest.game_year);
            let turn = seat
                .turn(
                    &game.play.built.game_data,
                    &game.seats[usize::from(empire) - 1],
                    temperature,
                    map ^ (year << 32),
                )
                .expect("the network runs");
            let took = started.elapsed().as_secs_f64();
            played.seconds += took;
            played.slowest = played.slowest.max(took);
            played.turns += 1;
            game.submit_turn(&turn).expect("the network's turn submits");
        }
        game.run_maintenance().expect("maintenance runs");
        if game.play.winner_state.winner_empire.is_some() {
            break;
        }
    }
    played.win = win_share(
        &game.play.built.game_data,
        game.play.winner_state.winner_empire,
        learner,
    );
    played
}

/// 1 for the declared winner, or, with none, shared by the empires with the
/// most production, as `nc_ai::MatchResult::win_share` scores it.
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
