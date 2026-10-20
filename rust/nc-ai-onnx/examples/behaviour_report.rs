//! A behaviour report for exported agents: how each plays, not only how
//! often it wins, so a checkpoint that learns something dumb is caught
//! before it replaces the current agent.
//!
//! ```text
//! cargo run --release -p nc-ai-onnx --example behaviour_report -- A.onnx [B.onnx ...] \
//!     [--games 256] [--years 100] [--first-map 100000] [--threads 4] [--style land-grab|FILE.kdl] \
//!     [--opponents TARGET.onnx]
//! ```
//!
//! A model given as `A.onnx+convoy` also orders its convoys (transports
//! without warships), which the executor's ferry orders otherwise (t191).
//! A model given as `A.onnx@STYLE` plays that style (a name or a `.kdl`
//! file) in place of `--style`, so two styles can be paired on the same
//! maps (t207).
//!
//! Each model plays one seat against three tuned executors on the same
//! maps, the seat rotated game by game, greedy. One column per model.
//! `--opponents` fills the other three seats with that model instead (the
//! land-grab style), so an exploiter can be read against its target (t192).

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::thread;

use nc_ai::{
    Families, Weights, commission_submission, executor_turn, match_game, personality_or_file,
};
use nc_ai_onnx::NetworkSeat;
use nc_data::{
    CoreGameData, DiplomaticRelation, Fleet, FleetTurnAction, Order, PlanetTurnAction,
    ProductionItemKind,
};
use nc_engine::production_item_kind_code;

const PLAYERS: u8 = 4;
/// Mission codes 0 to 15, as the player manual numbers them.
const MISSION_CODES: usize = 16;

/// One game's measurements for the network seat.
#[derive(Default, Clone, Copy)]
struct Game {
    win: f32,
    swept_year: Option<u16>,
    early_invasions: u32,
    late_invasions: u32,
    troopless_bombard_years: u32,
    longest_bombard_run: u32,
    parked_fleet_years: u32,
    tax_sum: u32,
    turns: u32,
    declared_war: bool,
    defence_points: u64,
    build_points: u64,
    transports_built: u64,
    peak_worlds: u32,
    final_worlds: u32,
    worlds_lost: u32,
    lost_while_converting: u32,
    armies_at_loss: u32,
    batteries_at_loss: u32,
    /// Fleet orders given, by mission code.
    missions: [u32; MISSION_CODES],
    joins: u32,
    transfers: u32,
    unloads: u32,
    /// Rendezvous orders to a sector with no world: staging in deep space.
    deep_rendezvous: u32,
    /// Worlds under attack in a year, the fleets attacking them, and their
    /// share of the seat's warship strength, summed over those worlds.
    strike_targets: u32,
    strike_fleets: u32,
    strike_share: f32,
    /// The largest fleet's share of the seat's warship strength, summed
    /// over the years it had warships.
    largest_share: f32,
    armed_years: u32,
}

