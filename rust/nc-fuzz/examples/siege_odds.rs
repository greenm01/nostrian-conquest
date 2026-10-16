//! Engine-side siege odds (t202): battleships bombard a world's ground
//! batteries year after year through real maintenance, so
//! `tools/combat-model/siege_review.py` can be checked against the engine.
//!
//! Run in release:
//! `cargo run --release -p nc-fuzz --example siege_odds -- --trials 400 --json /tmp/siege.json`
//!
//! Options: `--trials N` (default 400), `--seed S` (first campaign seed,
//! default 1), `--json PATH`.
//!
//! Every trial builds the same two-player game, empties every fleet, and
//! gives empire 2's homeworld the case's batteries and no dock. Empire 1's
//! battleships, split between two fleets so they bombard together, start at
//! rest in its orbit under Bombard with ROE 10. Each later year adds the
//! case's reinforcement to the first fleet, as if it had just arrived. A
//! trial ends when the batteries are gone (cleared), the battleships are
//! gone, or after 15 years.

use std::fmt::Write as _;
use std::thread;

use nc_data::{
    CoreGameData, DiplomaticRelation, Order, WinnerState, default_player_lifecycle_states,
    set_fleet_at_rest,
};
use nc_engine::run_maintenance_turn_with_context_seed_and_lifecycle;
use nc_harness::{
    DiplomacySpec, HouseSpec, ScenarioBaseline, ScenarioMetadata, ScenarioSpec, build_scenario,
};

const MAP_SEED: u64 = 1515;
const COST_BB: f64 = 45.0;
const COST_BATTERY: f64 = 20.0;
const MAX_YEARS: u16 = 15;

#[derive(Clone, Copy)]
struct Case {
    batteries: u16,
    /// Battleship points over battery points at the start.
    force: f64,
    /// Battleship points over battery points added each later year.
    reinforce: f64,
}

/// The cases of `tools/combat-model/siege_review.py`.
fn cases() -> Vec<Case> {
    let mut cases = Vec::new();
    for batteries in [2, 4, 7, 10, 20, 50] {
        for (force, reinforce) in [(1.5, 0.0), (2.0, 0.0), (3.0, 0.0), (2.0, 0.5), (1.0, 1.0)] {
            cases.push(Case {
                batteries,
                force,
                reinforce,
            });
        }
    }
    cases
}

fn battleships(case: Case, share: f64) -> u16 {
    (f64::from(case.batteries) * COST_BATTERY * share / COST_BB).round() as u16
}

fn scenario() -> ScenarioSpec {
    let house = |record| HouseSpec {
        record_index_1_based: record,
        handle: None,
        empire_name: None,
        homeworld_name: None,
        tax_rate: None,
        autopilot: false,
    };
    let enemy = |from_empire, to_empire| DiplomacySpec {
        from_empire,
        to_empire,
        relation: DiplomaticRelation::Enemy,
    };
    ScenarioSpec {
        metadata: ScenarioMetadata {
            label: Some("siege-odds".to_string()),
            player_count: 2,
            year: 3000,
            seed: MAP_SEED,
            baseline: ScenarioBaseline::BuilderCompatible,
        },
        houses: vec![house(1), house(2)],
        diplomacy: vec![enemy(1, 2), enemy(2, 1)],
        planets: Vec::new(),
        fleets: Vec::new(),
        turn_files: Vec::new(),
        queued_mail: Vec::new(),
        results_blocks: Vec::new(),
        message_blocks: Vec::new(),
    }
}

/// The game for `case` and the target world's index. Every fleet is emptied
/// (maintenance culls the empty ones) except empire 1's two attackers and a
/// reserve of one scout parked in empty space, which carries reinforcements
/// if the attackers are all lost.
fn setup(case: Case) -> (CoreGameData, usize) {
    let mut data = build_scenario(&scenario())
        .expect("scenario should build")
        .game_data;
    let target = data
        .planets
        .iter()
        .position(|planet| planet.owner.0 == 2)
        .expect("empire 2 has a homeworld");
    let sector = data.planets[target].sector;
    let planet = &mut data.planets[target];
    planet.ground_batteries = case.batteries;
    planet.crippled_batteries = 0;
    planet.armies = 50;
    planet.stardock = nc_data::Stardock::default();
    planet.build_queue = Default::default();
    for fleet in &mut data.fleets {
        fleet.ships = Default::default();
    }
    // The first three fleet records, whoever held them, become empire 1's.
    let attackers = [0, 1];
    let empty = (1..=nc_data::map_size_for_player_count(2))
        .flat_map(|x| (1..=nc_data::map_size_for_player_count(2)).map(move |y| [x, y]))
        .find(|&coords| !data.planets.iter().any(|p| p.sector.to_array() == coords))
        .expect("an empty sector");
    let reserve = &mut data.fleets[2];
    reserve.owner.0 = 1;
    reserve.ships.scouts = 1;
    reserve.sector = nc_data::Coords::from_array(empty);
    reserve.target = reserve.sector;
    reserve.order = Order::HoldPosition;
    reserve.rules_of_engagement = 0;
    set_fleet_at_rest(reserve);
    let total = battleships(case, case.force).max(1);
    for (k, &idx) in attackers.iter().enumerate() {
        let fleet = &mut data.fleets[idx];
        fleet.owner.0 = 1;
        fleet.ships.battleships = if k == 0 { total.div_ceil(2) } else { total / 2 };
        fleet.sector = sector;
        fleet.target = sector;
        fleet.order = Order::BombardWorld;
        fleet.rules_of_engagement = 10;
        fleet.speed = 3;
        fleet.target_depth = 1;
        set_fleet_at_rest(fleet);
    }
    (data, target)
}

