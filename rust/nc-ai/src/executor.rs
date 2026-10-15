//! The scripted executor: a seat's whole turn from a handful of strategy
//! weights, chosen through the same masks and actions a learned seat uses.
//!
//! It reads only what [`crate::observe`] may read: the empire's own records,
//! its [`SeatMemory`], and public world positions.
//!
//! Each year it:
//! - sets tax from the weights;
//! - declares every other empire an enemy once `war_year` years have passed;
//! - at each world orders up to two builds, most urgent first, and spends
//!   what is left on the preferred hull. While worlds remain to colonize,
//!   the home world builds ETACs first, each with a destroyer escort built
//!   the same year so they commission into one fleet, then scouts, then its
//!   armies and batteries, then starbases and transports. Other worlds build
//!   their garrison;
//! - sends ETAC fleets at ROE 0 to colonize the most valuable world not
//!   known to be owned, one fleet per world;
//! - sends scouts, and during the land grab lone destroyers as pickets, to
//!   the world it knows least about;
//! - gathers warships and transports at home by Rendezvous, loads armies
//!   there, and strikes the best known enemy world once the home force is
//!   `attack_ratio` times its estimated defense: Invade with enough armies
//!   aboard, otherwise Bombard;
//! - once at war, posts one new warship fleet that cannot strike on
//!   Guard/Blockade at an enemy world the strike force is not attacking.
//!
//! A turn is the executor's [`Goals`] ([`plan_goals`]) carried out by
//! [`execute_goals`], which a strategist network can drive with goals of its
//! own.

use nc_data::{
    CoreGameData, Fleet, IntelTier, Order, PlanetIntelSnapshot, ProductionItemKind,
    has_combat_ships,
};
use nc_engine::BUILD_UNITS;

use crate::action::{
    BUILD_AMOUNT_CHOICES, BUILD_SLOTS, FleetChoice, FleetMask, MISSIONS, PlanetChoice, SeatAction,
    SeatMasks, TAX_STEP, build_units,
};
use crate::goals::{Goals, claimed_worlds, colonizable, posture_weights, world_at};
use crate::memory::SeatMemory;

/// Strategy weights. Counts are targets, not limits per year.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Weights {
    /// Empire tax rate, 0 to 100.
    pub tax: f32,
    /// Armies to keep at the home world.
    pub home_armies: f32,
    /// Armies to keep at every other world.
    pub colony_armies: f32,
    /// Ground batteries to keep at every world.
    pub batteries: f32,
    /// ETACs to keep while worlds remain to colonize.
    pub etacs: f32,
    pub scouts: f32,
    /// Starbases to keep at the home world.
    pub starbases: f32,
    /// Troop transports to keep.
    pub transports: f32,
    /// Hull preference; the affordable hull with the largest weight is built.
    pub battleship: f32,
    pub cruiser: f32,
    pub destroyer: f32,
    /// Home-force strength over a target's estimated defense needed to strike.
    pub attack_ratio: f32,
    /// Rules of engagement for warships, 0 to 10.
    pub roe: f32,
    /// Years from the start before every other empire is declared an enemy.
    pub war_year: f32,
    /// Behaviour switches under trial; not tuned.
    pub doctrine: Doctrine,
}

/// Executor behaviours under trial, switched so that a changed executor can
/// play its predecessor in one game (t159). A switch is removed once its
/// change is accepted or rejected.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Doctrine {
    /// The land-grab build order: up to two builds a year, ETACs with escorts
    /// and scouts before garrisons; colony fleets at ROE 0; destroyer
    /// pickets. Off, t154's order: one build a year, garrison first.
    pub land_grab: bool,
    /// Score strike targets by value over estimated defence, so the
    /// cheapest conquests come first (t160, accepted). Off, by value alone,
    /// as t154 did.
    pub strike_weakest: bool,
    /// Bombard a target on arrival while its batteries may stand, then
    /// invade from there the next year with enough armies aboard (t161).
    /// Off, a force invades straight away whenever it carries enough armies,
    /// and goes home after one year at the target.
    pub suppress_first: bool,
    /// Scale each colony's army target to its production (t164):
    /// `colony_armies` per 25 points, at least one. Off, a flat
    /// `colony_armies` at every colony.
    pub scaled_garrisons: bool,
    /// A dedicated blockader (t171, accepted): once war is declared, a new warship
    /// fleet at home that cannot strike blockades the most valuable enemy
    /// world per unit of defence that no fleet of the seat strikes, if
    /// none blockades yet. It comes home when the world is no longer the
    /// enemy's or a declared enemy is reported near home. Off, it gathers
    /// with the strike force.
    pub blockader: bool,
}

impl Doctrine {
    /// The accepted behaviours.
    pub const CURRENT: Doctrine = Doctrine {
        land_grab: true,
        strike_weakest: true,
        suppress_first: true,
        scaled_garrisons: true,
        blockader: true,
    };
}

