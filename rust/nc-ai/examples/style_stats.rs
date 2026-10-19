//! How each personality plays (t167): one seat of the style against three
//! default seats, rotated game by game, measured on its build mix, war
//! timing and expansion.
//!
//! ```text
//! cargo run --release -p nc-ai --example style_stats -- [--games 1024] [--years 60] [--style FILE.kdl]
//! ```
//!
//! For each style it prints the win share, the share of build points spent
//! on each kind, the mean year of its first declaration of war and of its
//! first attack, its mean worlds at years 10, 20 and 40, and how much it
//! ferries armies: fleet-years of transports without warships, and armies
//! unloaded at its own worlds.

use std::thread;

use nc_ai::{
    PERSONALITIES, Weights, commission_submission, decode_action, executor_turn, match_game,
    personality, personality_or_file, plan_turn, seat_masks,
};
use nc_data::{DiplomaticRelation, FleetTurnAction, PlanetTurnAction};
use nc_engine::BUILD_UNITS;

const PLAYERS: u8 = 4;
const CHECK_YEARS: [u16; 3] = [10, 20, 40];

#[derive(Default, Clone)]
struct Stats {
    games: u32,
    wins: f32,
    /// Build points by [`BUILD_UNITS`] row.
    points: Vec<u64>,
    war_years: Vec<u16>,
    attack_years: Vec<u16>,
    worlds: [u32; CHECK_YEARS.len()],
    /// Destroyer-years in all, and in a fleet with a cruiser or battleship
    /// to cover them (t206).
    destroyers: u64,
    covered: u64,
    /// Fleet-years of transports sailing without warships, and armies
    /// unloaded at the seat's own worlds (t191's ferry).
    convoys: u64,
    unloaded: u64,
}

impl Stats {
    fn merge(&mut self, other: Stats) {
        self.games += other.games;
        self.wins += other.wins;
        self.points.resize(BUILD_UNITS.len(), 0);
        for (sum, points) in self.points.iter_mut().zip(other.points) {
            *sum += points;
        }
        self.war_years.extend(other.war_years);
        self.attack_years.extend(other.attack_years);
        for (sum, worlds) in self.worlds.iter_mut().zip(other.worlds) {
            *sum += worlds;
        }
        self.destroyers += other.destroyers;
        self.covered += other.covered;
        self.convoys += other.convoys;
        self.unloaded += other.unloaded;
    }
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let arg = |name: &str, default: u64| {
        args.iter()
            .position(|arg| arg == name)
            .map_or(default, |idx| args[idx + 1].parse().expect(name))
    };
    let games = arg("--games", 1024);
    let years = arg("--years", 60) as u16;
    let default = personality("land-grab").expect("built in").weights;
    let file = args
        .iter()
        .position(|arg| arg == "--style")
        .map(|idx| args[idx + 1].clone());
    let names: Vec<String> = match &file {
        Some(file) => vec![file.clone()],
        None => PERSONALITIES.iter().map(|name| name.to_string()).collect(),
    };
    for name in &names {
        let style = personality_or_file(name)
            .unwrap_or_else(|err| panic!("{err}"))
            .weights;
        let stats = run(style, default, games, years);
        let total: u64 = stats.points.iter().sum::<u64>().max(1);
        let mix: Vec<String> = BUILD_UNITS
            .iter()
            .zip(&stats.points)
            .filter(|(_, points)| **points * 100 >= total)
            .map(|(unit, points)| {
                format!(
                    "{:?} {:.0}%",
                    unit.kind,
                    100.0 * *points as f64 / total as f64
                )
            })
            .collect();
        let mean = |years: &[u16]| {
            if years.is_empty() {
                "never".to_string()
            } else {
                format!(
                    "{:.1} ({} of {} games)",
                    years.iter().map(|&y| f64::from(y)).sum::<f64>() / years.len() as f64,
                    years.len(),
                    stats.games
                )
            }
        };
        let n = f64::from(stats.games);
        println!(
            "{name}: win share {:.3}; builds {}; first war year {}; first attack year {}; \
             worlds at years 10/20/40 {:.1}/{:.1}/{:.1}; destroyers with a heavy {:.0}% \
             of {:.1} destroyer-years a game; {:.1} convoy fleet-years and {:.1} armies \
             unloaded a game",
            stats.wins / stats.games as f32,
            mix.join(", "),
            mean(&stats.war_years),
            mean(&stats.attack_years),
            f64::from(stats.worlds[0]) / n,
            f64::from(stats.worlds[1]) / n,
            f64::from(stats.worlds[2]) / n,
            100.0 * stats.covered as f64 / stats.destroyers.max(1) as f64,
            stats.destroyers as f64 / n,
            stats.convoys as f64 / n,
            stats.unloaded as f64 / n,
        );
    }
}

