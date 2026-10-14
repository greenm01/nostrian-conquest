//! One empire's observation as fixed-width rows of numbers.
//!
//! Values are raw game numbers; scaling belongs to the learner. A value the
//! empire does not know is 0 with its "known" flag 0. Rows are not padded:
//! each entity list has as many rows as the empire knows entities, and
//! batching code pads them.
//!
//! The observation reads only the empire's own records (its player record,
//! planets, fleets and starbases), its [`SeatMemory`], and public facts:
//! the year, the player count, and every world's position.

use nc_data::{CoreGameData, DiplomaticRelation, IntelTier, PlanetIntelSnapshot, ShipCounts};

use crate::memory::{Contact, ContactSource, SeatMemory};

/// Global row: year, player count, tax rate, own world, production, stored
/// point, fleet and starbase totals, then one enemy flag per empire slot.
pub const GLOBAL_WIDTH: usize = 8 + 25;
/// One row per world in the galaxy, in planet-record order.
pub const PLANET_WIDTH: usize = 31;
/// One row per fleet the empire owns.
pub const FLEET_WIDTH: usize = 19;
/// One row per remembered foreign contact.
pub const CONTACT_WIDTH: usize = 19;
/// One row per starbase the empire owns.
pub const STARBASE_WIDTH: usize = 7;

/// Row-major entity tables; each `Vec` holds whole rows of its width.
#[derive(Debug, Clone, PartialEq)]
pub struct Observation {
    pub global: Vec<f32>,
    pub planets: Vec<f32>,
    pub fleets: Vec<f32>,
    pub contacts: Vec<f32>,
    pub starbases: Vec<f32>,
}

/// What `memory`'s empire sees of `game_data` now.
pub fn observe(game_data: &CoreGameData, memory: &SeatMemory) -> Observation {
    let empire = memory.empire;
    let year = game_data.conquest.game_year;
    Observation {
        global: global_row(game_data, empire),
        planets: game_data
            .planets
            .iter()
            .enumerate()
            .flat_map(|(idx, planet)| {
                planet_row(planet, memory.planet_intel.get(&(idx + 1)), empire, year)
            })
            .collect(),
        fleets: game_data
            .fleets
            .iter()
            .filter(|fleet| fleet.owner.0 == empire)
            .flat_map(fleet_row)
            .collect(),
        contacts: memory
            .contacts
            .iter()
            .flat_map(|contact| contact_row(contact, year))
            .collect(),
        starbases: game_data
            .bases
            .iter()
            .filter(|base| base.owner.0 == empire)
            .flat_map(starbase_row)
            .collect(),
    }
}

fn global_row(game_data: &CoreGameData, empire: u8) -> Vec<f32> {
    let player = &game_data.players[usize::from(empire) - 1];
    let owned = game_data
        .planets
        .iter()
        .filter(|planet| planet.owner.0 == empire);
    let (worlds, production, stored) = owned.fold((0u32, 0u32, 0u32), |sum, planet| {
        (
            sum.0 + 1,
            sum.1 + u32::from(nc_data::present_production_points(planet)),
            sum.2 + nc_data::stored_production_points(planet),
        )
    });
    let fleets = game_data
        .fleets
        .iter()
        .filter(|fleet| fleet.owner.0 == empire)
        .count();
    let starbases = game_data
        .bases
        .iter()
        .filter(|base| base.owner.0 == empire)
        .count();
    let mut row = vec![
        f32::from(game_data.conquest.game_year),
        f32::from(game_data.conquest.player_count),
        f32::from(player.tax_rate),
        worlds as f32,
        production as f32,
        stored as f32,
        fleets as f32,
        starbases as f32,
    ];
    row.extend(
        player
            .diplomacy
            .iter()
            .map(|relation| flag(*relation == DiplomaticRelation::Enemy)),
    );
    row
}

