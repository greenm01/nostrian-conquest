//! Engine-side assault odds (t202): the whole chain against a defended world,
//! orbit to ground, through real maintenance. Three plans: Invade at once,
//! Blitz at once, and Bombard until the batteries are gone and then Invade
//! with the same fleet.
//!
//! Run in release:
//! `cargo run --release -p nc-fuzz --example assault_odds -- --trials 400 --json /tmp/assault.json`
//!
//! Options: `--trials N` (default 400), `--seed S` (first campaign seed,
//! default 1), `--json PATH`.
//!
//! Every trial builds the same two-player game, empties every fleet but one
//! of empire 1, and gives empire 2's homeworld the case's batteries and
//! armies, production 100, 500 goods, and no dock. The attacking fleet starts
//! at rest in orbit with ROE 10: its battleships and as many loaded
//! transports as the case's armies. A trial ends at capture, after a blitz,
//! when the fleet has no armies left aboard, or after 8 years.

use std::fmt::Write as _;
use std::thread;

use nc_data::{
    CoreGameData, DiplomaticRelation, Order, Real48, WinnerState, default_player_lifecycle_states,
    set_armies_aboard, set_fleet_at_rest, set_troop_transports,
};
use nc_engine::run_maintenance_turn_with_context_seed_and_lifecycle;
use nc_harness::{
    DiplomacySpec, HouseSpec, ScenarioBaseline, ScenarioMetadata, ScenarioSpec, build_scenario,
};

const MAP_SEED: u64 = 1515;
const COST_BB: f64 = 45.0;
const COST_TT: f64 = 5.0;
const MAX_YEARS: u16 = 8;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Plan {
    Invade,
    Blitz,
    BombardThenInvade,
}

impl Plan {
    fn name(self) -> &'static str {
        match self {
            Plan::Invade => "invade",
            Plan::Blitz => "blitz",
            Plan::BombardThenInvade => "bombard-then-invade",
        }
    }
}

#[derive(Clone, Copy)]
struct Case {
    batteries: u16,
    defenders: u16,
    battleships: u16,
    armies: u16,
    plan: Plan,
}

fn cases() -> Vec<Case> {
    let mut cases = Vec::new();
    for (batteries, defenders) in [(0, 10), (2, 10), (4, 10), (4, 20), (8, 20)] {
        for battleships in [2, 5, 10] {
            for plan in [Plan::Invade, Plan::Blitz, Plan::BombardThenInvade] {
                cases.push(Case {
                    batteries,
                    defenders,
                    battleships,
                    armies: 2 * defenders,
                    plan,
                });
            }
        }
    }
    cases
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
            label: Some("assault-odds".to_string()),
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

fn first_order(plan: Plan) -> Order {
    match plan {
        Plan::Invade => Order::InvadeWorld,
        Plan::Blitz => Order::BlitzWorld,
        Plan::BombardThenInvade => Order::BombardWorld,
    }
}

/// The game for `case` and the target world's index.
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
    planet.armies = case.defenders;
    planet.production = Real48::from_int(100);
    planet.stored_goods = Real48::from_int(500);
    planet.stardock = nc_data::Stardock::default();
    planet.build_queue = Default::default();
    for fleet in &mut data.fleets {
        fleet.ships = Default::default();
    }
    let fleet = &mut data.fleets[0];
    fleet.owner.0 = 1;
    fleet.ships.battleships = case.battleships;
    set_troop_transports(fleet, case.armies);
    set_armies_aboard(fleet, case.armies);
    fleet.sector = sector;
    fleet.target = sector;
    fleet.order = first_order(case.plan);
    fleet.rules_of_engagement = 10;
    fleet.speed = 3;
    fleet.target_depth = 1;
    set_fleet_at_rest(fleet);
    (data, target)
}

#[derive(Default)]
struct Tally {
    trials: u32,
    captured: u32,
    years: u32,
    ship_points_lost: f64,
    armies_lost: f64,
    production_kept: f64,
    goods_kept: f64,
}

fn attacker(data: &CoreGameData) -> (u32, u32, u32) {
    data.fleets.iter().filter(|fleet| fleet.owner.0 == 1).fold(
        (0, 0, 0),
        |(bb, tt, armies), fleet| {
            (
                bb + u32::from(fleet.ships.battleships),
                tt + u32::from(fleet.ships.empty_transports + fleet.ships.loaded_transports),
                armies + u32::from(fleet.ships.loaded_transports),
            )
        },
    )
}

