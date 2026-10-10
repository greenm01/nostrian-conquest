use nc_data::PlanetIntelSource;

use super::super::{
    MaintenanceEvents, Mission, MissionEvent, MissionOutcome,
    timing::{apply_timing_offset, event_base_week, mission_timing_code},
};

pub(super) fn assign_event_weeks(events: &mut MaintenanceEvents, fleet_travel_speeds: &[u8]) {
    assign_mission_event_weeks(events, fleet_travel_speeds);
    assign_combat_event_weeks(events);
    assign_assault_event_weeks(events);
    assign_bombard_event_weeks(events);
    assign_scout_contact_weeks(events);
    assign_planet_intel_event_weeks(events);
    assign_ownership_change_weeks(events);
    assign_fleet_merge_weeks(events);
    assign_colonization_weeks(events);
    assign_salvage_weeks(events);
    assign_encounter_disposition_weeks(events);
    assign_civil_disorder_weeks(events);
    assign_campaign_weeks(events);
    assign_diplomatic_escalation_weeks(events);
}

/// Uses the speed the fleet moved at this turn: by the time events are
/// stamped, arrival has usually zeroed the fleet's current speed.
fn week_for_mission_event(event: &MissionEvent, fleet_travel_speeds: &[u8]) -> u8 {
    let fleet_speed = fleet_travel_speeds
        .get(event.fleet_idx)
        .copied()
        .unwrap_or(0);
    let travel_time_years: u8 = if fleet_speed > 0 { 1 } else { 0 };

    let base = event_base_week(event.kind, travel_time_years, fleet_speed);
    let code = mission_timing_code(event.kind);
    apply_timing_offset(base, code)
}

fn assign_mission_event_weeks(events: &mut MaintenanceEvents, fleet_travel_speeds: &[u8]) {
    for event in &mut events.mission_events {
        if event.stardate_week.is_none() {
            event.stardate_week = Some(week_for_mission_event(event, fleet_travel_speeds));
        }
    }
}

fn assign_combat_event_weeks(events: &mut MaintenanceEvents) {
    for e in &mut events.fleet_battle_events {
        if e.stardate_week.is_none() {
            e.stardate_week = Some(2);
        }
    }
    for e in &mut events.fleet_destroyed_events {
        if e.stardate_week.is_none() {
            e.stardate_week = Some(2);
        }
    }
    for e in &mut events.starbase_destroyed_events {
        if e.stardate_week.is_none() {
            e.stardate_week = Some(2);
        }
    }
}

fn assign_assault_event_weeks(events: &mut MaintenanceEvents) {
    let mission_weeks: Vec<(usize, u8, u8)> = events
        .mission_events
        .iter()
        .filter_map(|e| {
            if matches!(e.kind, Mission::InvadeWorld | Mission::BlitzWorld) {
                Some((
                    e.planet_idx.unwrap_or(usize::MAX),
                    e.owner_empire_raw,
                    e.stardate_week.unwrap_or(1),
                ))
            } else {
                None
            }
        })
        .collect();

    for e in &mut events.assault_report_events {
        if e.stardate_week.is_some() {
            continue;
        }
        let week = mission_weeks
            .iter()
            .find(|(pidx, emp, _)| *pidx == e.planet_idx && *emp == e.attacker_empire_raw)
            .map(|(_, _, w)| *w)
            .unwrap_or(2);
        e.stardate_week = Some(week);
    }
}

fn assign_bombard_event_weeks(events: &mut MaintenanceEvents) {
    let bombard_weeks: Vec<(usize, u8, u8)> = events
        .mission_events
        .iter()
        .filter_map(|e| {
            if e.kind == Mission::BombardWorld && e.outcome == MissionOutcome::Succeeded {
                Some((
                    e.planet_idx.unwrap_or(usize::MAX),
                    e.owner_empire_raw,
                    e.stardate_week.unwrap_or(1),
                ))
            } else {
                None
            }
        })
        .collect();

    for e in &mut events.bombard_events {
        if e.stardate_week.is_some() {
            continue;
        }
        let week = bombard_weeks
            .iter()
            .find(|(pidx, emp, _)| *pidx == e.planet_idx && *emp == e.attacker_empire_raw)
            .map(|(_, _, w)| *w)
            .unwrap_or(2);
        e.stardate_week = Some(week);
    }
}

