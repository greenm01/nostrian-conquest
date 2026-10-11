//! Engine-side combat odds (t055): fixed two-fleet battles run many times
//! with fresh campaign seeds, so the Python model in `tools/combat-model`
//! can be checked against what the engine does under the current rules.
//!
//! Run in release:
//! `cargo run --release -p nc-fuzz --example combat_odds -- --trials 4000 --json /tmp/engine.json`
//!
//! Options: `--trials N` (default 2000), `--seed S` (first campaign seed,
//! default 1), `--years Y` (maintenance years to wait for a battle, default
//! 6), `--case NAME` (repeatable; default every case), `--json PATH`.
//!
//! Every trial builds the same two-player game (map seed fixed), places one
//! fleet of each empire in an empty sector, at rest, at war, on Hold, with
//! both seats off autopilot (it would recall idle fleets), and runs
//! maintenance years with campaign seed `S + trial` until the engine
//! reports a battle in that sector. The campaign seed keys the sighting and
//! combat rolls, so each trial samples new luck; `ScenarioPlay::run_maintenance`
//! would pass seed 0 every time, which is why the engine entry point is
//! called directly. Rounds fought and crippled survivors are not in the
//! maintenance events, so only outcomes and destroyed build points are
//! reported; `sweep.py --engine` compares them with the model.

use std::fmt::Write as _;
use std::thread;

use nc_data::{
    DiplomaticRelation, EncounterDispositionEvent, MaintenanceEvents, Order, ShipLosses,
    WinnerState, default_player_lifecycle_states, map_size_for_player_count,
};
use nc_engine::run_maintenance_turn_with_context_seed_and_lifecycle;
use nc_harness::{
    BuiltScenario, DiplomacySpec, FleetOrderSpec, FleetShipsSpec, FleetSpec, HouseSpec,
    ScenarioBaseline, ScenarioMetadata, ScenarioSpec, build_scenario,
};

const MAP_SEED: u64 = 1515;
const START_YEAR: u16 = 3000;
const COST_DD: u32 = 5;
const COST_CA: u32 = 15;
const COST_BB: u32 = 45;

#[derive(Clone, Copy, Default)]
struct Force {
    dd: u16,
    ca: u16,
    bb: u16,
}

struct Case {
    name: &'static str,
    left: Force,
    right: Force,
    roe: (u8, u8),
}

/// The cases `tools/combat-model/sweep.py` tags `engine`, under the same names.
fn cases() -> Vec<Case> {
    let ca = |n| Force {
        ca: n,
        ..Force::default()
    };
    let dd = |n| Force {
        dd: n,
        ..Force::default()
    };
    let bb = |n| Force {
        bb: n,
        ..Force::default()
    };
    let case = |name, left, right| Case {
        name,
        left,
        right,
        roe: (10, 10),
    };
    vec![
        case("10 CA vs 10 CA", ca(10), ca(10)),
        case("11 CA vs 10 CA", ca(11), ca(10)),
        case("15 CA vs 10 CA", ca(15), ca(10)),
        case("20 CA vs 10 CA", ca(20), ca(10)),
        case("30 DD vs 10 CA", dd(30), ca(10)),
        case("90 DD vs 10 BB", dd(90), bb(10)),
        case("30 CA vs 10 BB", ca(30), bb(10)),
        case(
            "29 CA + 3 DD vs 30 CA",
            Force {
                ca: 29,
                dd: 3,
                bb: 0,
            },
            ca(30),
        ),
        Case {
            name: "10 CA ROE 6 vs 15 CA",
            left: ca(10),
            right: ca(15),
            roe: (6, 10),
        },
    ]
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Outcome {
    Left,
    Right,
    Mutual,
    Stalemate,
}

#[derive(Default)]
struct Tally {
    trials: u32,
    battles: u32,
    years: u32,
    left: u32,
    right: u32,
    mutual: u32,
    stalemate: u32,
    loss_left: u64,
    loss_right: u64,
}

impl Tally {
    fn add(&mut self, other: &Tally) {
        self.trials += other.trials;
        self.battles += other.battles;
        self.years += other.years;
        self.left += other.left;
        self.right += other.right;
        self.mutual += other.mutual;
        self.stalemate += other.stalemate;
        self.loss_left += other.loss_left;
        self.loss_right += other.loss_right;
    }
}

fn ships(force: Force) -> FleetShipsSpec {
    FleetShipsSpec {
        destroyers: force.dd,
        cruisers: force.ca,
        battleships: force.bb,
        ..FleetShipsSpec::default()
    }
}

/// The first fleet record of each empire and an empty sector, read from a
/// bare build of the game every case starts from.
fn placement() -> ([usize; 2], [u8; 2]) {
    let built = build_scenario(&bare_scenario()).expect("bare scenario should build");
    let data = &built.game_data;
    let first_fleet = |empire: u8| {
        data.fleets
            .iter()
            .position(|fleet| fleet.owner.0 == empire)
            .map(|idx| idx + 1)
            .expect("empire should own a fleet record")
    };
    let map = map_size_for_player_count(2);
    let occupied = |coords: [u8; 2]| {
        data.planets.iter().any(|p| p.sector.to_array() == coords)
            || data.bases.iter().any(|b| b.sector.to_array() == coords)
    };
    let mut empty = None;
    'search: for x in 1..=map {
        for y in 1..=map {
            if !occupied([x, y]) {
                empty = Some([x, y]);
                break 'search;
            }
        }
    }
    (
        [first_fleet(1), first_fleet(2)],
        empty.expect("map should have an empty sector"),
    )
}

