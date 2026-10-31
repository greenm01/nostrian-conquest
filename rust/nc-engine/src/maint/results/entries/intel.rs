use nc_data::{
    ContactReportSource, CoreGameData, MaintenanceEvents, Mission, MissionOutcome,
    PlanetIntelEvent, PlanetIntelSnapshot, PlanetIntelSource, StarbaseAlert,
};

use crate::maint::results::combat::*;
use crate::maint::results::entries::{
    NarrativePhase, ReportEntry, ReportTarget, narrative_phase_for_report_text,
};
use crate::maint::results::format::*;
use crate::maint::results::mod_constants::*;
use crate::maint::results::structured::*;

pub fn matching_planet_intel_event<'a>(
    events: &'a MaintenanceEvents,
    event: &nc_data::MissionEvent,
) -> Option<&'a PlanetIntelEvent> {
    let source = match event.kind {
        Mission::ViewWorld => PlanetIntelSource::ViewWorld,
        Mission::ScoutSolarSystem => PlanetIntelSource::ScoutSolarSystem,
        _ => return None,
    };
    events.planet_intel_events.iter().find(|intel_event| {
        intel_event.viewer_empire == event.owner_empire
            && intel_event.source == source
            && intel_event.source_fleet_idx == Some(event.fleet_idx)
    })
}

pub fn owner_clause_from_snapshot(
    snapshot: &PlanetIntelSnapshot,
    game_data: &CoreGameData,
) -> String {
    match snapshot.known_owner_empire_id {
        Some(0) => "unowned".to_string(),
        Some(owner) => format!("owned by {}", classic_empire_clause(game_data, owner)),
        None => "of unknown ownership".to_string(),
    }
}

pub fn stardock_scan_summary_from_snapshot(snapshot: &PlanetIntelSnapshot) -> String {
    match snapshot.known_docked_summary.as_deref() {
        None | Some("Nothing") => "The planet's stardock appears to be empty.".to_string(),
        Some(summary) => format!("Scanning the planet's stardock, we detected {summary}."),
    }
}

const SCOUTED_WORLD_LABELS: [&str; 6] = [
    "Owned by:",
    "Potential production:",
    "Estimated production:",
    "Estimated amount of stored goods:",
    "Number of armies:",
    "Number of ground batteries:",
];

/// Scout Solar System report body: a narrative lead-in, the world's facts as
/// labeled rows in `SCOUTED_WORLD_LABELS` order, then the stardock scan.
pub fn scouted_world_report_body(
    planet_name: &str,
    facts: [String; 6],
    stardock_summary: &str,
) -> String {
    let mut lines = vec![format!(
        " Scouting mission report: We are in extended orbit around planet \"{planet_name}\" and have compiled the following data:"
    )];
    for (label, value) in SCOUTED_WORLD_LABELS.iter().zip(facts) {
        lines.extend(wrap_labeled_lines(&format!("{label} "), &value));
    }
    lines.push(stardock_summary.to_string());
    lines.join("\n")
}

/// Scout Solar System report body for an unowned world: classic gives only
/// its potential.
pub fn unowned_world_scout_body(coords: [u8; 2], potential: u16) -> String {
    let [x, y] = coords;
    format!(
        " Scouting mission report: We are in extended orbit around the world in System({x},{y}). The world is unowned but has a potential value of {potential} points."
    )
}

