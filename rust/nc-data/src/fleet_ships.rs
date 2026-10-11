//! What a fleet carries: its ship totals, its troop transports and the
//! armies aboard them, the top speed its slowest ship allows, and the
//! composition labels the screens and reports show.

use crate::{CoreGameData, Fleet, HullClass, ShipCounts, fleet_notation};

/// All troop transports, empty and loaded.
pub fn troop_transports(fleet: &Fleet) -> u16 {
    fleet
        .ships
        .empty_transports
        .saturating_add(fleet.ships.loaded_transports)
}

/// Sets the transport total, keeping loaded transports (up to the new
/// total) and making the rest empty.
pub fn set_troop_transports(fleet: &mut Fleet, total: u16) {
    let loaded = fleet.ships.loaded_transports.min(total);
    fleet.ships.loaded_transports = loaded;
    fleet.ships.empty_transports = total - loaded;
}

/// Loads or unloads armies, one per transport, moving transports between
/// the empty and loaded counts with their recovery state. The transport
/// total grows if it cannot hold `armies`.
pub fn set_armies_aboard(fleet: &mut Fleet, armies: u16) {
    let loaded = fleet.ships.loaded_transports;
    if armies > loaded {
        load_transports(fleet, armies - loaded);
    } else {
        unload_transports(fleet, loaded - armies);
    }
}

/// `count` transports take an army aboard: healthy empty transports first,
/// and crippled ones only when no healthy empty transport is left, those
/// nearest to recovery first. Transports the fleet does not have are added
/// loaded and whole. The order is a default awaiting the owner's
/// confirmation (t096); it lives here alone.
fn load_transports(fleet: &mut Fleet, count: u16) {
    let empty = fleet.ships.empty_transports;
    let moved = count.min(empty);
    let healthy = u32::from(empty).saturating_sub(fleet.crippled.empty_transports.total());
    let crippled = u32::from(moved).saturating_sub(healthy);
    let cohorts = fleet.crippled.empty_transports.take_nearest(crippled);
    fleet.crippled.loaded_transports.add(&cohorts);
    fleet.ships.empty_transports = empty - moved;
    fleet.ships.loaded_transports += count;
}

/// `count` loaded transports put their army ashore, unloading or landing:
/// crippled ones first, the most turns remaining first, then healthy ones.
/// The order is a default awaiting the owner's confirmation (t096); it
/// lives here alone.
fn unload_transports(fleet: &mut Fleet, count: u16) {
    let crippled = u32::from(count).min(fleet.crippled.loaded_transports.total());
    let cohorts = fleet.crippled.loaded_transports.take(crippled);
    fleet.crippled.empty_transports.add(&cohorts);
    fleet.ships.loaded_transports -= count;
    fleet.ships.empty_transports = fleet.ships.empty_transports.saturating_add(count);
}

/// Every ship in the fleet.
pub fn starship_count(fleet: &Fleet) -> u32 {
    u32::from(fleet.ships.battleships)
        + u32::from(fleet.ships.cruisers)
        + u32::from(fleet.ships.destroyers)
        + u32::from(troop_transports(fleet))
        + u32::from(fleet.ships.scouts)
        + u32::from(fleet.ships.etacs)
}

/// Whether the fleet has a destroyer, cruiser, or battleship.
pub fn has_combat_ships(fleet: &Fleet) -> bool {
    fleet.ships.destroyers > 0 || fleet.ships.cruisers > 0 || fleet.ships.battleships > 0
}

/// Ships, but no combat ships.
pub fn is_support_only(fleet: &Fleet) -> bool {
    !has_combat_ships(fleet) && starship_count(fleet) > 0
}

/// Whether the fleet has any ship or army left.
pub fn has_any_force(fleet: &Fleet) -> bool {
    fleet.ships.scouts > 0
        || fleet.ships.battleships > 0
        || fleet.ships.cruisers > 0
        || fleet.ships.destroyers > 0
        || troop_transports(fleet) > 0
        || fleet.ships.loaded_transports > 0
        || fleet.ships.etacs > 0
}

/// ECGAME's recompute of the maximum speed: start at 7 and take the minimum
/// over the ship types present (ETAC 3, BB 4, CA 5, transports 5, DD 6,
/// scouts 6). The current speed drops to the new maximum.
pub fn recompute_max_speed(fleet: &mut Fleet) {
    let max_speed = slowest_speed(&fleet.ships);
    fleet.max_speed = max_speed;
    if fleet.speed > max_speed {
        fleet.speed = max_speed;
    }
}

/// The speed of the slowest class among `ships`: ETAC 3, battleship 4,
/// cruiser and transport 5, destroyer and scout 6; 7 with no ships at all.
pub fn slowest_speed(ships: &ShipCounts) -> u8 {
    let mut max_speed = 7;
    for (count, speed) in [
        (ships.etacs, 3),
        (ships.battleships, 4),
        (ships.cruisers, 5),
        (
            ships
                .empty_transports
                .saturating_add(ships.loaded_transports),
            5,
        ),
        (ships.destroyers, 6),
        (ships.scouts, 6),
    ] {
        if count > 0 {
            max_speed = max_speed.min(speed);
        }
    }
    max_speed
}

/// The fleet's ships in the compact notation (`SC1 BB2!1 CA12!3 TT*4 TT5`),
/// as the fleet lists, details, and previews show them.
pub fn ship_composition_table_summary(fleet: &Fleet) -> String {
    let summary = fleet_notation(fleet);
    assert!(!summary.is_empty(), "empty fleet record is not a fleet");
    summary
}

/// The ships of one class in the fleet.
pub fn hull_count(fleet: &Fleet, class: HullClass) -> u32 {
    let ships = &fleet.ships;
    u32::from(match class {
        HullClass::Destroyer => ships.destroyers,
        HullClass::Cruiser => ships.cruisers,
        HullClass::Battleship => ships.battleships,
        HullClass::Scout => ships.scouts,
        HullClass::LoadedTransport => ships.loaded_transports,
        HullClass::EmptyTransport => ships.empty_transports,
        HullClass::Etac => ships.etacs,
    })
}

/// Every crippled hull in the game, fleets and starbases together.
pub fn crippled_hull_count(game_data: &CoreGameData) -> u32 {
    let ships: u32 = game_data
        .fleets
        .iter()
        .map(|fleet| fleet.crippled.total())
        .sum();
    ships
        + game_data
            .bases
            .iter()
            .map(|base| base.crippled.total())
            .sum::<u32>()
}
