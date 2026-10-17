//! A seat's orders as factored choices, legality masks drawn from the
//! game's own validators, and decoding into a [`TurnSubmission`].
//!
//! A seat chooses, all at once:
//! - a tax rate, in steps of [`TAX_STEP`];
//! - for every other empire, to keep, make neutral or declare enemy;
//! - for each world it owns, up to [`BUILD_SLOTS`] units to build (or none),
//!   each with how much of the budget still left to spend on it. Units built
//!   in one year commission into one fleet, so an ETAC and its escort can
//!   sail together;
//! - for each fleet it owns, whether to load armies from the world it is at,
//!   a mission (or keep its orders), a target world, a waypoint, a speed,
//!   rules of engagement, and whether to detach part of it (see
//!   [`DETACHES`]); or, in place of the mission, another of its fleets to
//!   join; and whether to unload its armies at the world it is at.
//!   Loading and an invasion cannot share a year: orders are checked against
//!   the turn-start fleet, so a fleet invades the year after it loads. A
//!   fleet that both loads and unloads only unloads, and one whose new or
//!   kept order is Invade or Blitz keeps its armies aboard.
//!
//! Detached ships form a new fleet on Hold at the donor's sector, with the
//! donor's rules of engagement, after every other order of the year, so the
//! donor's mission applies to the ships it keeps. The new fleet takes orders
//! the next year. A fleet that joins another and detaches keeps the detached
//! ships where it is, as the new fleet, and sends the rest to the host: the
//! detach picks what stays.
//!
//! Masks read only what the empire may know. Fleet missions and targets go
//! through `validate_fleet_order_payload`, which checks the fleet's own ships
//! and whether a world at the target is the fleet's own; whether a world
//! exists at a sector is public. Every target is a world's sector, except
//! that the sector missions (Move, Patrol a Sector, Scout a Sector and
//! Rendezvous) may stop short of it in deep space, by [`WAYPOINT_CHOICES`].
//! Those missions accept any sector, so the waypoint needs no mask. Guard a
//! Starbase targets the world where one of the seat's own starbases sits,
//! and names that base. Join targets one of the seat's other fleets, by its
//! row among the seat's fleets (t190).
//!
//! A learned seat commissions every finished ship before it chooses, through
//! [`commission_submission`], so its masks see the fleets that creates.

use std::collections::BTreeMap;

use nc_data::{
    CoreGameData, DiplomaticRelation, Fleet, FleetDetachSelection, FleetTurnAction, FleetTurnBlock,
    Order, PlanetTurnAction, PlanetTurnBlock, STARDOCK_SLOT_COUNT, TurnDiplomacyDirective,
    TurnSubmission, has_combat_ships, is_support_only, starship_count,
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
/// Builds a world may order in one year.
pub const BUILD_SLOTS: usize = 2;
/// Keep the fleet's orders, or one entry per [`MISSIONS`] row.
pub const MISSION_CHOICES: usize = 1 + MISSIONS.len();
/// Full, three-quarter, half or quarter speed, rounded up.
pub const SPEED_CHOICES: usize = 4;
/// Rules of engagement 0 to 10.
pub const ROE_CHOICES: usize = 11;
/// For a sector mission, how far toward the target world the target sector
/// lies from the fleet: all the way, three quarters, half or a quarter,
/// rounded to the nearest sector. Other missions ignore it.
pub const WAYPOINT_CHOICES: usize = 4;
/// Keep the fleet whole, or one entry per [`DETACHES`] row.
pub const DETACH_CHOICES: usize = 1 + DETACHES.len();

/// What part of a fleet a seat may detach.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Detach {
    /// Half of each kind of warship, rounded down.
    HalfWarships,
    /// One warship of the lightest kind aboard.
    OneWarship,
    /// Every troop transport, loaded or empty.
    Transports,
}

/// The detach choices, in choice order after "keep whole".
pub const DETACHES: [Detach; 3] = [Detach::HalfWarships, Detach::OneWarship, Detach::Transports];

