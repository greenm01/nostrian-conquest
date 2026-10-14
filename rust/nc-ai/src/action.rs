//! A seat's orders as factored choices, legality masks drawn from the
//! game's own validators, and decoding into a [`TurnSubmission`].
//!
//! A seat chooses, all at once:
//! - a tax rate, in steps of [`TAX_STEP`];
//! - for every other empire, to keep, make neutral or declare enemy;
//! - for each world it owns, a unit to build (or none) and how much of the
//!   world's remaining budget to spend on it;
//! - for each fleet it owns, whether to load armies from the world it is at,
//!   a mission (or keep its orders), a target world, a speed and rules of
//!   engagement. Loading and an invasion cannot share a year: orders are
//!   checked against the turn-start fleet, so a fleet invades the year after
//!   it loads.
//!
//! Masks read only what the empire may know. Fleet missions and targets go
//! through `validate_fleet_order_payload`, which checks the fleet's own ships
//! and whether a world at the target is the fleet's own; whether a world
//! exists at a sector is public. Every target is a world's sector. Guard a
//! Starbase and Join need a starbase or fleet target and are not offered yet.
//!
//! A learned seat commissions every finished ship before it chooses, through
//! [`commission_submission`], so its masks see the fleets that creates.

use std::collections::BTreeMap;

use nc_data::{
    CoreGameData, DiplomaticRelation, Fleet, FleetTurnAction, FleetTurnBlock, Order,
    PlanetTurnAction, PlanetTurnBlock, STARDOCK_SLOT_COUNT, TurnDiplomacyDirective, TurnSubmission,
    has_combat_ships, is_support_only,
};
use nc_engine::{BUILD_UNITS, fleet_record_supports_mission_code, production_item_kind_code};

/// Tax rates offered: 0, 5, ... 100.
pub const TAX_STEP: u8 = 5;
pub const TAX_CHOICES: usize = 21;
/// Keep the current relation, make it neutral, or declare enemy.
pub const DIPLOMACY_CHOICES: usize = 3;
/// No build, or one entry per [`BUILD_UNITS`] row.
pub const BUILD_CHOICES: usize = 1 + BUILD_UNITS.len();
/// A quarter, half, three quarters or all of the world's remaining budget.
pub const BUILD_AMOUNT_CHOICES: usize = 4;
/// Keep the fleet's orders, or one entry per [`MISSIONS`] row.
pub const MISSION_CHOICES: usize = 1 + MISSIONS.len();
/// Full, three-quarter, half or quarter speed, rounded up.
pub const SPEED_CHOICES: usize = 4;
/// Rules of engagement 0 to 10.
pub const ROE_CHOICES: usize = 11;

/// The missions a seat can give, in mission-choice order.
pub const MISSIONS: [Order; 14] = [
    Order::HoldPosition,
    Order::MoveOnly,
    Order::SeekHome,
    Order::PatrolSector,
    Order::GuardBlockadeWorld,
    Order::BombardWorld,
    Order::InvadeWorld,
    Order::BlitzWorld,
    Order::ViewWorld,
    Order::ScoutSector,
    Order::ScoutSolarSystem,
    Order::ColonizeWorld,
    Order::RendezvousSector,
    Order::Salvage,
];

/// What may be chosen for one world the seat owns.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlanetMask {
    /// 1-based planet record.
    pub record: usize,
    pub build: [bool; BUILD_CHOICES],
}

/// What may be chosen for one fleet the seat owns.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FleetMask {
    /// 1-based fleet record.
    pub record: usize,
    pub mission: [bool; MISSION_CHOICES],
    /// By mission choice, one flag per world in planet-record order. Keep,
    /// Hold Position and Seek Home take no target: every world is allowed
    /// and the choice is ignored.
    pub targets: Vec<Vec<bool>>,
    pub roe: [bool; ROE_CHOICES],
    /// The fleet is at one of the empire's worlds with an army to spare and
    /// has an empty transport.
    pub load: bool,
}