fn bare_scenario() -> ScenarioSpec {
    // Both seats are human and off autopilot, or the autopilot recalls an
    // idle fleet in deep space to its nearest world before the year starts.
    let house = |record| HouseSpec {
        record_index_1_based: record,
        handle: None,
        empire_name: None,
        homeworld_name: None,
        tax_rate: None,
        autopilot: false,
    };
    ScenarioSpec {
        metadata: ScenarioMetadata {
            label: Some("combat-odds".to_string()),
            player_count: 2,
            year: START_YEAR,
            seed: MAP_SEED,
            baseline: ScenarioBaseline::BuilderCompatible,
        },
        houses: vec![house(1), house(2)],
        diplomacy: vec![
            DiplomacySpec {
                from_empire: 1,
                to_empire: 2,
                relation: DiplomaticRelation::Enemy,
            },
            DiplomacySpec {
                from_empire: 2,
                to_empire: 1,
                relation: DiplomaticRelation::Enemy,
            },
        ],
        planets: Vec::new(),
        fleets: Vec::new(),
        turn_files: Vec::new(),
        queued_mail: Vec::new(),
        results_blocks: Vec::new(),
        message_blocks: Vec::new(),
    }
}

fn build_case(case: &Case, records: [usize; 2], coords: [u8; 2]) -> BuiltScenario {
    let fleet = |record, force, roe| FleetSpec {
        record_index_1_based: record,
        coords: Some(coords),
        ships: Some(ships(force)),
        rules_of_engagement: Some(roe),
        order: Some(FleetOrderSpec {
            kind: Order::HoldPosition,
            speed: 0,
            target: coords,
            aux0: None,
            aux1: None,
        }),
        ..FleetSpec::default()
    };
    let mut spec = bare_scenario();
    spec.fleets = vec![
        fleet(records[0], case.left, case.roe.0),
        fleet(records[1], case.right, case.roe.1),
    ];
    build_scenario(&spec).expect("case scenario should build")
}

fn cost(losses: &ShipLosses) -> u64 {
    u64::from(
        losses.destroyers * COST_DD + losses.cruisers * COST_CA + losses.battleships * COST_BB,
    )
}

fn line_wiped(losses: &ShipLosses, initial: &ShipLosses) -> bool {
    losses.destroyers == initial.destroyers
        && losses.cruisers == initial.cruisers
        && losses.battleships == initial.battleships
}

/// The first battle of the year at `coords`, as (outcome, left loss, right loss).
fn first_battle(events: &MaintenanceEvents, coords: [u8; 2]) -> Option<(Outcome, u64, u64)> {
    let report = |empire: u8| {
        events
            .fleet_battle_events
            .iter()
            .find(|event| event.coords == coords && event.reporting_empire == empire)
    };
    let (left, right) = (report(1)?, report(2)?);
    let withdrew = |empire: u8| {
        events.encounter_disposition_events.iter().any(|event| {
            matches!(
                event,
                EncounterDispositionEvent::Retreated { owner_empire, coords: at, .. }
                    if *owner_empire == empire && *at == coords
            )
        })
    };
    let outcome = match (
        line_wiped(&left.friendly_losses, &left.friendly_initial),
        line_wiped(&right.friendly_losses, &right.friendly_initial),
    ) {
        (true, true) => Outcome::Mutual,
        (false, true) => Outcome::Left,
        (true, false) => Outcome::Right,
        (false, false) => match (withdrew(1), withdrew(2)) {
            (false, true) => Outcome::Left,
            (true, false) => Outcome::Right,
            _ => Outcome::Stalemate,
        },
    };
    Some((
        outcome,
        cost(&left.friendly_losses),
        cost(&right.friendly_losses),
    ))
}

fn trial(base: &BuiltScenario, coords: [u8; 2], seed: u64, years: u16, tally: &mut Tally) {
    let mut data = base.game_data.clone();
    let mut lifecycle = default_player_lifecycle_states(data.conquest.player_count);
    let mut winner = WinnerState::default();
    tally.trials += 1;
    for year in 1..=years {
        let events = run_maintenance_turn_with_context_seed_and_lifecycle(
            &mut data,
            seed,
            &[],
            &[],
            &mut lifecycle,
            &mut winner,
        )
        .expect("maintenance should run");
        if let Some((outcome, loss_left, loss_right)) = first_battle(&events, coords) {
            tally.battles += 1;
            tally.years += u32::from(year);
            tally.loss_left += loss_left;
            tally.loss_right += loss_right;
            match outcome {
                Outcome::Left => tally.left += 1,
                Outcome::Right => tally.right += 1,
                Outcome::Mutual => tally.mutual += 1,
                Outcome::Stalemate => tally.stalemate += 1,
            }
            return;
        }
    }
}