/// The choice index of `detach`.
pub fn detach_choice(detach: Detach) -> usize {
    1 + DETACHES
        .iter()
        .position(|row| *row == detach)
        .expect("every detach is offered")
}

/// Units of cost `cost` that build amount choice `amount` buys from
/// `points_left`, which must cover one unit.
pub(crate) fn build_units(points_left: u32, cost: u32, amount: usize) -> u32 {
    (points_left / cost * (amount as u32 + 1))
        .div_ceil(BUILD_AMOUNT_CHOICES as u32)
        .max(1)
}

/// The missions a seat can give, in mission-choice order.
pub const MISSIONS: [Order; 15] = [
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
    Order::GuardStarbase,
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
    /// By detach choice: the part exists and leaves the donor a ship.
    pub detach: [bool; DETACH_CHOICES],
    /// Whom to join: 0 for none, then one flag per row of
    /// [`SeatMasks::fleets`]; a fleet may join any other of the seat's.
    pub join: Vec<bool>,
    /// The fleet is at one of the empire's worlds, with armies aboard and
    /// room for them there.
    pub unload: bool,
}

/// Everything one seat may choose this year. Tax, speed, waypoint and
/// build-amount choices are always all allowed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SeatMasks {
    pub empire: u8,
    /// One row per empire slot, 1 to the player count; the seat's own row
    /// allows only "keep".
    pub diplomacy: Vec<[bool; DIPLOMACY_CHOICES]>,
    pub planets: Vec<PlanetMask>,
    pub fleets: Vec<FleetMask>,
}

/// One world's choice, by index into its [`PlanetMask`]. Both build slots
/// draw on the mask; the second spends what the first leaves and is skipped
/// when that cannot buy one unit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlanetChoice {
    pub record: usize,
    pub build: [usize; BUILD_SLOTS],
    pub amount: [usize; BUILD_SLOTS],
}

/// One fleet's choice, by index into its [`FleetMask`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FleetChoice {
    pub record: usize,
    pub mission: usize,
    /// 0-based world index.
    pub target: usize,
    /// See [`WAYPOINT_CHOICES`].
    pub waypoint: usize,
    pub speed: usize,
    pub roe: usize,
    /// Load as many armies as the empty transports hold, leaving the world
    /// at least one.
    pub load: bool,
    /// Keep whole (0), or detach the [`DETACHES`] row before it.
    pub detach: usize,
    /// Join another fleet of the seat (t189): its 1-based record, or 0.
    /// With a join, the mission is ignored and the other choices only matter
    /// for the fleet's rules of engagement, load, unload and detach.
    pub join: usize,
    /// Unload every army aboard that the world it is at has room for.
    pub unload: bool,
    /// For a sector mission, the sector to go to, in place of the waypoint
    /// toward the target world (t189's early-warning stations). The network
    /// never sets it.
    pub station: Option<[u8; 2]>,
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
            let records: Vec<usize> =
                owned_records(game_data.fleets.iter().map(|fleet| fleet.owner.0), empire).collect();
            records
                .iter()
                .map(|&record| {
                    let mut mask = fleet_mask(game_data, record, &mut shared);
                    mask.join = std::iter::once(true)
                        .chain(records.iter().map(|&host| host != record))
                        .collect();
                    mask
                })
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
                            let aux1 = match order {
                                // Guard a Starbase names the seat's own
                                // starbase at the target world.
                                Order::GuardStarbase => {
                                    match own_base_at(game_data, fleet.owner.0, planet.sector) {
                                        Some(base) => Some(base),
                                        None => return false,
                                    }
                                }
                                _ => None,
                            };
                            game_data
                                .validate_fleet_order_payload(
                                    record,
                                    order.code(),
                                    planet.sector.to_array(),
                                    None,
                                    aux1,
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
        detach: std::array::from_fn(|choice| {
            choice == 0 || detach_selection(fleet, choice).is_some()
        }),
        join: Vec::new(),
        unload: armies_to_unload(game_data, fleet, 0).is_some(),
    }
}