impl Weights {
    pub const COUNT: usize = 14;

    /// Hand-set starting weights, before tuning.
    pub const BASELINE: Weights = Weights {
        tax: 45.0,
        home_armies: 8.0,
        colony_armies: 2.0,
        batteries: 2.0,
        etacs: 3.0,
        scouts: 2.0,
        starbases: 1.0,
        transports: 6.0,
        battleship: 0.6,
        cruiser: 0.8,
        destroyer: 0.3,
        attack_ratio: 1.5,
        roe: 8.0,
        war_year: 5.0,
        doctrine: Doctrine::CURRENT,
    };

    /// Weights tuned by evolution against the executor itself, with the
    /// current doctrine; see the `tune_executor` example.
    pub const TUNED: Weights = Weights {
        tax: 65.0,
        home_armies: 20.0,
        colony_armies: 10.0,
        batteries: 9.67,
        etacs: 3.93,
        scouts: 0.14,
        starbases: 1.67,
        transports: 16.0,
        battleship: 0.38,
        cruiser: 0.59,
        destroyer: 0.0,
        attack_ratio: 2.43,
        roe: 6.26,
        war_year: 20.7,
        doctrine: Doctrine::CURRENT,
    };

    /// t154's tuned weights with t154's build order, kept to measure the
    /// land-grab doctrine against.
    pub const CLASSIC: Weights = Weights {
        tax: 64.4,
        home_armies: 15.7,
        colony_armies: 9.9,
        batteries: 9.0,
        etacs: 6.4,
        scouts: 0.9,
        starbases: 0.0,
        transports: 15.8,
        battleship: 0.92,
        cruiser: 0.98,
        destroyer: 0.43,
        attack_ratio: 3.44,
        roe: 4.2,
        war_year: 10.5,
        doctrine: Doctrine {
            land_grab: false,
            strike_weakest: false,
            suppress_first: false,
            scaled_garrisons: false,
            blockader: false,
        },
    };

    /// The lowest and highest value of each weight, in field order.
    pub const RANGES: [(f32, f32); Self::COUNT] = [
        (0.0, 65.0),
        (0.0, 20.0),
        (0.0, 10.0),
        (0.0, 10.0),
        (0.0, 8.0),
        (0.0, 5.0),
        (0.0, 3.0),
        (0.0, 16.0),
        (0.0, 1.0),
        (0.0, 1.0),
        (0.0, 1.0),
        (0.5, 4.0),
        (0.0, 10.0),
        (0.0, 30.0),
    ];

    pub fn to_array(self) -> [f32; Self::COUNT] {
        [
            self.tax,
            self.home_armies,
            self.colony_armies,
            self.batteries,
            self.etacs,
            self.scouts,
            self.starbases,
            self.transports,
            self.battleship,
            self.cruiser,
            self.destroyer,
            self.attack_ratio,
            self.roe,
            self.war_year,
        ]
    }

    /// Weights from an array in field order, clamped to [`Self::RANGES`].
    pub fn from_array(values: [f32; Self::COUNT]) -> Self {
        let v: [f32; Self::COUNT] =
            std::array::from_fn(|idx| values[idx].clamp(Self::RANGES[idx].0, Self::RANGES[idx].1));
        Weights {
            tax: v[0],
            home_armies: v[1],
            colony_armies: v[2],
            batteries: v[3],
            etacs: v[4],
            scouts: v[5],
            starbases: v[6],
            transports: v[7],
            battleship: v[8],
            cruiser: v[9],
            destroyer: v[10],
            attack_ratio: v[11],
            roe: v[12],
            war_year: v[13],
            doctrine: Doctrine::CURRENT,
        }
    }
}

/// Assumed defense facts for a world whose intel lacks them.
const UNKNOWN_ARMIES: f32 = 6.0;
const UNKNOWN_BATTERIES: f32 = 3.0;

/// A declared enemy's force reported within this many sectors of a world,
/// in the last [`THREAT_YEARS`] years, threatens it.
const THREAT_RANGE: f32 = 4.0;
const THREAT_YEARS: u16 = 2;

/// The executor's turn for `memory`'s empire, within `masks`. `start_year`
/// is the game's first year.
pub fn plan_turn(
    game_data: &CoreGameData,
    memory: &SeatMemory,
    masks: &SeatMasks,
    weights: &Weights,
    start_year: u16,
) -> SeatAction {
    let goals = plan_goals(game_data, memory, masks, weights, start_year);
    execute_goals(game_data, memory, masks, weights, &goals)
}

