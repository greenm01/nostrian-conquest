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
    BUILD_AMOUNT_CHOICES, BUILD_SLOTS, Detach, FleetChoice, FleetMask, MISSIONS, PlanetChoice,
    SeatAction, SeatMasks, TAX_STEP, build_units, detach_choice,
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
    /// Set the fleet allocation goal by threat and opportunity (t175):
    /// 1 when a declared enemy's force was reported near one of the seat's
    /// worlds in recent years, 2 when that force was near home and half the
    /// home force could still take its strike target (or it has none).
    /// Off, the executor always sets 0.
    pub allocate: bool,
    /// Count toward the transport target only transports at home or with
    /// troops aboard, so the ones left empty far away are replaced (t179,
    /// accepted). Off,
    /// every transport the empire owns counts.
    pub supply: bool,
    /// Defend by exposure (t181, accepted): a colony's defence follows its
    /// strategic situation. Exposure is 1 where a declared enemy's force was seen
    /// near it lately, and otherwise falls from 1 to 0 with distance to the
    /// nearest known rival world over [`EXPOSURE_RANGE`]. Its army target is
    /// its scaled garrison times one plus exposure, its battery target the
    /// weight times exposure, and while it is short of both, armies take at
    /// most half its budget. Interior worlds build warships instead. Off,
    /// every colony aims for the same batteries, armies first.
    pub defend: bool,
    /// Build every hull toward its weight's share of the empire's warships
    /// by cost (t189, accepted under t206's cover rules), so a
    /// personality's hull weights set its fleet mix. Off, the affordable
    /// hull with the largest weight, every time.
    pub mixed_hulls: bool,
    /// Round a positive scout target up, so a weight under one half still
    /// keeps a scout (t189, accepted). Off, it rounds like every other
    /// target and a small weight never replaces a lost scout.
    pub keep_scouts: bool,
    /// After the land grab, pickets and spare scouts stand watch in the
    /// approaches between the seat's exposed worlds and the nearest rival
    /// world, for early warning (t189, accepted). Off, pickets join the
    /// strike force and scouts revisit stale worlds.
    pub watch: bool,
    /// Warships and transports gather at the seat's world nearest a known
    /// rival world, not always at the home world, so a strike force that
    /// has taken a world launches its next strike from the front (t189,
    /// accepted). Off, every fleet returns home between strikes, which
    /// costs a slow force a long round trip each time.
    pub forward_muster: bool,
    /// After the land grab, keep [`PICKET_POSTS`] watchers on the
    /// approaches: while fewer stand watch, the first colony whose budget
    /// left after its garrison buys one or two destroyers builds them in
    /// place of its warships, so they commission alone as a picket, one a
    /// year (t203). Off, only the starting destroyers and scouts the weights
    /// buy stand watch.
    pub pickets: bool,
    /// Ferry armies between the seat's own worlds (t191): transports with no
    /// warships load a world's armies beyond its garrison target, sail
    /// (Move Only) to the colony furthest short of its own, weighted by
    /// exposure, and unload there. Off, a colony's armies are only those it
    /// builds, and transports carry armies only to invasions.
    pub ferry: bool,
}

impl Doctrine {
    /// Turn on the behaviour called `name` (`mixed_hulls`, `watch`,
    /// and so on), for trials.
    pub fn enable(&mut self, name: &str) -> Result<(), String> {
        let switch = match name {
            "land_grab" => &mut self.land_grab,
            "strike_weakest" => &mut self.strike_weakest,
            "suppress_first" => &mut self.suppress_first,
            "scaled_garrisons" => &mut self.scaled_garrisons,
            "blockader" => &mut self.blockader,
            "allocate" => &mut self.allocate,
            "supply" => &mut self.supply,
            "defend" => &mut self.defend,
            "mixed_hulls" => &mut self.mixed_hulls,
            "keep_scouts" => &mut self.keep_scouts,
            "watch" => &mut self.watch,
            "forward_muster" => &mut self.forward_muster,
            "pickets" => &mut self.pickets,
            "ferry" => &mut self.ferry,
            other => return Err(format!("unknown doctrine switch {other}")),
        };
        *switch = true;
        Ok(())
    }

