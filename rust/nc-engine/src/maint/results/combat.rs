use super::format::{join_report_parts, ship_loss_summary, unit_count_text};
use nc_data::{MaintenanceEvents, Mission, MissionOutcome, ShipLosses};

pub fn enemy_losses_sentence(losses: ShipLosses) -> String {
    let summary = ship_loss_summary(losses);
    if summary == "no ship losses" {
        "We were unable to inflict any losses.".to_string()
    } else {
        format!("We observed alien ship casualties of {summary}.")
    }
}

pub fn enemy_retreat_reported_for_battle(
    events: &MaintenanceEvents,
    battle_event: &nc_data::FleetBattleEvent,
) -> bool {
    if !battle_event.held_field {
        return false;
    }

    events.encounter_disposition_events.iter().any(|event| {
        matches!(
            event,
            nc_data::EncounterDispositionEvent::Retreated {
                owner_empire_raw,
                coords,
                ..
            } if *coords == battle_event.coords
                && battle_event.enemy_empires_raw.contains(owner_empire_raw)
        )
    })
}

pub fn battle_outcome_sentence(
    events: &MaintenanceEvents,
    battle_event: &nc_data::FleetBattleEvent,
) -> String {
    if battle_event.held_field {
        if enemy_retreat_reported_for_battle(events, battle_event) {
            "The enemy fled the field.".to_string()
        } else if battle_event.enemy_initial == battle_event.enemy_losses
            && battle_event.enemy_initial_starbases == battle_event.enemy_starbases_destroyed
        {
            "The aliens were completely destroyed.".to_string()
        } else {
            "We held the field.".to_string()
        }
    } else {
        "We were forced to disengage.".to_string()
    }
}

pub fn enemy_starbase_outcome_line(starbases_destroyed: u32) -> Option<String> {
    match starbases_destroyed {
        0 => None,
        1 => Some("Enemy starbase destroyed.".to_string()),
        count => Some(format!("Enemy starbases destroyed: {count}.")),
    }
}

pub fn fleet_command_last_contact_value(
    enemy: &str,
    coords: [u8; 2],
    was_intercepting: bool,
) -> String {
    let [x, y] = coords;
    if was_intercepting {
        format!("destroyed while intercepting {enemy} in System({x},{y})")
    } else {
        format!("destroyed by {enemy} in System({x},{y})")
    }
}

pub fn contact_fleet_description(event: &nc_data::ScoutContactEvent) -> String {
    let summary = contact_size_summary_from_counts(
        event.small_vessels,
        event.medium_vessels,
        event.large_vessels,
    );
    if event.small_vessels == 0 && event.medium_vessels == 0 && event.large_vessels == 0 {
        summary
    } else {
        format!("{summary} of unknown type")
    }
}

pub fn contact_size_summary_from_counts(
    small_vessels: u32,
    medium_vessels: u32,
    large_vessels: u32,
) -> String {
    match (large_vessels > 0, medium_vessels > 0, small_vessels > 0) {
        (true, true, true) => format!(
            "{} large, {} medium, and {} small vessel(s)",
            large_vessels, medium_vessels, small_vessels
        ),
        (true, true, false) => format!(
            "{} large and {} medium vessel(s)",
            large_vessels, medium_vessels
        ),
        (true, false, true) => format!(
            "{} large and {} small vessel(s)",
            large_vessels, small_vessels
        ),
        (false, true, true) => format!(
            "{} medium and {} small vessel(s)",
            medium_vessels, small_vessels
        ),
        (true, false, false) => format!("{} large vessel(s)", large_vessels),
        (false, true, false) => format!("{} medium vessel(s)", medium_vessels),
        (false, false, true) => format!("{} small vessel(s)", small_vessels),
        (false, false, false) => "no combat vessels".to_string(),
    }
}

pub fn assault_attacker_force_summary(event: &nc_data::AssaultReportEvent) -> String {
    fleet_force_summary(event.attacker_initial, event.attacker_loaded_armies_initial)
}

pub fn ground_force_summary(batteries: u8, armies: u8) -> Option<String> {
    let mut parts = Vec::new();
    if batteries > 0 {
        parts.push(unit_count_text(
            batteries.into(),
            "ground battery",
            "ground batteries",
        ));
    }
    if armies > 0 {
        parts.push(unit_count_text(armies.into(), "army", "armies"));
    }
    if parts.is_empty() {
        None
    } else {
        Some(join_report_parts(&parts))
    }
}

pub fn ground_losses_summary(batteries: u8, armies: u8) -> Option<String> {
    ground_force_summary(batteries, armies)
}

pub fn combat_losses_value(losses: ShipLosses, starbases: u32) -> String {
    combat_loss_summary(losses, starbases, "none")
}