/// Empire 1's battleships, wherever they are.
fn battleships_left(data: &CoreGameData) -> u32 {
    data.fleets
        .iter()
        .filter(|fleet| fleet.owner.0 == 1)
        .map(|fleet| u32::from(fleet.ships.battleships))
        .sum()
}

#[derive(Default)]
struct Tally {
    trials: u32,
    cleared: u32,
    years: u32,
    lost: f64,
}

fn trial(case: Case, seed: u64) -> Tally {
    let (mut data, target) = setup(case);
    let mut lifecycle = default_player_lifecycle_states(data.conquest.player_count);
    let mut winner = WinnerState::default();
    let reinforce = battleships(case, case.reinforce);
    let mut sent = battleships_left(&data);
    let mut tally = Tally {
        trials: 1,
        ..Tally::default()
    };
    for year in 1..=MAX_YEARS {
        if year > 1 && reinforce > 0 {
            let sector = data.planets[target].sector;
            let carrier = data
                .fleets
                .iter()
                .position(|f| f.owner.0 == 1 && f.ships.battleships > 0)
                .or_else(|| data.fleets.iter().position(|f| f.owner.0 == 1))
                .expect("the reserve fleet survives");
            let fleet = &mut data.fleets[carrier];
            fleet.ships.battleships += reinforce;
            sent += u32::from(reinforce);
            if fleet.order != Order::BombardWorld {
                fleet.order = Order::BombardWorld;
                fleet.target = sector;
                fleet.sector = sector;
                fleet.rules_of_engagement = 10;
                fleet.speed = 3;
                fleet.target_depth = 1;
                set_fleet_at_rest(fleet);
            }
        }
        run_maintenance_turn_with_context_seed_and_lifecycle(
            &mut data,
            seed,
            &[],
            &[],
            &mut lifecycle,
            &mut winner,
        )
        .expect("maintenance should run");
        if data.planets[target].ground_batteries == 0 {
            tally.cleared = 1;
            tally.years = u32::from(year);
            break;
        }
        if battleships_left(&data) == 0 && reinforce == 0 {
            break;
        }
    }
    tally.lost = f64::from(sent - battleships_left(&data)) * COST_BB;
    tally
}

fn run_case(case: Case, trials: u32, seed: u64) -> Tally {
    let threads = thread::available_parallelism().map_or(1, |n| n.get() as u32);
    thread::scope(|scope| {
        let workers: Vec<_> = (0..threads)
            .map(|worker| {
                scope.spawn(move || {
                    let mut tally = Tally::default();
                    for k in (worker..trials).step_by(threads as usize) {
                        let one = trial(case, seed + u64::from(k));
                        tally.trials += one.trials;
                        tally.cleared += one.cleared;
                        tally.years += one.years;
                        tally.lost += one.lost;
                    }
                    tally
                })
            })
            .collect();
        workers
            .into_iter()
            .fold(Tally::default(), |mut sum, worker| {
                let part = worker.join().expect("worker should finish");
                sum.trials += part.trials;
                sum.cleared += part.cleared;
                sum.years += part.years;
                sum.lost += part.lost;
                sum
            })
    })
}

fn main() {
    let mut trials = 400u32;
    let mut seed = 1u64;
    let mut json_path = None;
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        let mut value = || args.next().unwrap_or_else(|| panic!("{arg} needs a value"));
        match arg.as_str() {
            "--trials" => trials = value().parse().expect("--trials takes a number"),
            "--seed" => seed = value().parse().expect("--seed takes a number"),
            "--json" => json_path = Some(value()),
            other => panic!("unknown option {other}"),
        }
    }
    let mut json = String::from("[\n");
    for (k, case) in cases().into_iter().enumerate() {
        let tally = run_case(case, trials, seed);
        let n = f64::from(tally.trials);
        let cleared = f64::from(tally.cleared) / n;
        let years = if tally.cleared > 0 {
            f64::from(tally.years) / f64::from(tally.cleared)
        } else {
            0.0
        };
        let lost = tally.lost / n / (f64::from(case.batteries) * COST_BATTERY);
        println!(
            "{:2} batteries, {:.1}x +{:.1}x/yr: cleared {cleared:.2}, years {years:.1}, lost {lost:.1}x",
            case.batteries, case.force, case.reinforce
        );
        if k > 0 {
            json.push_str(",\n");
        }
        let _ = write!(
            json,
            "  {{\"batteries\": {}, \"force\": {}, \"reinforce\": {}, \"trials\": {}, \
             \"cleared\": {cleared:.4}, \"years\": {years:.4}, \"lost_vs_battery_cost\": {lost:.4}}}",
            case.batteries, case.force, case.reinforce, tally.trials
        );
    }
    json.push_str("\n]\n");
    if let Some(path) = json_path {
        std::fs::write(&path, json).expect("json file should be written");
    }
}
