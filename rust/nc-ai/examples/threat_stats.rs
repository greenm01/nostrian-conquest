//! How well the executor's threat rule foresees attacks (t244): four
//! land-grab executors play each map, and every year, before the turns,
//! each world is judged threatened or not from its owner's memory, by the
//! rule (a declared enemy's force reported within 4 sectors in the last 2
//! years) and by looser variants. A world is attacked when a bombardment or
//! an assault (invasion or blitz) targets it while its owner holds it.
//!
//! ```text
//! cargo run --release -p nc-ai --example threat_stats -- [--games 256] [--years 60] [--first-map 100000]
//! ```
//!
//! For each variant and horizon (attacked this year, or this year or next),
//! it prints the share of world-years flagged, the share of flags followed
//! by an attack (precision), and the share of attacks flagged beforehand
//! (recall). It also prints the share of attacks whose attacker the owner had
//! not declared an enemy when the year began.

use std::collections::BTreeSet;
use std::thread;

use nc_ai::{SeatMemory, executor_turn, match_game, personality, threatened};
use nc_data::{CoreGameData, DiplomaticRelation};

const PLAYERS: u8 = 4;
const HORIZONS: [u16; 2] = [1, 2];

/// A threat rule: a force within `range` sectors, reported in the last
/// `years` years, of an empire the owner declared an enemy (`declared`) or of
/// any other empire.
struct Variant {
    name: &'static str,
    range: f32,
    years: u16,
    declared: bool,
}

const VARIANTS: [Variant; 4] = [
    Variant {
        name: "rule: declared, 4 sectors, 2 years",
        range: 4.0,
        years: 2,
        declared: true,
    },
    Variant {
        name: "any empire, 4 sectors, 2 years",
        range: 4.0,
        years: 2,
        declared: false,
    },
    Variant {
        name: "any empire, 8 sectors, 2 years",
        range: 8.0,
        years: 2,
        declared: false,
    },
    Variant {
        name: "any empire, 8 sectors, 4 years",
        range: 8.0,
        years: 4,
        declared: false,
    },
];

fn flagged(game_data: &CoreGameData, memory: &SeatMemory, sector: [u8; 2], v: &Variant) -> bool {
    let year = game_data.conquest.game_year;
    let declared = &game_data.players[usize::from(memory.empire) - 1].diplomacy;
    memory.contacts.iter().any(|contact| {
        let (dx, dy) = (
            f32::from(contact.coords[0]) - f32::from(sector[0]),
            f32::from(contact.coords[1]) - f32::from(sector[1]),
        );
        contact.empire != 0
            && contact.empire != memory.empire
            && (!v.declared
                || declared.get(usize::from(contact.empire) - 1)
                    == Some(&DiplomaticRelation::Enemy))
            && year.saturating_sub(contact.year) < v.years
            && (dx * dx + dy * dy).sqrt() <= v.range
    })
}

/// Counts for one variant and horizon.
#[derive(Default, Clone, Copy)]
struct Cell {
    /// World-years whose whole horizon was played.
    judged: u64,
    flagged: u64,
    flagged_attacked: u64,
    attacked: u64,
}

#[derive(Default, Clone, Copy)]
struct Stats {
    games: u64,
    attacks: u64,
    undeclared_attacks: u64,
    cells: [[Cell; HORIZONS.len()]; VARIANTS.len()],
}

impl Stats {
    fn merge(&mut self, other: Stats) {
        self.games += other.games;
        self.attacks += other.attacks;
        self.undeclared_attacks += other.undeclared_attacks;
        for (row, other_row) in self.cells.iter_mut().zip(other.cells) {
            for (cell, other) in row.iter_mut().zip(other_row) {
                cell.judged += other.judged;
                cell.flagged += other.flagged;
                cell.flagged_attacked += other.flagged_attacked;
                cell.attacked += other.attacked;
            }
        }
    }
}

/// One world-year: the world, its owner, the year index, and each variant's flag.
struct Judged {
    planet: usize,
    owner: u8,
    turn: u16,
    flags: [bool; VARIANTS.len()],
}

