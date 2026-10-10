use crate::maint::timing::format_report_first_line;
use nc_data::{CoreGameData, Mission, MissionOutcome, ShipLosses};

pub fn report_header(source_clause: &str, week: Option<u8>, year: u16) -> String {
    format_report_first_line(source_clause, week.unwrap_or(1), year)
}

pub fn empire_label(game_data: &CoreGameData, empire_raw: u8) -> String {
    let idx = empire_raw.saturating_sub(1) as usize;
    let Some(player) = game_data.player.records.get(idx) else {
        return format!("Empire #{empire_raw}");
    };
    let empire = player.controlled_empire_name_summary();
    let handle = player.assigned_player_handle_summary();
    let legacy = player.legacy_status_name_summary();
    if !empire.is_empty() {
        format!("Empire #{empire_raw} \"{empire}\"")
    } else if !handle.is_empty() {
        format!("Empire #{empire_raw} \"{handle}\"")
    } else if !legacy.is_empty() {
        format!("Empire #{empire_raw} \"{legacy}\"")
    } else {
        format!("Empire #{empire_raw}")
    }
}

pub fn classic_empire_display_name(game_data: &CoreGameData, empire_raw: u8) -> Option<String> {
    let idx = empire_raw.saturating_sub(1) as usize;
    let player = game_data.player.records.get(idx)?;
    let empire = player.controlled_empire_name_summary();
    if !empire.is_empty() {
        return Some(empire);
    }
    let legacy = player.legacy_status_name_summary();
    if !legacy.is_empty() {
        return Some(legacy);
    }
    let handle = player.assigned_player_handle_summary();
    if !handle.is_empty() {
        return Some(handle);
    }
    None
}

pub fn classic_empire_clause(game_data: &CoreGameData, empire_raw: u8) -> String {
    if let Some(name) = classic_empire_display_name(game_data, empire_raw) {
        format!("\"{name}\", (Empire #{empire_raw})")
    } else {
        format!("Empire #{empire_raw}")
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
        .records
        .get(fleet_idx)
        .map(|fleet| fleet.local_slot_word_raw() as u8)
        .filter(|fleet_number| *fleet_number != 0)
}

pub fn known_hostile_fleet_label(
    game_data: &CoreGameData,
    fleet_number: Option<u8>,
    empire_raw: u8,
) -> Option<String> {
    let fleet_number = fleet_number.filter(|fleet_number| *fleet_number != 0)?;
    Some(format!(
        "the {} of {}",
        fleet_label(fleet_number),
        classic_empire_clause(game_data, empire_raw)
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
    empire_raw: u8,
) -> String {
    known_hostile_fleet_label(game_data, fleet_number, empire_raw)
        .unwrap_or_else(|| classic_empire_clause(game_data, empire_raw))
}

pub fn classic_enemy_reference_titlecase(
    game_data: &CoreGameData,
    fleet_number: Option<u8>,
    empire_raw: u8,
) -> String {
    capitalize_leading_ascii(&classic_enemy_reference(
        game_data,
        fleet_number,
        empire_raw,
    ))
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
    empire_raw: u8,
    coords: [u8; 2],
) -> String {
    if let Some(planet) = game_data.planets.records.iter().find(|planet| {
        planet.coords_raw() == coords && planet.owner_empire_slot_raw() == empire_raw
    }) {
        format!(
            "planet \"{}\", located in {}",
            planet.planet_name(),
            coords_system_text(coords)
        )
    } else {
        coords_system_text(coords)
    }
}

pub fn ship_loss_summary(losses: ShipLosses) -> String {
    let mut parts = Vec::new();
    if losses.battleships > 0 {
        parts.push(format!("{}BB", losses.battleships));
    }
    if losses.cruisers > 0 {
        parts.push(format!("{}CA", losses.cruisers));
    }
    if losses.destroyers > 0 {
        parts.push(format!("{}DD", losses.destroyers));
    }
    if losses.scouts > 0 {
        parts.push(format!("{}SC", losses.scouts));
    }
    if losses.transports > 0 {
        parts.push(format!("{}TT", losses.transports));
    }
    if losses.etacs > 0 {
        parts.push(format!("{}ET", losses.etacs));
    }
    if parts.is_empty() {
        "no ship losses".to_string()
    } else {
        parts.join(", ")
    }
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
    fleet: &nc_data::FleetRecord,
    empire_raw: u8,
) -> String {
    use nc_data::Order;
    if fleet.standing_order_kind() == Order::SeekHome && fleet.current_speed() > 0 {
        let retreat_target = fleet.standing_order_target_coords_raw();
        format!(
            "withdrawing toward {}",
            nearest_owned_destination_text(game_data, empire_raw, retreat_target)
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
    fleet: &nc_data::FleetRecord,
    event: &nc_data::MissionEvent,
) -> String {
    if let (Some(at), Some(target)) = (event.location_coords, event.target_coords)
        && at != target
        && fleet.standing_order_kind() == nc_data::Order::HoldPosition
        && fleet.current_location_coords_raw() == target
    {
        return format!(
            "withdrawing toward {}",
            nearest_owned_destination_text(game_data, event.owner_empire_raw, target)
        );
    }
    aborted_mission_follow_on_text(game_data, fleet, event.owner_empire_raw)
}

pub fn aborted_mission_follow_on_text_from_idx(
    game_data: &CoreGameData,
    fleet_idx: usize,
    empire_raw: u8,
) -> String {
    game_data
        .fleets
        .records
        .get(fleet_idx)
        .map(|fleet| aborted_mission_follow_on_text(game_data, fleet, empire_raw))
        .unwrap_or_else(|| "holding position and awaiting orders".to_string())
}
