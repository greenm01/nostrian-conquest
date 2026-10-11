//! Action generators shared by the policies. Each reads the live state,
//! spends choices on indexes into it, and proposes one turn action; the
//! submission path decides whether the game accepts it.

use nc_data::{
    CoreGameData, DiplomaticRelation, FleetDetachSelection, FleetTurnAction, Order,
    PlanetTurnAction, STARDOCK_SLOT_COUNT, TurnDiplomacyDirective, troop_transports,
};
use nc_engine::{BUILD_UNITS, FLEET_MISSION_OPTIONS, fleet_record_supports_mission_code};

use super::view::{
    fleet, foreign_fleet_coords, map_size, owned_fleets, owned_planet_at, owned_planets, planet,
    planets_where,
};
use crate::choice::{ChoiceSource, pick_from, pick_index};
use crate::turn::TurnAction;

pub type Choices<'a> = &'a mut dyn ChoiceSource;

const MAX_ROE: u32 = 10;

/// Where a fleet order aims.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Aim {
    /// Any mission the fleet can fly, at any plausible target.
    Anywhere,
    /// Hostile missions against foreign worlds.
    Attack,
    /// Gather at one of a few shared points.
    Gather,
}

pub fn order_fleet(
    state: &CoreGameData,
    empire: u8,
    choices: Choices,
    aim: Aim,
) -> Option<TurnAction> {
    let record = pick_from(choices, &owned_fleets(state, empire))?;
    let current = fleet(state, record);
    let supported = FLEET_MISSION_OPTIONS
        .iter()
        .map(|option| option.code)
        // Guard Starbase needs a starbase id and Join has its own action.
        .filter(|&code| {
            code != Order::GuardStarbase.code() && code != Order::JoinAnotherFleet.code()
        })
        .filter(|&code| fleet_record_supports_mission_code(current, code))
        .filter(|&code| match aim {
            Aim::Anywhere => true,
            Aim::Attack => matches!(
                Order::from_code(code),
                Order::BombardWorld
                    | Order::InvadeWorld
                    | Order::BlitzWorld
                    | Order::GuardBlockadeWorld
                    | Order::MoveOnly
            ),
            Aim::Gather => matches!(
                Order::from_code(code),
                Order::RendezvousSector | Order::MoveOnly | Order::HoldPosition
            ),
        })
        .collect::<Vec<_>>();
    // Prefer the strongest mission: the last supported one is the most specific.
    let code = supported[supported.len() - 1 - pick_index(choices, supported.len())?];
    // Seek Home leaves the target to the turn file's nearest-world rule.
    let target = match Order::from_code(code) {
        Order::SeekHome => None,
        order => Some(order_target(state, empire, choices, record, order, aim)?),
    };
    let max_speed = u32::from(current.max_speed);
    let speed = (max_speed - choices.pick(0, max_speed)) as u8;
    Some(TurnAction::Fleet(
        record,
        FleetTurnAction::Order {
            speed,
            order_code: code,
            target,
            aux0: None,
            aux1: None,
        },
    ))
}

fn order_target(
    state: &CoreGameData,
    empire: u8,
    choices: Choices,
    record: usize,
    order: Order,
    aim: Aim,
) -> Option<[u8; 2]> {
    let here = fleet(state, record).sector.to_array();
    let coords_of = |records: Vec<usize>| {
        records
            .into_iter()
            .map(|record| planet(state, record).sector.to_array())
            .collect::<Vec<_>>()
    };
    let foreign_worlds = coords_of(planets_where(state, |planet| {
        planet.owner.0 != empire && planet.owner.0 != 0
    }));
    let candidates = match order {
        Order::HoldPosition => vec![here],
        Order::ColonizeWorld => coords_of(planets_where(state, |planet| planet.owner.0 == 0)),
        Order::BombardWorld | Order::InvadeWorld | Order::BlitzWorld => foreign_worlds,
        Order::Salvage => coords_of(owned_planets(state, empire)),
        Order::ViewWorld | Order::ScoutSolarSystem | Order::GuardBlockadeWorld => {
            coords_of(planets_where(state, |_| true))
        }
        _ if aim == Aim::Gather => gathering_points(state, empire),
        _ if aim == Aim::Attack => {
            let mut points = foreign_worlds;
            points.extend(foreign_fleet_coords(state, empire));
            points
        }
        _ => {
            let mut points = coords_of(planets_where(state, |_| true));
            points.extend(foreign_fleet_coords(state, empire));
            points.push(random_sector(state, choices));
            points
        }
    };
    pick_from(choices, &candidates)
}

