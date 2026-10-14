//! The scripted executor: a seat's whole turn from a handful of strategy
//! weights, chosen through the same masks and actions a learned seat uses.
//!
//! It reads only what [`crate::observe`] may read: the empire's own records,
//! its [`SeatMemory`], and public world positions.
//!
//! Each year it:
//! - sets tax from the weights;
//! - declares every other empire an enemy once `war_year` years have passed;
//! - at each world builds the first unmet need: armies, then batteries, then
//!   at the home world ETACs, scouts, starbases and transports, then the
//!   preferred hull it can afford;
//! - sends ETAC fleets to colonize the most valuable world not known to be
//!   owned, one fleet per world;
//! - sends scouts to the nearest world it knows least about;
//! - gathers warships and transports at home by Rendezvous, loads armies
//!   there, and strikes the best known enemy world once the home force is
//!   `attack_ratio` times its estimated defense: Invade with enough armies
//!   aboard, otherwise Bombard.

use nc_data::{
    CoreGameData, Fleet, IntelTier, Order, PlanetIntelSnapshot, ProductionItemKind,
    has_combat_ships,
};
use nc_engine::BUILD_UNITS;

use crate::action::{
    BUILD_AMOUNT_CHOICES, FleetChoice, FleetMask, MISSIONS, PlanetChoice, SeatAction, SeatMasks,
    TAX_STEP,
};
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
    };

    /// Weights tuned by evolution against aggressor seats; see the
    /// `tune_executor` example.
    pub const TUNED: Weights = Weights {
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
        }
    }
}

/// Assumed defense facts for a world whose intel lacks them.
const UNKNOWN_ARMIES: f32 = 6.0;
const UNKNOWN_BATTERIES: f32 = 3.0;

