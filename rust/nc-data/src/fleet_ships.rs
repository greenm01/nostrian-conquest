//! What a fleet carries: its ship totals, its troop transports and the
//! armies aboard them, the top speed its slowest ship allows, and the
//! composition labels the screens and reports show.

use crate::Fleet;

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
/// the empty and loaded counts. The transport total grows if it cannot
/// hold `armies`.
pub fn set_armies_aboard(fleet: &mut Fleet, armies: u16) {
    let total = troop_transports(fleet);
    fleet.ships.loaded_transports = armies;
    fleet.ships.empty_transports = total.saturating_sub(armies);
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
    let mut max_speed = 7;
    for (count, speed) in [
        (fleet.ships.etacs, 3),
        (fleet.ships.battleships, 4),
        (fleet.ships.cruisers, 5),
        (troop_transports(fleet), 5),
        (fleet.ships.destroyers, 6),
        (fleet.ships.scouts, 6),
    ] {
        if count > 0 {
            max_speed = max_speed.min(speed);
        }
    }
    fleet.max_speed = max_speed;
    if fleet.speed > max_speed {
        fleet.speed = max_speed;
    }
}

/// The fleet's ships as `SC=1 BB=2 ...`, armies counted as `AR`.
pub fn ship_composition_summary(fleet: &Fleet) -> String {
    let parts: Vec<String> = [
        ("SC", fleet.ships.scouts),
        ("BB", fleet.ships.battleships),
        ("CA", fleet.ships.cruisers),
        ("DD", fleet.ships.destroyers),
        ("TT", troop_transports(fleet)),
        ("AR", fleet.ships.loaded_transports),
        ("ET", fleet.ships.etacs),
    ]
    .into_iter()
    .filter(|&(_, count)| count > 0)
    .map(|(label, count)| format!("{label}={count}"))
    .collect();
    assert!(!parts.is_empty(), "empty fleet record is not a fleet");
    parts.join(" ")
}

/// The fleet's ships as the fleet list shows them: `2BB DD TT* 3TT`, with
/// loaded transports marked `TT*`.
pub fn ship_composition_table_summary(fleet: &Fleet) -> String {
    fn token(label: &str, count: u16) -> Option<String> {
        match count {
            0 => None,
            1 => Some(label.to_string()),
            _ => Some(format!("{count}{label}")),
        }
    }

    let parts: Vec<String> = [
        token("SC", fleet.ships.scouts),
        token("BB", fleet.ships.battleships),
        token("CA", fleet.ships.cruisers),
        token("DD", fleet.ships.destroyers),
        token("TT*", fleet.ships.loaded_transports),
        token("TT", fleet.ships.empty_transports),
        token("ET", fleet.ships.etacs),
    ]
    .into_iter()
    .flatten()
    .collect();
    assert!(!parts.is_empty(), "empty fleet record is not a fleet");
    parts.join(" ")
}
