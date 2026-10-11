use crate::maint::timing::format_report_first_line;
use nc_data::{Composition, CoreGameData, Mission, MissionOutcome, ShipCategory, ShipLosses};

pub fn report_header(source_clause: &str, week: Option<u8>, year: u16) -> String {
    format_report_first_line(source_clause, week.unwrap_or(1), year)
}

pub fn empire_label(game_data: &CoreGameData, empire_code: u8) -> String {
    let idx = empire_code.saturating_sub(1) as usize;
    let Some(player) = game_data.players.get(idx) else {
        return format!("Empire #{empire_code}");
    };
    let empire = player.empire_name.trim().to_string();
    let handle = player.handle.trim().to_string();
    let legacy = player.empire_name.trim().to_string();
    if !empire.is_empty() {
        format!("Empire #{empire_code} \"{empire}\"")
    } else if !handle.is_empty() {
        format!("Empire #{empire_code} \"{handle}\"")
    } else if !legacy.is_empty() {
        format!("Empire #{empire_code} \"{legacy}\"")
    } else {
        format!("Empire #{empire_code}")
    }
}

pub fn classic_empire_display_name(game_data: &CoreGameData, empire_code: u8) -> Option<String> {
    let idx = empire_code.saturating_sub(1) as usize;
    let player = game_data.players.get(idx)?;
    let empire = player.empire_name.trim().to_string();
    if !empire.is_empty() {
        return Some(empire);
    }
    let legacy = player.empire_name.trim().to_string();
    if !legacy.is_empty() {
        return Some(legacy);
    }
    let handle = player.handle.trim().to_string();
    if !handle.is_empty() {
        return Some(handle);
    }
    None
}

pub fn classic_empire_clause(game_data: &CoreGameData, empire: u8) -> String {
    if let Some(name) = classic_empire_display_name(game_data, empire) {
        format!("\"{name}\", (Empire #{empire})")
    } else {
        format!("Empire #{empire}")
    }
}

pub fn ordinal_number(value: usize) -> String {
    let suffix = match value % 100 {
        11..=13 => "th",
        _ => match value % 10 {
            1 => "st",
            2 => "nd",
            3 => "rd",
            _ => "th",
        },
    };
    format!("{value}{suffix}")
}

pub fn fleet_label(fleet_number: u8) -> String {
    format!("{} Fleet", ordinal_number(fleet_number as usize))
}

pub fn owned_fleet_source_clause(fleet_number: Option<u8>, location: &str) -> String {
    match fleet_number.filter(|fleet_number| *fleet_number != 0) {
        Some(fleet_number) => format!(
            "From your {}, located in {}:",
            fleet_label(fleet_number),
            location
        ),
        None => format!("From your fleet, located in {}:", location),
    }
}

pub fn owned_fleet_source_clause_from_idx(
    game_data: &CoreGameData,
    fleet_idx: usize,
    location: &str,
) -> String {
    let fleet_number = fleet_number_from_idx(game_data, fleet_idx);
    owned_fleet_source_clause(fleet_number, location)
}

pub fn fleet_number_from_idx(game_data: &CoreGameData, fleet_idx: usize) -> Option<u8> {
    game_data
        .fleets
        .get(fleet_idx)
        .map(|fleet| fleet.fleet_number.0 as u8)
        .filter(|fleet_number| *fleet_number != 0)
}

pub fn known_hostile_fleet_label(
    game_data: &CoreGameData,
    fleet_number: Option<u8>,
    empire: u8,
) -> Option<String> {
    let fleet_number = fleet_number.filter(|fleet_number| *fleet_number != 0)?;
    Some(format!(
        "the {} of {}",
        fleet_label(fleet_number),
        classic_empire_clause(game_data, empire)
    ))
}

fn capitalize_leading_ascii(text: &str) -> String {
    let mut chars = text.chars();
    match chars.next() {
        Some(first) => {
            let mut value = first.to_ascii_uppercase().to_string();
            value.push_str(chars.as_str());
            value
        }
        None => String::new(),
    }
}