/// The goals the executor sets itself this year.
pub fn plan_goals(
    game_data: &CoreGameData,
    memory: &SeatMemory,
    masks: &SeatMasks,
    weights: &Weights,
    start_year: u16,
) -> Goals {
    let empire = memory.empire;
    let year = game_data.conquest.game_year;
    let at_war = f32::from(year.saturating_sub(start_year)) >= weights.war_year;
    let home = home_world(game_data, empire);
    let (_, labels) = plan_fleets(game_data, memory, masks, weights, home, None);
    Goals {
        tax: ((weights.tax / f32::from(TAX_STEP)).round() as usize)
            .min(100 / usize::from(TAX_STEP)),
        diplomacy: masks
            .diplomacy
            .iter()
            .enumerate()
            .map(|(idx, mask)| {
                let other = idx as u8 + 1;
                let relation = game_data.players[usize::from(empire) - 1].diplomacy[idx];
                if at_war
                    && mask[2]
                    && other != empire
                    && relation != nc_data::DiplomaticRelation::Enemy
                {
                    2
                } else {
                    0
                }
            })
            .collect(),
        posture: 0,
        strike: labels.strike,
        colonize: labels.colonize,
    }
}

/// Carry out `goals` within `masks`. The goals must respect
/// [`crate::goal_masks`].
pub fn execute_goals(
    game_data: &CoreGameData,
    memory: &SeatMemory,
    masks: &SeatMasks,
    weights: &Weights,
    goals: &Goals,
) -> SeatAction {
    let home = home_world(game_data, memory.empire);
    let stock = Stock::count(game_data, memory.empire);
    let build_weights = posture_weights(weights, goals.posture);
    let planets = masks
        .planets
        .iter()
        .map(|mask| {
            let is_home = Some(mask.record) == home;
            let (build, amount) = choose_builds(
                game_data,
                mask.record,
                &mask.build,
                is_home,
                &stock,
                memory,
                &build_weights,
            );
            PlanetChoice {
                record: mask.record,
                build,
                amount,
            }
        })
        .collect();
    let (fleets, _) = plan_fleets(game_data, memory, masks, weights, home, Some(goals));
    SeatAction {
        tax: goals.tax,
        diplomacy: goals.diplomacy.clone(),
        planets,
        fleets,
    }
}

fn home_world(game_data: &CoreGameData, empire: u8) -> Option<usize> {
    let player = &game_data.players[usize::from(empire) - 1];
    let named = usize::from(player.homeworld);
    if named > 0
        && game_data
            .planets
            .get(named - 1)
            .is_some_and(|planet| planet.owner.0 == empire)
    {
        return Some(named);
    }
    game_data
        .planets
        .iter()
        .enumerate()
        .filter(|(_, planet)| planet.owner.0 == empire)
        .max_by_key(|(_, planet)| planet.potential_production)
        .map(|(idx, _)| idx + 1)
}

/// The empire's units by kind: in fleets, in stardocks and queued.
struct Stock {
    etacs: u32,
    scouts: u32,
    transports: u32,
}

impl Stock {
    fn count(game_data: &CoreGameData, empire: u8) -> Self {
        let mut stock = Stock {
            etacs: 0,
            scouts: 0,
            transports: 0,
        };
        for fleet in game_data
            .fleets
            .iter()
            .filter(|fleet| fleet.owner.0 == empire)
        {
            stock.etacs += u32::from(fleet.ships.etacs);
            stock.scouts += u32::from(fleet.ships.scouts);
            stock.transports +=
                u32::from(fleet.ships.empty_transports) + u32::from(fleet.ships.loaded_transports);
        }
        for planet in game_data
            .planets
            .iter()
            .filter(|planet| planet.owner.0 == empire)
        {
            stock.etacs += planet.stardock.etacs + queued(planet, ProductionItemKind::Etac);
            stock.scouts += planet.stardock.scouts + queued(planet, ProductionItemKind::Scout);
            stock.transports +=
                planet.stardock.transports + queued(planet, ProductionItemKind::Transport);
        }
        stock
    }
}

fn queued(planet: &nc_data::Planet, kind: ProductionItemKind) -> u32 {
    let cost = kind.build_cost().unwrap_or(1).max(1);
    planet
        .build_queue
        .iter()
        .filter(|slot| slot.kind == kind)
        .map(|slot| slot.points_owed.div_ceil(cost))
        .sum()
}

fn build_choice(kind: ProductionItemKind) -> usize {
    1 + BUILD_UNITS
        .iter()
        .position(|unit| unit.kind == kind)
        .expect("every build kind is a build unit")
}

/// The smallest amount choice that buys `units` of a unit costing `cost`
/// from `points`, or all of them when it cannot.
fn amount_for(points: u32, cost: u32, units: u32) -> usize {
    (0..BUILD_AMOUNT_CHOICES)
        .find(|&amount| build_units(points, cost, amount) >= units)
        .unwrap_or(BUILD_AMOUNT_CHOICES - 1)
}