/// Everything one seat may choose this year. Tax, speed and build-amount
/// choices are always all allowed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SeatMasks {
    pub empire: u8,
    /// One row per empire slot, 1 to the player count; the seat's own row
    /// allows only "keep".
    pub diplomacy: Vec<[bool; DIPLOMACY_CHOICES]>,
    pub planets: Vec<PlanetMask>,
    pub fleets: Vec<FleetMask>,
}

/// One world's choice, by index into its [`PlanetMask`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlanetChoice {
    pub record: usize,
    pub build: usize,
    pub amount: usize,
}

/// One fleet's choice, by index into its [`FleetMask`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FleetChoice {
    pub record: usize,
    pub mission: usize,
    /// 0-based world index.
    pub target: usize,
    pub speed: usize,
    pub roe: usize,
    /// Load as many armies as the empty transports hold, leaving the world
    /// at least one.
    pub load: bool,
}

/// One seat's whole turn as choice indexes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SeatAction {
    pub tax: usize,
    /// One entry per empire slot, as in [`SeatMasks::diplomacy`].
    pub diplomacy: Vec<usize>,
    pub planets: Vec<PlanetChoice>,
    pub fleets: Vec<FleetChoice>,
}

/// A decoded turn, and the fleets whose new orders a fleet decoded earlier
/// in the same turn ruled out (two fleets colonizing one world).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecodedTurn {
    pub submission: TurnSubmission,
    pub dropped_fleet_orders: Vec<usize>,
}

/// A submission that commissions every unit in `empire`'s stardocks, or
/// `None` when no stardock holds one.
pub fn commission_submission(game_data: &CoreGameData, empire: u8) -> Option<TurnSubmission> {
    let owned: Vec<usize> = owned_records(
        game_data.planets.iter().map(|planet| planet.owner.0),
        empire,
    )
    .collect();
    let waiting = owned.iter().any(|&record| {
        let dock = &game_data.planets[record - 1].stardock;
        (0..STARDOCK_SLOT_COUNT).any(|slot| dock[slot] > 0)
    });
    let first = *owned.first()?;
    waiting.then(|| TurnSubmission {
        player_record_index_1_based: usize::from(empire),
        year: game_data.conquest.game_year,
        tax_rate: None,
        diplomacy: Vec::new(),
        planets: vec![PlanetTurnBlock {
            planet_record_index_1_based: first,
            actions: vec![PlanetTurnAction::AutoCommission],
        }],
        fleets: Vec::new(),
        messages: Vec::new(),
    })
}

/// What `empire` may choose in `game_data` this year.
pub fn seat_masks(game_data: &CoreGameData, empire: u8) -> SeatMasks {
    let player_count = game_data.conquest.player_count;
    SeatMasks {
        empire,
        diplomacy: (1..=player_count)
            .map(|other| {
                let others = other != empire;
                [true, others, others]
            })
            .collect(),
        planets: owned_records(
            game_data.planets.iter().map(|planet| planet.owner.0),
            empire,
        )
        .map(|record| planet_mask(game_data, record))
        .collect(),
        fleets: {
            let mut shared = BTreeMap::new();
            owned_records(game_data.fleets.iter().map(|fleet| fleet.owner.0), empire)
                .map(|record| fleet_mask(game_data, record, &mut shared))
                .collect()
        },
    }
}

/// What `validate_fleet_order_payload` reads of a fleet besides its owner.
type ShipsKey = (bool, bool, bool, bool);

fn ships_key(fleet: &Fleet) -> ShipsKey {
    (
        has_combat_ships(fleet),
        fleet.ships.scouts > 0,
        fleet.ships.etacs > 0,
        fleet.ships.loaded_transports > 0,
    )
}

