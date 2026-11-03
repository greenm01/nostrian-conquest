//! Engine-side assault odds (t202): the whole chain against a defended world,
//! orbit to ground, through real maintenance. Three plans: Invade at once,
//! Blitz at once, and Bombard until the batteries are gone and then Invade
//! with the same fleet, or then Blitz.
//!
//! Run in release:
//! `cargo run --release -p nc-fuzz --example assault_odds -- --trials 400 --json /tmp/assault.json`
//!
//! Options: `--trials N` (default 400), `--seed S` (first campaign seed,
//! default 1), `--json PATH`.
//!
//! `--grid DEFENCE` (t254) replaces the fixed cases with the attack-and-defence
//! game: every attack spending `--attack POINTS` (default DEFENCE) against
//! every defence spending DEFENCE. An attack is a warship mix in 90-point
//! shares of DD, CA or BB, with the rest spent on loaded transports at 7
//! points an army, and one of three plans. A siege or a bombard-then-blitz
//! takes two shares or more; a blitz may take none. A defence is no starbase or one (50
//! points), batteries in 100-point steps, a guard fleet on Guard/Blockade a
//! World in 90-point shares of one hull, and the rest in armies. A siege
//! bombards until the batteries are gone and its armies outnumber the
//! garrison, then invades. A bombard-then-blitz blitzes once the batteries
//! are gone. A blitz lands at once, past the batteries. Its JSON feeds
//! `tools/combat-model/assault_game.py`.
//!
//! Every trial builds the same two-player game, empties every fleet but one
//! of empire 1, and gives empire 2's homeworld the case's batteries and
//! armies, production 100, 500 goods, and no dock. The attacking fleet starts
//! at rest in orbit with ROE 10: its warships and as many loaded transports
//! as the case's armies. A grid defence's starbase and guard fleet (ROE 10)
//! start at rest in the same orbit. A trial ends at capture, after a blitz,
//! when the fleet has no armies left aboard, or after 8 years.

use std::fmt::Write as _;
use std::thread;

use nc_data::{
    CoreGameData, DiplomaticRelation, Order, Real48, WinnerState, default_player_lifecycle_states,
    set_armies_aboard, set_fleet_at_rest, set_starbase_at_rest, set_troop_transports,
};
use nc_engine::run_maintenance_turn_with_context_seed_and_lifecycle;
use nc_harness::{
    DiplomacySpec, HouseSpec, ScenarioBaseline, ScenarioMetadata, ScenarioSpec, build_scenario,
};

const MAP_SEED: u64 = 1515;
const COST_DD: u16 = 5;
const COST_CA: u16 = 15;
const COST_BB: u16 = 45;
const COST_TT: u16 = 5;
const COST_ARMY: u16 = 2;
const COST_BATTERY: u16 = 20;
const COST_STARBASE: u16 = 50;
/// A grid's warship share, and its battery step.
const SHARE: u16 = 90;
const BATTERY_STEP: u16 = 100;
const MAX_YEARS: u16 = 8;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Plan {
    Invade,
    Blitz,
    BombardThenInvade,
    BombardThenBlitz,
    Siege,
}

impl Plan {
    fn name(self) -> &'static str {
        match self {
            Plan::Invade => "invade",
            Plan::Blitz => "blitz",
            Plan::BombardThenInvade => "bombard-then-invade",
            Plan::BombardThenBlitz => "bombard-then-blitz",
            Plan::Siege => "siege",
        }
    }
}

#[derive(Clone, Copy, Default)]
struct Force {
    dd: u16,
    ca: u16,
    bb: u16,
}

impl Force {
    fn points(self) -> u16 {
        self.dd * COST_DD + self.ca * COST_CA + self.bb * COST_BB
    }
}

#[derive(Clone, Copy)]
struct Case {
    batteries: u16,
    defenders: u16,
    starbase: bool,
    guard: Force,
    fleet: Force,
    armies: u16,
    plan: Plan,
}

