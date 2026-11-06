use std::collections::{BTreeSet, HashSet};

use nc_data::{CoreGameData, MaintenanceEvents, MissionOutcome, Order, has_any_force};

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
            .filter(|fleet| has_any_force(fleet))
            .filter_map(|fleet| {
                let fleet_number = fleet.fleet_number.0 as u8;
                (fleet.owner.0 != 0 && fleet_number != 0).then_some((fleet.owner.0, fleet_number))
            })
            .collect();
        let destroyed_fleet_report_keys = events
            .fleet_destroyed_events
            .iter()
            .filter_map(|event| {
                (event.fleet_number != 0).then_some((event.reporting_empire, event.fleet_number))
            })
            .collect();
        let disposition_covered_fleet_keys = events
            .encounter_disposition_events
            .iter()
            .filter_map(|event| match *event {
                nc_data::EncounterDispositionEvent::Retreated {
                    fleet_number,
                    owner_empire,
                    ..
                }
                | nc_data::EncounterDispositionEvent::PursuitFire {
                    fleet_number,
                    owner_empire,
                    ..
                } => (fleet_number != 0).then_some((owner_empire, fleet_number)),
                _ => None,
            })
            .collect();
        Self {
            surviving_fleet_report_keys,
            destroyed_fleet_report_keys,
            disposition_covered_fleet_keys,
        }
    }

    pub(super) fn fleet_survives(&self, empire: u8, fleet_number: u8) -> bool {
        self.surviving_fleet_report_keys
            .contains(&(empire, fleet_number))
    }

    pub(super) fn destroyed_supersedes_battle(&self, empire: u8, fleet_number: u8) -> bool {
        self.destroyed_fleet_report_keys
            .contains(&(empire, fleet_number))
    }

    pub(super) fn disposition_supersedes_battle(&self, empire: u8, fleet_number: u8) -> bool {
        self.disposition_covered_fleet_keys
            .contains(&(empire, fleet_number))
    }

    /// The battle gets its own briefing: no loss or escape report of its
    /// reporting fleet tells it instead.
    pub(super) fn battle_has_briefing(&self, event: &nc_data::FleetBattleEvent) -> bool {
        !event.reporting_fleet_number.is_some_and(|fleet_number| {
            self.destroyed_supersedes_battle(event.reporting_empire, fleet_number)
                || self.disposition_supersedes_battle(event.reporting_empire, fleet_number)
        })
    }
}

/// A fleet's sighting of the enemy it then fought that same week. The battle
/// briefing carries the sighting, so it sends no report of its own.
pub(super) fn contact_folds_into_battle(
    contact: &nc_data::ScoutContactEvent,
    battle: &nc_data::FleetBattleEvent,
) -> bool {
    let Some(fleet_number) = contact.reporting_fleet_number else {
        return false;
    };
    let in_battle = if battle.reporting_fleet_numbers.is_empty() {
        battle.reporting_fleet_number == Some(fleet_number)
    } else {
        battle.reporting_fleet_numbers.contains(&fleet_number)
    };
    !matches!(
        contact.source,
        nc_data::ContactReportSource::Starbase { .. }
    ) && in_battle
        && contact.viewer_empire == battle.reporting_empire
        && contact.coords == battle.coords
        && contact.stardate_week == battle.stardate_week
        && battle.enemy_empires.contains(&contact.target_empire)
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
            && assault.attacker_empire == event.owner_empire
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
    let fleet_number = fleet.fleet_number.0 as u8;
    events.fleet_destroyed_events.iter().any(|destroyed| {
        destroyed.fleet_number == fleet_number && destroyed.reporting_empire == event.owner_empire
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
                owner_empire,
                mission: Some(mission),
                coords: disposition_coords,
                reason:
                    nc_data::EncounterDispositionReason::RoeWithdrawal
                    | nc_data::EncounterDispositionReason::UnarmedEscape,
                ..
            }
            | nc_data::EncounterDispositionEvent::PursuitFire {
                fleet_idx,
                owner_empire,
                mission: Some(mission),
                coords: disposition_coords,
                reason:
                    nc_data::EncounterDispositionReason::RoeWithdrawal
                    | nc_data::EncounterDispositionReason::UnarmedEscape,
                ..
            } => {
                *fleet_idx == event.fleet_idx
                    && *owner_empire == event.owner_empire
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
    if fleet.order == Order::SeekHome && fleet.speed > 0 {
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