pub fn classic_enemy_reference(
    game_data: &CoreGameData,
    fleet_number: Option<u8>,
    empire: u8,
) -> String {
    known_hostile_fleet_label(game_data, fleet_number, empire)
        .unwrap_or_else(|| classic_empire_clause(game_data, empire))
}

pub fn classic_enemy_reference_titlecase(
    game_data: &CoreGameData,
    fleet_number: Option<u8>,
    empire: u8,
) -> String {
    capitalize_leading_ascii(&classic_enemy_reference(game_data, fleet_number, empire))
}

pub fn mission_short_label(kind: Mission) -> &'static str {
    match kind {
        Mission::MoveOnly => "Move",
        Mission::SeekHome => "Seek Home",
        Mission::PatrolSector => "Patrol",
        Mission::ViewWorld => "View",
        Mission::GuardStarbase => "Guard SB",
        Mission::GuardBlockadeWorld => "Guard",
        Mission::ScoutSector => "Scout",
        Mission::ScoutSolarSystem => "Scout SS",
        Mission::BombardWorld => "Bombard",
        Mission::InvadeWorld => "Invade",
        Mission::BlitzWorld => "Blitz",
        Mission::Salvage => "Salvage",
        Mission::JoinAnotherFleet => "Join",
        Mission::RendezvousSector => "Rendezvous",
        _ => "Mission",
    }
}

pub fn mission_report_label(kind: Mission) -> &'static str {
    match kind {
        Mission::MoveOnly => "Move mission report",
        Mission::PatrolSector => "Patrol mission report",
        Mission::GuardStarbase => "Guard Starbase mission report",
        Mission::JoinAnotherFleet => "Join mission report",
        Mission::RendezvousSector => "Rendezvous mission report",
        Mission::GuardBlockadeWorld => "Guard/Blockade World mission report",
        Mission::Salvage => "Salvage mission report",
        Mission::ViewWorld => "Viewing mission report",
        Mission::BombardWorld => "Bombardment mission report",
        Mission::InvadeWorld => "Invasion mission report",
        Mission::BlitzWorld => "Blitz mission report",
        Mission::SeekHome => "Seek-Home mission report",
        _ => "Scouting mission report",
    }
}

pub fn mission_report_prefix(kind: Mission) -> String {
    format!(" {}:", mission_report_label(kind))
}

pub fn mission_location_phrase(kind: Mission, coords: [u8; 2]) -> String {
    let [x, y] = coords;
    match kind {
        Mission::MoveOnly
        | Mission::PatrolSector
        | Mission::ScoutSector
        | Mission::JoinAnotherFleet
        | Mission::RendezvousSector => {
            format!("Sector({x},{y})")
        }
        _ => format!("System({x},{y})"),
    }
}

pub fn coords_system_text(coords: [u8; 2]) -> String {
    let [x, y] = coords;
    format!("System({x},{y})")
}

pub fn nearest_owned_destination_text(
    game_data: &CoreGameData,
    empire: u8,
    coords: [u8; 2],
) -> String {
    if let Some(planet) = game_data
        .planets
        .iter()
        .find(|planet| planet.sector.to_array() == coords && planet.owner.0 == empire)
    {
        format!(
            "planet \"{}\", located in {}",
            planet.name.clone(),
            coords_system_text(coords)
        )
    } else {
        coords_system_text(coords)
    }
}