fn cases() -> Vec<Case> {
    let mut cases = Vec::new();
    for (batteries, defenders) in [(0, 10), (2, 10), (4, 10), (4, 20), (8, 20)] {
        for battleships in [2, 5, 10] {
            for plan in [
                Plan::Invade,
                Plan::Blitz,
                Plan::BombardThenInvade,
                Plan::BombardThenBlitz,
            ] {
                cases.push(Case {
                    batteries,
                    defenders,
                    starbase: false,
                    guard: Force::default(),
                    fleet: Force {
                        bb: battleships,
                        ..Force::default()
                    },
                    armies: 2 * defenders,
                    plan,
                });
            }
        }
    }
    cases
}

/// Every warship mix of `shares` 90-point shares, one hull to a share.
fn mixes(shares: u16) -> Vec<Force> {
    let mut mixes = Vec::new();
    for bb in 0..=shares {
        for ca in 0..=shares - bb {
            mixes.push(Force {
                dd: (shares - bb - ca) * (SHARE / COST_DD),
                ca: ca * (SHARE / COST_CA),
                bb: bb * (SHARE / COST_BB),
            });
        }
    }
    mixes
}

/// The attack-and-defence game (t254): every attack spending `attack`
/// points against every defence spending `defence`.
fn grid(defence: u16, attack: u16) -> Vec<Case> {
    // A blitz may go in with transports alone; a bombardment needs warships.
    let attacks: Vec<(Plan, Force, u16)> = [Plan::Siege, Plan::BombardThenBlitz, Plan::Blitz]
        .into_iter()
        .flat_map(|plan| {
            let first = if plan == Plan::Blitz { 0 } else { 2 };
            (first..=attack / SHARE).flat_map(mixes).map(move |fleet| {
                let armies = (attack - fleet.points()) / (COST_TT + COST_ARMY);
                (plan, fleet, armies)
            })
        })
        .filter(|&(_, _, armies)| armies > 0)
        .collect();
    let mut defences = Vec::new();
    for starbase in [false, true] {
        let left = defence - u16::from(starbase) * COST_STARBASE;
        for batteries in (0..=left / BATTERY_STEP).map(|k| k * BATTERY_STEP) {
            let left = left - batteries;
            let mut guards = vec![Force::default()];
            for shares in 1..=left / SHARE {
                let n = |cost| shares * (SHARE / cost);
                guards.push(Force {
                    dd: n(COST_DD),
                    ..Force::default()
                });
                guards.push(Force {
                    ca: n(COST_CA),
                    ..Force::default()
                });
                guards.push(Force {
                    bb: n(COST_BB),
                    ..Force::default()
                });
            }
            for guard in guards {
                let defenders = (left - guard.points()) / COST_ARMY;
                defences.push((starbase, batteries / COST_BATTERY, guard, defenders));
            }
        }
    }
    let mut cases = Vec::new();
    for &(plan, fleet, armies) in &attacks {
        for &(starbase, batteries, guard, defenders) in &defences {
            cases.push(Case {
                batteries,
                defenders,
                starbase,
                guard,
                fleet,
                armies,
                plan,
            });
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
        Plan::BombardThenInvade | Plan::BombardThenBlitz | Plan::Siege => Order::BombardWorld,
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
    let guard = data
        .fleets
        .iter()
        .position(|fleet| fleet.owner.0 == 2)
        .expect("empire 2 has a fleet record");
    assert_ne!(guard, 0, "record 0 is the attacker's");
    if case.guard.points() > 0 {
        let fleet = &mut data.fleets[guard];
        fleet.ships.destroyers = case.guard.dd;
        fleet.ships.cruisers = case.guard.ca;
        fleet.ships.battleships = case.guard.bb;
        fleet.sector = sector;
        fleet.target = sector;
        fleet.order = Order::GuardBlockadeWorld;
        fleet.rules_of_engagement = 10;
        fleet.target_depth = 1;
        set_fleet_at_rest(fleet);
    }
    if case.starbase {
        let mut base = nc_data::Starbase::default();
        base.base_id.0 = 1;
        base.record.0 = 1;
        base.sector = sector;
        base.target = sector;
        base.owner.0 = 2;
        set_starbase_at_rest(&mut base);
        base.target_depth = 1;
        data.bases.push(base);
    }
    let fleet = &mut data.fleets[0];
    fleet.owner.0 = 1;
    fleet.ships.destroyers = case.fleet.dd;
    fleet.ships.cruisers = case.fleet.ca;
    fleet.ships.battleships = case.fleet.bb;
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
    defence_points_lost: f64,
}

impl Tally {
    fn add(&mut self, other: &Tally) {
        self.trials += other.trials;
        self.captured += other.captured;
        self.years += other.years;
        self.ship_points_lost += other.ship_points_lost;
        self.armies_lost += other.armies_lost;
        self.production_kept += other.production_kept;
        self.goods_kept += other.goods_kept;
        self.defence_points_lost += other.defence_points_lost;
    }
}

fn warship_points(fleet: &nc_data::Fleet) -> u32 {
    let ships = &fleet.ships;
    u32::from(ships.destroyers) * u32::from(COST_DD)
        + u32::from(ships.cruisers) * u32::from(COST_CA)
        + u32::from(ships.battleships) * u32::from(COST_BB)
}

/// The attacker's warship points, transports and armies aboard.
fn attacker(data: &CoreGameData) -> (u32, u32, u32) {
    data.fleets.iter().filter(|fleet| fleet.owner.0 == 1).fold(
        (0, 0, 0),
        |(points, tt, armies), fleet| {
            (
                points + warship_points(fleet),
                tt + u32::from(fleet.ships.empty_transports + fleet.ships.loaded_transports),
                armies + u32::from(fleet.ships.loaded_transports),
            )
        },
    )
}

/// What empire 2 still holds at the target, in build points: the world's
/// batteries and armies while it keeps the world, its guard and its bases.
fn defender(data: &CoreGameData, target: usize) -> u32 {
    let planet = &data.planets[target];
    let world = if planet.owner.0 == 2 {
        u32::from(planet.ground_batteries) * u32::from(COST_BATTERY)
            + u32::from(planet.armies) * u32::from(COST_ARMY)
    } else {
        0
    };
    let fleets: u32 = data
        .fleets
        .iter()
        .filter(|fleet| fleet.owner.0 == 2)
        .map(warship_points)
        .sum();
    let bases = data.bases.iter().filter(|base| base.owner.0 == 2).count() as u32;
    world + fleets + bases * u32::from(COST_STARBASE)
}

fn trial(case: Case, seed: u64) -> Tally {
    let (mut data, target) = setup(case);
    let mut lifecycle = default_player_lifecycle_states(data.conquest.player_count);
    let mut winner = WinnerState::default();
    let (points0, tt0, armies0) = attacker(&data);
    let defence0 = defender(&data, target);
    let mut tally = Tally {
        trials: 1,
        ..Tally::default()
    };
    for year in 1..=MAX_YEARS {
        let landing = match case.plan {
            Plan::BombardThenInvade => Some(Order::InvadeWorld),
            Plan::BombardThenBlitz => Some(Order::BlitzWorld),
            Plan::Siege if attacker(&data).2 > u32::from(data.planets[target].armies) => {
                Some(Order::InvadeWorld)
            }
            _ => None,
        };
        if let Some(landing) = landing
            && data.planets[target].ground_batteries == 0
        {
            let sector = data.planets[target].sector;
            for fleet in data.fleets.iter_mut().filter(|f| f.owner.0 == 1) {
                if fleet.ships.loaded_transports > 0 && fleet.order != landing {
                    fleet.order = landing;
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
        let (points, _, armies) = attacker(&data);
        // A siege's transports may still land after its warships are gone.
        let bombarding = matches!(case.plan, Plan::BombardThenInvade | Plan::BombardThenBlitz);
        if armies == 0 || (points == 0 && bombarding) {
            break;
        }
        // A Blitz makes one assault and holds; an Invade the batteries block
        // keeps its order and tries again next year (t202).
        if case.plan == Plan::Blitz {
            break;
        }
    }
    let (points, tt, armies) = attacker(&data);
    let garrison = if tally.captured == 1 {
        u32::from(data.planets[target].armies)
    } else {
        0
    };
    tally.ship_points_lost =
        f64::from(points0 - points) + f64::from(tt0.saturating_sub(tt) * u32::from(COST_TT));
    tally.defence_points_lost = f64::from(defence0.saturating_sub(defender(&data, target)));
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
                        sum.add(&trial(case, seed + u64::from(k)));
                    }
                    sum
                })
            })
            .collect();
        workers
            .into_iter()
            .fold(Tally::default(), |mut sum, worker| {
                sum.add(&worker.join().expect("worker should finish"));
                sum
            })
    })
}