/// A world's builds for the year, most urgent first, in up to
/// [`BUILD_SLOTS`] slots sharing its budget; any budget left then goes to
/// the preferred hull.
///
/// While worlds remain to colonize, the home world builds ETACs first, each
/// with one destroyer as escort (built the same year, so they commission
/// into one fleet), then scouts, and only then its garrison. Expansion and
/// intelligence win the early game.
#[allow(clippy::too_many_arguments)]
fn choose_builds(
    game_data: &CoreGameData,
    record: usize,
    allowed: &[bool],
    is_home: bool,
    stock: &Stock,
    memory: &SeatMemory,
    weights: &Weights,
) -> ([usize; BUILD_SLOTS], [usize; BUILD_SLOTS]) {
    if !weights.doctrine.land_grab {
        return classic_builds(game_data, record, allowed, is_home, stock, memory, weights);
    }
    let planet = &game_data.planets[record - 1];
    let short = |have: u32, want: f32| (want.round() as u32).saturating_sub(have);
    let garrison = [
        Need::Units(
            ProductionItemKind::Army,
            short(
                planet.armies.into(),
                if is_home {
                    weights.home_armies
                } else if weights.doctrine.scaled_garrisons {
                    let production = f32::from(nc_data::present_production_points(planet));
                    (weights.colony_armies * production / 25.0).max(1.0)
                } else {
                    weights.colony_armies
                },
            ),
        ),
        Need::Units(
            ProductionItemKind::GroundBattery,
            short(planet.ground_batteries.into(), weights.batteries),
        ),
    ];
    let mut needs = Vec::new();
    if is_home {
        if land_grab(memory) {
            needs.push(Need::Units(
                ProductionItemKind::Etac,
                short(stock.etacs, weights.etacs),
            ));
            needs.push(Need::Escorts);
        }
        needs.push(Need::Units(
            ProductionItemKind::Scout,
            short(stock.scouts, weights.scouts),
        ));
        needs.extend(garrison);
        needs.push(Need::Units(
            ProductionItemKind::Starbase,
            short(
                planet.stardock.starbases + home_starbases(game_data, planet),
                weights.starbases,
            ),
        ));
        needs.push(Need::Units(
            ProductionItemKind::Transport,
            short(stock.transports, weights.transports),
        ));
    } else {
        needs.extend(garrison);
    }

    let mut points = game_data.planet_build_points_left(record).unwrap_or(0);
    let mut builds = [0; BUILD_SLOTS];
    let mut amounts = [0; BUILD_SLOTS];
    let mut slot = 0;
    let mut etacs_bought = 0;
    let mut order = |kind: ProductionItemKind, units: u32, points: &mut u32| {
        let choice = build_choice(kind);
        let cost = kind.build_cost().unwrap_or(1).max(1);
        if slot == BUILD_SLOTS || units == 0 || !allowed[choice] || *points < cost {
            return 0;
        }
        let amount = amount_for(*points, cost, units);
        let bought = build_units(*points, cost, amount);
        *points -= bought * cost;
        builds[slot] = choice;
        amounts[slot] = amount;
        slot += 1;
        bought
    };
    for need in needs {
        match need {
            Need::Units(kind, units) => {
                let bought = order(kind, units, &mut points);
                if kind == ProductionItemKind::Etac {
                    etacs_bought = bought;
                }
            }
            Need::Escorts => {
                order(ProductionItemKind::Destroyer, etacs_bought, &mut points);
            }
        }
    }
    let hull = [
        (ProductionItemKind::Battleship, weights.battleship),
        (ProductionItemKind::Cruiser, weights.cruiser),
        (ProductionItemKind::Destroyer, weights.destroyer),
    ]
    .into_iter()
    .filter(|(kind, weight)| {
        *weight > 0.0
            && allowed[build_choice(*kind)]
            && points >= kind.build_cost().unwrap_or(1).max(1)
    })
    .max_by(|a, b| a.1.total_cmp(&b.1));
    if let Some((kind, _)) = hull {
        order(kind, u32::MAX, &mut points);
    }
    (builds, amounts)
}