/// The ships and starbases a side lost, in ship notation (`BB2 CA1 SB1`).
/// A loss record counts transports without their cargo, and the notation
/// keeps loaded (`TT*`) and empty (`TT`) transports apart, so lost transports
/// are named in words rather than given a cargo they may not have had.
/// `None` when nothing was lost.
pub fn loss_summary(losses: ShipLosses, starbases: u32) -> Option<String> {
    let tokens = Composition::undamaged([
        (ShipCategory::Scouts, losses.scouts),
        (ShipCategory::Battleships, losses.battleships),
        (ShipCategory::Cruisers, losses.cruisers),
        (ShipCategory::Destroyers, losses.destroyers),
        (ShipCategory::Etacs, losses.etacs),
    ])
    .with_starbases(starbases)
    .notation();
    let transports = (losses.transports > 0)
        .then(|| unit_count_text(losses.transports, "transport", "transports"));
    match (tokens.is_empty(), transports) {
        (true, None) => None,
        (false, None) => Some(tokens),
        (true, Some(transports)) => Some(transports),
        (false, Some(transports)) => Some(format!("{tokens} and {transports}")),
    }
}

pub fn ship_loss_summary(losses: ShipLosses) -> String {
    loss_summary(losses, 0).unwrap_or_else(|| "no ship losses".to_string())
}

pub fn unit_count_text(count: u32, singular: &str, plural: &str) -> String {
    if count == 1 {
        format!("1 {singular}")
    } else {
        format!("{count} {plural}")
    }
}

pub fn join_report_parts(parts: &[String]) -> String {
    match parts {
        [] => String::new(),
        [only] => only.clone(),
        [left, right] => format!("{left} and {right}"),
        _ => {
            let mut text = parts[..parts.len() - 1].join(", ");
            text.push_str(" and ");
            text.push_str(parts.last().unwrap());
            text
        }
    }
}

pub fn compact_fleet_number_list(numbers: &[u8]) -> String {
    let mut sorted = numbers.to_vec();
    sorted.sort_unstable();
    let parts = sorted
        .iter()
        .map(|fleet_number| fleet_number.to_string())
        .collect::<Vec<_>>();
    match parts.as_slice() {
        [] => "0 fleets".to_string(),
        [only] => format!("fleet {only}"),
        [left, right] => format!("fleets {left} and {right}"),
        _ => {
            let mut text = String::from("fleets ");
            text.push_str(&parts[..parts.len() - 1].join(", "));
            text.push_str(", and ");
            text.push_str(parts.last().unwrap());
            text
        }
    }
}

pub fn structured_fleet_battle_alert() -> &'static str {
    "ALERT: Enemy fleet contact!"
}

pub fn structured_fleet_destroyed_alert() -> &'static str {
    "ALERT: Fleet contact lost!"
}

pub fn structured_starbase_destroyed_alert() -> &'static str {
    "ALERT: Starbase contact lost!"
}

pub fn structured_bombardment_alert() -> &'static str {
    "ALERT: Orbital bombardment underway!"
}

pub fn structured_assault_alert(kind: Mission, outcome: MissionOutcome) -> &'static str {
    match (kind, outcome) {
        (Mission::InvadeWorld, MissionOutcome::Succeeded) => {
            "ALERT: Planetary invasion successful!"
        }
        (Mission::InvadeWorld, MissionOutcome::Failed) => "ALERT: Planetary invasion repulsed!",
        (Mission::InvadeWorld, MissionOutcome::Aborted) => "ALERT: Planetary invasion blocked!",
        (Mission::BlitzWorld, MissionOutcome::Succeeded) => "ALERT: Blitz assault successful!",
        (Mission::BlitzWorld, MissionOutcome::Failed) => "ALERT: Blitz assault failed!",
        (Mission::BlitzWorld, MissionOutcome::Aborted) => "ALERT: Blitz assault blocked!",
        _ => "ALERT: Combat underway!",
    }
}

pub fn structured_capture_alert(kind: Mission) -> &'static str {
    match kind {
        Mission::BlitzWorld => "ALERT: Planet lost to enemy blitz!",
        _ => "ALERT: Planet lost to enemy invasion!",
    }
}