fn trial(case: Case, seed: u64) -> Tally {
    let (mut data, target) = setup(case);
    let mut lifecycle = default_player_lifecycle_states(data.conquest.player_count);
    let mut winner = WinnerState::default();
    let (bb0, tt0, armies0) = attacker(&data);
    let mut tally = Tally {
        trials: 1,
        ..Tally::default()
    };
    for year in 1..=MAX_YEARS {
        if case.plan == Plan::BombardThenInvade && data.planets[target].ground_batteries == 0 {
            let sector = data.planets[target].sector;
            for fleet in data.fleets.iter_mut().filter(|f| f.owner.0 == 1) {
                if fleet.ships.loaded_transports > 0 && fleet.order != Order::InvadeWorld {
                    fleet.order = Order::InvadeWorld;
                    fleet.target = sector;
                }
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
        if data.planets[target].owner.0 == 1 {
            tally.captured = 1;
            tally.years = u32::from(year);
            break;
        }
        let (bb, _, armies) = attacker(&data);
        if armies == 0 || (bb == 0 && case.plan == Plan::BombardThenInvade) {
            break;
        }
        // A Blitz makes one assault and holds; an Invade the batteries block
        // keeps its order and tries again next year (t202).
        if case.plan == Plan::Blitz {
            break;
        }
    }
    let (bb, tt, armies) = attacker(&data);
    let garrison = if tally.captured == 1 {
        u32::from(data.planets[target].armies)
    } else {
        0
    };
    tally.ship_points_lost =
        f64::from(bb0 - bb) * COST_BB + f64::from(tt0.saturating_sub(tt)) * COST_TT;
    tally.armies_lost = f64::from(armies0.saturating_sub(armies + garrison));
    tally.production_kept = data.planets[target].production.to_f64() / 100.0;
    tally.goods_kept = data.planets[target].stored_goods.to_f64() / 500.0;
    tally
}

fn run_case(case: Case, trials: u32, seed: u64) -> Tally {
    let threads = thread::available_parallelism().map_or(1, |n| n.get() as u32);
    thread::scope(|scope| {
        let workers: Vec<_> = (0..threads)
            .map(|worker| {
                scope.spawn(move || {
                    let mut sum = Tally::default();
                    for k in (worker..trials).step_by(threads as usize) {
                        let one = trial(case, seed + u64::from(k));
                        sum.trials += one.trials;
                        sum.captured += one.captured;
                        sum.years += one.years;
                        sum.ship_points_lost += one.ship_points_lost;
                        sum.armies_lost += one.armies_lost;
                        sum.production_kept += one.production_kept;
                        sum.goods_kept += one.goods_kept;
                    }
                    sum
                })
            })
            .collect();
        workers
            .into_iter()
            .fold(Tally::default(), |mut sum, worker| {
                let part = worker.join().expect("worker should finish");
                sum.trials += part.trials;
                sum.captured += part.captured;
                sum.years += part.years;
                sum.ship_points_lost += part.ship_points_lost;
                sum.armies_lost += part.armies_lost;
                sum.production_kept += part.production_kept;
                sum.goods_kept += part.goods_kept;
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
        let t = run_case(case, trials, seed);
        let n = f64::from(t.trials);
        let captured = f64::from(t.captured) / n;
        let years = if t.captured > 0 {
            f64::from(t.years) / f64::from(t.captured)
        } else {
            0.0
        };
        println!(
            "{:2} bat {:2} armies vs {:2} BB + {:2} armies, {:19}: captured {captured:.2}, years {years:.1}, \
             ships lost {:5.1} pts, armies lost {:4.1}, production kept {:.2}, goods kept {:.2}",
            case.batteries,
            case.defenders,
            case.battleships,
            case.armies,
            case.plan.name(),
            t.ship_points_lost / n,
            t.armies_lost / n,
            t.production_kept / n,
            t.goods_kept / n,
        );
        if k > 0 {
            json.push_str(",\n");
        }
        let _ = write!(
            json,
            "  {{\"batteries\": {}, \"defenders\": {}, \"battleships\": {}, \"armies\": {}, \"plan\": \"{}\", \
             \"trials\": {}, \"captured\": {captured:.4}, \"years\": {years:.4}, \"ship_points_lost\": {:.4}, \
             \"armies_lost\": {:.4}, \"production_kept\": {:.4}, \"goods_kept\": {:.4}}}",
            case.batteries,
            case.defenders,
            case.battleships,
            case.armies,
            case.plan.name(),
            t.trials,
            t.ship_points_lost / n,
            t.armies_lost / n,
            t.production_kept / n,
            t.goods_kept / n,
        );
    }
    json.push_str("\n]\n");
    if let Some(path) = json_path {
        std::fs::write(&path, json).expect("json file should be written");
    }
}