/// t154's builds: the first unmet need of armies, batteries, then at home
/// ETACs, scouts, starbases and transports, in one slot; with every need
/// met, the preferred hull with the whole budget.
#[allow(clippy::too_many_arguments)]
fn classic_builds(
    game_data: &CoreGameData,
    record: usize,
    allowed: &[bool],
    is_home: bool,
    stock: &Stock,
    memory: &SeatMemory,
    weights: &Weights,
) -> ([usize; BUILD_SLOTS], [usize; BUILD_SLOTS]) {
    let planet = &game_data.planets[record - 1];
    let short = |have: u32, want: f32| (want.round() as u32).saturating_sub(have);
    let army_target = if is_home {
        weights.home_armies
    } else {
        weights.colony_armies
    };
    let mut needs = vec![
        (
            ProductionItemKind::Army,
            short(planet.armies.into(), army_target),
        ),
        (
            ProductionItemKind::GroundBattery,
            short(planet.ground_batteries.into(), weights.batteries),
        ),
    ];
    if is_home {
        if land_grab(memory) {
            needs.push((ProductionItemKind::Etac, short(stock.etacs, weights.etacs)));
        }
        needs.push((
            ProductionItemKind::Scout,
            short(stock.scouts, weights.scouts),
        ));
        needs.push((
            ProductionItemKind::Starbase,
            short(
                planet.stardock.starbases + home_starbases(game_data, planet),
                weights.starbases,
            ),
        ));
        needs.push((
            ProductionItemKind::Transport,
            short(stock.transports, weights.transports),
        ));
    }
    let points = game_data.planet_build_points_left(record).unwrap_or(0);
    let one = |choice: usize, amount: usize| ([choice, 0], [amount, 0]);
    for (kind, units) in needs {
        let choice = build_choice(kind);
        if units > 0 && allowed[choice] {
            let cost = kind.build_cost().unwrap_or(1).max(1);
            return one(choice, amount_for(points, cost, units));
        }
    }
    [
        (ProductionItemKind::Battleship, weights.battleship),
        (ProductionItemKind::Cruiser, weights.cruiser),
        (ProductionItemKind::Destroyer, weights.destroyer),
    ]
    .iter()
    .filter(|(kind, weight)| *weight > 0.0 && allowed[build_choice(*kind)])
    .max_by(|a, b| a.1.total_cmp(&b.1))
    .map_or(one(0, 0), |(kind, _)| {
        one(build_choice(*kind), BUILD_AMOUNT_CHOICES - 1)
    })
}

/// One build need, most urgent first.
enum Need {
    Units(ProductionItemKind, u32),
    /// One destroyer for each ETAC ordered this year.
    Escorts,
}

/// Worlds remain that the seat may colonize.
fn land_grab(memory: &SeatMemory) -> bool {
    memory
        .planet_intel
        .values()
        .any(|intel| colonizable(intel, memory.empire))
}

fn home_starbases(game_data: &CoreGameData, planet: &nc_data::Planet) -> u32 {
    game_data
        .bases
        .iter()
        .filter(|base| base.owner == planet.owner && base.sector == planet.sector)
        .count() as u32
}

fn distance(a: [u8; 2], b: [u8; 2]) -> f32 {
    let dx = f32::from(a[0]) - f32::from(b[0]);
    let dy = f32::from(a[1]) - f32::from(b[1]);
    (dx * dx + dy * dy).sqrt()
}

fn mission_choice(order: Order) -> usize {
    1 + MISSIONS
        .iter()
        .position(|mission| *mission == order)
        .expect("every planned mission is offered")
}

fn strength(fleet: &Fleet) -> f32 {
    f32::from(fleet.ships.destroyers)
        + 3.0 * f32::from(fleet.ships.cruisers)
        + 9.0 * f32::from(fleet.ships.battleships)
}

fn defense(intel: &PlanetIntelSnapshot) -> f32 {
    let armies = intel.known_armies.map_or(UNKNOWN_ARMIES, f32::from);
    let batteries = intel
        .known_ground_batteries
        .map_or(UNKNOWN_BATTERIES, f32::from);
    let starbases = intel.known_starbase_count.map_or(0.0, f32::from);
    armies + 4.0 * batteries + 10.0 * starbases
}

/// The strike and colonize goals the executor's own fleet plan implies.
#[derive(Default)]
struct Labels {
    strike: usize,
    colonize: usize,
}