/// The world a fleet is at and how many of its armies that world has room
/// for, if any, after `unloaded` armies have already landed there this turn.
fn armies_to_unload(
    game_data: &CoreGameData,
    fleet: &Fleet,
    unloaded: u16,
) -> Option<(usize, u16)> {
    let (idx, planet) = game_data
        .planets
        .iter()
        .enumerate()
        .find(|(_, planet)| planet.sector == fleet.sector)?;
    // The submission path holds at most 255 armies on a world.
    let room = u16::from(u8::MAX)
        .saturating_sub(planet.armies.min(u16::from(u8::MAX)))
        .saturating_sub(unloaded);
    let qty = room.min(fleet.ships.loaded_transports);
    (planet.owner.0 == fleet.owner.0 && qty > 0).then_some((idx + 1, qty))
}

/// The ships detach choice `choice` (nonzero) takes from `fleet`, if there
/// are any and the fleet keeps at least one. Loading armies does not change
/// the transport count, so the turn-start fleet decides the mask.
fn detach_selection(fleet: &Fleet, choice: usize) -> Option<FleetDetachSelection> {
    let ships = &fleet.ships;
    let mut selection = FleetDetachSelection::default();
    match DETACHES[choice - 1] {
        Detach::HalfWarships => {
            selection.battleships = ships.battleships / 2;
            selection.cruisers = ships.cruisers / 2;
            selection.destroyers = ships.destroyers / 2;
        }
        Detach::OneWarship => {
            if ships.destroyers > 0 {
                selection.destroyers = 1;
            } else if ships.cruisers > 0 {
                selection.cruisers = 1;
            } else if ships.battleships > 0 {
                selection.battleships = 1;
            }
        }
        Detach::Transports => {
            selection.full_transports = ships.loaded_transports;
            selection.empty_transports = ships.empty_transports;
        }
    }
    let moved = selection.total_ships();
    (moved > 0 && moved < starship_count(fleet)).then_some(selection)
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
///
/// Detaches follow every other fleet block. A detach can re-sort the
/// empire's fleet records, so each donor is found again by fleet number.
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
        let mut points_left = game_data
            .planet_build_points_left(choice.record)
            .unwrap_or(0);
        for (&build, &amount) in choice.build.iter().zip(&choice.amount) {
            if build == 0 {
                continue;
            }
            let unit = &BUILD_UNITS[build - 1];
            if points_left < unit.cost {
                continue;
            }
            let points = build_units(points_left, unit.cost, amount) * unit.cost;
            points_left -= points;
            actions.push(PlanetTurnAction::Build {
                points_remaining: points,
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
    let mut loads = Vec::new();
    let mut unloads = Vec::new();
    let mut unloaded_to: BTreeMap<usize, u16> = BTreeMap::new();
    for choice in &action.fleets {
        let mut actions = Vec::new();
        let fleet = &game_data.fleets[choice.record - 1];
        let order = if choice.mission > 0 {
            MISSIONS[choice.mission - 1]
        } else {
            fleet.order
        };
        let lands = choice.join == 0 && matches!(order, Order::InvadeWorld | Order::BlitzWorld);
        if choice.unload && !lands {
            let world = game_data
                .planets
                .iter()
                .position(|planet| planet.sector == fleet.sector);
            let unloaded = world.map_or(0, |idx| unloaded_to.get(&idx).copied().unwrap_or(0));
            if let Some((planet, qty)) = armies_to_unload(game_data, fleet, unloaded) {
                *unloaded_to.entry(planet - 1).or_insert(0) += qty;
                actions.push(FleetTurnAction::UnloadArmies {
                    planet_record_index_1_based: planet,
                    qty,
                });
                unloads.push((planet, choice.record, qty));
            }
        } else if choice.load {
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
                loads.push((planet, choice.record, qty));
            }
        }
        if choice.join > 0 {
            if working
                .set_join_fleet_order(usize::from(empire), choice.record, choice.join)
                .is_ok()
            {
                actions.push(FleetTurnAction::Join {
                    host_fleet_record_index_1_based: choice.join,
                });
            } else {
                dropped_fleet_orders.push(choice.record);
            }
        } else if choice.mission > 0 {
            let order = fleet_order(game_data, choice);
            let FleetTurnAction::Order {
                speed,
                order_code,
                target,
                aux0,
                aux1,
            } = order
            else {
                unreachable!("fleet_order builds an Order action");
            };
            let target = target.or_else(|| working.nearest_owned_world(choice.record, false));
            let accepted = target.is_some_and(|target| {
                working
                    .set_fleet_order(choice.record, speed, order_code, target, aux0, aux1)
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
    // Orders are checked against the turn-start fleet; detaches see the
    // loads and unloads.
    for (planet, record, qty) in loads {
        working
            .load_planet_armies_onto_fleet(usize::from(empire), planet, record, qty)
            .expect("a masked load applies");
    }
    for (planet, record, qty) in unloads {
        working
            .unload_fleet_armies_to_planet(usize::from(empire), planet, record, qty)
            .expect("a masked unload applies");
    }
    let donors: Vec<(u16, &FleetChoice)> = action
        .fleets
        .iter()
        .filter(|choice| choice.detach > 0)
        .map(|choice| (game_data.fleets[choice.record - 1].fleet_number.0, choice))
        .collect();
    for (number, choice) in donors {
        let record = 1 + working
            .fleets
            .iter()
            .position(|fleet| fleet.owner.0 == empire && fleet.fleet_number.0 == number)
            .expect("a donor fleet stays in the empire's chain");
        let selection = detach_selection(&working.fleets[record - 1], choice.detach)
            .expect("a masked detach has ships to move");
        let detach = FleetTurnAction::Detach {
            selection,
            donor_speed: None,
            new_fleet_roe: choice.roe as u8,
        };
        working
            .detach_ships_to_new_fleet(
                usize::from(empire),
                record,
                selection,
                None,
                choice.roe as u8,
            )
            .expect("a masked detach applies");
        submission.fleets.push(FleetTurnBlock {
            fleet_record_index_1_based: record,
            actions: vec![detach],
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
        _ if sector_mission(order) => Some(choice.station.unwrap_or_else(|| {
            waypoint(
                fleet.sector.to_array(),
                game_data.planets[choice.target].sector.to_array(),
                choice.waypoint,
            )
        })),
        _ => Some(game_data.planets[choice.target].sector.to_array()),
    };
    let aux1 = (order == Order::GuardStarbase)
        .then(|| {
            own_base_at(
                game_data,
                fleet.owner.0,
                game_data.planets[choice.target].sector,
            )
        })
        .flatten();
    FleetTurnAction::Order {
        speed: speed as u8,
        order_code: order.code(),
        target,
        aux0: None,
        aux1,
    }
}

/// A mission whose target may be any sector.
pub fn sector_mission(order: Order) -> bool {
    matches!(
        order,
        Order::MoveOnly | Order::PatrolSector | Order::ScoutSector | Order::RendezvousSector
    )
}

/// The sector waypoint choice `choice` names on the way from `from` to
/// `to`.
pub fn waypoint(from: [u8; 2], to: [u8; 2], choice: usize) -> [u8; 2] {
    let share = (WAYPOINT_CHOICES - choice) as f32 / WAYPOINT_CHOICES as f32;
    std::array::from_fn(|axis| {
        let step = (f32::from(to[axis]) - f32::from(from[axis])) * share;
        (f32::from(from[axis]) + step).round() as u8
    })
}

/// The number of `owner`'s starbase in `sector`, for Guard a Starbase.
fn own_base_at(game_data: &CoreGameData, owner: u8, sector: nc_data::Coords) -> Option<u8> {
    game_data
        .bases
        .iter()
        .find(|base| base.owner.0 == owner && base.sector == sector)
        .and_then(|base| u8::try_from(base.base_id.0).ok())
}