fn owned_records(owners: impl Iterator<Item = u8>, empire: u8) -> impl Iterator<Item = usize> {
    owners
        .enumerate()
        .filter(move |(_, owner)| *owner == empire)
        .map(|(idx, _)| idx + 1)
}

fn planet_mask(game_data: &CoreGameData, record: usize) -> PlanetMask {
    let points_left = game_data.planet_build_points_left(record).unwrap_or(0);
    let mut build = [false; BUILD_CHOICES];
    build[0] = true;
    for (idx, unit) in BUILD_UNITS.iter().enumerate() {
        build[idx + 1] = points_left >= unit.cost;
    }
    PlanetMask { record, build }
}

/// One fleet's mask. Target flags depend only on the mission and the
/// fleet's kinds of ships, except Colonize, which also depends on which of
/// the empire's other fleets claim a world; the rest are shared through
/// `shared` between fleets of one seat.
fn fleet_mask(
    game_data: &CoreGameData,
    record: usize,
    shared: &mut BTreeMap<(usize, ShipsKey), Vec<bool>>,
) -> FleetMask {
    let fleet = &game_data.fleets[record - 1];
    let worlds = game_data.planets.len();
    let mut mission = [false; MISSION_CHOICES];
    let mut targets = vec![vec![true; worlds]; MISSION_CHOICES];
    mission[0] = true;
    for (idx, order) in MISSIONS.iter().enumerate() {
        let choice = idx + 1;
        if !fleet_record_supports_mission_code(fleet, order.code()) {
            targets[choice] = vec![false; worlds];
            continue;
        }
        mission[choice] = match order {
            Order::HoldPosition => true,
            Order::SeekHome => game_data.nearest_owned_world(record, false).is_some(),
            _ => {
                let validate = || {
                    game_data
                        .planets
                        .iter()
                        .map(|planet| {
                            game_data
                                .validate_fleet_order_payload(
                                    record,
                                    order.code(),
                                    planet.sector.to_array(),
                                    None,
                                    None,
                                )
                                .is_ok()
                        })
                        .collect::<Vec<_>>()
                };
                targets[choice] = if *order == Order::ColonizeWorld {
                    validate()
                } else {
                    shared
                        .entry((choice, ships_key(fleet)))
                        .or_insert_with(validate)
                        .clone()
                };
                targets[choice].contains(&true)
            }
        };
    }
    let support_only = is_support_only(fleet);
    let mut roe = [!support_only; ROE_CHOICES];
    roe[0] = true;
    FleetMask {
        record,
        mission,
        targets,
        roe,
        load: armies_to_load(game_data, fleet, 0).is_some(),
    }
}

/// The world a fleet is at and how many armies it may load there, if any,
/// after `loaded` armies have already left that world this turn.
fn armies_to_load(game_data: &CoreGameData, fleet: &Fleet, loaded: u16) -> Option<(usize, u16)> {
    let (idx, planet) = game_data
        .planets
        .iter()
        .enumerate()
        .find(|(_, planet)| planet.sector == fleet.sector)?;
    // The submission path counts at most 255 armies on a world.
    let spare = planet
        .armies
        .min(u16::from(u8::MAX))
        .saturating_sub(1)
        .saturating_sub(loaded);
    let qty = spare.min(fleet.ships.empty_transports);
    (planet.owner.0 == fleet.owner.0 && qty > 0).then_some((idx + 1, qty))
}