pub fn push_intel_entries(
    entries: &mut Vec<ReportEntry>,
    game_data: &CoreGameData,
    events: &MaintenanceEvents,
    year: u16,
) {
    let mut seen_contacts: std::collections::HashSet<(u8, u8, [u8; 2], bool)> = {
        let mut set = std::collections::HashSet::new();
        for event in &events.encounter_disposition_events {
            let (owner, target, coords) = match *event {
                nc_data::EncounterDispositionEvent::NoEngagement {
                    owner_empire,
                    target_empire,
                    coords,
                    ..
                }
                | nc_data::EncounterDispositionEvent::Retreated {
                    owner_empire,
                    target_empire,
                    coords,
                    ..
                }
                | nc_data::EncounterDispositionEvent::PursuitFire {
                    owner_empire,
                    target_empire,
                    coords,
                    ..
                } => (owner_empire, target_empire, coords),
            };
            // Their reports name the fleet; an earlier contact still stands.
            set.insert((owner, target, coords, true));
        }
        set
    };
    for event in &events.scout_contact_events {
        // Suppress empty fleet contacts — no intelligence value.
        if event.small_vessels == 0 && event.medium_vessels == 0 && event.large_vessels == 0 {
            continue;
        }
        // Deduplicate fleet-source contacts; starbase contacts always pass through
        // as they carry unique intelligence about the starbase's detection.
        if !matches!(event.source, ContactReportSource::Starbase { .. }) {
            let contact_key = (
                event.viewer_empire,
                event.target_empire,
                event.coords,
                event.identified,
            );
            if !seen_contacts.insert(contact_key) {
                continue;
            }
        }
        let [x, y] = event.coords;
        let fleet_description = contact_fleet_description(event);
        match event.source {
            ContactReportSource::FleetMission(_) | ContactReportSource::Fleet(_) => {
                let label = match event.source {
                    ContactReportSource::FleetMission(kind) => {
                        format!(" {}:", mission_report_label(kind))
                    }
                    _ => String::new(),
                };
                let location = contact_location(event);
                let source = owned_fleet_source_clause(event.reporting_fleet_number, &location);
                let header = report_header(&source, event.stardate_week, year);
                let body = format!(
                    "{label} {}",
                    fleet_contact_body(game_data, event, &location, &fleet_description)
                );
                entries.push(ReportEntry {
                    text: format!("{header}{body}"),
                    kind: 0x06,
                    tail: RESULTS_TAIL_SCOUTING,
                    target: ReportTarget::Both {
                        recipient: event.viewer_empire,
                    },
                    repeat_next_pointer: false,
                    stardate_week: event.stardate_week,
                    narrative_phase: narrative_phase_for_report_text(&body),
                });
            }
            ContactReportSource::Starbase { id, alert } => {
                let source = format!("From Starbase {id}, located in System({x},{y}):");
                let header = report_header(&source, event.stardate_week, year);
                let enemy = known_hostile_fleet_label(
                    game_data,
                    event.target_fleet_number,
                    event.target_empire,
                )
                .unwrap_or_else(|| classic_empire_clause(game_data, event.target_empire));
                let outcome = match alert {
                    StarbaseAlert::HeadedAtOurWorld => {
                        let planet = game_data
                            .planets
                            .iter()
                            .find(|planet| planet.sector.to_array() == event.coords)
                            .map(|planet| planet.name.clone())
                            .unwrap_or_default();
                        format!(
                            "The aliens appear to be headed at our planet {planet}! We are alerting all fleets in the area."
                        )
                    }
                    StarbaseAlert::Enemy => "We are alerting all fleets in the area.".to_string(),
                    StarbaseAlert::Scanning => "Continuing with scanning assignment...".to_string(),
                };
                let body = format!(
                    " We have located and identified an alien fleet in System({x},{y}). It is {enemy}. Their fleet contains {fleet_description}. {outcome}"
                );
                entries.push(ReportEntry {
                    text: format!("{header}{body}"),
                    kind: 0x06,
                    tail: RESULTS_TAIL_SCOUTING,
                    target: ReportTarget::Both {
                        recipient: event.viewer_empire,
                    },
                    repeat_next_pointer: false,
                    stardate_week: event.stardate_week,
                    narrative_phase: narrative_phase_for_report_text(&body),
                });
            }
        }
    }

    for event in &events.starbase_sighting_events {
        let [x, y] = event.coords;
        let place = if event.in_system { "System" } else { "Sector" };
        let source = format!(
            "From Starbase {}, located in {place}({x},{y}):",
            event.starbase_id
        );
        let header = report_header(&source, event.stardate_week, year);
        let outcome = if event.alerting {
            "We are alerting all fleets in the area."
        } else {
            "Continuing with scanning assignment..."
        };
        let body = format!(
            " We have located and identified an alien starbase in {place}({x},{y}). It is Starbase {} of {}. {outcome}",
            event.target_starbase_id,
            classic_empire_clause(game_data, event.target_empire),
        );
        entries.push(ReportEntry {
            text: format!("{header}{body}"),
            kind: 0x06,
            tail: RESULTS_TAIL_SCOUTING,
            target: ReportTarget::Both {
                recipient: event.viewer_empire,
            },
            repeat_next_pointer: false,
            stardate_week: event.stardate_week,
            narrative_phase: narrative_phase_for_report_text(&body),
        });
    }

    // ----- Ownership change events -----
    for event in &events.ownership_change_events {
        if event.reporting_empire == 0 {
            continue;
        }
        let Some(planet) = game_data.planets.get(event.planet_idx) else {
            continue;
        };
        let [x, y] = planet.sector.to_array();
        let source = format!(
            "From planet \"{}\" in System({x},{y}):",
            planet.name.clone()
        );
        let header = report_header(&source, event.stardate_week, year);
        let items = if let Some(assault) = events.assault_report_events.iter().find(|assault| {
            assault.planet_idx == event.planet_idx
                && assault.attacker_empire == event.new_owner_empire
                && assault.defender_empire == event.reporting_empire
                && assault.outcome == MissionOutcome::Succeeded
        }) {
            let invader = classic_empire_clause(game_data, event.new_owner_empire);
            let context_rows = vec![StructuredBodyItem::Label {
                label: LABEL_INVADER.to_string(),
                value: invader.clone(),
            }];
            let force_rows = vec![
                StructuredBodyItem::Label {
                    label: LABEL_ATTACKING_FORCE.to_string(),
                    value: assault_attacker_force_summary(assault),
                },
                StructuredBodyItem::Label {
                    label: LABEL_OUR_DEFENSES.to_string(),
                    value: ground_force_value(
                        assault.defender_batteries_initial,
                        assault.defender_armies_initial,
                        "none",
                    ),
                },
            ];
            let mut outcome_rows = Vec::new();
            if assault.defender_batteries_initial > 0 || assault.defender_armies_initial > 0 {
                outcome_rows.push(StructuredBodyItem::Text(planetary_defense_outcome_line(
                    assault.defender_batteries_initial,
                    assault.defender_armies_initial,
                    assault.defender_battery_losses,
                    assault.defender_army_losses,
                )));
            }
            outcome_rows.push(StructuredBodyItem::Label {
                label: LABEL_ENEMY_LOSSES.to_string(),
                value: assault_friendly_losses_summary(
                    assault.attacker_ship_losses,
                    assault.attacker_army_losses,
                    assault.transport_army_losses,
                ),
            });
            if let Some(softening_losses) = invasion_softening_losses_summary(assault) {
                outcome_rows.push(StructuredBodyItem::Label {
                    label: format!("{} ", invasion_softening_losses_label(false)),
                    value: softening_losses,
                });
            }
            if let Some(ground_battle_losses) = invasion_ground_battle_losses_summary(assault) {
                outcome_rows.push(StructuredBodyItem::Label {
                    label: format!("{} ", invasion_ground_battle_losses_label(false)),
                    value: ground_battle_losses,
                });
            }
            structured_combat_body(
                structured_capture_alert(assault.kind),
                context_rows,
                force_rows,
                outcome_rows,
            )
        } else {
            structured_combat_body(
                structured_capture_alert(Mission::InvadeWorld),
                Vec::new(),
                Vec::new(),
                vec![StructuredBodyItem::Text(format!(
                    "Planet captured by {}.",
                    classic_empire_clause(game_data, event.new_owner_empire),
                ))],
            )
        };
        let text = structured_report_text(&header, items);
        entries.push(ReportEntry {
            text,
            kind: 0x0c,
            tail: RESULTS_TAIL_INVASION,
            target: ReportTarget::Both {
                recipient: event.reporting_empire,
            },
            repeat_next_pointer: false,
            stardate_week: event.stardate_week,
            narrative_phase: NarrativePhase::DefenderAftermath,
        });
    }
}