/// A few shared points a swarm keeps converging on: its own worlds and the
/// map center.
pub fn gathering_points(state: &CoreGameData, empire: u8) -> Vec<[u8; 2]> {
    let mut points = owned_planets(state, empire)
        .into_iter()
        .map(|record| planet(state, record).sector.to_array())
        .collect::<Vec<_>>();
    let center = map_size(state).div_ceil(2);
    points.push([center, center]);
    points
}

fn random_sector(state: &CoreGameData, choices: Choices) -> [u8; 2] {
    let size = u32::from(map_size(state));
    [choices.pick(1, size) as u8, choices.pick(1, size) as u8]
}

pub fn set_roe(state: &CoreGameData, empire: u8, choices: Choices) -> Option<TurnAction> {
    let record = pick_from(choices, &owned_fleets(state, empire))?;
    let value = choices.pick(0, MAX_ROE) as u8;
    Some(TurnAction::Fleet(
        record,
        FleetTurnAction::RulesOfEngagement { value },
    ))
}

pub fn join_fleet(state: &CoreGameData, empire: u8, choices: Choices) -> Option<TurnAction> {
    let fleets = owned_fleets(state, empire);
    let record = pick_from(choices, &fleets)?;
    let hosts = fleets
        .into_iter()
        .filter(|&host| host != record)
        .collect::<Vec<_>>();
    let host = pick_from(choices, &hosts)?;
    Some(TurnAction::Fleet(
        record,
        FleetTurnAction::Join {
            host_fleet_record_index_1_based: host,
        },
    ))
}

/// Split some ships off into a new fleet.
pub fn detach(state: &CoreGameData, empire: u8, choices: Choices) -> Option<TurnAction> {
    let record = pick_from(choices, &owned_fleets(state, empire))?;
    let selection = pick_selection(state, record, choices);
    let new_fleet_roe = choices.pick(0, MAX_ROE) as u8;
    Some(TurnAction::Fleet(
        record,
        FleetTurnAction::Detach {
            selection,
            donor_speed: None,
            new_fleet_roe,
        },
    ))
}

/// Split exactly one ship off, the swarm's favorite move.
pub fn detach_one(state: &CoreGameData, empire: u8, choices: Choices) -> Option<TurnAction> {
    let record = pick_from(choices, &owned_fleets(state, empire))?;
    let donor = fleet(state, record);
    let mut kinds = Vec::new();
    for (count, kind) in [
        (u32::from(donor.ships.destroyers), 0),
        (u32::from(donor.ships.cruisers), 1),
        (u32::from(donor.ships.battleships), 2),
        (u32::from(donor.ships.scouts), 3),
        (u32::from(donor.ships.etacs), 4),
    ] {
        if count > 0 {
            kinds.push(kind);
        }
    }
    let mut selection = FleetDetachSelection::default();
    match pick_from(choices, &kinds)? {
        0 => selection.destroyers = 1,
        1 => selection.cruisers = 1,
        2 => selection.battleships = 1,
        3 => selection.scouts = 1,
        _ => selection.etacs = 1,
    }
    Some(TurnAction::Fleet(
        record,
        FleetTurnAction::Detach {
            selection,
            donor_speed: None,
            new_fleet_roe: choices.pick(0, MAX_ROE) as u8,
        },
    ))
}

pub fn transfer(state: &CoreGameData, empire: u8, choices: Choices) -> Option<TurnAction> {
    let fleets = owned_fleets(state, empire);
    let record = pick_from(choices, &fleets)?;
    let here = fleet(state, record).sector.to_array();
    let hosts = fleets
        .into_iter()
        .filter(|&host| host != record && fleet(state, host).sector.to_array() == here)
        .collect::<Vec<_>>();
    let host = pick_from(choices, &hosts)?;
    let selection = pick_selection(state, record, choices);
    Some(TurnAction::Fleet(
        record,
        FleetTurnAction::Transfer {
            host_fleet_record_index_1_based: host,
            selection,
        },
    ))
}

fn pick_selection(state: &CoreGameData, record: usize, choices: Choices) -> FleetDetachSelection {
    let donor = fleet(state, record);
    let full = u32::from(donor.ships.loaded_transports).min(u32::from(troop_transports(donor)));
    let empty = u32::from(troop_transports(donor)).saturating_sub(full);
    FleetDetachSelection {
        battleships: choices.pick(0, u32::from(donor.ships.battleships)) as u16,
        cruisers: choices.pick(0, u32::from(donor.ships.cruisers)) as u16,
        destroyers: choices.pick(0, u32::from(donor.ships.destroyers)) as u16,
        full_transports: choices.pick(0, full) as u16,
        empty_transports: choices.pick(0, empty) as u16,
        scouts: choices.pick(0, u32::from(donor.ships.scouts)) as u16,
        etacs: choices.pick(0, u32::from(donor.ships.etacs)) as u16,
    }
}