fn play(map: u64, years: u16) -> Stats {
    let weights = personality("land-grab").expect("built in").weights;
    let mut game = match_game(map, map, &[false; PLAYERS as usize], years);
    let start = game.play.built.game_data.conquest.game_year;
    let mut stats = Stats {
        games: 1,
        ..Stats::default()
    };
    let mut judged = Vec::new();
    // (planet, owner, turn) attacked.
    let mut attacked = BTreeSet::new();
    let mut turns = 0;
    for turn in 0..years {
        let game_data = &game.play.built.game_data;
        for (planet, record) in game_data.planets.iter().enumerate() {
            let owner = record.owner.0;
            if owner == 0 || owner > PLAYERS {
                continue;
            }
            let memory = &game.seats[usize::from(owner) - 1];
            let sector = record.sector.to_array();
            let flags = VARIANTS
                .each_ref()
                .map(|v| flagged(game_data, memory, sector, v));
            assert_eq!(
                flags[0],
                threatened(game_data, memory, sector),
                "the first variant is the executor's rule"
            );
            judged.push(Judged {
                planet,
                owner,
                turn,
                flags,
            });
        }
        let declared: Vec<_> = game_data
            .players
            .iter()
            .map(|player| player.diplomacy)
            .collect();
        for empire in 1..=PLAYERS {
            executor_turn(&mut game, empire, &weights, start);
        }
        let events = game.run_maintenance().expect("maintenance runs");
        let targets = events
            .bombard_events
            .iter()
            .map(|e| (e.planet_idx, e.defender_empire, e.attacker_empire))
            .chain(
                events
                    .assault_report_events
                    .iter()
                    .map(|e| (e.planet_idx, e.defender_empire, e.attacker_empire)),
            );
        for (planet, defender, attacker) in targets {
            if defender == 0 || !attacked.insert((planet, defender, turn)) {
                continue;
            }
            stats.attacks += 1;
            stats.undeclared_attacks += u64::from(
                declared[usize::from(defender) - 1].get(usize::from(attacker) - 1)
                    != Some(&DiplomaticRelation::Enemy),
            );
        }
        turns = turn + 1;
        if game.play.winner_state.winner_empire.is_some() {
            break;
        }
    }
    for world in &judged {
        let hit = |horizon: u16| {
            (world.turn..world.turn + horizon)
                .any(|turn| attacked.contains(&(world.planet, world.owner, turn)))
        };
        for (h, &horizon) in HORIZONS.iter().enumerate() {
            if world.turn + horizon > turns {
                continue;
            }
            let was_hit = hit(horizon);
            for (v, &flag) in world.flags.iter().enumerate() {
                let cell = &mut stats.cells[v][h];
                cell.judged += 1;
                cell.flagged += u64::from(flag);
                cell.flagged_attacked += u64::from(flag && was_hit);
                cell.attacked += u64::from(was_hit);
            }
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
    let (games, years, first) = (
        flag("--games", 256),
        flag("--years", 60) as u16,
        flag("--first-map", 100_000),
    );
    let threads = thread::available_parallelism().map_or(1, |n| n.get());
    let mut stats = Stats::default();
    thread::scope(|scope| {
        let workers: Vec<_> = (0..threads)
            .map(|worker| {
                scope.spawn(move || {
                    let mut stats = Stats::default();
                    for map in (first..first + games).skip(worker).step_by(threads) {
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

    println!(
        "{} games of {years} years: {} world-years, {} attacks ({:.1}% by an empire the owner \
         had not declared an enemy)",
        stats.games,
        stats.cells[0][0].judged,
        stats.attacks,
        100.0 * stats.undeclared_attacks as f64 / stats.attacks.max(1) as f64,
    );
    for (h, horizon) in HORIZONS.iter().enumerate() {
        let base = stats.cells[0][h];
        println!(
            "\nattacked within {horizon} year(s): {:.2}% of world-years",
            100.0 * base.attacked as f64 / base.judged.max(1) as f64
        );
        for (v, variant) in VARIANTS.iter().enumerate() {
            let cell = stats.cells[v][h];
            println!(
                "{:>36}: flagged {:5.1}%, precision {:5.1}%, recall {:5.1}%",
                variant.name,
                100.0 * cell.flagged as f64 / cell.judged.max(1) as f64,
                100.0 * cell.flagged_attacked as f64 / cell.flagged.max(1) as f64,
                100.0 * cell.flagged_attacked as f64 / cell.attacked.max(1) as f64,
            );
        }
    }
}