fn run_case(base: &BuiltScenario, coords: [u8; 2], trials: u32, seed: u64, years: u16) -> Tally {
    let threads = thread::available_parallelism().map_or(1, |n| n.get() as u32);
    let mut tally = Tally::default();
    thread::scope(|scope| {
        let workers: Vec<_> = (0..threads)
            .map(|thread| {
                scope.spawn(move || {
                    let mut tally = Tally::default();
                    for index in (thread..trials).step_by(threads as usize) {
                        trial(
                            base,
                            coords,
                            seed.wrapping_add(u64::from(index)),
                            years,
                            &mut tally,
                        );
                    }
                    tally
                })
            })
            .collect();
        for worker in workers {
            tally.add(&worker.join().expect("worker should finish"));
        }
    });
    tally
}

/// Wilson 95% score interval.
fn interval(wins: u32, trials: u32) -> (f64, f64) {
    if trials == 0 {
        return (0.0, 1.0);
    }
    let (z, n, p) = (
        1.959_964f64,
        f64::from(trials),
        f64::from(wins) / f64::from(trials),
    );
    let centre = p + z * z / (2.0 * n);
    let spread = z * ((p * (1.0 - p) + z * z / (4.0 * n)) / n).sqrt();
    let denominator = 1.0 + z * z / n;
    (
        ((centre - spread) / denominator).max(0.0),
        ((centre + spread) / denominator).min(1.0),
    )
}

fn pct(count: u32, total: u32) -> String {
    if total == 0 {
        return "-".to_string();
    }
    format!("{:.1}%", 100.0 * f64::from(count) / f64::from(total))
}

fn main() {
    let mut trials = 2000u32;
    let mut seed = 1u64;
    let mut years = 6u16;
    let mut selected = Vec::new();
    let mut json_path = None;
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        let mut value = || args.next().unwrap_or_else(|| panic!("{arg} needs a value"));
        match arg.as_str() {
            "--trials" => trials = value().parse().expect("--trials takes a number"),
            "--seed" => seed = value().parse().expect("--seed takes a number"),
            "--years" => years = value().parse().expect("--years takes a number"),
            "--case" => selected.push(value()),
            "--json" => json_path = Some(value()),
            other => panic!("unknown option {other}"),
        }
    }

    let (records, coords) = placement();
    println!(
        "Two-player game, map seed {MAP_SEED}, fleets {} and {} at rest in empty sector ({}, {}), at war, on Hold; \
         campaign seeds {seed}.. ; up to {years} years per trial.",
        records[0], records[1], coords[0], coords[1]
    );
    println!();
    println!(
        "| case | trials | battles | years/battle | left win [95%] | right win | mutual | stalemate | left lost | right lost |"
    );
    println!("| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |");
    let mut json = String::from("{\n");
    let _ = writeln!(
        json,
        "  \"map_seed\": {MAP_SEED}, \"seed\": {seed}, \"trials\": {trials}, \"years\": {years},"
    );
    json.push_str("  \"cases\": [\n");
    let mut first = true;
    for case in cases() {
        if !selected.is_empty() && !selected.iter().any(|name| name == case.name) {
            continue;
        }
        let base = build_case(&case, records, coords);
        let tally = run_case(&base, coords, trials, seed, years);
        let (low, high) = interval(tally.left, tally.battles);
        let per_battle = |total: u64| {
            if tally.battles == 0 {
                0.0
            } else {
                total as f64 / f64::from(tally.battles)
            }
        };
        println!(
            "| {} | {} | {} | {:.2} | {} [{:.1}, {:.1}] | {} | {} | {} | {:.1} | {:.1} |",
            case.name,
            tally.trials,
            tally.battles,
            per_battle(u64::from(tally.years)),
            pct(tally.left, tally.battles),
            100.0 * low,
            100.0 * high,
            pct(tally.right, tally.battles),
            pct(tally.mutual, tally.battles),
            pct(tally.stalemate, tally.battles),
            per_battle(tally.loss_left),
            per_battle(tally.loss_right),
        );
        if !first {
            json.push_str(",\n");
        }
        first = false;
        let _ = write!(
            json,
            "    {{\"case\": \"{}\", \"trials\": {}, \"battles\": {}, \"years_per_battle\": {:.4}, \
             \"counts\": {{\"left\": {}, \"right\": {}, \"mutual\": {}, \"stalemate\": {}}}, \
             \"loss_left\": {:.4}, \"loss_right\": {:.4}}}",
            case.name,
            tally.trials,
            tally.battles,
            per_battle(u64::from(tally.years)),
            tally.left,
            tally.right,
            tally.mutual,
            tally.stalemate,
            per_battle(tally.loss_left),
            per_battle(tally.loss_right),
        );
    }
    json.push_str("\n  ]\n}\n");
    if let Some(path) = json_path {
        std::fs::write(&path, json).expect("json file should be written");
        println!();
        println!("Wrote {path}");
    }
}
