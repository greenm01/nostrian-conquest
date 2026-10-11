use std::collections::{BTreeSet, HashSet};

use nc_data::{CoreGameData, MaintenanceEvents, MissionOutcome, Order};

pub(super) type FleetReportKey = (u8, u8);

pub(super) struct ReportSuppressionPlan {
    surviving_fleet_report_keys: BTreeSet<FleetReportKey>,
    destroyed_fleet_report_keys: BTreeSet<FleetReportKey>,
    disposition_covered_fleet_keys: HashSet<FleetReportKey>,
}

impl ReportSuppressionPlan {
    pub(super) fn build(game_data: &CoreGameData, events: &MaintenanceEvents) -> Self {
        let surviving_fleet_report_keys = game_data
            .fleets
            .iter()
            .filter(|fleet| fleet.has_any_force())
            .filter_map(|fleet| {
                let fleet_number = fleet.local_slot_word_raw() as u8;
                (fleet.owner_empire_raw() != 0 && fleet_number != 0)
                    .then_some((fleet.owner_empire_raw(), fleet_number))
            })
            .collect();
        let destroyed_fleet_report_keys = events
            .fleet_destroyed_events
            .iter()
            .filter_map(|event| {
                (event.fleet_number != 0)
                    .then_some((event.reporting_empire_raw, event.fleet_number))
            })
            .collect();
        let disposition_covered_fleet_keys = events
            .encounter_disposition_events
            .iter()
            .filter_map(|event| match *event {
                nc_data::EncounterDispositionEvent::Retreated {
                    fleet_idx,
                    owner_empire_raw,
                    ..
                }
                | nc_data::EncounterDispositionEvent::PursuitFire {
                    fleet_idx,
                    owner_empire_raw,
                    ..
                } => {
                    let fleet_number = game_data
                        .fleets
                        .get(fleet_idx)
                        .map(|f| f.local_slot_word_raw() as u8)
                        .filter(|n| *n != 0)?;
                    Some((owner_empire_raw, fleet_number))
                }
                _ => None,
            })
            .collect();
        Self {
            surviving_fleet_report_keys,
            destroyed_fleet_report_keys,
            disposition_covered_fleet_keys,
        }
    }

    pub(super) fn fleet_survives(&self, empire_raw: u8, fleet_number: u8) -> bool {
        self.surviving_fleet_report_keys
            .contains(&(empire_raw, fleet_number))
    }

    pub(super) fn destroyed_supersedes_battle(&self, empire_raw: u8, fleet_number: u8) -> bool {
        self.destroyed_fleet_report_keys
            .contains(&(empire_raw, fleet_number))
    }

    pub(super) fn disposition_supersedes_battle(&self, empire_raw: u8, fleet_number: u8) -> bool {
        self.disposition_covered_fleet_keys
            .contains(&(empire_raw, fleet_number))
    }
}

pub(super) fn mission_event_has_assault_report(
    events: &MaintenanceEvents,
    event: &nc_data::MissionEvent,
) -> bool {
    let Some(planet_idx) = event.planet_idx else {
        return false;
    };
    events.assault_report_events.iter().any(|assault| {
        assault.kind == event.kind
            && assault.planet_idx == planet_idx
            && assault.attacker_empire_raw == event.owner_empire_raw
            && assault.outcome == event.outcome
    })
}

pub(super) fn mission_event_has_fleet_destroyed(
    game_data: &CoreGameData,
    events: &MaintenanceEvents,
    event: &nc_data::MissionEvent,
) -> bool {
    let Some(fleet) = game_data.fleets.get(event.fleet_idx) else {
        return false;
    };
    let fleet_number = fleet.local_slot_word_raw() as u8;
    events.fleet_destroyed_events.iter().any(|destroyed| {
        destroyed.fleet_number == fleet_number
            && destroyed.reporting_empire_raw == event.owner_empire_raw
    })
}

pub(super) fn matching_roe_abort_disposition_index(
    events: &MaintenanceEvents,
    event: &nc_data::MissionEvent,
) -> Option<usize> {
    if event.outcome != MissionOutcome::Aborted {
        return None;
    }
    let coords = event.location_coords?;
    events
        .encounter_disposition_events
        .iter()
        .position(|disposition| match disposition {
            nc_data::EncounterDispositionEvent::Retreated {
                fleet_idx,
                owner_empire_raw,
                mission: Some(mission),
                coords: disposition_coords,
                reason: nc_data::EncounterDispositionReason::RoeWithdrawal,
                ..
            }
            | nc_data::EncounterDispositionEvent::PursuitFire {
                fleet_idx,
                owner_empire_raw,
                mission: Some(mission),
                coords: disposition_coords,
                reason: nc_data::EncounterDispositionReason::RoeWithdrawal,
                ..
            } => {
                *fleet_idx == event.fleet_idx
                    && *owner_empire_raw == event.owner_empire_raw
                    && *mission == event.kind
                    && *disposition_coords == coords
            }
            _ => false,
        })
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum AbortDisposition {
    Retreating,
    Holding,
}

pub(super) fn fleet_abort_disposition(fleet: &nc_data::Fleet) -> AbortDisposition {
    if fleet.standing_order_kind() == Order::SeekHome && fleet.current_speed() > 0 {
        AbortDisposition::Retreating
    } else {
        AbortDisposition::Holding
    }
}

pub(super) fn fleet_abort_disposition_text(disposition: AbortDisposition) -> &'static str {
    match disposition {
        AbortDisposition::Retreating => "withdrawing toward safety",
        AbortDisposition::Holding => "holding position and awaiting orders",
    }
}
