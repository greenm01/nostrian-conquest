//! How well the executor's threat rule foresees attacks (t244): four
//! land-grab executors play each map, and every year, before the turns,
//! each world is judged threatened or not from its owner's memory, by the
//! rule (a declared enemy's force reported within 4 sectors in the last 2
//! years) and by looser variants. A world is attacked when a bombardment or
//! an assault (invasion or blitz) targets it while its owner holds it.
//!
//! ```text
//! cargo run --release -p nc-ai --example threat_stats -- [--games 256] [--years 60] [--first-map 100000] \
//!     [--dump FILE.csv]
//! ```
//!
//! For each variant and horizon (attacked this year, or this year or next),
//! it prints the share of world-years flagged, the share of flags followed
//! by an attack (precision), and the share of attacks flagged beforehand
//! (recall). It also prints the share of attacks whose attacker the owner had
//! not declared an enemy when the year began.
//!
//! `--dump` also writes one CSV row per world-year (t247): the map, year,
//! world and owner, `threat_features` below, and whether the world was
//! attacked within one and within two years (-1 where the game ended first).

use std::collections::BTreeSet;
use std::fmt::Write as _;
use std::thread;

use nc_ai::{Contact, SeatMemory, executor_turn, match_game, personality, threatened};
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
    features: [f32; THREAT_FEATURES],
}