    /// The accepted behaviours.
    pub const CURRENT: Doctrine = Doctrine {
        land_grab: true,
        strike_weakest: true,
        suppress_first: true,
        scaled_garrisons: true,
        blockader: true,
        allocate: false,
        supply: true,
        defend: true,
        mixed_hulls: true,
        keep_scouts: true,
        watch: true,
        forward_muster: true,
        pickets: false,
        ferry: false,
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
        tax: 62.702686,
        home_armies: 20.0,
        colony_armies: 9.745128,
        batteries: 9.870895,
        etacs: 6.282104,
        scouts: 0.21374373,
        starbases: 0.44496453,
        transports: 13.457997,
        battleship: 0.0,
        cruiser: 0.26427263,
        destroyer: 0.3204773,
        attack_ratio: 3.742486,
        roe: 4.784798,
        war_year: 24.076294,
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
            allocate: false,
            supply: false,
            defend: false,
            mixed_hulls: false,
            keep_scouts: false,
            watch: false,
            forward_muster: false,
            pickets: false,
            ferry: false,
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
/// Sectors over which a colony's exposure falls from 1 to 0 with distance
/// to the nearest known rival world (`Doctrine::defend`).
const EXPOSURE_RANGE: f32 = 8.0;
/// The most a colony's guard may hold: one battleship's worth.
const GUARD_STRENGTH: f32 = 9.0;
/// Watchers kept on the approaches under `Doctrine::pickets`: more lost
/// paired tests (t203).
const PICKET_POSTS: usize = 1;

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
        allocation: if weights.doctrine.allocate {
            allocation_rule(game_data, memory, home, weights, &labels)
        } else {
            0
        },
        strike: labels.strike,
        colonize: labels.colonize,
    }
}