/// The executor's turn for `memory`'s empire, within `masks`. `start_year`
/// is the game's first year.
pub fn plan_turn(
    game_data: &CoreGameData,
    memory: &SeatMemory,
    masks: &SeatMasks,
    weights: &Weights,
    start_year: u16,
) -> SeatAction {
    let empire = memory.empire;
    let year = game_data.conquest.game_year;
    let at_war = f32::from(year.saturating_sub(start_year)) >= weights.war_year;
    let home = home_world(game_data, empire);
    let stock = Stock::count(game_data, empire);

    let planets = masks
        .planets
        .iter()
        .map(|mask| {
            let is_home = Some(mask.record) == home;
            let (build, amount) = choose_build(
                game_data,
                mask.record,
                &mask.build,
                is_home,
                &stock,
                memory,
                weights,
            );
            PlanetChoice {
                record: mask.record,
                build,
                amount,
            }
        })
        .collect();

    let fleets = plan_fleets(game_data, memory, masks, weights, home);

    SeatAction {
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

/// The smallest amount choice that buys `units` of `kind`, or all of the
/// budget when it cannot.
fn amount_for(
    game_data: &CoreGameData,
    record: usize,
    kind: ProductionItemKind,
    units: u32,
) -> usize {
    let cost = kind.build_cost().unwrap_or(1).max(1);
    let most = game_data.planet_build_points_left(record).unwrap_or(0) / cost;
    (0..BUILD_AMOUNT_CHOICES)
        .find(|&amount| (most * (amount as u32 + 1)).div_ceil(4) >= units)
        .unwrap_or(BUILD_AMOUNT_CHOICES - 1)
}

#[allow(clippy::too_many_arguments)]
fn choose_build(
    game_data: &CoreGameData,
    record: usize,
    allowed: &[bool],
    is_home: bool,
    stock: &Stock,
    memory: &SeatMemory,
    weights: &Weights,
) -> (usize, usize) {
    let planet = &game_data.planets[record - 1];
    let short = |have: u32, want: f32| (want.round() as u32).saturating_sub(have);
    let mut needs = vec![
        (
            ProductionItemKind::Army,
            short(
                planet.armies.into(),
                if is_home {
                    weights.home_armies
                } else {
                    weights.colony_armies
                },
            ),
        ),
        (
            ProductionItemKind::GroundBattery,
            short(planet.ground_batteries.into(), weights.batteries),
        ),
    ];
    if is_home {
        let open_worlds = memory
            .planet_intel
            .values()
            .any(|intel| colonizable(intel, memory.empire));
        if open_worlds {
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
    for (kind, units) in needs {
        let choice = build_choice(kind);
        if units > 0 && allowed[choice] {
            return (choice, amount_for(game_data, record, kind, units));
        }
    }
    let hulls = [
        (ProductionItemKind::Battleship, weights.battleship),
        (ProductionItemKind::Cruiser, weights.cruiser),
        (ProductionItemKind::Destroyer, weights.destroyer),
    ];
    hulls
        .iter()
        .filter(|(kind, weight)| *weight > 0.0 && allowed[build_choice(*kind)])
        .max_by(|a, b| a.1.total_cmp(&b.1))
        .map_or((0, 0), |(kind, _)| {
            (build_choice(*kind), BUILD_AMOUNT_CHOICES - 1)
        })
}

fn home_starbases(game_data: &CoreGameData, planet: &nc_data::Planet) -> u32 {
    game_data
        .bases
        .iter()
        .filter(|base| base.owner == planet.owner && base.sector == planet.sector)
        .count() as u32
}

/// A world worth sending an ETAC to: not known to be owned by anyone.
fn colonizable(intel: &PlanetIntelSnapshot, empire: u8) -> bool {
    intel.intel_tier != IntelTier::Owned
        && intel.known_owner_empire_id.is_none_or(|owner| owner == 0)
        && intel.known_owner_empire_id != Some(empire)
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

fn plan_fleets(
    game_data: &CoreGameData,
    memory: &SeatMemory,
    masks: &SeatMasks,
    weights: &Weights,
    home: Option<usize>,
) -> Vec<FleetChoice> {
    let empire = memory.empire;
    let home_sector = home.map(|record| game_data.planets[record - 1].sector.to_array());
    let roe = (weights.roe.round() as usize).min(10);
    let mut claimed: Vec<usize> = Vec::new();
    let mut scouted: Vec<usize> = Vec::new();

    // Worlds an ETAC already flies to are claimed before anything else.
    for mask in &masks.fleets {
        let fleet = &game_data.fleets[mask.record - 1];
        if fleet.ships.etacs > 0
            && fleet.order == Order::ColonizeWorld
            && let Some(idx) = world_at(game_data, fleet.target.to_array())
        {
            claimed.push(idx);
        }
    }

    masks
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
                if fleet.order == Order::ColonizeWorld {
                    return keep;
                }
                return best_world(game_data, memory, fleet, |idx, intel| {
                    (!claimed.contains(&idx) && colonizable(intel, empire))
                        .then(|| f32::from(intel.known_potential_production.unwrap_or(40)) + 1.0)
                })
                .and_then(|idx| {
                    let choice = order(Order::ColonizeWorld, idx)?;
                    claimed.push(idx);
                    Some(choice)
                })
                .unwrap_or(keep);
            }

            if !has_combat_ships(fleet) && fleet.ships.scouts > 0 && troops(fleet) == 0 {
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
                    let choice = order(Order::ScoutSolarSystem, idx)?;
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
            if !at_home {
                return order(Order::RendezvousSector, home_idx).unwrap_or(keep);
            }
            let load = mask.load && fleet.ships.empty_transports > 0;
            if let Some(choice) = strike(game_data, memory, mask, fleet, weights, keep) {
                return FleetChoice { load, ..choice };
            }
            let gather = if fleet.order == Order::RendezvousSector && fleet.target == fleet.sector {
                keep
            } else {
                order(Order::RendezvousSector, home_idx).unwrap_or(keep)
            };
            FleetChoice { load, ..gather }
        })
        .collect()
}

fn troops(fleet: &Fleet) -> u16 {
    fleet.ships.empty_transports + fleet.ships.loaded_transports
}

fn world_at(game_data: &CoreGameData, coords: [u8; 2]) -> Option<usize> {
    game_data
        .planets
        .iter()
        .position(|planet| planet.sector.to_array() == coords)
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

/// Strike the best known enemy world the home force can take or batter.
fn strike(
    game_data: &CoreGameData,
    memory: &SeatMemory,
    mask: &FleetMask,
    fleet: &Fleet,
    weights: &Weights,
    keep: FleetChoice,
) -> Option<FleetChoice> {
    if !has_combat_ships(fleet) {
        return None;
    }
    let empire = memory.empire;
    let power = strength(fleet);
    let target = best_world(game_data, memory, fleet, |_, intel| {
        let owner = intel.known_owner_empire_id?;
        if owner == 0 || owner == empire {
            return None;
        }
        (power >= weights.attack_ratio * defense(intel)).then(|| {
            f32::from(
                intel
                    .known_current_production
                    .map(u16::from)
                    .or(intel.known_potential_production)
                    .unwrap_or(20),
            ) + 1.0
        })
    })?;
    let intel = &memory.planet_intel[&(target + 1)];
    let armies = intel.known_armies.map_or(UNKNOWN_ARMIES, f32::from);
    let invade = f32::from(fleet.ships.loaded_transports) >= 2.0 * armies + 1.0;
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