pub fn load_armies(state: &CoreGameData, empire: u8, choices: Choices) -> Option<TurnAction> {
    let carriers = owned_fleets(state, empire)
        .into_iter()
        .filter_map(|record| {
            let carrier = fleet(state, record);
            let room = u32::from(troop_transports(carrier))
                .saturating_sub(u32::from(carrier.ships.loaded_transports));
            let world = owned_planet_at(state, empire, carrier.sector.to_array())?;
            let armies = u32::from(u8::try_from(planet(state, world).armies).unwrap_or(u8::MAX));
            (room > 0 && armies > 0).then_some((record, world, room.min(armies)))
        })
        .collect::<Vec<_>>();
    let (record, world, most) = pick_from(choices, &carriers)?;
    Some(TurnAction::Fleet(
        record,
        FleetTurnAction::LoadArmies {
            planet_record_index_1_based: world,
            qty: choices.pick(1, most) as u16,
        },
    ))
}

pub fn unload_armies(state: &CoreGameData, empire: u8, choices: Choices) -> Option<TurnAction> {
    let carriers = owned_fleets(state, empire)
        .into_iter()
        .filter_map(|record| {
            let carrier = fleet(state, record);
            let world = owned_planet_at(state, empire, carrier.sector.to_array())?;
            let armies = u32::from(carrier.ships.loaded_transports);
            (armies > 0).then_some((record, world, armies))
        })
        .collect::<Vec<_>>();
    let (record, world, most) = pick_from(choices, &carriers)?;
    Some(TurnAction::Fleet(
        record,
        FleetTurnAction::UnloadArmies {
            planet_record_index_1_based: world,
            qty: choices.pick(1, most) as u16,
        },
    ))
}

/// Queue units of one of `kinds` (build-unit numbers) within the budget.
pub fn build(
    state: &CoreGameData,
    empire: u8,
    choices: Choices,
    kinds: &[u8],
) -> Option<TurnAction> {
    let world = pick_from(choices, &owned_planets(state, empire))?;
    let number = pick_from(choices, kinds)?;
    let unit = BUILD_UNITS.iter().find(|unit| unit.number == number)?;
    let points_left = state
        .planet_build_points_left(world)
        .ok()?
        .min(u32::from(u8::MAX));
    let most = points_left / unit.cost;
    if most == 0 {
        return None;
    }
    let qty = choices.pick(1, most);
    Some(TurnAction::Planet(
        world,
        PlanetTurnAction::Build {
            points_remaining_raw: (qty * unit.cost) as u8,
            kind_raw: nc_engine::production_item_kind_raw(unit.kind),
        },
    ))
}

pub fn commission(state: &CoreGameData, empire: u8, choices: Choices) -> Option<TurnAction> {
    let docks = owned_planets(state, empire)
        .into_iter()
        .flat_map(|world| {
            (0..STARDOCK_SLOT_COUNT)
                .filter(move |&slot| planet(state, world).stardock[slot] > 0)
                .map(move |slot| (world, slot))
        })
        .collect::<Vec<_>>();
    let (world, slot_0_based) = pick_from(choices, &docks)?;
    Some(TurnAction::Planet(
        world,
        PlanetTurnAction::Commission { slot_0_based },
    ))
}

pub fn auto_commission(state: &CoreGameData, empire: u8, choices: Choices) -> Option<TurnAction> {
    let world = pick_from(choices, &owned_planets(state, empire))?;
    Some(TurnAction::Planet(world, PlanetTurnAction::AutoCommission))
}

pub fn scorch(state: &CoreGameData, empire: u8, choices: Choices) -> Option<TurnAction> {
    let world = pick_from(choices, &owned_planets(state, empire))?;
    Some(TurnAction::Planet(world, PlanetTurnAction::Scorch))
}

pub fn tax(choices: Choices) -> Option<TurnAction> {
    Some(TurnAction::Tax(choices.pick(0, 100) as u8))
}

pub fn diplomacy(
    state: &CoreGameData,
    empire: u8,
    choices: Choices,
    hostile: bool,
) -> Option<TurnAction> {
    let others = (1..=state.conquest.player_count)
        .filter(|&other| other != empire)
        .collect::<Vec<_>>();
    let to_empire = pick_from(choices, &others)?;
    let relation = if hostile || choices.flip() {
        DiplomaticRelation::Enemy
    } else {
        DiplomaticRelation::Neutral
    };
    Some(TurnAction::Diplomacy(TurnDiplomacyDirective {
        to_empire,
        relation,
    }))
}