/// The executor's allocation: hold back enough to meet the threat seen,
/// unless the strike target falls to the rest. See [`Doctrine::allocate`].
fn allocation_rule(
    game_data: &CoreGameData,
    memory: &SeatMemory,
    home: Option<usize>,
    weights: &Weights,
    labels: &Labels,
) -> usize {
    let threatened_world = game_data.planets.iter().any(|planet| {
        planet.owner.0 == memory.empire && threatened(game_data, memory, planet.sector.to_array())
    });
    if !threatened_world {
        return 0;
    }
    let home_threatened = home.is_some_and(|record| {
        threatened(
            game_data,
            memory,
            game_data.planets[record - 1].sector.to_array(),
        )
    });
    let half_takes = labels.strike == 0
        || memory
            .planet_intel
            .get(&labels.strike)
            .is_some_and(|intel| labels.force / 2.0 >= weights.attack_ratio * defense(intel));
    if home_threatened && half_takes { 2 } else { 1 }
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
    let home_sector = weights
        .doctrine
        .supply
        .then(|| home.map(|record| game_data.planets[record - 1].sector))
        .flatten();
    let mut stock = Stock::count(game_data, memory.empire, home_sector);
    let build_weights = posture_weights(weights, goals.posture);
    let mut picket = weights.doctrine.pickets && pickets_wanted(game_data, memory);
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
                &mut stock,
                memory,
                &build_weights,
                goals.allocation,
                &mut picket,
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

/// A picket is wanted this year (`Doctrine::pickets`): after the land
/// grab, while fewer than [`PICKET_POSTS`] watchers stand or wait in a
/// stardock and an approach is open.
fn pickets_wanted(game_data: &CoreGameData, memory: &SeatMemory) -> bool {
    if land_grab(memory) {
        return false;
    }
    let empire = memory.empire;
    let own_fleets = || {
        game_data
            .fleets
            .iter()
            .filter(|fleet| fleet.owner.0 == empire)
    };
    let docked = game_data
        .planets
        .iter()
        .filter(|planet| {
            let dock = &planet.stardock;
            planet.owner.0 == empire && dock.battleships == 0 && dock.cruisers == 0
        })
        .map(|planet| planet.stardock.destroyers as usize)
        .sum::<usize>();
    let watchers = own_fleets()
        .filter(|fleet| fleet_family(fleet) == Family::Scout)
        .count()
        + docked;
    let watched: Vec<[u8; 2]> = own_fleets()
        .filter(|fleet| matches!(fleet.order, Order::ScoutSector | Order::PatrolSector))
        .map(|fleet| fleet.target.to_array())
        .collect();
    watchers < PICKET_POSTS && approach(game_data, memory, &watched).is_some()
}

/// A colony's leftover budget buys a picket in place of the preferred
/// hull, if it buys one or two destroyers, so they commission alone.
fn picket_fits(points: u32) -> bool {
    let cost = ProductionItemKind::Destroyer
        .build_cost()
        .unwrap_or(1)
        .max(1);
    points >= cost && build_units(points, cost, 0) <= 2
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
    /// Warship build points, in [`HULLS`] order: in fleets, stardocks,
    /// queued, and bought this turn.
    hulls: [u32; 3],
}

/// The warship hulls, largest first.
const HULLS: [ProductionItemKind; 3] = [
    ProductionItemKind::Battleship,
    ProductionItemKind::Cruiser,
    ProductionItemKind::Destroyer,
];

impl Stock {
    /// With `home`, fleet transports count only at home or with troops
    /// aboard (`Doctrine::supply`).
    fn count(game_data: &CoreGameData, empire: u8, home: Option<nc_data::Coords>) -> Self {
        let mut stock = Stock {
            etacs: 0,
            scouts: 0,
            transports: 0,
            hulls: [0; 3],
        };
        for fleet in game_data
            .fleets
            .iter()
            .filter(|fleet| fleet.owner.0 == empire)
        {
            stock.etacs += u32::from(fleet.ships.etacs);
            stock.scouts += u32::from(fleet.ships.scouts);
            let ships = &fleet.ships;
            stock.add_hulls([
                u32::from(ships.battleships),
                u32::from(ships.cruisers),
                u32::from(ships.destroyers),
            ]);
            if home.is_none_or(|home| fleet.sector == home || fleet.ships.loaded_transports > 0) {
                stock.transports += u32::from(fleet.ships.empty_transports)
                    + u32::from(fleet.ships.loaded_transports);
            }
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
            let dock = &planet.stardock;
            stock.add_hulls([dock.battleships, dock.cruisers, dock.destroyers]);
            stock.add_hulls(HULLS.map(|kind| queued(planet, kind)));
        }
        stock
    }

    /// Add warships, counted in [`HULLS`] order.
    fn add_hulls(&mut self, counts: [u32; 3]) {
        for ((points, count), kind) in self.hulls.iter_mut().zip(counts).zip(HULLS) {
            *points += count * kind.build_cost().unwrap_or(1);
        }
    }

    fn bought(&mut self, kind: ProductionItemKind, units: u32) {
        if let Some(idx) = HULLS.iter().position(|&hull| hull == kind) {
            let mut counts = [0; 3];
            counts[idx] = units;
            self.add_hulls(counts);
        }
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
    stock: &mut Stock,
    memory: &SeatMemory,
    weights: &Weights,
    allocation: usize,
    picket: &mut bool,
) -> ([usize; BUILD_SLOTS], [usize; BUILD_SLOTS]) {
    if !weights.doctrine.land_grab {
        return classic_builds(game_data, record, allowed, is_home, stock, memory, weights);
    }
    let planet = &game_data.planets[record - 1];
    let short = |have: u32, want: f32| (want.round() as u32).saturating_sub(have);
    let scout_target = if weights.doctrine.keep_scouts && weights.scouts > 0.0 {
        weights.scouts.ceil()
    } else {
        weights.scouts
    };
    if weights.doctrine.defend && !is_home {
        let exposure = exposure(game_data, memory, planet.sector.to_array());
        let armies = army_target(game_data, memory, weights, record, false);
        return defended_builds(
            game_data,
            record,
            allowed,
            stock,
            memory,
            weights,
            allocation,
            short(planet.armies.into(), armies),
            short(planet.ground_batteries.into(), weights.batteries * exposure),
            picket,
        );
    }
    let garrison = [
        Need::Units(
            ProductionItemKind::Army,
            short(
                planet.armies.into(),
                army_target(game_data, memory, weights, record, is_home),
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
            short(stock.scouts, scout_target),
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
        // A threatened colony builds its guard, alone.
        if allocation >= 1
            && threatened(game_data, memory, planet.sector.to_array())
            && !guarded(game_data, memory.empire, planet.sector.to_array())
        {
            needs.push(Need::Warship);
        }
        needs.extend(garrison);
    }

    let mut points = game_data.planet_build_points_left(record).unwrap_or(0);
    let mut builds = [0; BUILD_SLOTS];
    let mut amounts = [0; BUILD_SLOTS];
    let mut slot = 0;
    let mut etacs_bought = 0;
    let mut guard_bought = 0;
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
            Need::Warship => {
                if let Some(kind) = preferred_hull(allowed, weights, points, stock) {
                    guard_bought = order(kind, 1, &mut points);
                    stock.bought(kind, guard_bought);
                }
            }
        }
    }
    // A guard commissions alone, so the rest of the budget waits a year;
    // so does a picket.
    if guard_bought == 0
        && !is_home
        && *picket
        && picket_fits(points)
        && order(ProductionItemKind::Destroyer, 1, &mut points) > 0
    {
        *picket = false;
    } else if guard_bought == 0
        && let Some(kind) = preferred_hull(allowed, weights, points, stock)
    {
        let bought = order(kind, u32::MAX, &mut points);
        stock.bought(kind, bought);
    }
    (builds, amounts)
}

/// The armies the seat wants on world `record`: the home garrison, or a
/// colony's, scaled by its production and, under `defend`, by its
/// exposure.
fn army_target(
    game_data: &CoreGameData,
    memory: &SeatMemory,
    weights: &Weights,
    record: usize,
    is_home: bool,
) -> f32 {
    let planet = &game_data.planets[record - 1];
    if is_home {
        return weights.home_armies;
    }
    if !weights.doctrine.scaled_garrisons && !weights.doctrine.defend {
        return weights.colony_armies;
    }
    let production = f32::from(nc_data::present_production_points(planet));
    let armies = (weights.colony_armies * production / 25.0).max(1.0);
    if weights.doctrine.defend {
        armies * (1.0 + exposure(game_data, memory, planet.sector.to_array()))
    } else {
        armies
    }
}

/// A colony's builds under `Doctrine::defend`: its guard when threatened,
/// then armies and batteries in balance, then the preferred hull.
#[allow(clippy::too_many_arguments)]
fn defended_builds(
    game_data: &CoreGameData,
    record: usize,
    allowed: &[bool],
    stock: &mut Stock,
    memory: &SeatMemory,
    weights: &Weights,
    allocation: usize,
    armies: u32,
    batteries: u32,
    picket: &mut bool,
) -> ([usize; BUILD_SLOTS], [usize; BUILD_SLOTS]) {
    let planet = &game_data.planets[record - 1];
    let mut points = game_data.planet_build_points_left(record).unwrap_or(0);
    let mut builds = [0; BUILD_SLOTS];
    let mut amounts = [0; BUILD_SLOTS];
    let mut slot = 0;
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
    if allocation >= 1
        && threatened(game_data, memory, planet.sector.to_array())
        && !guarded(game_data, memory.empire, planet.sector.to_array())
        && let Some(kind) = preferred_hull(allowed, weights, points, stock)
        && order(kind, 1, &mut points) > 0
    {
        stock.bought(kind, 1);
        // A guard commissions alone, so the rest waits a year.
        return (builds, amounts);
    }
    if armies > 0 && batteries > 0 {
        let mut half = points / 2;
        let budget = half;
        order(ProductionItemKind::Army, armies, &mut half);
        points -= budget - half;
    } else {
        order(ProductionItemKind::Army, armies, &mut points);
    }
    order(ProductionItemKind::GroundBattery, batteries, &mut points);
    if *picket && picket_fits(points) && order(ProductionItemKind::Destroyer, 1, &mut points) > 0 {
        *picket = false;
    } else if let Some(kind) = preferred_hull(allowed, weights, points, stock) {
        let bought = order(kind, u32::MAX, &mut points);
        stock.bought(kind, bought);
    }
    (builds, amounts)
}

/// How exposed a world is, 0 to 1 (`Doctrine::defend`).
fn exposure(game_data: &CoreGameData, memory: &SeatMemory, sector: [u8; 2]) -> f32 {
    if threatened(game_data, memory, sector) {
        return 1.0;
    }
    memory
        .planet_intel
        .iter()
        .filter(|(_, intel)| {
            intel
                .known_owner_empire_id
                .is_some_and(|owner| owner != 0 && owner != memory.empire)
        })
        .map(|(&record, _)| distance(game_data.planets[record - 1].sector.to_array(), sector))
        .fold(None, |nearest: Option<f32>, d| {
            Some(nearest.map_or(d, |n| n.min(d)))
        })
        .map_or(0.0, |nearest| {
            (1.0 - nearest / EXPOSURE_RANGE).clamp(0.0, 1.0)
        })
}

/// The hull to build with `points`: the affordable one with the largest
/// weight, or under `Doctrine::mixed_hulls` the affordable one furthest
/// below its weight's share of the empire's warships by cost.
fn preferred_hull(
    allowed: &[bool],
    weights: &Weights,
    points: u32,
    stock: &Stock,
) -> Option<ProductionItemKind> {
    let hull_weights = [weights.battleship, weights.cruiser, weights.destroyer];
    let total_weight: f32 = hull_weights.iter().filter(|&&w| w > 0.0).sum();
    let total_points: u32 = stock.hulls.iter().sum();
    HULLS
        .into_iter()
        .zip(hull_weights)
        .zip(stock.hulls)
        .filter(|((kind, weight), _)| {
            *weight > 0.0
                && allowed[build_choice(*kind)]
                && points >= kind.build_cost().unwrap_or(1).max(1)
        })
        .map(|((kind, weight), have)| {
            let score = if weights.doctrine.mixed_hulls && total_points > 0 {
                weight / total_weight - have as f32 / total_points as f32
            } else {
                weight
            };
            (kind, score)
        })
        .max_by(|a, b| a.1.total_cmp(&b.1))
        .map(|(kind, _)| kind)
}

/// The seat's world nearest a known rival world, ties going to `home`: the
/// forward base its warships gather at (`Doctrine::forward_muster`).
fn muster_world(
    game_data: &CoreGameData,
    memory: &SeatMemory,
    home: Option<usize>,
) -> Option<usize> {
    let rivals: Vec<[u8; 2]> = memory
        .planet_intel
        .iter()
        .filter(|(_, intel)| {
            intel
                .known_owner_empire_id
                .is_some_and(|owner| owner != 0 && owner != memory.empire)
        })
        .map(|(&record, _)| game_data.planets[record - 1].sector.to_array())
        .collect();
    if rivals.is_empty() {
        return home;
    }
    game_data
        .planets
        .iter()
        .enumerate()
        .filter(|(_, planet)| planet.owner.0 == memory.empire)
        .map(|(idx, planet)| {
            let here = planet.sector.to_array();
            let gap = rivals
                .iter()
                .map(|&rival| distance(rival, here))
                .fold(f32::MAX, f32::min);
            (
                idx + 1,
                gap + if Some(idx + 1) == home { 0.0 } else { 0.01 },
            )
        })
        .min_by(|a, b| a.1.total_cmp(&b.1))
        .map(|(record, _)| record)
        .or(home)
}

/// The sector between the seat's most exposed world and the nearest rival
/// world that no fleet watches, with that rival's world index: within
/// [`EXPOSURE_RANGE`], the closest pair first.
fn approach(
    game_data: &CoreGameData,
    memory: &SeatMemory,
    watched: &[[u8; 2]],
) -> Option<([u8; 2], usize)> {
    let rivals: Vec<usize> = memory
        .planet_intel
        .iter()
        .filter(|(_, intel)| {
            intel
                .known_owner_empire_id
                .is_some_and(|owner| owner != 0 && owner != memory.empire)
        })
        .map(|(&record, _)| record - 1)
        .collect();
    game_data
        .planets
        .iter()
        .filter(|planet| planet.owner.0 == memory.empire)
        .filter_map(|planet| {
            let here = planet.sector.to_array();
            let (rival, gap) = rivals
                .iter()
                .map(|&idx| {
                    (
                        idx,
                        distance(game_data.planets[idx].sector.to_array(), here),
                    )
                })
                .min_by(|a, b| a.1.total_cmp(&b.1))?;
            let there = game_data.planets[rival].sector.to_array();
            let middle = std::array::from_fn(|axis| {
                ((f32::from(here[axis]) + f32::from(there[axis])) / 2.0).round() as u8
            });
            (gap <= EXPOSURE_RANGE && !watched.iter().any(|&w| distance(w, middle) < 1.5))
                .then_some((gap, middle, rival))
        })
        .min_by(|a, b| a.0.total_cmp(&b.0))
        .map(|(_, middle, rival)| (middle, rival))
}

/// One of `empire`'s warship fleets guards `sector`, or is on its way.
fn guarded(game_data: &CoreGameData, empire: u8, sector: [u8; 2]) -> bool {
    game_data.fleets.iter().any(|fleet| {
        fleet.owner.0 == empire
            && has_combat_ships(fleet)
            && guarding(fleet)
            && fleet.target.to_array() == sector
    })
}

fn guarding(fleet: &Fleet) -> bool {
    matches!(
        fleet.order,
        Order::GuardStarbase | Order::GuardBlockadeWorld
    )
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
    /// One warship of the preferred hull, to guard the world.
    Warship,
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
    /// The strength of the home force the strike label is for.
    force: f32,
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
    let capital = home;
    // Here "home" is where warships and transports gather.
    let home = if weights.doctrine.forward_muster {
        muster_world(game_data, memory, home)
    } else {
        home
    };
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
    let allocation = goals.map_or(0, |goals| goals.allocation);
    // Sectors where a fleet of the seat already stands watch.
    let mut watched: Vec<[u8; 2]> = game_data
        .fleets
        .iter()
        .filter(|fleet| {
            fleet.owner.0 == empire
                && matches!(fleet.order, Order::ScoutSector | Order::PatrolSector)
        })
        .map(|fleet| fleet.target.to_array())
        .collect();
    // Sectors given a guard this year, and whether a strike force is out.
    let mut posted: Vec<[u8; 2]> = Vec::new();
    // Worlds a ferry already sails to with armies aboard.
    let mut ferried: Vec<usize> = game_data
        .fleets
        .iter()
        .filter(|fleet| {
            fleet.owner.0 == empire
                && fleet.order == Order::MoveOnly
                && fleet.ships.loaded_transports > 0
                && !has_combat_ships(fleet)
        })
        .filter_map(|fleet| world_at(game_data, fleet.target.to_array()))
        .collect();
    let mut sailed = game_data.fleets.iter().any(|fleet| {
        fleet.owner.0 == empire
            && has_combat_ships(fleet)
            && matches!(
                fleet.order,
                Order::InvadeWorld | Order::BombardWorld | Order::BlitzWorld
            )
    });

    let fleets = masks
        .fleets
        .iter()
        .map(|mask| {
            let fleet = &game_data.fleets[mask.record - 1];
            let keep = FleetChoice {
                record: mask.record,
                mission: 0,
                target: 0,
                waypoint: 0,
                speed: 0,
                roe: if mask.roe[roe] { roe } else { 0 },
                load: false,
                detach: 0,
                join: 0,
                station: None,
                unload: false,
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
            if weights.doctrine.watch && !grabbing && (scout || picket(fleet)) {
                let watching = matches!(fleet.order, Order::ScoutSector | Order::PatrolSector);
                if watching && watched.contains(&fleet.target.to_array()) {
                    return keep;
                }
                if let Some((sector, rival)) = approach(game_data, memory, &watched) {
                    let look = if scout {
                        Order::ScoutSector
                    } else {
                        Order::PatrolSector
                    };
                    if let Some(choice) = order(look, rival) {
                        watched.push(sector);
                        return FleetChoice {
                            station: Some(sector),
                            ..choice
                        };
                    }
                }
            }
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
            let sector = fleet.sector.to_array();
            let own_world = |idx: usize| game_data.planets[idx].owner.0 == empire;
            if weights.doctrine.ferry && !has_combat_ships(fleet) && troops(fleet) > 0 {
                let here = world_at(game_data, sector).filter(|&idx| own_world(idx));
                // Armies a world lacks for its garrison target; negative
                // when it has more.
                let short = |idx: usize| {
                    army_target(
                        game_data,
                        memory,
                        weights,
                        idx + 1,
                        Some(idx + 1) == capital,
                    ) - f32::from(game_data.planets[idx].armies)
                };
                let loaded = fleet.ships.loaded_transports > 0;
                if loaded
                    && mask.unload
                    && let Some(idx) = here
                    && Some(idx + 1) != capital
                    && short(idx) >= 1.0
                {
                    return FleetChoice {
                        unload: true,
                        ..keep
                    };
                }
                if loaded && fleet.order == Order::MoveOnly && fleet.sector != fleet.target {
                    return keep;
                }
                // Load only armies the world can spare: every empty
                // transport fills, so the world keeps its own target.
                let spare = here.is_some_and(|idx| {
                    mask.load && short(idx) + f32::from(fleet.ships.empty_transports) <= 0.0
                });
                if loaded || spare {
                    let needy = (0..game_data.planets.len())
                        .filter(|&idx| {
                            own_world(idx)
                                && Some(idx + 1) != capital
                                && Some(idx) != here
                                && !ferried.contains(&idx)
                                && short(idx) >= 1.0
                        })
                        .filter_map(|idx| Some((idx, order(Order::MoveOnly, idx)?)))
                        .max_by(|(a, _), (b, _)| {
                            let away = |idx: usize| {
                                distance(sector, game_data.planets[idx].sector.to_array())
                            };
                            short(*a)
                                .total_cmp(&short(*b))
                                .then(away(*b).total_cmp(&away(*a)))
                        });
                    if let Some((idx, choice)) = needy {
                        ferried.push(idx);
                        return FleetChoice {
                            load: !loaded,
                            ..choice
                        };
                    }
                }
            }
            // Guards keep station while the allocation holds a share back.
            if allocation >= 1
                && has_combat_ships(fleet)
                && guarding(fleet)
                && world_at(game_data, fleet.target.to_array()).is_some_and(own_world)
            {
                return keep;
            }
            let mut post = |idx: usize, sector: [u8; 2]| {
                if posted.contains(&sector) || guarded(game_data, empire, sector) {
                    return None;
                }
                let choice = order(Order::GuardStarbase, idx)
                    .or_else(|| order(Order::GuardBlockadeWorld, idx))?;
                posted.push(sector);
                Some(choice)
            };
            // A threatened colony's new guard takes its post.
            if allocation >= 1
                && !at_home
                && fleet.order == Order::HoldPosition
                && has_combat_ships(fleet)
                && strength(fleet) <= GUARD_STRENGTH
                && let Some(idx) = world_at(game_data, sector)
                && own_world(idx)
                && threatened(game_data, memory, sector)
                && let Some(choice) = post(idx, sector)
            {
                return choice;
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
            let force = strength(fleet);
            let target = if !has_combat_ships(fleet) {
                None
            } else {
                match goals.map(|goals| goals.strike) {
                    Some(0) => None,
                    Some(goal) if can_take(memory, force, weights, goal - 1) => Some(goal - 1),
                    _ => {
                        let pick = strike_target(game_data, memory, fleet, force, weights);
                        if goals.is_none() && force > strongest {
                            strongest = force;
                            labels.strike = pick.map_or(0, |idx| idx + 1);
                            labels.force = strongest;
                        }
                        pick
                    }
                }
            };
            if let Some(choice) =
                target.and_then(|target| strike(memory, mask, fleet, weights, target, keep))
            {
                sailed = true;
                // Allocation 2 keeps half the warships at home.
                let half = detach_choice(Detach::HalfWarships);
                let detach = if allocation >= 2 && mask.detach[half] {
                    half
                } else {
                    0
                };
                return FleetChoice {
                    load,
                    detach,
                    ..choice
                };
            }
            // Once the strike force is out, new warships guard home.
            if allocation >= 1
                && sailed
                && fleet.order == Order::HoldPosition
                && has_combat_ships(fleet)
                && let Some(home_sector) = home_sector
                && let Some(choice) = post(home_idx, home_sector)
            {
                return choice;
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

/// Let a hybrid seat's fleets join only fleets its network orders too, so a
/// strike force never absorbs the executor's scouts or pickets (t190).
pub fn network_joins(game_data: &CoreGameData, masks: &mut SeatMasks, families: Families) {
    let ordered: Vec<bool> = masks
        .fleets
        .iter()
        .map(|mask| families.contains(fleet_family(&game_data.fleets[mask.record - 1])))
        .collect();
    for mask in &mut masks.fleets {
        for (host, &allowed) in ordered.iter().enumerate() {
            mask.join[host + 1] &= allowed;
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
fn can_take(memory: &SeatMemory, force: f32, weights: &Weights, idx: usize) -> bool {
    memory.planet_intel.get(&(idx + 1)).is_some_and(|intel| {
        intel
            .known_owner_empire_id
            .is_some_and(|owner| owner != 0 && owner != memory.empire)
            && force >= weights.attack_ratio * defense(intel)
    })
}

/// The best known enemy world the home force can take or batter: the most
/// value, or with `strike_weakest` the most value per unit of estimated
/// defence, discounted by distance.
fn strike_target(
    game_data: &CoreGameData,
    memory: &SeatMemory,
    fleet: &Fleet,
    force: f32,
    weights: &Weights,
) -> Option<usize> {
    best_world(game_data, memory, fleet, |idx, intel| {
        can_take(memory, force, weights, idx).then(|| {
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