pub fn structured_repulsed_assault_alert(kind: Mission, outcome: MissionOutcome) -> &'static str {
    match (kind, outcome) {
        (Mission::InvadeWorld, MissionOutcome::Failed) => "ALERT: Enemy invasion repulsed!",
        (Mission::InvadeWorld, MissionOutcome::Aborted) => "ALERT: Enemy invasion blocked!",
        (Mission::BlitzWorld, MissionOutcome::Failed) => "ALERT: Enemy blitz repulsed!",
        (Mission::BlitzWorld, MissionOutcome::Aborted) => "ALERT: Enemy blitz blocked!",
        _ => "ALERT: Hostile assault repulsed!",
    }
}

pub fn aborted_mission_follow_on_text(
    game_data: &CoreGameData,
    fleet: &nc_data::Fleet,
    empire: u8,
) -> String {
    use nc_data::Order;
    if fleet.order == Order::SeekHome && fleet.speed > 0 {
        let retreat_target = fleet.target.to_array();
        format!(
            "withdrawing toward {}",
            nearest_owned_destination_text(game_data, empire, retreat_target)
        )
    } else {
        "holding position and awaiting orders".to_string()
    }
}

/// What a fleet did after the abort in `event`. A fleet that withdrew early
/// in the year may already have reached the world it withdrew toward and
/// stopped there; it still withdrew.
pub fn aborted_event_follow_on_text(
    game_data: &CoreGameData,
    fleet: &nc_data::Fleet,
    event: &nc_data::MissionEvent,
) -> String {
    if let (Some(at), Some(target)) = (event.location_coords, event.target_coords)
        && at != target
        && fleet.order == nc_data::Order::HoldPosition
        && fleet.sector.to_array() == target
    {
        return format!(
            "withdrawing toward {}",
            nearest_owned_destination_text(game_data, event.owner_empire, target)
        );
    }
    aborted_mission_follow_on_text(game_data, fleet, event.owner_empire)
}

pub fn aborted_mission_follow_on_text_from_idx(
    game_data: &CoreGameData,
    fleet_idx: usize,
    empire: u8,
) -> String {
    game_data
        .fleets
        .get(fleet_idx)
        .map(|fleet| aborted_mission_follow_on_text(game_data, fleet, empire))
        .unwrap_or_else(|| "holding position and awaiting orders".to_string())
}

#[cfg(test)]
mod tests {
    //! The reports write compositions in the notation's one class order
    //! (`SC BB CA DD TT* TT ETAC SB`), whatever order their sources keep.

    use super::loss_summary;
    use crate::maint::results::combat::{
        fleet_force_summary_with_starbases, stardock_scan_summary,
    };
    use nc_data::{Planet, ShipLosses, stardock_slot_for_kind};

    #[test]
    fn losses_list_classes_in_the_notation_order() {
        let losses = ShipLosses {
            destroyers: 4,
            cruisers: 3,
            battleships: 2,
            scouts: 1,
            transports: 5,
            etacs: 6,
        };
        // Transports are named in words: a loss record has no cargo.
        assert_eq!(
            loss_summary(losses, 7).as_deref(),
            Some("SC1 BB2 CA3 DD4 ETAC6 SB7 and 5 transports")
        );
    }

    #[test]
    fn forces_list_classes_in_the_notation_order() {
        let force = ShipLosses {
            destroyers: 4,
            cruisers: 3,
            battleships: 2,
            scouts: 1,
            transports: 9,
            etacs: 6,
        };
        assert_eq!(
            fleet_force_summary_with_starbases(force, 4, 7),
            "SC1 BB2 CA3 DD4 TT*4 TT5 ETAC6 SB7"
        );
    }

    #[test]
    fn stardock_scans_list_classes_in_the_notation_order() {
        let mut planet = Planet::default();
        // Dock a ship of every kind a stardock holds.
        for code in [9, 6, 5, 4, 3, 2, 1] {
            let slot = stardock_slot_for_kind(code).expect("a stardock kind");
            planet.stardock[slot] = u32::from(code);
        }
        assert_eq!(
            stardock_scan_summary(&planet),
            "Scanning the planet's stardock, we detected SC4 BB3 CA2 DD1 TT5 ETAC6 SB9."
        );
    }
}