fn main() {
    let mut trials = 400u32;
    let mut seed = 1u64;
    let mut json_path = None;
    let mut grid_defence = None;
    let mut grid_attack = None;
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        let mut value = || args.next().unwrap_or_else(|| panic!("{arg} needs a value"));
        match arg.as_str() {
            "--trials" => trials = value().parse().expect("--trials takes a number"),
            "--seed" => seed = value().parse().expect("--seed takes a number"),
            "--json" => json_path = Some(value()),
            "--grid" => grid_defence = Some(value().parse().expect("--grid takes a number")),
            "--attack" => grid_attack = Some(value().parse().expect("--attack takes a number")),
            other => panic!("unknown option {other}"),
        }
    }
    let mut json = String::from("[\n");
    let all_cases = match grid_defence {
        Some(defence) => grid(defence, grid_attack.unwrap_or(defence)),
        None => cases(),
    };
    for (k, case) in all_cases.into_iter().enumerate() {
        let t = run_case(case, trials, seed);
        let n = f64::from(t.trials);
        let captured = f64::from(t.captured) / n;
        let years = if t.captured > 0 {
            f64::from(t.years) / f64::from(t.captured)
        } else {
            0.0
        };
        let force = |f: Force| format!("{}DD+{}CA+{}BB", f.dd, f.ca, f.bb);
        println!(
            "{:2} bat {:3} armies SB{} guard {} vs {} + {:2} armies, {:19}: captured {captured:.2}, \
             years {years:.1}, ships lost {:5.1} pts, armies lost {:4.1}, defence lost {:5.1} pts, \
             production kept {:.2}, goods kept {:.2}",
            case.batteries,
            case.defenders,
            u8::from(case.starbase),
            force(case.guard),
            force(case.fleet),
            case.armies,
            case.plan.name(),
            t.ship_points_lost / n,
            t.armies_lost / n,
            t.defence_points_lost / n,
            t.production_kept / n,
            t.goods_kept / n,
        );
        if k > 0 {
            json.push_str(",\n");
        }
        let _ = write!(
            json,
            "  {{\"batteries\": {}, \"defenders\": {}, \"starbase\": {}, \"guard\": [{}, {}, {}], \
             \"destroyers\": {}, \"cruisers\": {}, \"battleships\": {}, \"armies\": {}, \"plan\": \"{}\", \
             \"trials\": {}, \"captured\": {captured:.4}, \"years\": {years:.4}, \"ship_points_lost\": {:.4}, \
             \"armies_lost\": {:.4}, \"defence_points_lost\": {:.4}, \"production_kept\": {:.4}, \
             \"goods_kept\": {:.4}}}",
            case.batteries,
            case.defenders,
            case.starbase,
            case.guard.dd,
            case.guard.ca,
            case.guard.bb,
            case.fleet.dd,
            case.fleet.ca,
            case.fleet.bb,
            case.armies,
            case.plan.name(),
            t.trials,
            t.ship_points_lost / n,
            t.armies_lost / n,
            t.defence_points_lost / n,
            t.production_kept / n,
            t.goods_kept / n,
        );
    }
    json.push_str("\n]\n");
    if let Some(path) = json_path {
        std::fs::write(&path, json).expect("json file should be written");
    }
}
