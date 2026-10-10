//! Event canonicalization pass for the maintenance engine.
//!
//! This module runs after event assembly and before returning `MaintenanceEvents`
//! to the caller. It:
//!
//! 1. Walks each event vector and assigns `stardate_week` based on the
//!    recovered timing rules (see [`super::timing`]).
//! 2. Enforces corpus-backed same-week pairing rules.
//! 3. Sorts each event vector by `stardate_week` ascending.
//!
//! **No game-state mutation happens here.** The pass is purely additive:
//! it only fills in `stardate_week` fields that are currently `None`.

mod sorting;
mod weeks;

use super::MaintenanceEvents;

/// Run the canonicalization pass over `events`.
///
/// `fleet_travel_speeds` holds the speed each fleet moved at this turn,
/// indexed like the events' final fleet indexes.
///
/// After this call every report-visible event has a non-`None` `stardate_week`
/// and each event vector is sorted in ascending week order.
pub fn canonicalize_events(events: &mut MaintenanceEvents, fleet_travel_speeds: &[u8]) {
    weeks::assign_event_weeks(events, fleet_travel_speeds);
    sorting::sort_events(events);
}

#[cfg(test)]
mod tests {
    use super::super::{
        CampaignOutlookEvent, CivilDisorderEvent, ColonizationResolvedEvent, FleetBattleEvent,
        Mission, MissionEvent, MissionOutcome, ShipLosses,
    };
    use super::*;

    fn make_mission_event(kind: Mission, outcome: MissionOutcome) -> MissionEvent {
        MissionEvent {
            fleet_idx: 0,
            owner_empire_raw: 1,
            kind,
            outcome,
            abort_reason: None,
            planet_idx: None,
            location_coords: None,
            target_coords: None,
            stardate_week: None,
        }
    }

    #[test]
    fn canonicalize_populates_mission_event_weeks() {
        let mut events = MaintenanceEvents::default();
        events.mission_events.push(make_mission_event(
            Mission::MoveOnly,
            MissionOutcome::Succeeded,
        ));
        events.mission_events.push(make_mission_event(
            Mission::PatrolSector,
            MissionOutcome::Arrived,
        ));
        canonicalize_events(&mut events, &[]);
        assert!(
            events
                .mission_events
                .iter()
                .all(|e| e.stardate_week.is_some())
        );
    }

    #[test]
    fn canonicalize_sorts_mission_events_by_week() {
        let mut events = MaintenanceEvents::default();
        events.mission_events.push(make_mission_event(
            Mission::ScoutSector,
            MissionOutcome::Succeeded,
        ));
        events.mission_events.push(make_mission_event(
            Mission::PatrolSector,
            MissionOutcome::Arrived,
        ));
        canonicalize_events(&mut events, &[]);
        let weeks: Vec<u8> = events
            .mission_events
            .iter()
            .map(|e| e.stardate_week.unwrap())
            .collect();
        for pair in weeks.windows(2) {
            assert!(
                pair[0] <= pair[1],
                "events should be sorted ascending: {:?}",
                weeks
            );
        }
    }

    #[test]
    fn fleet_battle_event_gets_week_2() {
        let mut events = MaintenanceEvents::default();
        events.fleet_battle_events.push(FleetBattleEvent {
            reporting_empire_raw: 1,
            reporting_fleet_number: Some(1),
            reporting_mission: Some(Mission::GuardBlockadeWorld),
            perspective: crate::maint::FleetBattlePerspective::Intercepted,
            coords: [5, 5],
            enemy_empires_raw: vec![2],
            primary_enemy_fleet_number: Some(5),
            held_field: true,
            friendly_initial: ShipLosses::default(),
            friendly_initial_starbases: 0,
            friendly_loaded_armies_initial: 0,
            friendly_losses: ShipLosses::default(),
            friendly_starbases_lost: 0,
            enemy_initial: ShipLosses::default(),
            enemy_initial_starbases: 0,
            enemy_loaded_armies_initial: 0,
            enemy_losses: ShipLosses::default(),
            enemy_starbases_destroyed: 0,
            stardate_week: None,
        });
        canonicalize_events(&mut events, &[]);
        assert_eq!(events.fleet_battle_events[0].stardate_week, Some(2));
    }

    #[test]
    fn civil_disorder_events_get_week_52() {
        let mut events = MaintenanceEvents::default();
        events.civil_disorder_events.push(CivilDisorderEvent {
            reporting_empire_raw: 1,
            prior_label: "Empire Alpha".to_string(),
            stardate_week: None,
        });
        canonicalize_events(&mut events, &[]);
        assert_eq!(events.civil_disorder_events[0].stardate_week, Some(52));
    }

    #[test]
    fn campaign_outlook_gets_week_52() {
        let mut events = MaintenanceEvents::default();
        events.campaign_outlook_events.push(CampaignOutlookEvent {
            empire_raw: 1,
            stardate_week: None,
        });
        canonicalize_events(&mut events, &[]);
        assert_eq!(events.campaign_outlook_events[0].stardate_week, Some(52));
    }

    #[test]
    fn colonization_event_gets_week_from_mission_event() {
        let mut events = MaintenanceEvents::default();
        events.mission_events.push(MissionEvent {
            fleet_idx: 3,
            owner_empire_raw: 1,
            kind: Mission::ColonizeWorld,
            outcome: MissionOutcome::Succeeded,
            abort_reason: None,
            planet_idx: Some(5),
            location_coords: Some([10, 10]),
            target_coords: Some([10, 10]),
            stardate_week: Some(30),
        });
        events
            .colonization_events
            .push(ColonizationResolvedEvent::Succeeded {
                fleet_idx: 3,
                planet_idx: 5,
                colonizer_empire_raw: 1,
                stardate_week: None,
            });
        canonicalize_events(&mut events, &[]);
        let col_week = match events.colonization_events[0] {
            ColonizationResolvedEvent::Succeeded { stardate_week, .. } => stardate_week,
            _ => None,
        };
        assert_eq!(col_week, Some(30));
    }
}