/// Fleet orders, following `goals`' strike and colonize goals, or with
/// `None` choosing freely and reporting those choices as [`Labels`]: the
/// strike target of the strongest home force, and the world of the first
/// ETAC fleet needing orders.
fn plan_fleets(
    game_data: &CoreGameData,
    memory: &SeatMemory,
    masks: &SeatMasks,
    weights: &Weights,
    home: Option<usize>,
    goals: Option<&Goals>,
) -> (Vec<FleetChoice>, Labels) {
    let empire = memory.empire;
    let home_sector = home.map(|record| game_data.planets[record - 1].sector.to_array());
    let roe = (weights.roe.round() as usize).min(10);
    // Worlds an ETAC already flies to are claimed before anything else.
    let mut claimed = claimed_worlds(game_data, masks);
    let mut scouted: Vec<usize> = Vec::new();
    let mut labels = Labels::default();
    let mut first_colonizer = true;
    let grabbing = land_grab(memory);
    let mut strongest = f32::NEG_INFINITY;
    let enemy_world = |idx: usize| {
        memory
            .planet_intel
            .get(&(idx + 1))
            .and_then(|intel| intel.known_owner_empire_id)
            .is_some_and(|owner| {
                owner != 0 && owner != empire && at_war_with(game_data, empire, owner)
            })
    };
    let blockading = |fleet: &Fleet| {
        fleet.owner.0 == empire
            && fleet.order == Order::GuardBlockadeWorld
            && world_at(game_data, fleet.target.to_array()).is_some_and(enemy_world)
    };
    let mut blockaded = game_data.fleets.iter().any(blockading);
    let home_threatened = home_sector.is_some_and(|sector| threatened(game_data, memory, sector));

    let fleets = masks
        .fleets
        .iter()
        .map(|mask| {
            let fleet = &game_data.fleets[mask.record - 1];
            let keep = FleetChoice {
                record: mask.record,
                mission: 0,
                target: 0,
                speed: 0,
                roe: if mask.roe[roe] { roe } else { 0 },
                load: false,
                detach: 0,
            };
            let order = |order: Order, target: usize| {
                let mission = mission_choice(order);
                (mask.mission[mission] && mask.targets[mission][target]).then_some(FleetChoice {
                    mission,
                    target,
                    ..keep
                })
            };

            if fleet.ships.etacs > 0 {
                // A colony fleet's job is to arrive, not to fight.
                let land_grab_doctrine = weights.doctrine.land_grab;
                let calm = |choice: FleetChoice| {
                    if land_grab_doctrine {
                        FleetChoice { roe: 0, ..choice }
                    } else {
                        choice
                    }
                };
                if fleet.order == Order::ColonizeWorld {
                    return calm(keep);
                }
                let first = std::mem::replace(&mut first_colonizer, false);
                let pick = match goals {
                    Some(goals) if first && goals.colonize > 0 => Some(goals.colonize - 1),
                    _ => best_world(game_data, memory, fleet, |idx, intel| {
                        (!claimed.contains(&idx) && colonizable(intel, empire)).then(|| {
                            f32::from(intel.known_potential_production.unwrap_or(40)) + 1.0
                        })
                    }),
                };
                if first && goals.is_none() {
                    labels.colonize = pick.map_or(0, |idx| idx + 1);
                }
                return calm(
                    pick.and_then(|idx| {
                        let choice = order(Order::ColonizeWorld, idx)?;
                        claimed.push(idx);
                        Some(choice)
                    })
                    .unwrap_or(keep),
                );
            }

            let scout = !has_combat_ships(fleet) && fleet.ships.scouts > 0 && troops(fleet) == 0;
            if scout || (weights.doctrine.land_grab && grabbing && picket(fleet)) {
                if matches!(fleet.order, Order::ScoutSolarSystem | Order::ViewWorld)
                    && fleet.sector != fleet.target
                {
                    return keep;
                }
                let year = game_data.conquest.game_year;
                return best_world(game_data, memory, fleet, |idx, intel| {
                    let stale = match intel.intel_tier {
                        IntelTier::Owned => return None,
                        IntelTier::Unknown => 30.0,
                        _ => f32::from(year.saturating_sub(intel.last_intel_year.unwrap_or(0))),
                    };
                    (!scouted.contains(&idx) && stale >= 5.0).then_some(stale)
                })
                .and_then(|idx| {
                    let look = if scout {
                        Order::ScoutSolarSystem
                    } else {
                        Order::ViewWorld
                    };
                    let choice = order(look, idx)?;
                    scouted.push(idx);
                    Some(choice)
                })
                .unwrap_or(keep);
            }

            // Warships and transports.
            let Some(home) = home else {
                return keep;
            };
            let home_idx = home - 1;
            let at_home = Some(fleet.sector.to_array()) == home_sector;
            if matches!(
                fleet.order,
                Order::InvadeWorld | Order::BombardWorld | Order::BlitzWorld
            ) && world_at(game_data, fleet.target.to_array())
                .and_then(|idx| memory.planet_intel.get(&(idx + 1)))
                .is_some_and(|intel| intel.known_owner_empire_id != Some(empire))
                && fleet.sector != fleet.target
            {
                return keep;
            }
            if weights.doctrine.blockader
                && has_combat_ships(fleet)
                && fleet.order == Order::GuardBlockadeWorld
            {
                if blockading(fleet) && !home_threatened {
                    return keep;
                }
                return order(Order::RendezvousSector, home_idx).unwrap_or(keep);
            }
            // A strategist's hold stops new invasions here too.
            let hold = goals.is_some_and(|goals| goals.strike == 0);
            if weights.doctrine.suppress_first
                && !hold
                && fleet.order == Order::BombardWorld
                && fleet.sector == fleet.target
                && has_combat_ships(fleet)
                && let Some(idx) = world_at(game_data, fleet.sector.to_array())
                && let Some(intel) = memory.planet_intel.get(&(idx + 1))
                && intel
                    .known_owner_empire_id
                    .is_some_and(|owner| owner != 0 && owner != empire)
            {
                // One year of bombardment softens the target; then land.
                if enough_troops(fleet, intel)
                    && let Some(choice) = order(Order::InvadeWorld, idx)
                {
                    return choice;
                }
            }
            if !at_home {
                return order(Order::RendezvousSector, home_idx).unwrap_or(keep);
            }
            let load = mask.load && fleet.ships.empty_transports > 0;
            let target = if !has_combat_ships(fleet) {
                None
            } else {
                match goals.map(|goals| goals.strike) {
                    Some(0) => None,
                    Some(goal) if can_take(memory, fleet, weights, goal - 1) => Some(goal - 1),
                    _ => {
                        let pick = strike_target(game_data, memory, fleet, weights);
                        if goals.is_none() && strength(fleet) > strongest {
                            strongest = strength(fleet);
                            labels.strike = pick.map_or(0, |idx| idx + 1);
                        }
                        pick
                    }
                }
            };
            if let Some(choice) =
                target.and_then(|target| strike(memory, mask, fleet, weights, target, keep))
            {
                return FleetChoice { load, ..choice };
            }
            if weights.doctrine.blockader
                && !blockaded
                && !home_threatened
                && fleet.order == Order::HoldPosition
                && has_combat_ships(fleet)
                && troops(fleet) == 0
                && let Some(choice) =
                    blockade_target(game_data, memory, fleet, weights, &enemy_world)
                        .and_then(|idx| order(Order::GuardBlockadeWorld, idx))
            {
                blockaded = true;
                return choice;
            }
            let gather = if fleet.order == Order::RendezvousSector && fleet.target == fleet.sector {
                keep
            } else {
                order(Order::RendezvousSector, home_idx).unwrap_or(keep)
            };
            FleetChoice { load, ..gather }
        })
        .collect();
    (fleets, labels)
}