/// The layer the observer was in when it made the sighting.
pub fn contact_location(event: &nc_data::ScoutContactEvent) -> String {
    let [x, y] = event.coords;
    if event.in_system {
        format!("System({x},{y})")
    } else {
        format!("Sector({x},{y})")
    }
}

/// "a translight speed of N", or "sublight speed" for a fleet at rest.
pub fn contact_speed_text(speed: u8) -> String {
    if speed == 0 {
        "sublight speed".to_string()
    } else {
        format!("a translight speed of {speed}")
    }
}

/// A fleet's sighting of an alien fleet: its speed and, once identified,
/// whose it is, what it holds and what the fleet decided.
fn fleet_contact_body(
    game_data: &CoreGameData,
    event: &nc_data::ScoutContactEvent,
    location: &str,
    fleet_description: &str,
) -> String {
    let speed = contact_speed_text(event.target_speed);
    let force = fleet_force_summary(
        event.reporting_initial,
        event.reporting_loaded_armies_initial,
    );
    if !event.identified {
        return format!(
            "Sensor contact -- detected an alien fleet in {location} traveling at {speed}. We have {force}. We could not identify it."
        );
    }
    let owner = match known_hostile_fleet_label(
        game_data,
        event.target_fleet_number,
        event.target_empire,
    ) {
        Some(enemy) => format!("It is {enemy}."),
        None => format!(
            "It belongs to {}.",
            classic_empire_clause(game_data, event.target_empire)
        ),
    };
    let decision = match event.decision {
        Some(nc_data::ContactDecision::Ignore) => " Ignoring alien fleet.",
        Some(nc_data::ContactDecision::Engage) => " Accelerating to engage the enemy.",
        None => "",
    };
    format!(
        "Sensor contact -- detected and identified an alien fleet in {location} traveling at {speed}. We have {force}. {owner} Their fleet contains {fleet_description}.{decision}"
    )
}
