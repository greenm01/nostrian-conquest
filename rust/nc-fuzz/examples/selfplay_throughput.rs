//! Self-play throughput for AI training (t150): games built once per map and
//! cloned for each game, with a distinct campaign seed per game, played by
//! aggressor seats on parallel threads.
//!
//! Run in release: `cargo run --release -p nc-fuzz --example selfplay_throughput`.
//!
//! For each player count it reports what building every game from scratch
//! costs against cloning a prebuilt start state, how throughput scales with
//! threads, and how many campaign seeds on one map, with the same choices,
//! end in distinct states.

use std::collections::HashSet;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::thread;
use std::time::{Duration, Instant};

use nc_data::map_size_for_player_count;
use nc_fuzz::{CaseSetup, ChoiceSource, Policy, plan_turn};
use nc_harness::{ScenarioPlay, build_scenario};

const TURNS: u16 = 100;

/// Seeded splitmix64 choices, so every game is reproducible.
struct SeededChoices(u64);

impl SeededChoices {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
}

impl ChoiceSource for SeededChoices {
    fn pick(&mut self, lo: u32, hi: u32) -> u32 {
        lo + (self.next() % (u64::from(hi - lo) + 1)) as u32
    }

    fn flip(&mut self) -> bool {
        self.next() & 1 == 1
    }
}

fn setup(player_count: u8, map_seed: u64) -> CaseSetup {
    CaseSetup {
        player_count,
        map_seed,
        policies: vec![Policy::Aggressor; usize::from(player_count)],
        collapsed_empire: None,
        rally: false,
        war: false,
        armadas: vec![None; usize::from(player_count)],
        turns: TURNS,
    }
}

fn start_state(player_count: u8, map_seed: u64) -> ScenarioPlay {
    let built =
        build_scenario(&setup(player_count, map_seed).scenario_spec()).expect("setup should build");
    ScenarioPlay::new(built)
}

/// Play one game from `play` to the turn limit or a winner, with the given
/// campaign seed and choice seed. Returns the turns played and the final game.
fn play_game(play: ScenarioPlay, campaign_seed: u64, choice_seed: u64) -> (u32, ScenarioPlay) {
    let mut play = play.with_campaign_seed(campaign_seed);
    let player_count = play.built.game_data.conquest.player_count;
    let mut choices = SeededChoices(choice_seed);
    let mut turns = 0;
    for _ in 0..TURNS {
        for empire in 1..=player_count {
            let planned = plan_turn(
                Policy::Aggressor,
                &play.built.game_data,
                empire,
                &mut choices,
            );
            play.submit_turn(&planned.submission)
                .expect("planned turn should submit");
        }
        play.run_maintenance().expect("maintenance should run");
        turns += 1;
        if play.winner_state.winner_empire.is_some() {
            break;
        }
    }
    (turns, play)
}

/// Play `games` games per thread on `threads` threads. With `starts`, each
/// game clones a prebuilt start state (cycling through them); without, each
/// game builds its own map. Returns games, turns and wall time.
fn run(
    player_count: u8,
    threads: u64,
    games: u64,
    starts: Option<&[ScenarioPlay]>,
) -> (u64, u64, Duration) {
    let start = Instant::now();
    let mut totals = (0, 0);
    thread::scope(|scope| {
        let workers: Vec<_> = (0..threads)
            .map(|thread| {
                scope.spawn(move || {
                    let mut turns = 0;
                    for game in 0..games {
                        let seed = thread * games + game;
                        let play = match starts {
                            Some(starts) => starts[seed as usize % starts.len()].clone(),
                            None => start_state(player_count, seed),
                        };
                        turns += u64::from(play_game(play, seed, seed).0);
                    }
                    (games, turns)
                })
            })
            .collect();
        for worker in workers {
            let (games, turns) = worker.join().expect("worker should finish");
            totals.0 += games;
            totals.1 += turns;
        }
    });
    (totals.0, totals.1, start.elapsed())
}

fn report(label: &str, (games, turns, wall): (u64, u64, Duration)) {
    let secs = wall.as_secs_f64();
    println!(
        "{label:<34} {games:>6} {:>6.1} {:>9.0} {:>10.0}",
        turns as f64 / games as f64,
        turns as f64 / secs,
        games as f64 * 3600.0 / secs,
    );
}

fn main() {
    let cores = thread::available_parallelism().map_or(1, |n| n.get() as u64);
    println!(
        "{:<34} {:>6} {:>6} {:>9} {:>10}",
        "config", "games", "turns", "turns/s", "games/h"
    );
    for player_count in [2u8, 4] {
        let map = map_size_for_player_count(player_count);
        let label = format!("{player_count}p {map}x{map} aggressor {TURNS}t");
        let starts: Vec<_> = (0..8).map(|seed| start_state(player_count, seed)).collect();

        report(&format!("{label} built x1"), run(player_count, 1, 8, None));
        report(
            &format!("{label} cloned x1"),
            run(player_count, 1, 8, Some(&starts)),
        );
        let mut threads = 2;
        while threads <= cores {
            report(
                &format!("{label} cloned x{threads}"),
                run(player_count, threads, 8, Some(&starts)),
            );
            threads *= 2;
        }
        if !cores.is_power_of_two() {
            report(
                &format!("{label} cloned x{cores}"),
                run(player_count, cores, 8, Some(&starts)),
            );
        }

        // Same map and choices, different campaign seeds: count distinct ends.
        let ends: HashSet<u64> = (0..8)
            .map(|seed| {
                let (_, end) = play_game(starts[0].clone(), seed, 0);
                let mut hasher = DefaultHasher::new();
                format!("{:?}", end.built.game_data).hash(&mut hasher);
                hasher.finish()
            })
            .collect();
        println!(
            "{label}: 8 campaign seeds on map 0 gave {} distinct end states",
            ends.len()
        );
    }
}
