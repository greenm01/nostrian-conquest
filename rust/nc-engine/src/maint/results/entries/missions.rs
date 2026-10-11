use std::collections::{BTreeMap, BTreeSet};

use nc_data::{
    CoreGameData, MaintenanceEvents, Mission, MissionAbortReason, MissionOutcome,
    present_production_points, stored_production_points,
};

use crate::maint::results::combat::*;
use crate::maint::results::compose::{
    AbortDisposition, fleet_abort_disposition, fleet_abort_disposition_text,
    matching_roe_abort_disposition_index, mission_event_has_assault_report,
    mission_event_has_fleet_destroyed,
};
use crate::maint::results::entries::intel::{
    matching_planet_intel_event, owner_clause_from_snapshot, scouted_world_report_body,
    stardock_scan_summary_from_snapshot,
};
use crate::maint::results::entries::{
    NarrativePhase, ReportEntry, ReportTarget, narrative_phase_for_report_text,
};
use crate::maint::results::format::*;
use crate::maint::results::mod_constants::*;
use crate::maint::results::structured::*;

fn roe_abort_outcome_text(kind: Mission) -> &'static str {
    match kind {
        Mission::BombardWorld => {
            "This forced us to break off the bombardment mission and leave the target world."
        }
        Mission::InvadeWorld => {
            "This forced us to abort the invasion before the landing could begin."
        }
        Mission::BlitzWorld => {
            "This forced us to abort the assault before the landing could begin."
        }
        Mission::ColonizeWorld => {
            "This forced us to abandon our colony attempt before it could proceed."
        }
        Mission::ViewWorld => {
            "This forced us to abandon the viewing mission before it could be completed."
        }
        Mission::ScoutSector | Mission::ScoutSolarSystem => {
            "This forced us to abandon our scouting assignment."
        }
        Mission::GuardStarbase => "This forced us to abandon our starbase guard assignment.",
        Mission::GuardBlockadeWorld => {
            "This forced us to abandon our guarding/blockading assignment."
        }
        Mission::PatrolSector => "This forced us to abandon our patrol assignment.",
        Mission::MoveOnly => "This forced us to abandon our move mission.",
        Mission::Salvage => "This forced us to abandon salvage operations.",
        Mission::JoinAnotherFleet => "This forced us to abandon our join mission.",
        Mission::RendezvousSector => "This forced us to abandon our rendezvous assignment.",
        _ => "This forced us to abandon our mission.",
    }
}

fn hostile_assault_abort_text(
    kind: Mission,
    reason: Option<MissionAbortReason>,
    follow_on: &str,
) -> String {
    match (kind, reason) {
        (Mission::BombardWorld, Some(MissionAbortReason::OrbitBlocked)) => format!(
            " Bombardment mission report: Hostile forces denied us bombardment position around the target world. We are aborting the mission and {follow_on}."
        ),
        (Mission::InvadeWorld, Some(MissionAbortReason::OrbitBlocked)) => format!(
            " Invasion mission report: We could not secure orbit for the landing. We are aborting the mission and {follow_on}."
        ),
        (Mission::BlitzWorld, Some(MissionAbortReason::OrbitBlocked)) => format!(
            " Blitz mission report: We could not secure orbit for the landing. We are aborting the mission and {follow_on}."
        ),
        (Mission::BombardWorld, _) => format!(
            " Bombardment mission report: Hostile action stripped us of our bombardment capability. We are aborting the mission and {follow_on}."
        ),
        (Mission::InvadeWorld, _) => format!(
            " Invasion mission report: Hostile action stripped us of our invasion capability before the landing could begin. We are aborting the mission and {follow_on}."
        ),
        (Mission::BlitzWorld, _) => format!(
            " Blitz mission report: Hostile action stripped us of our assault capability before the landing could begin. We are aborting the mission and {follow_on}."
        ),
        _ => unreachable!("hostile assault abort text only applies to bombard/invade/blitz"),
    }
}

