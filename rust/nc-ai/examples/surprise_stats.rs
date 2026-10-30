//! How often surprise decides fleet battles: four land-grab executors play
//! each map, and every battle is counted once, from the attacker's report.
//!
//! ```text
//! cargo run --release -p nc-ai --example surprise_stats -- [--games 256] [--years 60]
//! ```
//!
//! It prints the share of battles that were surprises, the attacker's share
//! of battles won (it held the field) with and without surprise, by force
//! ratio (attacker's attack strength over the defender's, starbases
//! included), and the exchange ratio (defender's cost lost over the
//! attacker's) in each case. Battles against a defender with no warship or
//! starbase are left out: nothing there can fight back.

use std::thread;

use nc_ai::{executor_turn, match_game, personality};
use nc_data::maintenance_types::FleetBattlePerspective;
use nc_data::{FleetBattleEvent, ShipLosses};

const PLAYERS: u8 = 4;
const STARBASE_COST: u32 = 50;
const STARBASE_AS: u32 = 100;
/// Force-ratio buckets: under 2:3, even (2:3 to 3:2), over 3:2.
const BUCKETS: [&str; 3] = ["outgunned", "even", "outgunning"];

#[derive(Default, Clone, Copy)]
struct Tally {
    battles: u64,
    won: u64,
    attacker_lost: u64,
    defender_lost: u64,
}

impl Tally {
    fn add(&mut self, other: Tally) {
        self.battles += other.battles;
        self.won += other.won;
        self.attacker_lost += other.attacker_lost;
        self.defender_lost += other.defender_lost;
    }
}

/// By bucket, then surprise (0 no, 1 yes).
#[derive(Default, Clone, Copy)]
struct Stats {
    games: u64,
    cells: [[Tally; 2]; BUCKETS.len()],
    /// Battles where the defender had a starbase.
    with_starbase: u64,
}

impl Stats {
    fn merge(&mut self, other: Stats) {
        self.games += other.games;
        self.with_starbase += other.with_starbase;
        for (row, other_row) in self.cells.iter_mut().zip(other.cells) {
            for (cell, other_cell) in row.iter_mut().zip(other_row) {
                cell.add(other_cell);
            }
        }
    }
}

fn cost(ships: &ShipLosses) -> u32 {
    5 * ships.destroyers
        + 15 * ships.cruisers
        + 45 * ships.battleships
        + 15 * ships.scouts
        + 5 * ships.transports
        + 20 * ships.etacs
}

/// Attack strength: DD 12, CA 33, BB 90, starbase 100.
fn strength(ships: &ShipLosses, starbases: u32) -> u32 {
    12 * ships.destroyers + 33 * ships.cruisers + 90 * ships.battleships + STARBASE_AS * starbases
}

fn count(stats: &mut Stats, battle: &FleetBattleEvent) {
    if battle.perspective != FleetBattlePerspective::Intercepted {
        return;
    }
    let theirs = strength(&battle.enemy_initial, battle.enemy_initial_starbases);
    if theirs == 0 {
        return;
    }
    let ours = strength(&battle.friendly_initial, battle.friendly_initial_starbases);
    let ratio = f64::from(ours) / f64::from(theirs);
    let bucket = if ratio < 2.0 / 3.0 {
        0
    } else if ratio <= 1.5 {
        1
    } else {
        2
    };
    stats.with_starbase += u64::from(battle.enemy_initial_starbases > 0);
    let cell = &mut stats.cells[bucket][usize::from(battle.surprise)];
    cell.battles += 1;
    cell.won += u64::from(battle.held_field);
    cell.attacker_lost +=
        u64::from(cost(&battle.friendly_losses) + STARBASE_COST * battle.friendly_starbases_lost);
    cell.defender_lost +=
        u64::from(cost(&battle.enemy_losses) + STARBASE_COST * battle.enemy_starbases_destroyed);
}

fn play(map: u64, years: u16) -> Stats {
    let weights = personality("land-grab").expect("built in").weights;
    let mut game = match_game(map, map, &[false; PLAYERS as usize], years);
    let start = game.play.built.game_data.conquest.game_year;
    let mut stats = Stats {
        games: 1,
        ..Stats::default()
    };
    for _ in 0..years {
        for empire in 1..=PLAYERS {
            executor_turn(&mut game, empire, &weights, start);
        }
        let events = game.run_maintenance().expect("maintenance runs");
        for battle in &events.fleet_battle_events {
            count(&mut stats, battle);
        }
        if game.play.winner_state.winner_empire.is_some() {
            break;
        }
    }
    stats
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let flag = |name: &str, default: u64| {
        args.iter()
            .position(|arg| arg == name)
            .map_or(default, |idx| args[idx + 1].parse().expect("a number"))
    };
    let (games, years) = (flag("--games", 256), flag("--years", 60) as u16);
    let threads = thread::available_parallelism().map_or(1, |n| n.get());
    let mut stats = Stats::default();
    thread::scope(|scope| {
        let workers: Vec<_> = (0..threads)
            .map(|worker| {
                scope.spawn(move || {
                    let mut stats = Stats::default();
                    for map in (100_000..100_000 + games).skip(worker).step_by(threads) {
                        stats.merge(play(map, years));
                    }
                    stats
                })
            })
            .collect();
        for worker in workers {
            stats.merge(worker.join().expect("worker should finish"));
        }
    });

    let all: Tally = stats
        .cells
        .iter()
        .flatten()
        .fold(Tally::default(), |mut sum, cell| {
            sum.add(*cell);
            sum
        });
    let surprises: u64 = stats.cells.iter().map(|row| row[1].battles).sum();
    println!(
        "{} games of {years} years: {} battles ({:.1} a game), {} surprises ({:.1}%); \
         {} against a defender with a starbase",
        stats.games,
        all.battles,
        all.battles as f64 / stats.games as f64,
        surprises,
        100.0 * surprises as f64 / all.battles.max(1) as f64,
        stats.with_starbase,
    );
    for (name, row) in BUCKETS.iter().zip(stats.cells) {
        for (label, cell) in ["plain", "surprise"].iter().zip(row) {
            let won = cell.won as f64 / cell.battles.max(1) as f64;
            // A binomial share's standard error.
            let se = (won * (1.0 - won) / cell.battles.max(1) as f64).sqrt();
            println!(
                "{name:>10} {label:>8}: {:>6} battles, attacker won {:.1}% (se {:.1}), \
                 exchange {:.2}",
                cell.battles,
                100.0 * won,
                100.0 * se,
                cell.defender_lost as f64 / cell.attacker_lost.max(1) as f64,
            );
        }
    }
}