/// Loaded armies enough to invade a world with its known (or assumed)
/// garrison: twice the defenders and one more.
fn enough_troops(fleet: &Fleet, intel: &PlanetIntelSnapshot) -> bool {
    let armies = intel.known_armies.map_or(UNKNOWN_ARMIES, f32::from);
    f32::from(fleet.ships.loaded_transports) >= 2.0 * armies + 1.0
}

/// The world's batteries are known to stand, or not known to be gone.
fn batteries_standing(intel: &PlanetIntelSnapshot) -> bool {
    intel
        .known_ground_batteries
        .is_none_or(|batteries| batteries > 0)
}

/// Who decides a fleet's orders in a hybrid seat (t157): fleets fall into
/// the executor's mission families by what they carry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Family {
    /// Fleets with ETACs.
    Colonize,
    /// Scouts, and one or two lone destroyers acting as pickets.
    Scout,
    /// Every other fleet: warships and transports.
    Strike,
}

/// The mission families a hybrid seat's network orders; the executor
/// orders the rest.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Families {
    pub colonize: bool,
    pub scout: bool,
    pub strike: bool,
}

impl Families {
    pub const NONE: Families = Families {
        colonize: false,
        scout: false,
        strike: false,
    };
    pub const ALL: Families = Families {
        colonize: true,
        scout: true,
        strike: true,
    };

    pub fn contains(&self, family: Family) -> bool {
        match family {
            Family::Colonize => self.colonize,
            Family::Scout => self.scout,
            Family::Strike => self.strike,
        }
    }
}

/// The family `fleet` belongs to.
pub fn fleet_family(fleet: &Fleet) -> Family {
    if fleet.ships.etacs > 0 {
        Family::Colonize
    } else if picket(fleet)
        || (!has_combat_ships(fleet) && fleet.ships.scouts > 0 && troops(fleet) == 0)
    {
        Family::Scout
    } else {
        Family::Strike
    }
}

/// One or two lone destroyers: a picket that views worlds during the land
/// grab, as the starting destroyers should.
fn picket(fleet: &Fleet) -> bool {
    let ships = &fleet.ships;
    (1..=2).contains(&ships.destroyers)
        && ships.battleships == 0
        && ships.cruisers == 0
        && ships.scouts == 0
        && ships.etacs == 0
        && troops(fleet) == 0
}

fn troops(fleet: &Fleet) -> u16 {
    fleet.ships.empty_transports + fleet.ships.loaded_transports
}