fn assign_scout_contact_weeks(events: &mut MaintenanceEvents) {
    for e in &mut events.scout_contact_events {
        if e.stardate_week.is_none() {
            e.stardate_week = Some(2);
        }
    }
}

fn assign_planet_intel_event_weeks(events: &mut MaintenanceEvents) {
    for e in &mut events.planet_intel_events {
        if e.stardate_week.is_some() {
            continue;
        }
        e.stardate_week = events
            .mission_events
            .iter()
            .find(|mission| {
                mission.owner_empire_raw == e.viewer_empire_raw
                    && mission.fleet_idx == e.source_fleet_idx.unwrap_or(usize::MAX)
                    && matches!(
                        mission.outcome,
                        MissionOutcome::Succeeded | MissionOutcome::Failed
                    )
                    && matches!(
                        (e.source, mission.kind),
                        (PlanetIntelSource::ViewWorld, Mission::ViewWorld)
                            | (
                                PlanetIntelSource::ScoutSolarSystem,
                                Mission::ScoutSolarSystem
                            )
                            | (
                                PlanetIntelSource::ColonizeBlockedByOwner,
                                Mission::ColonizeWorld
                            )
                    )
            })
            .and_then(|mission| mission.stardate_week)
            .or_else(|| {
                events
                    .assault_report_events
                    .iter()
                    .find(|assault| assault.planet_idx == e.planet_idx)
                    .and_then(|assault| assault.stardate_week)
            })
            .or(Some(2));
    }
}

fn assign_ownership_change_weeks(events: &mut MaintenanceEvents) {
    let assault_weeks: Vec<(usize, u8)> = events
        .assault_report_events
        .iter()
        .filter_map(|e| {
            if matches!(e.outcome, super::super::MissionOutcome::Succeeded) {
                Some((e.planet_idx, e.stardate_week.unwrap_or(2)))
            } else {
                None
            }
        })
        .collect();

    for e in &mut events.ownership_change_events {
        if e.stardate_week.is_some() {
            continue;
        }
        let week = assault_weeks
            .iter()
            .find(|(pidx, _)| *pidx == e.planet_idx)
            .map(|(_, w)| *w)
            .unwrap_or(2);
        e.stardate_week = Some(week);
    }
}

fn assign_fleet_merge_weeks(events: &mut MaintenanceEvents) {
    for e in &mut events.fleet_merge_events {
        if e.stardate_week.is_none() {
            e.stardate_week = Some(1);
        }
    }
}

fn assign_colonization_weeks(events: &mut MaintenanceEvents) {
    let colonize_weeks: Vec<(usize, u8)> = events
        .mission_events
        .iter()
        .filter_map(|e| {
            if e.kind == Mission::ColonizeWorld {
                Some((e.fleet_idx, e.stardate_week.unwrap_or(1)))
            } else {
                None
            }
        })
        .collect();

    for e in &mut events.colonization_events {
        let already_set = match e {
            super::super::ColonizationResolvedEvent::Succeeded { stardate_week, .. } => {
                stardate_week.is_some()
            }
            super::super::ColonizationResolvedEvent::BlockedByOwner { stardate_week, .. } => {
                stardate_week.is_some()
            }
            super::super::ColonizationResolvedEvent::Aborted { stardate_week, .. } => {
                stardate_week.is_some()
            }
        };
        if already_set {
            continue;
        }

        let fleet_idx = match *e {
            super::super::ColonizationResolvedEvent::Succeeded { fleet_idx, .. } => fleet_idx,
            super::super::ColonizationResolvedEvent::BlockedByOwner { fleet_idx, .. } => fleet_idx,
            super::super::ColonizationResolvedEvent::Aborted { fleet_idx, .. } => fleet_idx,
        };
        let week = colonize_weeks
            .iter()
            .find(|(fi, _)| *fi == fleet_idx)
            .map(|(_, w)| *w)
            .unwrap_or(1);
        match e {
            super::super::ColonizationResolvedEvent::Succeeded { stardate_week, .. } => {
                *stardate_week = Some(week)
            }
            super::super::ColonizationResolvedEvent::BlockedByOwner { stardate_week, .. } => {
                *stardate_week = Some(week)
            }
            super::super::ColonizationResolvedEvent::Aborted { stardate_week, .. } => {
                *stardate_week = Some(week)
            }
        }
    }
}

