//! Starbase reports from the year's ticks, and the report of a guard that
//! arrives at its base and merges into the guard already there. The text is
//! classic ECMAINT's (`0b5d:5793`, `0b5d:0e47`).

use nc_data::{CoreGameData, LostWorldResponse, MaintenanceEvents, Mission, StarbaseReport};

use crate::maint::results::entries::{ReportEntry, ReportTarget, narrative_phase_for_report_text};
use crate::maint::results::format::*;
use crate::maint::results::mod_constants::*;

pub fn push_starbase_entries(
    entries: &mut Vec<ReportEntry>,
    game_data: &CoreGameData,
    events: &MaintenanceEvents,
    year: u16,
) {
    for event in &events.starbase_report_events {
        let [x, y] = event.coords;
        let place = if event.in_system { "System" } else { "Sector" };
        let source = format!(
            "From Starbase {}, located in {place}({x},{y}):",
            event.starbase_id
        );
        let body = match event.report {
            StarbaseReport::Arrived { orbit } => format!(
                " We have arrived at our destination and are beginning our assignment of {}",
                if orbit {
                    "Protection & Enhancement."
                } else {
                    "Deep-Space Surveillance."
                }
            ),
            StarbaseReport::LostWorld {
                planet_idx,
                system: [sx, sy],
                response,
            } => {
                let planet = planet_idx
                    .and_then(|idx| game_data.planets.get(idx))
                    .map_or_else(String::new, |planet| planet.name.clone());
                let action = match response {
                    LostWorldResponse::Holding => {
                        "we are holding our current position and awaiting new orders.".to_string()
                    }
                    LostWorldResponse::Leaving => format!(
                        "we are moving the base out of the solar system. Our new destination is Sector({x},{y})."
                    ),
                };
                format!(
                    " Since we have lost control of planet \"{planet}\" in System({sx},{sy}), {action}"
                )
            }
        };
        push(
            entries,
            &source,
            body,
            event.owner_empire_raw,
            event.stardate_week,
            year,
        );
    }

    for event in events
        .fleet_merge_events
        .iter()
        .filter(|event| event.kind == Mission::GuardStarbase)
    {
        let [x, y] = event.coords;
        let source = owned_fleet_source_clause(
            Some(event.absorbed_fleet_number),
            &format!("System({x},{y})"),
        );
        let body = format!(
            " Guard Starbase mission report: We have arrived at {} and are merging with the {}.",
            starbase_at(game_data, event.owner_empire_raw, event.coords),
            fleet_label(event.host_fleet_number)
        );
        push(
            entries,
            &source,
            body,
            event.owner_empire_raw,
            event.stardate_week,
            year,
        );
    }
}

/// "Starbase N" for the owner's base at `coords`.
pub fn starbase_at(game_data: &CoreGameData, owner: u8, coords: [u8; 2]) -> String {
    game_data
        .bases
        .iter()
        .find(|base| {
            base.sector.to_array() == coords && base.owner.0 == owner && base.base_id.0 != 0
        })
        .map(|base| format!("Starbase {}", base.base_id.0))
        .unwrap_or_else(|| "the assigned starbase".to_string())
}

fn push(
    entries: &mut Vec<ReportEntry>,
    source: &str,
    body: String,
    recipient: u8,
    stardate_week: Option<u8>,
    year: u16,
) {
    let header = report_header(source, stardate_week, year);
    entries.push(ReportEntry {
        text: format!("{header}{body}"),
        kind: 0x05,
        tail: RESULTS_TAIL_FLEET,
        target: ReportTarget::Both { recipient },
        repeat_next_pointer: false,
        stardate_week,
        narrative_phase: narrative_phase_for_report_text(&body),
    });
}