/// One game's tallies, and its dump rows when `dump` is set.
fn play(map: u64, years: u16, dump: bool) -> (Stats, String) {
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
            let mut flags = [false; VARIANTS.len()];
            for (flag, v) in flags.iter_mut().zip(&VARIANTS) {
                *flag = flagged(game_data, memory, sector, v);
            }
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
                features: threat_features(game_data, memory, planet),
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
    let mut rows = String::new();
    for world in &judged {
        let hit = |horizon: u16| {
            (world.turn..world.turn + horizon)
                .any(|turn| attacked.contains(&(world.planet, world.owner, turn)))
        };
        if dump {
            let _ = write!(
                rows,
                "{map},{},{},{}",
                world.turn, world.planet, world.owner
            );
            for value in world.features {
                let _ = write!(rows, ",{value}");
            }
            for horizon in HORIZONS {
                let label = if world.turn + horizon > turns {
                    -1
                } else {
                    i32::from(hit(horizon))
                };
                let _ = write!(rows, ",{label}");
            }
            rows.push('\n');
        }
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
    (stats, rows)
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
    let dump = args
        .iter()
        .position(|arg| arg == "--dump")
        .map(|idx| args[idx + 1].clone());
    let threads = thread::available_parallelism().map_or(1, |n| n.get());
    let mut stats = Stats::default();
    let mut rows = format!(
        "map,turn,planet,owner,{},attacked_1,attacked_2\n",
        THREAT_FEATURE_NAMES.join(",")
    );
    thread::scope(|scope| {
        let workers: Vec<_> = (0..threads)
            .map(|worker| {
                let dump = dump.is_some();
                scope.spawn(move || {
                    let (mut stats, mut rows) = (Stats::default(), String::new());
                    for map in (first..first + games).skip(worker).step_by(threads) {
                        let (game, game_rows) = play(map, years, dump);
                        stats.merge(game);
                        rows.push_str(&game_rows);
                    }
                    (stats, rows)
                })
            })
            .collect();
        for worker in workers {
            let (worker_stats, worker_rows) = worker.join().expect("worker should finish");
            stats.merge(worker_stats);
            rows.push_str(&worker_rows);
        }
    });
    if let Some(path) = &dump {
        std::fs::write(path, &rows).expect("the dump file should be writable");
    }

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

/// Search radii (sectors) and report ages (years) of the strength features.
const RADII: [f32; 3] = [2.0, 4.0, 8.0];
const AGES: [u16; 2] = [1, 2];
/// Distance reported when no foreign world is known.
const FAR: f32 = 30.0;

const THREAT_FEATURE_NAMES: [&str; 18] = [
    "batteries",
    "armies",
    "potential",
    "production",
    "starbase",
    "converting",
    "enemy_world_distance",
    "at_war",
    "strength_r2_y1",
    "strength_r2_y2",
    "strength_r4_y1",
    "strength_r4_y2",
    "strength_r8_y1",
    "strength_r8_y2",
    "declared_strength_r4_y2",
    "largest_force_r8_y2",
    "contacts_r4_y2",
    "rule",
];
const THREAT_FEATURES: usize = THREAT_FEATURE_NAMES.len();

/// A contact's attack strength: its ships when a report counted them,
/// otherwise its size classes counted at their strongest hull (small as a
/// destroyer, medium a cruiser, large a battleship), and its starbases.
fn strength(contact: &Contact) -> f32 {
    let ships = match contact.ships {
        Some(ships) => {
            12.0 * ships.destroyers as f32
                + 33.0 * ships.cruisers as f32
                + 90.0 * ships.battleships as f32
        }
        None => {
            12.0 * contact.small as f32 + 33.0 * contact.medium as f32 + 90.0 * contact.large as f32
        }
    };
    ships + 100.0 * contact.starbases as f32
}

fn distance(a: [u8; 2], b: [u8; 2]) -> f32 {
    let dx = f32::from(a[0]) - f32::from(b[0]);
    let dy = f32::from(a[1]) - f32::from(b[1]);
    (dx * dx + dy * dy).sqrt()
}

/// The features of world `planet_idx` (0-based) for its owner, `memory`'s
/// empire, in the order of [`THREAT_FEATURE_NAMES`].
fn threat_features(
    game_data: &CoreGameData,
    memory: &SeatMemory,
    planet_idx: usize,
) -> [f32; THREAT_FEATURES] {
    let empire = memory.empire;
    let year = game_data.conquest.game_year;
    let planet = &game_data.planets[planet_idx];
    let sector = planet.sector.to_array();
    let declared = &game_data.players[usize::from(empire) - 1].diplomacy;
    let enemy =
        |other: u8| declared.get(usize::from(other) - 1) == Some(&DiplomaticRelation::Enemy);
    let foreign: Vec<(&Contact, f32, u16)> = memory
        .contacts
        .iter()
        .filter(|c| c.empire != 0 && c.empire != empire)
        .map(|c| (c, distance(c.coords, sector), year.saturating_sub(c.year)))
        .collect();
    let sum = |radius: f32, age: u16| -> f32 {
        foreign
            .iter()
            .filter(|(_, d, a)| *d <= radius && *a < age)
            .map(|(c, _, _)| strength(c))
            .sum()
    };
    let enemy_world_distance = memory
        .planet_intel
        .iter()
        .filter(|(_, intel)| {
            intel
                .known_owner_empire_id
                .is_some_and(|o| o != 0 && o != empire)
        })
        .filter_map(|(record, _)| game_data.planets.get(record - 1))
        .map(|p| distance(p.sector.to_array(), sector))
        .fold(FAR, f32::min);
    let mut row = [0.0; THREAT_FEATURES];
    row[0] = f32::from(planet.ground_batteries);
    row[1] = f32::from(planet.armies);
    row[2] = f32::from(planet.potential_production);
    row[3] = planet.production.to_f64() as f32;
    row[4] = f32::from(u8::from(
        game_data
            .bases
            .iter()
            .any(|b| b.owner.0 == empire && b.sector.to_array() == sector),
    ));
    row[5] = f32::from(u8::from(planet.years_under_owner < 2));
    row[6] = enemy_world_distance;
    row[7] = f32::from(u8::from(
        (1..=game_data.conquest.player_count).any(|other| other != empire && enemy(other)),
    ));
    for (r, &radius) in RADII.iter().enumerate() {
        for (a, &age) in AGES.iter().enumerate() {
            row[8 + 2 * r + a] = sum(radius, age);
        }
    }
    row[14] = foreign
        .iter()
        .filter(|(c, d, a)| enemy(c.empire) && *d <= 4.0 && *a < 2)
        .map(|(c, _, _)| strength(c))
        .sum();
    row[15] = foreign
        .iter()
        .filter(|(_, d, a)| *d <= 8.0 && *a < 2)
        .map(|(c, _, _)| strength(c))
        .fold(0.0, f32::max);
    row[16] = foreign
        .iter()
        .filter(|(_, d, a)| *d <= 4.0 && *a < 2)
        .count() as f32;
    row[17] = f32::from(u8::from(threatened(game_data, memory, sector)));
    row
}