pub fn destroyed_fleet_enemy_force_value(event: &nc_data::FleetDestroyedEvent) -> String {
    let mut parts = Vec::new();
    let fleet_summary = fleet_force_summary_with_starbases(
        event.enemy_initial,
        event.enemy_loaded_armies_initial,
        event.enemy_initial_starbases,
    );
    if fleet_summary != "no ships" {
        parts.push(fleet_summary);
    }
    if let Some(ground_summary) = ground_force_summary(
        event.enemy_ground_batteries_initial,
        event.enemy_ground_armies_initial,
    ) {
        parts.push(ground_summary);
    }
    if parts.is_empty() {
        "none".to_string()
    } else {
        join_report_parts(&parts)
    }
}

pub fn destroyed_fleet_enemy_losses_value(event: &nc_data::FleetDestroyedEvent) -> String {
    let mut parts = Vec::new();
    let ship_summary = combat_loss_summary(
        event.enemy_losses,
        event.enemy_starbases_destroyed,
        "no ship losses",
    );
    if ship_summary != "no ship losses" {
        parts.push(ship_summary);
    }
    if let Some(ground_summary) = ground_losses_summary(
        event.enemy_ground_battery_losses,
        event.enemy_ground_army_losses,
    ) {
        parts.push(ground_summary);
    }
    if parts.is_empty() {
        "none".to_string()
    } else {
        join_report_parts(&parts)
    }
}

pub fn ground_force_value(batteries: u8, armies: u8, none_value: &str) -> String {
    ground_force_summary(batteries, armies).unwrap_or_else(|| none_value.to_string())
}

pub fn ground_losses_value(batteries: u8, armies: u8) -> String {
    ground_losses_summary(batteries, armies).unwrap_or_else(|| "none".to_string())
}

pub fn all_planetary_defenses_destroyed(
    initial_batteries: u8,
    initial_armies: u8,
    battery_losses: u8,
    army_losses: u8,
) -> bool {
    (initial_batteries > 0 || initial_armies > 0)
        && initial_batteries == battery_losses
        && initial_armies == army_losses
}

pub fn planetary_defense_outcome_line(
    initial_batteries: u8,
    initial_armies: u8,
    battery_losses: u8,
    army_losses: u8,
) -> String {
    if all_planetary_defenses_destroyed(
        initial_batteries,
        initial_armies,
        battery_losses,
        army_losses,
    ) {
        "All planetary defenses were destroyed.".to_string()
    } else {
        format!(
            "Defensive losses: {}.",
            ground_losses_value(battery_losses, army_losses)
        )
    }
}

pub fn blitz_cover_value(event: &nc_data::AssaultReportEvent) -> Option<String> {
    if event.defender_batteries_initial == 0 {
        None
    } else if event.defender_battery_losses > 0 {
        Some(format!(
            "{} ground batteries briefly suppressed",
            event.defender_battery_losses
        ))
    } else {
        Some("cover fire failed to suppress the defending batteries".to_string())
    }
}

pub fn transport_loss_value(event: &nc_data::AssaultReportEvent) -> String {
    if event.transport_army_losses > 0 {
        format!(
            "{} troop(s) lost in destroyed transports",
            event.transport_army_losses
        )
    } else if event.attacker_army_losses > 0 {
        "none in destroyed transports".to_string()
    } else {
        "none".to_string()
    }
}

pub fn bombardment_collateral_damage_lines(
    stardock_items_destroyed: u32,
    stored_goods_destroyed: u32,
    factories_destroyed: u16,
    is_attacker: bool,
) -> Vec<String> {
    let mut lines = Vec::new();
    let prefix = if is_attacker {
        "Bombing damage"
    } else {
        "Local damage"
    };
    if stardock_items_destroyed > 0 {
        lines.push(format!(
            "{prefix}: {} destroyed.",
            unit_count_text(stardock_items_destroyed, "stardock item", "stardock items")
        ));
    }
    if factories_destroyed > 0 {
        lines.push(format!(
            "{prefix}: {factories_destroyed} points of industry destroyed."
        ));
    }
    if stored_goods_destroyed > 0 {
        lines.push(format!(
            "{prefix}: {stored_goods_destroyed} stored production destroyed."
        ));
    }
    lines
}

pub fn assault_friendly_losses_summary(
    ship_losses: ShipLosses,
    army_losses: u32,
    transport_army_losses: u32,
) -> String {
    let ship_summary = ship_loss_summary(ship_losses);
    let mut parts = Vec::new();
    if ship_summary != "no ship losses" {
        parts.push(ship_summary);
    }
    let total_army_losses = army_losses.saturating_add(transport_army_losses);
    if total_army_losses > 0 {
        parts.push(unit_count_text(total_army_losses, "army", "armies"));
    }
    if parts.is_empty() {
        "none".to_string()
    } else {
        join_report_parts(&parts)
    }
}