fn run(style: Weights, default: Weights, games: u64, years: u16) -> Stats {
    let threads = thread::available_parallelism().map_or(1, |n| n.get()) as u64;
    let mut stats = Stats::default();
    thread::scope(|scope| {
        let workers: Vec<_> = (0..threads)
            .map(|worker| {
                scope.spawn(move || {
                    let mut stats = Stats::default();
                    for map in (100_000..100_000 + games)
                        .skip(worker as usize)
                        .step_by(threads as usize)
                    {
                        stats.merge(play(style, default, map, years));
                    }
                    stats
                })
            })
            .collect();
        for worker in workers {
            stats.merge(worker.join().expect("worker should finish"));
        }
    });
    stats
}

fn play(style: Weights, default: Weights, map: u64, years: u16) -> Stats {
    let mut game = match_game(map, map, &[false; PLAYERS as usize], years);
    let start = game.play.built.game_data.conquest.game_year;
    let seat = (map % u64::from(PLAYERS)) as u8 + 1;
    let mut stats = Stats {
        games: 1,
        points: vec![0; BUILD_UNITS.len()],
        ..Stats::default()
    };
    let (mut war, mut attack) = (None, None);
    for _ in 0..years {
        for empire in 1..=PLAYERS {
            if empire != seat {
                executor_turn(&mut game, empire, &default, start);
                continue;
            }
            if let Some(commission) = commission_submission(&game.play.built.game_data, empire) {
                game.submit_turn(&commission)
                    .expect("commissioning submits");
            }
            let game_data = &game.play.built.game_data;
            let masks = seat_masks(game_data, empire);
            let memory = &game.seats[usize::from(empire) - 1];
            let action = plan_turn(game_data, memory, &masks, &style, start);
            let turn = decode_action(game_data, empire, &action).submission;
            for planet in &turn.planets {
                for action in &planet.actions {
                    if let PlanetTurnAction::Build {
                        points_remaining,
                        kind,
                    } = action
                        && let Some(row) = BUILD_UNITS.iter().position(|unit| {
                            nc_engine::production_item_kind_code(unit.kind) == *kind
                        })
                    {
                        stats.points[row] += u64::from(*points_remaining);
                    }
                }
            }
            for block in &turn.fleets {
                for action in &block.actions {
                    if let FleetTurnAction::UnloadArmies { qty, .. } = action {
                        stats.unloaded += u64::from(*qty);
                    }
                }
            }
            if war.is_none()
                && turn
                    .diplomacy
                    .iter()
                    .any(|directive| directive.relation == DiplomaticRelation::Enemy)
            {
                war = Some(game_data.conquest.game_year - start);
            }
            game.submit_turn(&turn).expect("the style's turn submits");
        }
        let events = game.run_maintenance().expect("maintenance runs");
        let year = game.play.built.game_data.conquest.game_year - start;
        for fleet in &game.play.built.game_data.fleets {
            let ships = &fleet.ships;
            if fleet.owner.0 == seat
                && ships.empty_transports + ships.loaded_transports > 0
                && ships.battleships + ships.cruisers + ships.destroyers == 0
            {
                stats.convoys += 1;
            }
            if fleet.owner.0 == seat && fleet.ships.destroyers > 0 {
                let destroyers = u64::from(fleet.ships.destroyers);
                stats.destroyers += destroyers;
                if fleet.ships.cruisers + fleet.ships.battleships > 0 {
                    stats.covered += destroyers;
                }
            }
        }
        if attack.is_none()
            && (events
                .bombard_events
                .iter()
                .any(|e| e.attacker_empire == seat)
                || events
                    .assault_report_events
                    .iter()
                    .any(|e| e.attacker_empire == seat))
        {
            attack = Some(year);
        }
        if let Some(idx) = CHECK_YEARS.iter().position(|&check| check == year) {
            stats.worlds[idx] = game
                .play
                .built
                .game_data
                .planets
                .iter()
                .filter(|planet| planet.owner.0 == seat)
                .count() as u32;
        }
        if game.play.winner_state.winner_empire.is_some() {
            break;
        }
    }
    stats.war_years.extend(war);
    stats.attack_years.extend(attack);
    stats.wins = win_share(
        &game.play.built.game_data,
        game.play.winner_state.winner_empire,
        seat,
    );
    stats
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
