use super::super::MaintenanceEvents;

pub(super) fn sort_events(events: &mut MaintenanceEvents) {
    events
        .mission_events
        .sort_by_key(|e| e.stardate_week.unwrap_or(1));
    events
        .fleet_battle_events
        .sort_by_key(|e| e.stardate_week.unwrap_or(1));
    events
        .fleet_destroyed_events
        .sort_by_key(|e| e.stardate_week.unwrap_or(1));
    events
        .starbase_destroyed_events
        .sort_by_key(|e| e.stardate_week.unwrap_or(1));
    events
        .assault_report_events
        .sort_by_key(|e| e.stardate_week.unwrap_or(1));
    events
        .bombard_events
        .sort_by_key(|e| e.stardate_week.unwrap_or(1));
    events
        .scout_contact_events
        .sort_by_key(|e| e.stardate_week.unwrap_or(1));
    events
        .starbase_sighting_events
        .sort_by_key(|e| e.stardate_week.unwrap_or(1));
    events
        .planet_intel_events
        .sort_by_key(|e| e.stardate_week.unwrap_or(1));
    events
        .ownership_change_events
        .sort_by_key(|e| e.stardate_week.unwrap_or(1));
    events
        .fleet_merge_events
        .sort_by_key(|e| e.stardate_week.unwrap_or(1));
    events.colonization_events.sort_by_key(colonization_week);
    events.salvage_events.sort_by_key(salvage_week);
    events
        .encounter_disposition_events
        .sort_by_key(encounter_week);
    events
        .civil_disorder_events
        .sort_by_key(|e| e.stardate_week.unwrap_or(52));
    events
        .campaign_outlook_events
        .sort_by_key(|e| e.stardate_week.unwrap_or(52));
    events
        .campaign_outcome_events
        .sort_by_key(|e| e.stardate_week.unwrap_or(52));
    events
        .game_victory_notice_events
        .sort_by_key(|e| e.stardate_week.unwrap_or(52));
    events
        .empire_elimination_events
        .sort_by_key(|e| e.stardate_week.unwrap_or(52));
    events
        .fleet_defection_events
        .sort_by_key(|e| e.stardate_week.unwrap_or(52));
    events
        .diplomatic_escalation_events
        .sort_by_key(|e| e.stardate_week.unwrap_or(1));
}

fn colonization_week(e: &super::super::ColonizationResolvedEvent) -> u8 {
    match e {
        super::super::ColonizationResolvedEvent::Succeeded { stardate_week, .. } => {
            stardate_week.unwrap_or(1)
        }
        super::super::ColonizationResolvedEvent::BlockedByOwner { stardate_week, .. } => {
            stardate_week.unwrap_or(1)
        }
        super::super::ColonizationResolvedEvent::Aborted { stardate_week, .. } => {
            stardate_week.unwrap_or(1)
        }
    }
}

fn salvage_week(e: &super::super::SalvageResolvedEvent) -> u8 {
    match e {
        super::super::SalvageResolvedEvent::Succeeded { stardate_week, .. } => {
            stardate_week.unwrap_or(1)
        }
        super::super::SalvageResolvedEvent::Failed { stardate_week, .. } => {
            stardate_week.unwrap_or(1)
        }
    }
}

fn encounter_week(e: &super::super::EncounterDispositionEvent) -> u8 {
    match e {
        super::super::EncounterDispositionEvent::NoEngagement { stardate_week, .. } => {
            stardate_week.unwrap_or(2)
        }
        super::super::EncounterDispositionEvent::Retreated { stardate_week, .. } => {
            stardate_week.unwrap_or(2)
        }
        super::super::EncounterDispositionEvent::PursuitFire { stardate_week, .. } => {
            stardate_week.unwrap_or(2)
        }
    }
}