/// A report row: its label and the value for one model's games.
type Row = (&'static str, fn(&[Game]) -> String);

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let arg = |name: &str| {
        args.iter()
            .position(|arg| arg == name)
            .map(|idx| args[idx + 1].clone())
    };
    let models: Vec<(PathBuf, bool, Option<String>)> = args
        .iter()
        .take_while(|arg| !arg.starts_with("--"))
        .map(|arg| {
            let (arg, style) = match arg.split_once('@') {
                Some((model, style)) => (model, Some(style.to_string())),
                None => (arg.as_str(), None),
            };
            match arg.strip_suffix("+convoy") {
                Some(path) => (PathBuf::from(path), true, style),
                None => (PathBuf::from(arg), false, style),
            }
        })
        .collect();
    let games: u64 = arg("--games").map_or(256, |n| n.parse().expect("--games"));
    let years: u16 = arg("--years").map_or(100, |n| n.parse().expect("--years"));
    let first: u64 = arg("--first-map").map_or(100000, |n| n.parse().expect("--first-map"));
    let threads: u64 = arg("--threads").map_or_else(
        || thread::available_parallelism().map_or(1, |n| n.get()) as u64,
        |n| n.parse().expect("--threads"),
    );
    let style = arg("--style").unwrap_or_else(|| "land-grab".to_string());
    let opponents = arg("--opponents").map(PathBuf::from);

    let rows: Vec<Row> = vec![
        ("win share", |g| {
            format!("{:.3}", mean(g, |g| f64::from(g.win)))
        }),
        ("maps swept by it", |g| {
            format!(
                "{:.1}%",
                100.0 * mean(g, |g| f64::from(u8::from(g.swept_year.is_some())))
            )
        }),
        ("mean sweep year", |g| {
            let years: Vec<f64> = g
                .iter()
                .filter_map(|g| g.swept_year.map(f64::from))
                .collect();
            if years.is_empty() {
                "-".to_string()
            } else {
                format!("{:.1}", years.iter().sum::<f64>() / years.len() as f64)
            }
        }),
        ("invasions, years 1-30", |g| {
            format!("{:.2}", mean(g, |g| f64::from(g.early_invasions)))
        }),
        ("invasions, after year 30", |g| {
            format!("{:.2}", mean(g, |g| f64::from(g.late_invasions)))
        }),
        ("fleet-years bombarding, no troops", |g| {
            format!("{:.1}", mean(g, |g| f64::from(g.troopless_bombard_years)))
        }),
        ("longest bombardment of one world", |g| {
            format!("{:.1}", mean(g, |g| f64::from(g.longest_bombard_run)))
        }),
        ("warship fleet-years parked", |g| {
            format!("{:.1}", mean(g, |g| f64::from(g.parked_fleet_years)))
        }),
        ("mean tax", |g| {
            format!(
                "{:.1}%",
                mean(g, |g| f64::from(g.tax_sum) / f64::from(g.turns.max(1)))
            )
        }),
        ("games it declared war", |g| {
            format!(
                "{:.1}%",
                100.0 * mean(g, |g| f64::from(u8::from(g.declared_war)))
            )
        }),
        ("defence share of builds", |g| {
            let build: u64 = g.iter().map(|g| g.build_points).sum();
            let defence: u64 = g.iter().map(|g| g.defence_points).sum();
            format!("{:.0}%", 100.0 * defence as f64 / build.max(1) as f64)
        }),
        ("transports built", |g| {
            format!("{:.1}", mean(g, |g| g.transports_built as f64))
        }),
        ("worlds at peak", |g| {
            format!("{:.1}", mean(g, |g| f64::from(g.peak_worlds)))
        }),
        ("worlds at end", |g| {
            format!("{:.1}", mean(g, |g| f64::from(g.final_worlds)))
        }),
        ("worlds lost", |g| {
            format!("{:.1}", mean(g, |g| f64::from(g.worlds_lost)))
        }),
        ("  of them still converting", |g| {
            format!("{:.1}", mean(g, |g| f64::from(g.lost_while_converting)))
        }),
        ("  mean armies when lost", |g| {
            let lost: u32 = g.iter().map(|g| g.worlds_lost).sum();
            let armies: u32 = g.iter().map(|g| g.armies_at_loss).sum();
            format!("{:.1}", f64::from(armies) / f64::from(lost.max(1)))
        }),
        ("  mean batteries when lost", |g| {
            let lost: u32 = g.iter().map(|g| g.worlds_lost).sum();
            let batteries: u32 = g.iter().map(|g| g.batteries_at_loss).sum();
            format!("{:.1}", f64::from(batteries) / f64::from(lost.max(1)))
        }),
        ("missions used, of 16", |g| {
            let used = |g: &Game| g.missions.iter().filter(|&&n| n > 0).count() as f64;
            format!("{:.1}", mean(g, used))
        }),
        ("rendezvous in deep space", |g| {
            let rendezvous: u32 = g
                .iter()
                .map(|g| g.missions[Order::RendezvousSector.code() as usize])
                .sum();
            let deep: u32 = g.iter().map(|g| g.deep_rendezvous).sum();
            format!(
                "{:.1}%",
                100.0 * f64::from(deep) / f64::from(rendezvous.max(1))
            )
        }),
        ("fleets per attacked world", |g| {
            let targets: u32 = g.iter().map(|g| g.strike_targets).sum();
            let fleets: u32 = g.iter().map(|g| g.strike_fleets).sum();
            format!("{:.2}", f64::from(fleets) / f64::from(targets.max(1)))
        }),
        ("  their share of warship strength", |g| {
            let targets: u32 = g.iter().map(|g| g.strike_targets).sum();
            let share: f32 = g.iter().map(|g| g.strike_share).sum();
            format!("{:.0}%", 100.0 * share / targets.max(1) as f32)
        }),
        ("largest fleet's share of strength", |g| {
            let years: u32 = g.iter().map(|g| g.armed_years).sum();
            let share: f32 = g.iter().map(|g| g.largest_share).sum();
            format!("{:.0}%", 100.0 * share / years.max(1) as f32)
        }),
        ("joins, transfers, unloads", |g| {
            format!(
                "{:.1}/{:.1}/{:.1}",
                mean(g, |g| f64::from(g.joins)),
                mean(g, |g| f64::from(g.transfers)),
                mean(g, |g| f64::from(g.unloads))
            )
        }),
    ];

    let results: Vec<Vec<Game>> = models
        .iter()
        .map(|(model, convoy, own)| {
            run(
                model,
                *convoy,
                opponents.as_ref(),
                own.as_ref().unwrap_or(&style),
                games,
                years,
                first,
                threads,
            )
        })
        .collect();
    let against = opponents
        .as_ref()
        .map_or_else(|| "executors".to_string(), |path| short(path));
    println!("{games} games, {years} years, one network seat ({style}) against three {against}\n");
    print!("{:<36}", "");
    for (model, convoy, own) in &models {
        let mut name = short(model);
        if *convoy {
            name.push_str("+convoy");
        }
        if let Some(own) = own {
            name = format!("{name}@{}", short(&PathBuf::from(own)));
        }
        print!("{:>16}", name);
    }
    println!();
    for (label, value) in rows {
        print!("{label:<36}");
        for games in &results {
            print!("{:>16}", value(games));
        }
        println!();
    }
    if results.len() > 1 {
        println!(
            "\npaired difference from {}, same maps and seats (± standard error)",
            short(&models[0].0)
        );
        let measures: [(&str, fn(&Game) -> f64); 3] = [
            ("win share", |g| f64::from(g.win)),
            ("worlds at end", |g| f64::from(g.final_worlds)),
            ("worlds lost", |g| f64::from(g.worlds_lost)),
        ];
        for (label, value) in measures {
            print!("{label:<36}{:>16}", "");
            for games in &results[1..] {
                let (diff, error) = paired(&results[0], games, value);
                print!("{:>16}", format!("{diff:+.3} ± {error:.3}"));
            }
            println!();
        }
    }
    println!("\nfleet orders per game, by mission");
    for code in 0..MISSION_CODES {
        print!(
            "{:<36}",
            format!("  {code:02} {:?}", Order::from_code(code as u8))
        );
        for games in &results {
            print!("{:>16.1}", mean(games, |g| f64::from(g.missions[code])));
        }
        println!();
    }
}