fn merged_roe_abort_report_body(
    game_data: &CoreGameData,
    event: &nc_data::MissionEvent,
    disposition: &nc_data::EncounterDispositionEvent,
) -> String {
    let prefix = mission_report_prefix(event.kind);
    match disposition {
        nc_data::EncounterDispositionEvent::Retreated {
            friendly_initial,
            friendly_loaded_armies_initial,
            target_empire_raw,
            target_fleet_number,
            enemy_initial,
            retreat_target_coords,
            losses_sustained,
            enemy_losses_inflicted,
            ..
        } => format!(
            "{prefix} We engaged {}. We had {}. The alien force contained {}. In accordance with our ROE, we withdrew toward {} {}. {} {}",
            classic_enemy_reference(game_data, *target_fleet_number, *target_empire_raw),
            fleet_force_summary(*friendly_initial, *friendly_loaded_armies_initial),
            fleet_force_summary(*enemy_initial, 0),
            nearest_owned_destination_text(
                game_data,
                event.owner_empire_raw,
                *retreat_target_coords
            ),
            roe_retreat_loss_clause(*losses_sustained),
            enemy_losses_sentence(*enemy_losses_inflicted),
            roe_abort_outcome_text(event.kind),
        ),
        nc_data::EncounterDispositionEvent::PursuitFire {
            friendly_initial,
            friendly_loaded_armies_initial,
            enemy_initial,
            target_empire_raw,
            target_fleet_number,
            retreat_target_coords,
            losses_sustained,
            enemy_losses_inflicted,
            ..
        } => format!(
            "{prefix} We had {}. We attempted to disengage from {} in accordance with our ROE, but suffered pursuit fire from an alien force containing {} while withdrawing toward {} {}. {} {}",
            fleet_force_summary(*friendly_initial, *friendly_loaded_armies_initial),
            classic_enemy_reference(game_data, *target_fleet_number, *target_empire_raw),
            fleet_force_summary(*enemy_initial, 0),
            nearest_owned_destination_text(
                game_data,
                event.owner_empire_raw,
                *retreat_target_coords
            ),
            roe_retreat_loss_clause(*losses_sustained),
            enemy_losses_sentence(*enemy_losses_inflicted),
            roe_abort_outcome_text(event.kind),
        ),
        _ => unreachable!("only ROE retreat dispositions should reach merged abort text"),
    }
}