fn planet_row(
    planet: &nc_data::Planet,
    intel: Option<&PlanetIntelSnapshot>,
    empire: u8,
    year: u16,
) -> Vec<f32> {
    let tier = intel.map_or(IntelTier::Unknown, |intel| intel.intel_tier);
    let owner = intel.and_then(|intel| intel.known_owner_empire_id);
    let known = |value: Option<f32>| [flag(value.is_some()), value.unwrap_or(0.0)];
    let field = |get: fn(&PlanetIntelSnapshot) -> Option<f32>| known(intel.and_then(get));
    let mut row = vec![
        f32::from(planet.sector.x),
        f32::from(planet.sector.y),
        flag(tier == IntelTier::Owned),
        flag(tier == IntelTier::Full),
        flag(tier == IntelTier::Partial),
        flag(tier == IntelTier::Unknown),
        flag(owner.is_some()),
        flag(owner == Some(empire)),
        flag(owner == Some(0)),
        f32::from(owner.unwrap_or(0)),
    ];
    row.extend(field(|i| i.known_potential_production.map(f32::from)));
    row.extend(field(|i| i.known_current_production.map(f32::from)));
    row.extend(field(|i| i.known_stored_points.map(f32::from)));
    row.extend(field(|i| i.known_armies.map(f32::from)));
    row.extend(field(|i| i.known_ground_batteries.map(f32::from)));
    row.extend(field(|i| i.known_starbase_count.map(f32::from)));
    row.extend(known(
        intel
            .and_then(|intel| intel.last_intel_year)
            .map(|seen| f32::from(year.saturating_sub(seen))),
    ));
    // A stardock is visible only at the empire's own worlds.
    let dock = &planet.stardock;
    let own = planet.owner.0 == empire;
    row.extend(
        [
            dock.destroyers,
            dock.cruisers,
            dock.battleships,
            dock.scouts,
            dock.transports,
            dock.etacs,
            dock.starbases,
        ]
        .map(|count| if own { count as f32 } else { 0.0 }),
    );
    row
}

fn fleet_row(fleet: &nc_data::Fleet) -> Vec<f32> {
    let mut row = vec![
        f32::from(fleet.sector.x),
        f32::from(fleet.sector.y),
        f32::from(fleet.fleet_number.0),
        f32::from(fleet.speed),
        f32::from(fleet.max_speed),
        f32::from(fleet.order.code()),
        f32::from(fleet.target.x),
        f32::from(fleet.target.y),
        f32::from(fleet.target_depth),
        f32::from(fleet.rules_of_engagement),
    ];
    row.extend(ship_counts(&fleet.ships));
    row.push(fleet.crippled.total() as f32);
    row.push(f32::from(fleet.mission_target));
    row
}

fn ship_counts(ships: &ShipCounts) -> [f32; 7] {
    [
        ships.battleships,
        ships.cruisers,
        ships.destroyers,
        ships.empty_transports,
        ships.loaded_transports,
        ships.etacs,
        ships.scouts,
    ]
    .map(f32::from)
}

fn contact_row(contact: &Contact, year: u16) -> Vec<f32> {
    let ships = contact.ships.unwrap_or_default();
    vec![
        f32::from(year.saturating_sub(contact.year)),
        f32::from(contact.coords[0]),
        f32::from(contact.coords[1]),
        f32::from(contact.empire),
        flag(contact.source == ContactSource::Scout),
        flag(contact.source == ContactSource::Encounter),
        flag(contact.source == ContactSource::Battle),
        flag(contact.source == ContactSource::Starbase),
        contact.small as f32,
        contact.medium as f32,
        contact.large as f32,
        flag(contact.ships.is_some()),
        ships.battleships as f32,
        ships.cruisers as f32,
        ships.destroyers as f32,
        ships.scouts as f32,
        ships.transports as f32,
        ships.etacs as f32,
        contact.starbases as f32,
    ]
}

fn starbase_row(base: &nc_data::Starbase) -> Vec<f32> {
    vec![
        f32::from(base.sector.x),
        f32::from(base.sector.y),
        f32::from(base.base_id.0),
        flag(base.moving),
        f32::from(base.target.x),
        f32::from(base.target.y),
        base.crippled.total() as f32,
    ]
}

fn flag(value: bool) -> f32 {
    if value { 1.0 } else { 0.0 }
}