fn short(model: &std::path::Path) -> String {
    let name = model
        .file_stem()
        .map(|stem| stem.to_string_lossy().to_string())
        .unwrap_or_default();
    name.chars()
        .rev()
        .take(15)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect()
}

/// The mean of `value` over `other`'s games less `base`'s on the same maps,
/// with its standard error. Both lists hold the games in the same map order.
fn paired(base: &[Game], other: &[Game], value: impl Fn(&Game) -> f64) -> (f64, f64) {
    let diffs: Vec<f64> = base
        .iter()
        .zip(other)
        .map(|(base, other)| value(other) - value(base))
        .collect();
    let n = diffs.len().max(1) as f64;
    let mean = diffs.iter().sum::<f64>() / n;
    let variance = diffs.iter().map(|d| (d - mean).powi(2)).sum::<f64>() / (n - 1.0).max(1.0);
    (mean, (variance / n).sqrt())
}

fn mean(games: &[Game], value: impl Fn(&Game) -> f64) -> f64 {
    games.iter().map(value).sum::<f64>() / games.len().max(1) as f64
}

fn run(
    model: &PathBuf,
    convoy: bool,
    opponents: Option<&PathBuf>,
    style: &str,
    games: u64,
    years: u16,
    first: u64,
    threads: u64,
) -> Vec<Game> {
    let style = personality_or_file(style).unwrap_or_else(|err| panic!("{err}"));
    thread::scope(|scope| {
        let workers: Vec<_> = (0..threads)
            .map(|worker| {
                let style = style.clone();
                scope.spawn(move || {
                    let seat = NetworkSeat::load(
                        model,
                        Families {
                            strike: true,
                            convoy,
                            ..Families::NONE
                        },
                        style,
                    )
                    .expect("the model loads");
                    let opponent = opponents.map(|path| {
                        NetworkSeat::load(
                            path,
                            Families {
                                strike: true,
                                ..Families::NONE
                            },
                            personality_or_file("land-grab").expect("land-grab is a style"),
                        )
                        .expect("the opponent model loads")
                    });
                    (first..first + games)
                        .skip(worker as usize)
                        .step_by(threads as usize)
                        .map(|map| play(&seat, opponent.as_ref(), map, years))
                        .collect::<Vec<_>>()
                })
            })
            .collect();
        workers
            .into_iter()
            .flat_map(|worker| worker.join().expect("worker should finish"))
            .collect()
    })
}