fn assign_salvage_weeks(events: &mut MaintenanceEvents) {
    let salvage_weeks: Vec<(usize, u8)> = events
        .mission_events
        .iter()
        .filter_map(|e| {
            if e.kind == Mission::Salvage {
                Some((e.fleet_idx, e.stardate_week.unwrap_or(1)))
            } else {
                None
            }
        })
        .collect();

    for e in &mut events.salvage_events {
        let fleet_idx = match *e {
            super::super::SalvageResolvedEvent::Succeeded { fleet_idx, .. } => fleet_idx,
            super::super::SalvageResolvedEvent::Failed { fleet_idx, .. } => fleet_idx,
        };
        let week = salvage_weeks
            .iter()
            .find(|(fi, _)| *fi == fleet_idx)
            .map(|(_, w)| *w)
            .unwrap_or(1);
        match e {
            super::super::SalvageResolvedEvent::Succeeded { stardate_week, .. } => {
                if stardate_week.is_none() {
                    *stardate_week = Some(week);
                }
            }
            super::super::SalvageResolvedEvent::Failed { stardate_week, .. } => {
                if stardate_week.is_none() {
                    *stardate_week = Some(week);
                }
            }
        }
    }
}

fn assign_encounter_disposition_weeks(events: &mut MaintenanceEvents) {
    for e in &mut events.encounter_disposition_events {
        let already_set = match e {
            super::super::EncounterDispositionEvent::NoEngagement { stardate_week, .. } => {
                stardate_week.is_some()
            }
            super::super::EncounterDispositionEvent::Retreated { stardate_week, .. } => {
                stardate_week.is_some()
            }
            super::super::EncounterDispositionEvent::PursuitFire { stardate_week, .. } => {
                stardate_week.is_some()
            }
        };
        if !already_set {
            match e {
                super::super::EncounterDispositionEvent::NoEngagement { stardate_week, .. } => {
                    *stardate_week = Some(2);
                }
                super::super::EncounterDispositionEvent::Retreated { stardate_week, .. } => {
                    *stardate_week = Some(2);
                }
                super::super::EncounterDispositionEvent::PursuitFire { stardate_week, .. } => {
                    *stardate_week = Some(2);
                }
            }
        }
    }
}

fn assign_civil_disorder_weeks(events: &mut MaintenanceEvents) {
    for e in &mut events.civil_disorder_events {
        if e.stardate_week.is_none() {
            e.stardate_week = Some(52);
        }
    }
    for e in &mut events.fleet_defection_events {
        if e.stardate_week.is_none() {
            e.stardate_week = Some(52);
        }
    }
}

fn assign_campaign_weeks(events: &mut MaintenanceEvents) {
    for e in &mut events.campaign_outlook_events {
        if e.stardate_week.is_none() {
            e.stardate_week = Some(52);
        }
    }
    for e in &mut events.campaign_outcome_events {
        if e.stardate_week.is_none() {
            e.stardate_week = Some(52);
        }
    }
    for e in &mut events.game_victory_notice_events {
        if e.stardate_week.is_none() {
            e.stardate_week = Some(52);
        }
    }
    for e in &mut events.empire_elimination_events {
        if e.stardate_week.is_none() {
            e.stardate_week = Some(52);
        }
    }
}

fn assign_diplomatic_escalation_weeks(events: &mut MaintenanceEvents) {
    for e in &mut events.diplomatic_escalation_events {
        if e.stardate_week.is_none() {
            e.stardate_week = Some(2);
        }
    }
}