pub fn assault_enemy_losses_summary(batteries: u8, armies: u8) -> String {
    ground_losses_summary(batteries, armies).unwrap_or_else(|| "none".to_string())
}

pub fn invasion_softening_losses_summary(event: &nc_data::AssaultReportEvent) -> Option<String> {
    if event.kind != Mission::InvadeWorld
        || event.outcome == MissionOutcome::Aborted
        || event.defender_armies_initial == 0
    {
        return None;
    }
    Some(if event.defender_army_losses_softening > 0 {
        unit_count_text(
            u32::from(event.defender_army_losses_softening),
            "army",
            "armies",
        )
    } else {
        "none".to_string()
    })
}

pub fn invasion_softening_losses_label(is_attacker_report: bool) -> &'static str {
    if is_attacker_report {
        "Enemy orbital softening losses:"
    } else {
        "Our orbital softening losses:"
    }
}

pub fn invasion_ground_battle_losses_summary(
    event: &nc_data::AssaultReportEvent,
) -> Option<String> {
    if event.kind != Mission::InvadeWorld
        || event.outcome == MissionOutcome::Aborted
        || event.defender_armies_initial == 0
    {
        return None;
    }
    let losses = event
        .defender_army_losses
        .saturating_sub(event.defender_army_losses_softening);
    Some(if losses > 0 {
        unit_count_text(u32::from(losses), "army", "armies")
    } else {
        "none".to_string()
    })
}

pub fn invasion_ground_battle_losses_label(is_attacker_report: bool) -> &'static str {
    if is_attacker_report {
        "Enemy ground battle losses:"
    } else {
        "Our ground battle losses:"
    }
}

pub fn roe_retreat_loss_clause(losses: ShipLosses) -> String {
    let summary = ship_loss_summary(losses);
    if summary == "no ship losses" {
        "without suffering losses".to_string()
    } else {
        format!("after suffering losses of {summary}")
    }
}

pub fn stardock_scan_summary(planet: &nc_data::Planet) -> String {
    use nc_data::ProductionItemKind;

    let mut parts = Vec::new();
    for slot in 0..nc_data::STARDOCK_SLOT_COUNT {
        let count = planet.stardock_count_raw(slot);
        if count == 0 {
            continue;
        }
        let kind = ProductionItemKind::from_raw(planet.stardock_kind_raw(slot));
        let name = match kind {
            ProductionItemKind::Destroyer => format!("{}DD", count),
            ProductionItemKind::Cruiser => format!("{}CA", count),
            ProductionItemKind::Battleship => format!("{}BB", count),
            ProductionItemKind::Scout => format!("{}SC", count),
            ProductionItemKind::Transport => format!("{}TT", count),
            ProductionItemKind::Etac => format!("{}ET", count),
            ProductionItemKind::Starbase => format!("{}SB", count),
            ProductionItemKind::GroundBattery
            | ProductionItemKind::Army
            | ProductionItemKind::Unknown(_) => continue,
        };
        parts.push(name);
    }
    if parts.is_empty() {
        "The planet's stardock appears to be empty.".to_string()
    } else {
        format!(
            "Scanning the planet's stardock, we detected {}.",
            parts.join(", ")
        )
    }
}

pub fn combat_loss_summary(losses: ShipLosses, starbases: u32, no_loss_text: &str) -> String {
    let ship_summary = ship_loss_summary(losses);
    let mut parts = Vec::new();
    if ship_summary != "no ship losses" {
        parts.push(ship_summary);
    }
    if starbases > 0 {
        parts.push(format!("{}SB", starbases));
    }
    if parts.is_empty() {
        no_loss_text.to_string()
    } else {
        parts.join(", ")
    }
}

pub fn fleet_force_summary(losses: ShipLosses, loaded_armies: u32) -> String {
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
        if loaded_armies > 0 {
            parts.push(format!("{}TT*", loaded_armies));
            if losses.transports > loaded_armies {
                parts.push(format!("{}TT", losses.transports - loaded_armies));
            }
        } else {
            parts.push(format!("{}TT", losses.transports));
        }
    }
    if losses.etacs > 0 {
        parts.push(format!("{}ET", losses.etacs));
    }
    if parts.is_empty() {
        "no ships".to_string()
    } else {
        parts.join(", ")
    }
}

pub fn fleet_force_summary_with_starbases(
    losses: ShipLosses,
    loaded_armies: u32,
    starbases: u32,
) -> String {
    let ship_summary = fleet_force_summary(losses, loaded_armies);
    if starbases > 0 {
        let sb = format!("{}SB", starbases);
        if ship_summary == "no ships" {
            sb
        } else {
            format!("{ship_summary}, {sb}")
        }
    } else {
        ship_summary
    }
}

pub fn is_starbase_only_force(losses: ShipLosses, loaded_armies: u32, starbases: u32) -> bool {
    losses == ShipLosses::default() && loaded_armies == 0 && starbases > 0
}