fn play(network: &NetworkSeat, opponent: Option<&NetworkSeat>, map: u64, years: u16) -> Game {
    let mut game = match_game(map, map, &[false; PLAYERS as usize], years);
    let start = game.play.built.game_data.conquest.game_year;
    let seat = (map % u64::from(PLAYERS)) as u8 + 1;
    let mut out = Game::default();
    let mut runs: BTreeMap<usize, u32> = BTreeMap::new();
    let defence = [
        ProductionItemKind::Army,
        ProductionItemKind::GroundBattery,
        ProductionItemKind::Starbase,
    ]
    .map(production_item_kind_code);
    let transport = production_item_kind_code(ProductionItemKind::Transport);
    for _ in 0..years {
        // Network seats commission at the start of the year and the
        // executors at their own turns, as the batch environment has them.
        for empire in 1..=PLAYERS {
            if empire == seat || opponent.is_some() {
                if let Some(commission) = commission_submission(&game.play.built.game_data, empire)
                {
                    game.submit_turn(&commission)
                        .expect("commissioning submits");
                }
            }
        }
        for empire in 1..=PLAYERS {
            if empire != seat {
                match opponent {
                    Some(opponent) => {
                        let turn = opponent
                            .turn(
                                &game.play.built.game_data,
                                &game.seats[usize::from(empire) - 1],
                                0.0,
                                0,
                            )
                            .expect("the opponent network runs");
                        game.submit_turn(&turn)
                            .expect("the opponent's turn submits");
                    }
                    None => executor_turn(&mut game, empire, &Weights::TUNED, start),
                }
                continue;
            }
            let game_data = &game.play.built.game_data;
            let turn = network
                .turn(game_data, &game.seats[usize::from(seat) - 1], 0.0, 0)
                .expect("the network runs");
            out.turns += 1;
            out.tax_sum += u32::from(
                turn.tax_rate
                    .unwrap_or(game_data.players[usize::from(seat) - 1].tax_rate),
            );
            out.declared_war |= turn
                .diplomacy
                .iter()
                .any(|directive| directive.relation == DiplomaticRelation::Enemy);
            for action in turn.planets.iter().flat_map(|planet| &planet.actions) {
                if let PlanetTurnAction::Build {
                    points_remaining,
                    kind,
                } = action
                {
                    out.build_points += u64::from(*points_remaining);
                    if defence.contains(kind) {
                        out.defence_points += u64::from(*points_remaining);
                    }
                    if *kind == transport {
                        out.transports_built += u64::from(*points_remaining)
                            / u64::from(
                                ProductionItemKind::Transport
                                    .build_cost()
                                    .unwrap_or(1)
                                    .max(1),
                            );
                    }
                }
            }
            for action in turn.fleets.iter().flat_map(|block| &block.actions) {
                match action {
                    FleetTurnAction::Order {
                        order_code, target, ..
                    } => {
                        if let Some(n) = out.missions.get_mut(usize::from(*order_code)) {
                            *n += 1;
                        }
                        let deep = target.is_some_and(|sector| {
                            !game_data
                                .planets
                                .iter()
                                .any(|planet| planet.sector.to_array() == sector)
                        });
                        if Order::from_code(*order_code) == Order::RendezvousSector && deep {
                            out.deep_rendezvous += 1;
                        }
                    }
                    FleetTurnAction::Join { .. } => out.joins += 1,
                    FleetTurnAction::Transfer { .. } => out.transfers += 1,
                    FleetTurnAction::UnloadArmies { .. } => out.unloads += 1,
                    _ => {}
                }
            }
            game.submit_turn(&turn).expect("the network's turn submits");
        }
        let game_data = &game.play.built.game_data;
        let own: Vec<&Fleet> = game_data
            .fleets
            .iter()
            .filter(|fleet| fleet.owner.0 == seat)
            .collect();
        let total: u32 = own.iter().map(|fleet| strength(fleet)).sum();
        if total > 0 {
            let largest = own.iter().map(|fleet| strength(fleet)).max().unwrap_or(0);
            out.largest_share += largest as f32 / total as f32;
            out.armed_years += 1;
            let mut attacks: BTreeMap<[u8; 2], (u32, u32)> = BTreeMap::new();
            for fleet in own.iter().filter(|fleet| {
                matches!(
                    fleet.order,
                    Order::BombardWorld | Order::InvadeWorld | Order::BlitzWorld
                )
            }) {
                let attack = attacks.entry(fleet.target.to_array()).or_default();
                attack.0 += 1;
                attack.1 += strength(fleet);
            }
            for (fleets, attack_strength) in attacks.into_values() {
                out.strike_targets += 1;
                out.strike_fleets += fleets;
                out.strike_share += attack_strength as f32 / total as f32;
            }
        }
        for fleet in game_data
            .fleets
            .iter()
            .filter(|fleet| fleet.owner.0 == seat)
        {
            let warships = nc_data::has_combat_ships(fleet);
            if fleet.order == Order::BombardWorld && fleet.ships.loaded_transports == 0 {
                out.troopless_bombard_years += 1;
            }
            if warships
                && matches!(fleet.order, Order::HoldPosition | Order::RendezvousSector)
                && fleet.sector == fleet.target
            {
                out.parked_fleet_years += 1;
            }
        }
        // Each of the seat's worlds going into the year: its ownership
        // status (under 2 while a captured world converts, earning little
        // or nothing), and its garrison.
        let before: Vec<Option<(u8, u16, u16)>> = game_data
            .planets
            .iter()
            .map(|planet| {
                (planet.owner.0 == seat).then_some((
                    planet.years_under_owner,
                    planet.armies,
                    planet.ground_batteries,
                ))
            })
            .collect();
        let events = game.run_maintenance().expect("maintenance runs");
        let game_data = &game.play.built.game_data;
        let year = game_data.conquest.game_year - start;
        for (planet, held) in game_data.planets.iter().zip(&before) {
            if let Some((years, armies, batteries)) = held
                && planet.owner.0 != seat
            {
                out.worlds_lost += 1;
                out.lost_while_converting += u32::from(*years < 2);
                out.armies_at_loss += u32::from(*armies);
                out.batteries_at_loss += u32::from(*batteries);
            }
        }
        let invasions = events
            .assault_report_events
            .iter()
            .filter(|e| e.attacker_empire == seat)
            .count() as u32;
        if year <= 30 {
            out.early_invasions += invasions;
        } else {
            out.late_invasions += invasions;
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
            out.longest_bombard_run = out.longest_bombard_run.max(*run);
        }
        let worlds = worlds_of(game_data, seat);
        out.peak_worlds = out.peak_worlds.max(worlds);
        out.final_worlds = worlds;
        if let Some(winner) = game.play.winner_state.winner_empire {
            if winner == seat {
                out.swept_year = Some(year);
            }
            break;
        }
    }
    out.win = win_share(
        &game.play.built.game_data,
        game.play.winner_state.winner_empire,
        seat,
    );
    out
}

/// Warship strength: destroyers 1, cruisers 3, battleships 9.
fn strength(fleet: &Fleet) -> u32 {
    u32::from(fleet.ships.destroyers)
        + 3 * u32::from(fleet.ships.cruisers)
        + 9 * u32::from(fleet.ships.battleships)
}

fn worlds_of(game_data: &CoreGameData, empire: u8) -> u32 {
    game_data
        .planets
        .iter()
        .filter(|planet| planet.owner.0 == empire)
        .count() as u32
}

fn win_share(game_data: &CoreGameData, winner: Option<u8>, empire: u8) -> f32 {
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