/// Turn `action` into the submission it stands for. The action must respect
/// `seat_masks(game_data, empire)`. A fleet's new orders are dropped, and the
/// fleet keeps its old ones, only when a fleet decoded before it in the same
/// turn already claims the world it would colonize.
pub fn decode_action(game_data: &CoreGameData, empire: u8, action: &SeatAction) -> DecodedTurn {
    let mut submission = TurnSubmission {
        player_record_index_1_based: usize::from(empire),
        year: game_data.conquest.game_year,
        tax_rate: Some(TAX_STEP * action.tax as u8),
        diplomacy: action
            .diplomacy
            .iter()
            .enumerate()
            .filter_map(|(idx, &choice)| {
                let relation = match choice {
                    1 => DiplomaticRelation::Neutral,
                    2 => DiplomaticRelation::Enemy,
                    _ => return None,
                };
                Some(TurnDiplomacyDirective {
                    to_empire: idx as u8 + 1,
                    relation,
                })
            })
            .collect(),
        planets: Vec::new(),
        fleets: Vec::new(),
        messages: Vec::new(),
    };

    for choice in &action.planets {
        let mut actions = Vec::new();
        if choice.build > 0 {
            let unit = &BUILD_UNITS[choice.build - 1];
            let points_left = game_data
                .planet_build_points_left(choice.record)
                .unwrap_or(0);
            let units = (points_left / unit.cost * (choice.amount as u32 + 1)).div_ceil(4);
            actions.push(PlanetTurnAction::Build {
                points_remaining: units.max(1) * unit.cost,
                kind: production_item_kind_code(unit.kind),
            });
        }
        if !actions.is_empty() {
            submission.planets.push(PlanetTurnBlock {
                planet_record_index_1_based: choice.record,
                actions,
            });
        }
    }
    // Fleet orders are checked in order against a working copy, as the
    // submission path's preflight checks them.
    let mut working = game_data.clone();
    let mut dropped_fleet_orders = Vec::new();
    // Armies already loaded from each world (0-based) this turn.
    let mut loaded_from: BTreeMap<usize, u16> = BTreeMap::new();
    for choice in &action.fleets {
        let mut actions = Vec::new();
        if choice.load {
            let fleet = &game_data.fleets[choice.record - 1];
            let world = game_data
                .planets
                .iter()
                .position(|planet| planet.sector == fleet.sector);
            let loaded = world.map_or(0, |idx| loaded_from.get(&idx).copied().unwrap_or(0));
            if let Some((planet, qty)) = armies_to_load(game_data, fleet, loaded) {
                *loaded_from.entry(planet - 1).or_insert(0) += qty;
                actions.push(FleetTurnAction::LoadArmies {
                    planet_record_index_1_based: planet,
                    qty,
                });
            }
        }
        if choice.mission > 0 {
            let order = fleet_order(game_data, choice);
            let FleetTurnAction::Order {
                speed,
                order_code,
                target,
                ..
            } = order
            else {
                unreachable!("fleet_order builds an Order action");
            };
            let target = target.or_else(|| working.nearest_owned_world(choice.record, false));
            let accepted = target.is_some_and(|target| {
                working
                    .set_fleet_order(choice.record, speed, order_code, target, None, None)
                    .is_ok()
            });
            if accepted {
                actions.push(order);
            } else {
                dropped_fleet_orders.push(choice.record);
            }
        }
        actions.push(FleetTurnAction::RulesOfEngagement {
            value: choice.roe as u8,
        });
        submission.fleets.push(FleetTurnBlock {
            fleet_record_index_1_based: choice.record,
            actions,
        });
    }
    DecodedTurn {
        submission,
        dropped_fleet_orders,
    }
}

fn fleet_order(game_data: &CoreGameData, choice: &FleetChoice) -> FleetTurnAction {
    let fleet = &game_data.fleets[choice.record - 1];
    let order = MISSIONS[choice.mission - 1];
    let max_speed = u32::from(fleet.max_speed);
    let speed = (max_speed * (SPEED_CHOICES - choice.speed) as u32).div_ceil(SPEED_CHOICES as u32);
    let target = match order {
        Order::SeekHome => None,
        Order::HoldPosition => Some(fleet.sector.to_array()),
        _ => Some(game_data.planets[choice.target].sector.to_array()),
    };
    FleetTurnAction::Order {
        speed: speed as u8,
        order_code: order.code(),
        target,
        aux0: None,
        aux1: None,
    }
}