/// The world, by 0-based index, with the best `score` over distance.
fn best_world(
    game_data: &CoreGameData,
    memory: &SeatMemory,
    fleet: &Fleet,
    score: impl Fn(usize, &PlanetIntelSnapshot) -> Option<f32>,
) -> Option<usize> {
    let here = fleet.sector.to_array();
    game_data
        .planets
        .iter()
        .enumerate()
        .filter_map(|(idx, planet)| {
            let intel = memory.planet_intel.get(&(idx + 1))?;
            let value = score(idx, intel)?;
            Some((
                idx,
                value / (1.0 + distance(here, planet.sector.to_array())),
            ))
        })
        .max_by(|a, b| a.1.total_cmp(&b.1))
        .map(|(idx, _)| idx)
}

/// Whether `fleet` is `attack_ratio` times world `idx`'s estimated defense,
/// a world known to be another empire's.
fn can_take(memory: &SeatMemory, fleet: &Fleet, weights: &Weights, idx: usize) -> bool {
    memory.planet_intel.get(&(idx + 1)).is_some_and(|intel| {
        intel
            .known_owner_empire_id
            .is_some_and(|owner| owner != 0 && owner != memory.empire)
            && strength(fleet) >= weights.attack_ratio * defense(intel)
    })
}

/// The best known enemy world the home force can take or batter: the most
/// value, or with `strike_weakest` the most value per unit of estimated
/// defence, discounted by distance.
fn strike_target(
    game_data: &CoreGameData,
    memory: &SeatMemory,
    fleet: &Fleet,
    weights: &Weights,
) -> Option<usize> {
    best_world(game_data, memory, fleet, |idx, intel| {
        can_take(memory, fleet, weights, idx).then(|| {
            let value = f32::from(
                intel
                    .known_current_production
                    .map(u16::from)
                    .or(intel.known_potential_production)
                    .unwrap_or(20),
            ) + 1.0;
            if weights.doctrine.strike_weakest {
                value / defense(intel).max(1.0)
            } else {
                value
            }
        })
    })
}

/// The enemy world `fleet` should blockade: no fleet of the seat strikes
/// it, `fleet` is `attack_ratio` times its batteries and starbases, and it
/// has the most value per unit of estimated defence.
fn blockade_target(
    game_data: &CoreGameData,
    memory: &SeatMemory,
    fleet: &Fleet,
    weights: &Weights,
    enemy_world: &dyn Fn(usize) -> bool,
) -> Option<usize> {
    let struck = |idx: usize| {
        game_data.fleets.iter().any(|other| {
            other.owner.0 == memory.empire
                && matches!(
                    other.order,
                    Order::InvadeWorld | Order::BombardWorld | Order::BlitzWorld
                )
                && other.target == game_data.planets[idx].sector
        })
    };
    best_world(game_data, memory, fleet, |idx, intel| {
        let guns = 4.0
            * intel
                .known_ground_batteries
                .map_or(UNKNOWN_BATTERIES, f32::from)
            + 10.0 * intel.known_starbase_count.map_or(0.0, f32::from);
        (enemy_world(idx) && !struck(idx) && strength(fleet) >= weights.attack_ratio * guns).then(
            || {
                let value = f32::from(
                    intel
                        .known_current_production
                        .map(u16::from)
                        .or(intel.known_potential_production)
                        .unwrap_or(20),
                ) + 1.0;
                value / defense(intel).max(1.0)
            },
        )
    })
}

/// `empire` has declared `other` an enemy.
fn at_war_with(game_data: &CoreGameData, empire: u8, other: u8) -> bool {
    game_data.players[usize::from(empire) - 1]
        .diplomacy
        .get(usize::from(other) - 1)
        == Some(&nc_data::DiplomaticRelation::Enemy)
}

/// A declared enemy's force was reported near `sector` in recent years.
fn threatened(game_data: &CoreGameData, memory: &SeatMemory, sector: [u8; 2]) -> bool {
    let year = game_data.conquest.game_year;
    memory.contacts.iter().any(|contact| {
        contact.empire != 0
            && contact.empire != memory.empire
            && at_war_with(game_data, memory.empire, contact.empire)
            && year.saturating_sub(contact.year) < THREAT_YEARS
            && distance(contact.coords, sector) <= THREAT_RANGE
    })
}

/// Strike world `target`: Invade with enough armies aboard, otherwise
/// Bombard. With `suppress_first`, Bombard while its batteries may stand.
fn strike(
    memory: &SeatMemory,
    mask: &FleetMask,
    fleet: &Fleet,
    weights: &Weights,
    target: usize,
    keep: FleetChoice,
) -> Option<FleetChoice> {
    let intel = &memory.planet_intel[&(target + 1)];
    let invade = enough_troops(fleet, intel)
        && !(weights.doctrine.suppress_first && batteries_standing(intel));
    let mission = mission_choice(if invade {
        Order::InvadeWorld
    } else {
        Order::BombardWorld
    });
    (mask.mission[mission] && mask.targets[mission][target]).then_some(FleetChoice {
        mission,
        target,
        ..keep
    })
}