pub fn push_mission_entries(
    entries: &mut Vec<ReportEntry>,
    game_data: &CoreGameData,
    events: &MaintenanceEvents,
    year: u16,
) -> BTreeSet<usize> {
    let mut consumed_roe_disposition_indices = BTreeSet::new();
    //
    // Pre-pass: group MissionOutcome::Aborted events for the same empire at
    // the same coords into a single batched report when 2+ fleets qualify.
    // Events consumed here are tracked in `batched_abort_indices` and skipped
    // in the main loop below.
    //
    // Suppression rules (applied before grouping):
    //   1. Fleet was destroyed → FleetDestroyedEvent already covers it.
    //   2. Assault succeeded/failed → AssaultReportEvent already covers it
    //      (existing rule, preserved here).
    //
    // Grouping key: (owner_empire_raw, coords, AbortDisposition)
    // Within a group, fleets are listed as "Fleet N (ShortLabel)".
    let mut batched_abort_indices: BTreeSet<usize> = BTreeSet::new();
    {
        // Collect qualifying aborted events by group key.
        // Value: Vec of (event_index, fleet_id, mission_short_label, stardate_week)
        let mut groups: BTreeMap<
            (u8, [u8; 2], AbortDisposition),
            Vec<(usize, u8, &'static str, Option<u8>)>,
        > = BTreeMap::new();
        for (ev_idx, event) in events.mission_events.iter().enumerate() {
            if event.outcome != MissionOutcome::Aborted {
                continue;
            }
            // Suppression 1: fleet was destroyed.
            if mission_event_has_fleet_destroyed(game_data, events, event) {
                batched_abort_indices.insert(ev_idx);
                continue;
            }
            // Suppression 2: assault report covers this (existing rule).
            if mission_event_has_assault_report(events, event) {
                batched_abort_indices.insert(ev_idx);
                continue;
            }
            if matching_roe_abort_disposition_index(events, event).is_some() {
                continue;
            }
            let Some(fleet) = game_data.fleets.get(event.fleet_idx) else {
                continue;
            };
            let coords = event
                .location_coords
                .unwrap_or_else(|| fleet.sector.to_array());
            let disposition = fleet_abort_disposition(fleet);
            let fleet_number = fleet.fleet_number.0 as u8;
            let label = mission_short_label(event.kind);
            groups
                .entry((event.owner_empire_raw, coords, disposition))
                .or_default()
                .push((ev_idx, fleet_number, label, event.stardate_week));
        }
        // Emit one batched report per group that has 2+ fleets; mark all as handled.
        // Single-fleet groups are left for the main loop.
        for ((empire_raw, coords, disposition), mut fleet_entries) in groups {
            if fleet_entries.len() < 2 {
                continue;
            }
            // Sort by fleet number for stable, predictable output.
            fleet_entries.sort_by_key(|&(_, fleet_number, _, _)| fleet_number);
            let [x, y] = coords;
            let fleet_list = join_report_parts(
                &fleet_entries
                    .iter()
                    .map(|(_, fleet_number, label, _)| {
                        format!("Fleet {} ({})", fleet_number, label)
                    })
                    .collect::<Vec<_>>(),
            );
            let disposition_text = fleet_abort_disposition_text(disposition);
            let source = "From your Fleet Command Center:".to_string();
            // Use the earliest stardate_week among the group for the header.
            let week = fleet_entries.iter().filter_map(|(_, _, _, w)| *w).min();
            let header = report_header(&source, week, year);
            let body = format!(
                " Hostile action forced {} to abort their missions in System({x},{y}). They are {}.",
                fleet_list, disposition_text,
            );
            entries.push(ReportEntry {
                text: format!("{header}{body}"),
                kind: 0x05,
                tail: RESULTS_TAIL_FLEET,
                target: ReportTarget::Both {
                    recipient: empire_raw,
                },
                repeat_next_pointer: false,
                stardate_week: week,
                narrative_phase: narrative_phase_for_report_text(&body),
            });
            for (ev_idx, _, _, _) in &fleet_entries {
                batched_abort_indices.insert(*ev_idx);
            }
        }
    }

    let rendezvous_merged_fleet_indices: std::collections::HashSet<usize> = events
        .fleet_merge_events
        .iter()
        // Absorbed-side indices are stale once the fleet merges away.
        .filter(|e| e.kind == Mission::RendezvousSector && e.survivor_side)
        .map(|e| e.fleet_idx)
        .collect();

    for (ev_idx, event) in events.mission_events.iter().enumerate() {
        // Skip events already handled by the batched-abort pre-pass.
        if batched_abort_indices.contains(&ev_idx) {
            continue;
        }
        let Some(fleet) = game_data.fleets.get(event.fleet_idx) else {
            continue;
        };
        let coords = event
            .location_coords
            .unwrap_or_else(|| fleet.sector.to_array());
        let [x, y] = coords;
        let mission_location = mission_location_phrase(event.kind, coords);
        let source_clause =
            owned_fleet_source_clause_from_idx(game_data, event.fleet_idx, &mission_location);
        let merged_roe_abort_disposition = matching_roe_abort_disposition_index(events, event)
            .and_then(|idx| {
                events
                    .encounter_disposition_events
                    .get(idx)
                    .map(|disp| (idx, disp))
            });
        let (kind, tail, source, body) = if let Some((disposition_idx, disposition)) =
            merged_roe_abort_disposition
        {
            consumed_roe_disposition_indices.insert(disposition_idx);
            let tail = match event.kind {
                Mission::BombardWorld => RESULTS_TAIL_BOMBARD,
                Mission::InvadeWorld | Mission::BlitzWorld => RESULTS_TAIL_INVASION,
                Mission::ColonizeWorld => RESULTS_TAIL_COLONIZATION,
                Mission::ViewWorld | Mission::ScoutSector | Mission::ScoutSolarSystem => {
                    RESULTS_TAIL_SCOUTING
                }
                _ => RESULTS_TAIL_FLEET,
            };
            let kind = match event.kind {
                Mission::BombardWorld => 0x08,
                Mission::InvadeWorld | Mission::BlitzWorld => 0x0c,
                Mission::ColonizeWorld => 0x09,
                Mission::ViewWorld | Mission::ScoutSector | Mission::ScoutSolarSystem => 0x07,
                _ => 0x05,
            };
            (
                kind,
                tail,
                source_clause.clone(),
                MissionReportBody::Narrative(merged_roe_abort_report_body(
                    game_data,
                    event,
                    disposition,
                )),
            )
        } else {
            match (event.kind, event.outcome) {
            (Mission::MoveOnly, MissionOutcome::Succeeded) => (
                0x05u8,
                RESULTS_TAIL_FLEET,
                source_clause.clone(),
                MissionReportBody::Narrative(
                    " Move mission report: We have arrived at our destination and are awaiting new orders.".to_string(),
                ),
            ),
            (Mission::RendezvousSector, MissionOutcome::Arrived) => {
                if rendezvous_merged_fleet_indices.contains(&event.fleet_idx) {
                    continue;
                }
                (
                    0x05,
                    RESULTS_TAIL_FLEET,
                    source_clause.clone(),
                    MissionReportBody::Narrative(
                        " Rendezvous mission report: We have arrived at our rendezvous point and are waiting for more fleets to arrive.".to_string(),
                    ),
                )
            }
            (Mission::GuardStarbase, MissionOutcome::Arrived) => {
                let starbase_text = game_data
                    .bases
                    .iter()
                    .find(|base| {
                        base.sector.to_array() == coords
                            && base.owner.0 == event.owner_empire_raw
                            && base.base_id.0 != 0
                    })
                    .map(|base| format!("Starbase {}", base.base_id.0))
                    .unwrap_or_else(|| "the assigned starbase".to_string());
                (
                    0x05,
                    RESULTS_TAIL_FLEET,
                    source_clause.clone(),
                    MissionReportBody::Narrative(
                        format!(" Guard Starbase mission report: We have arrived at {starbase_text} and are beginning our guard/escort mission."),
                    ),
                )
            }
            (Mission::GuardStarbase, MissionOutcome::Failed) => {
                // A guard already at the base would pass the ship limit if
                // this one merged into it.
                let guard_label = fleet.mission_target;
                let partner = game_data
                    .fleets
                    .iter()
                    .enumerate()
                    .find(|(idx, other)| {
                        *idx != event.fleet_idx
                            && other.owner.0 == event.owner_empire_raw
                            && other.sector.to_array() == coords
                            && other.order == nc_data::Order::GuardStarbase
                            && other.mission_target == guard_label
                    })
                    .map_or_else(
                        || "a fleet".to_string(),
                        |(_, other)| format!("the {}", fleet_label(other.fleet_number.0 as u8)),
                    );
                (
                    0x05,
                    RESULTS_TAIL_FLEET,
                    source_clause.clone(),
                    MissionReportBody::Narrative(format!(
                        " Guard Starbase mission report: We have arrived at {} and have found {partner} already guarding it. However, since a fleet cannot have more than 3000 ships, we cannot merge with them. We are therefore aborting our mission and are awaiting new orders.",
                        super::starbase::starbase_at(game_data, event.owner_empire_raw, coords)
                    )),
                )
            }
            (Mission::GuardBlockadeWorld, MissionOutcome::Arrived) => {
                let body = if let Some(planet_idx) = event.planet_idx {
                    if let Some(planet) = game_data.planets.get(planet_idx) {
                        format!(
                            " Guard/Blockade World mission report: We have arrived at planet \"{}\" in Sector({x},{y}) and are beginning our guarding/blockading assignment.",
                            planet.name.clone(),
                        )
                    } else {
                        " Guard/Blockade World mission report: We have arrived at our assigned world and are beginning our guarding/blockading assignment.".to_string()
                    }
                } else {
                    " Guard/Blockade World mission report: We have arrived at our assigned world and are beginning our guarding/blockading assignment.".to_string()
                };
                (
                    0x05,
                    RESULTS_TAIL_FLEET,
                    source_clause.clone(),
                    MissionReportBody::Narrative(body),
                )
            }
            (Mission::PatrolSector, MissionOutcome::Arrived) => (
                0x05,
                RESULTS_TAIL_FLEET,
                source_clause.clone(),
                MissionReportBody::Narrative(
                    " Patrol mission report: We have arrived at our destination and are beginning our patrolling assignment.".to_string(),
                ),
            ),
            (Mission::SeekHome, MissionOutcome::Succeeded) => (
                0x05,
                RESULTS_TAIL_FLEET,
                source_clause.clone(),
                MissionReportBody::Narrative(
                    " Seek-Home mission report: We have arrived at our destination and are awaiting new orders.".to_string(),
                ),
            ),
            (Mission::BombardWorld, MissionOutcome::Arrived) => (
                // Arrival-only notice (Rust-only; original ECMAINT bombards
                // on the same turn the fleet arrives). Use kind=0x05 to
                // avoid blank padding — the text is only ~3 lines.
                0x05,
                RESULTS_TAIL_BOMBARD,
                source_clause.clone(),
                MissionReportBody::Narrative(
                    " Bombardment mission report: We have arrived at our target world and are preparing for bombardment.".to_string(),
                ),
            ),
            (Mission::InvadeWorld, MissionOutcome::Arrived) => (
                0x05,
                RESULTS_TAIL_INVASION,
                source_clause.clone(),
                MissionReportBody::Narrative(
                    " Invasion mission report: We have arrived at our target world and are preparing to begin the invasion.".to_string(),
                ),
            ),
            (Mission::BlitzWorld, MissionOutcome::Arrived) => (
                0x05,
                RESULTS_TAIL_INVASION,
                source_clause.clone(),
                MissionReportBody::Narrative(
                    " Blitz mission report: We have arrived at our target world and are preparing to launch the assault.".to_string(),
                ),
            ),
            (Mission::MoveOnly, MissionOutcome::Aborted) => {
                let destination = fleet.target.to_array();
                let [dx, dy] = destination;
                (
                    0x05,
                    RESULTS_TAIL_FLEET,
                    source_clause.clone(),
                    MissionReportBody::Narrative(
                        format!(" Move mission report: Hostile action forced us to abort our mission and seek safety in System({dx},{dy})."),
                    ),
                )
            }
            (Mission::ColonizeWorld, MissionOutcome::Aborted) => (
                0x09,
                RESULTS_TAIL_COLONIZATION,
                source_clause.clone(),
                MissionReportBody::Narrative(format!(
                    " Colonization mission report: Hostile action forced us to abandon our colony attempt. We are {}.",
                    aborted_event_follow_on_text(game_data, fleet, event)
                )),
            ),
            (Mission::ViewWorld, MissionOutcome::Succeeded) => {
                let body = if let Some(intel_event) = matching_planet_intel_event(events, event) {
                    if let Some(snapshot) = intel_event.observed_snapshot.as_ref() {
                        format!(
                            " Viewing mission report: We have entered System({x},{y}) and have completed a long range viewing analysis of the world found within. The world is {} and has a potential of {} points. Until ordered otherwise, we will be moving out of the solar system.",
                            owner_clause_from_snapshot(snapshot, game_data),
                            snapshot.known_potential_production.unwrap_or(0),
                        )
                    } else if let Some(planet_idx) = event.planet_idx {
                        if let Some(planet) = game_data.planets.get(planet_idx) {
                            let owner_clause = if planet.owner.0 == 0 {
                                "unowned".to_string()
                            } else {
                                format!(
                                    "owned by {}",
                                    classic_empire_clause(
                                        game_data,
                                        planet.owner.0,
                                    )
                                )
                            };
                            format!(
                                " Viewing mission report: We have entered System({x},{y}) and have completed a long range viewing analysis of the world found within. The world is {owner_clause} and has a potential of {} points. Until ordered otherwise, we will be moving out of the solar system.",
                                u16::from(planet.potential_production),
                            )
                        } else {
                            format!(" Viewing mission report: We have entered System({x},{y}) and completed a long range viewing analysis.")
                        }
                    } else {
                        format!(" Viewing mission report: We have entered System({x},{y}) and completed a long range viewing analysis.")
                    }
                } else {
                    format!(" Viewing mission report: We have entered System({x},{y}) and completed a long range viewing analysis.")
                };
                (
                    0x07,
                    RESULTS_TAIL_SCOUTING,
                    source_clause.clone(),
                    MissionReportBody::Narrative(body),
                )
            }
            (Mission::ViewWorld, MissionOutcome::Failed) => (
                0x07,
                RESULTS_TAIL_SCOUTING,
                source_clause.clone(),
                MissionReportBody::Narrative(
                    " Viewing mission report: We found no world to analyze at the assigned destination.".to_string(),
                ),
            ),
            (Mission::ViewWorld, MissionOutcome::Aborted) => {
                (
                    0x07,
                    RESULTS_TAIL_SCOUTING,
                    source_clause.clone(),
                    MissionReportBody::Narrative(format!(
                        " Viewing mission report: We were attacked before the viewing mission could be completed. We are aborting our assignment and {}.",
                        aborted_event_follow_on_text(game_data, fleet, event)
                    )),
                )
            }
            (Mission::BombardWorld, MissionOutcome::Succeeded) => {
                let bombard_event = events.bombard_events.iter().find(|bombard| {
                    bombard.planet_idx == event.planet_idx.unwrap_or(usize::MAX)
                        && bombard.attacker_empire_raw == event.owner_empire_raw
                });
                let bombard_force_destroyed =
                    mission_event_has_fleet_destroyed(game_data, events, event);
                let body = if let Some(planet_idx) = event.planet_idx {
                    if let Some(planet) = game_data.planets.get(planet_idx) {
                        let context_rows = vec![StructuredBodyItem::Label {
                            label: LABEL_TARGET_WORLD.to_string(),
                            value: format!("planet \"{}\"", planet.name.clone()),
                        }];
                        let force_rows = vec![
                            StructuredBodyItem::Label {
                                label: LABEL_OUR_FORCES.to_string(),
                                value: bombard_event
                                    .map(|e| {
                                        fleet_force_summary(
                                            e.attacker_initial,
                                            e.attacker_loaded_armies_initial,
                                        )
                                    })
                                    .unwrap_or_else(|| "unknown force levels".to_string()),
                            },
                            StructuredBodyItem::Label {
                                label: LABEL_WORLD_DEFENSES.to_string(),
                                value: bombard_event
                                    .map(|e| {
                                        ground_force_value(
                                            e.defender_batteries_initial,
                                            e.defender_armies_initial,
                                            "undefended",
                                        )
                                    })
                                    .unwrap_or_else(|| "unknown".to_string()),
                            },
                        ];
                        let mut outcome_rows = vec![
                            StructuredBodyItem::Text(format!(
                                "We have just concluded a bombing run against planet \"{}\".",
                                planet.name.clone()
                            )),
                            StructuredBodyItem::Label {
                                label: LABEL_OUR_LOSSES.to_string(),
                                value: bombard_event
                                    .map(|e| ship_loss_summary(e.attacker_losses))
                                    .unwrap_or_else(|| "unknown".to_string()),
                            },
                            StructuredBodyItem::Label {
                                label: LABEL_ENEMY_LOSSES.to_string(),
                                value: bombard_event
                                    .map(|e| {
                                        ground_losses_value(
                                            e.defender_battery_losses,
                                            e.defender_army_losses,
                                        )
                                    })
                                    .unwrap_or_else(|| "unknown".to_string()),
                            },
                        ];
                        if let Some(bombard_event) = bombard_event.filter(|e| e.breakthrough) {
                            outcome_rows.extend(
                                bombardment_collateral_damage_lines(
                                    bombard_event.stardock_items_destroyed,
                                    bombard_event.stored_goods_destroyed,
                                    bombard_event.factories_destroyed,
                                    true,
                                )
                                .into_iter()
                                .map(StructuredBodyItem::Text),
                            );
                        }
                        outcome_rows.push(StructuredBodyItem::Text(if bombard_force_destroyed {
                            "Hostile return fire destroyed the bombardment force."
                                .to_string()
                        } else {
                            "We are maintaining bombardment position and will continue next turn."
                                .to_string()
                        }));
                        MissionReportBody::Structured(structured_combat_body(
                            structured_bombardment_alert(),
                            context_rows,
                            force_rows,
                            outcome_rows,
                        ))
                    } else {
                        MissionReportBody::Narrative(
                            " Bombardment mission report: We have concluded our bombing run and are awaiting new orders.".to_string(),
                        )
                    }
                } else {
                    MissionReportBody::Narrative(
                        " Bombardment mission report: We have concluded our bombing run and are awaiting new orders.".to_string(),
                    )
                };
                (
                    0x08,
                    RESULTS_TAIL_BOMBARD,
                    source_clause.clone(),
                    body,
                )
            }
            (Mission::BombardWorld, MissionOutcome::Aborted) => (
                0x08,
                RESULTS_TAIL_BOMBARD,
                source_clause.clone(),
                MissionReportBody::Narrative(hostile_assault_abort_text(
                    Mission::BombardWorld,
                    event.abort_reason,
                    &aborted_event_follow_on_text(game_data, fleet, event),
                )),
            ),
            (Mission::InvadeWorld, MissionOutcome::Aborted) => (
                0x0c,
                RESULTS_TAIL_INVASION,
                source_clause.clone(),
                MissionReportBody::Narrative(hostile_assault_abort_text(
                    Mission::InvadeWorld,
                    event.abort_reason,
                    &aborted_event_follow_on_text(game_data, fleet, event),
                )),
            ),
            (Mission::BlitzWorld, MissionOutcome::Aborted) => (
                0x0c,
                RESULTS_TAIL_INVASION,
                source_clause.clone(),
                MissionReportBody::Narrative(hostile_assault_abort_text(
                    Mission::BlitzWorld,
                    event.abort_reason,
                    &aborted_event_follow_on_text(game_data, fleet, event),
                )),
            ),
            (Mission::InvadeWorld, _) | (Mission::BlitzWorld, _) => continue,
            (Mission::ScoutSector, MissionOutcome::Arrived) => (
                0x07,
                RESULTS_TAIL_SCOUTING,
                source_clause.clone(),
                MissionReportBody::Narrative(
                    " Scouting mission report: We have arrived at our destination and are beginning to scout this sector.".to_string(),
                ),
            ),
            (Mission::ScoutSector, MissionOutcome::Succeeded) => {
                // On-station scouts only report when they detect something.
                // No news is good news — suppress the repeating status message.
                continue;
            }
            (Mission::ScoutSector, MissionOutcome::Aborted) => {
                (
                    0x07,
                    RESULTS_TAIL_SCOUTING,
                    source_clause.clone(),
                    MissionReportBody::Narrative(format!(
                        " Scouting mission report: Hostile action forced us to abort our scouting mission and {}.",
                        aborted_event_follow_on_text(game_data, fleet, event)
                    )),
                )
            }
            (Mission::ScoutSolarSystem, MissionOutcome::Arrived)
            | (Mission::ScoutSolarSystem, MissionOutcome::Succeeded) => {
                if let Some(intel_event) = matching_planet_intel_event(events, event) {
                    if let Some(snapshot) = intel_event.observed_snapshot.as_ref() {
                        let owner = match snapshot.known_owner_empire_id {
                            Some(0) => "Unowned world".to_string(),
                            Some(owner) => classic_empire_clause(game_data, owner),
                            None => "Unknown".to_string(),
                        };
                        let body = scouted_world_report_body(
                            snapshot.known_name.as_deref().unwrap_or("Unknown"),
                            [
                                owner,
                                format!("{} points", snapshot.known_potential_production.unwrap_or(0)),
                                format!("{} points", snapshot.known_current_production.unwrap_or(snapshot.known_potential_production.unwrap_or(0) as u8)),
                                format!("{} points", snapshot.known_stored_points.unwrap_or(0)),
                                snapshot.known_armies.unwrap_or(0).to_string(),
                                snapshot.known_ground_batteries.unwrap_or(0).to_string(),
                            ],
                            &stardock_scan_summary_from_snapshot(snapshot),
                        );
                        (
                            0x0Bu8,
                            RESULTS_TAIL_SCOUTING,
                            source_clause.clone(),
                            MissionReportBody::Narrative(body),
                        )
                    } else if let Some(planet) = game_data
                        .planets
                        .iter()
                        .find(|planet| planet.sector.to_array() == [x, y])
                    {
                        let owner = if planet.owner.0 == 0 {
                            "Unowned world".to_string()
                        } else {
                            classic_empire_clause(game_data, planet.owner.0)
                        };
                        let stardock_summary = stardock_scan_summary(planet);
                        let body = scouted_world_report_body(
                            &planet.name.clone(),
                            [
                                owner,
                                format!("{} points", u16::from(planet.potential_production)),
                                format!(
                                    "{} points",
                                    present_production_points(planet)
                                ),
                                format!("{} points", stored_production_points(planet)),
                                u8::try_from(planet.armies).unwrap_or(u8::MAX).to_string(),
                                u8::try_from(planet.ground_batteries).unwrap_or(u8::MAX).to_string(),
                            ],
                            &stardock_summary,
                        );
                        (
                            0x0Bu8,
                            RESULTS_TAIL_SCOUTING,
                            source_clause.clone(),
                            MissionReportBody::Narrative(body),
                        )
                    } else {
                        (
                            0x07,
                            RESULTS_TAIL_SCOUTING,
                            source_clause.clone(),
                            MissionReportBody::Narrative(
                                " Scouting mission report: We have arrived at our destination and are beginning to scout this solar system.".to_string(),
                            ),
                        )
                    }
                } else {
                    (
                        0x07,
                        RESULTS_TAIL_SCOUTING,
                        source_clause.clone(),
                        MissionReportBody::Narrative(
                            " Scouting mission report: We have arrived at our destination and are beginning to scout this solar system.".to_string(),
                        ),
                    )
                }
            }
            (Mission::ScoutSolarSystem, MissionOutcome::Aborted) => {
                (
                    0x07,
                    RESULTS_TAIL_SCOUTING,
                    source_clause.clone(),
                    MissionReportBody::Narrative(format!(
                        " Scouting mission report: We were forced to break off our close reconnaissance and {}.",
                        aborted_event_follow_on_text(game_data, fleet, event)
                    )),
                )
            }
            _ => continue,
        }
        };
        let header = report_header(&source, event.stardate_week, year);
        let (text, narrative_phase) = match body {
            MissionReportBody::Narrative(body) => {
                let phase = narrative_phase_for_report_text(&body);
                (format!("{header}{body}"), phase)
            }
            MissionReportBody::Structured(items) => {
                let phase = match event.kind {
                    Mission::BombardWorld => NarrativePhase::AttackerAftermath,
                    _ => NarrativePhase::AttackerAftermath,
                };
                (structured_report_text(&header, items), phase)
            }
        };
        entries.push(ReportEntry {
            text,
            kind,
            tail,
            target: ReportTarget::Both {
                recipient: event.owner_empire_raw,
            },
            repeat_next_pointer: false,
            stardate_week: event.stardate_week,
            narrative_phase,
        });
    }
    consumed_roe_disposition_indices
}
