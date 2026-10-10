//! Engine throughput for AI-training estimates (t025): maintenance turns per
//! second and whole games per hour on fixed setups, single-threaded and with
//! independent games on parallel threads.
//!
//! Run in release: `cargo run --release -p nc-fuzz --example maint_throughput`.
//!
//! Modes:
//! - `autopilot`: every seat runs on the engine's autopilot; only maintenance
//!   is timed, so this is the bare engine cost including the built-in AI.
//! - `policy`: seats cycle the aggressor, random, and swarm policies, which
//!   plan each turn through turn KDL and submit it; planning and maintenance
//!   are timed separately, with none of the fuzzer's checks.
//! - `fuzz`: whole fuzz cases from `run_case` on drawn 2-4 player setups,
//!   including its twin maintenance run, invariants, and SQLite round trip.

use std::thread;
use std::time::{Duration, Instant};

use nc_data::map_size_for_player_count;
use nc_fuzz::{CaseSetup, ChoiceSource, FuzzConfig, Policy, plan_turn, run_case};
use nc_harness::{ScenarioPlay, build_scenario};

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

#[derive(Clone, Copy, PartialEq)]
enum Mode {
    Autopilot,
    Policy,
}

impl Mode {
    fn name(self) -> &'static str {
        match self {
            Mode::Autopilot => "autopilot",
            Mode::Policy => "policy",
        }
    }
}

#[derive(Default)]
struct Totals {
    games: u32,
    turns: u32,
    setup: Duration,
    plan: Duration,
    maintenance: Duration,
}

impl Totals {
    fn add(&mut self, other: Totals) {
        self.games += other.games;
        self.turns += other.turns;
        self.setup += other.setup;
        self.plan += other.plan;
        self.maintenance += other.maintenance;
    }
}

fn setup(mode: Mode, player_count: u8, seed: u64, turns: u16) -> CaseSetup {
    const PLAYED: [Policy; 3] = [Policy::Aggressor, Policy::RandomValid, Policy::Swarm];
    CaseSetup {
        player_count,
        map_seed: seed,
        policies: (0..usize::from(player_count))
            .map(|idx| match mode {
                Mode::Autopilot => Policy::Autopilot,
                Mode::Policy => PLAYED[idx % PLAYED.len()],
            })
            .collect(),
        collapsed_empire: None,
        rally: false,
        war: false,
        armadas: vec![None; usize::from(player_count)],
        turns,
    }
}

/// Play one game until its turn limit or a winner.
fn play_game(mode: Mode, player_count: u8, seed: u64, turns: u16) -> Totals {
    let start = Instant::now();
    let setup = setup(mode, player_count, seed, turns);
    let built = build_scenario(&setup.scenario_spec()).expect("setup should build");
    let mut play = ScenarioPlay::new(built);
    let mut choices = SeededChoices(seed);
    let mut totals = Totals {
        games: 1,
        setup: start.elapsed(),
        ..Totals::default()
    };
    for _ in 0..turns {
        if mode == Mode::Policy {
            let start = Instant::now();
            for empire in 1..=player_count {
                let planned = plan_turn(
                    setup.policy(empire),
                    &play.built.game_data,
                    empire,
                    &mut choices,
                );
                play.submit_turn(&planned.submission)
                    .expect("planned turn should submit");
            }
            totals.plan += start.elapsed();
        }
        let start = Instant::now();
        play.run_maintenance().expect("maintenance should run");
        totals.maintenance += start.elapsed();
        totals.turns += 1;
        if play.winner_state.winner_empire_raw.is_some() {
            break;
        }
    }
    totals
}

/// Play `games` games on each of `threads` threads; returns the totals and
/// the wall time.
fn run(mode: Mode, player_count: u8, turns: u16, threads: u64, games: u64) -> (Totals, Duration) {
    let start = Instant::now();
    let mut totals = Totals::default();
    thread::scope(|scope| {
        let workers: Vec<_> = (0..threads)
            .map(|thread| {
                scope.spawn(move || {
                    let mut totals = Totals::default();
                    for game in 0..games {
                        let seed = thread * games + game;
                        totals.add(play_game(mode, player_count, seed, turns));
                    }
                    totals
                })
            })
            .collect();
        for worker in workers {
            totals.add(worker.join().expect("worker should finish"));
        }
    });
    (totals, start.elapsed())
}

/// Per game: setup ms. Per turn: plan ms and maintenance ms, and the
/// maintenance-only rate. Then wall-clock turns/s and games/h, which
/// include setup and planning.
fn report(label: &str, totals: &Totals, wall: Duration) {
    let turns = f64::from(totals.turns);
    let per_turn_ms = |time: Duration| time.as_secs_f64() * 1e3 / turns;
    let wall_secs = wall.as_secs_f64();
    println!(
        "{label:<30} {:>5} {:>6.1} {:>8.2} {:>8.3} {:>8.4} {:>9.0} {:>9.0} {:>10.0}",
        totals.games,
        turns / f64::from(totals.games),
        totals.setup.as_secs_f64() * 1e3 / f64::from(totals.games),
        per_turn_ms(totals.plan),
        per_turn_ms(totals.maintenance),
        turns / totals.maintenance.as_secs_f64(),
        turns / wall_secs,
        f64::from(totals.games) * 3600.0 / wall_secs,
    );
}

fn main() {
    println!(
        "{:<30} {:>5} {:>6} {:>8} {:>8} {:>8} {:>9} {:>9} {:>10}",
        "config",
        "games",
        "turns",
        "setup ms",
        "plan ms",
        "maint ms",
        "maint t/s",
        "turns/s",
        "games/h"
    );
    for mode in [Mode::Autopilot, Mode::Policy] {
        for (player_count, games) in [(2, 20), (4, 10), (9, 4), (16, 2), (25, 1)] {
            let map = map_size_for_player_count(player_count);
            for turns in [50, 100] {
                let (totals, wall) = run(mode, player_count, turns, 1, games);
                let label = format!("{} {player_count}p {map}x{map} {turns}t", mode.name());
                report(&label, &totals, wall);
            }
        }
    }

    // Independent 4-player games on parallel threads.
    let cores = thread::available_parallelism().map_or(1, |n| n.get() as u64);
    for mode in [Mode::Autopilot, Mode::Policy] {
        let mut threads = 1;
        while threads <= cores {
            let (totals, wall) = run(mode, 4, 100, threads, 4);
            let label = format!("{} 4p 18x18 100t x{threads}", mode.name());
            report(&label, &totals, wall);
            threads *= 2;
        }
    }

    // The fuzz loop with all its checks, for the overhead comparison.
    let config = FuzzConfig { max_turns: 50 };
    let (games, start) = (40, Instant::now());
    let turns: usize = (0..games)
        .map(|seed| run_case(&mut SeededChoices(seed), &config).turns.len())
        .sum();
    let wall_secs = start.elapsed().as_secs_f64();
    println!(
        "fuzz run_case, drawn 2-4p setups, up to 50t: {games} games, {:.1} turns each, {:.3} ms/turn, {:.0} turns/s",
        turns as f64 / games as f64,
        wall_secs * 1e3 / turns as f64,
        turns as f64 / wall_secs
    );
}
