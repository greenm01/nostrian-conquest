//! Fleet identity across removals: every fleet index an event carries must
//! still name the fleet the event described after the turn's merges,
//! salvages, and defections compacted the fleet list.
//!
//! An index either names a fleet record after maintenance or is the
//! engine's removed-fleet marker (`usize::MAX`). A named record must belong
//! to the event's empire; where the event also captured a fleet number or a
//! location, the record must match those too. Fleets do not move after the
//! movement phase, so a location is the fleet's final one, or its starting
//! one for an event from the departure interception before movement.

use nc_data::{
    ColonizationResolvedEvent, CoreGameData, EncounterDispositionEvent, FleetRecord,
    InvalidPlayerStateEvent, JoinMissionHostEvent, MissionRetargetEvent, SalvageResolvedEvent,
};

use super::{TurnObservation, Violation};

const REMOVED_FLEET_IDX: usize = usize::MAX;

/// What one event says about the fleet its index names.
struct FleetClaim {
    fleet_idx: usize,
    owner: u8,
    fleet_number: Option<u8>,
    coords: Option<[u8; 2]>,
}

impl FleetClaim {
    fn owned(fleet_idx: usize, owner: u8) -> Self {
        Self {
            fleet_idx,
            owner,
            fleet_number: None,
            coords: None,
        }
    }

    fn at(fleet_idx: usize, owner: u8, coords: [u8; 2]) -> Self {
        Self {
            coords: Some(coords),
            ..Self::owned(fleet_idx, owner)
        }
    }

    fn mismatch(&self, fleets: &[FleetRecord], before: &[FleetRecord]) -> Option<String> {
        if self.fleet_idx == REMOVED_FLEET_IDX {
            return None;
        }
        let Some(fleet) = fleets.get(self.fleet_idx) else {
            return Some(format!(
                "fleet index {} is past the {} fleet records",
                self.fleet_idx,
                fleets.len()
            ));
        };
        let number = fleet.local_slot_word_raw();
        if fleet.owner_empire_raw() != self.owner {
            return Some(format!(
                "index {} is empire {} fleet {number}, not an empire {} fleet",
                self.fleet_idx,
                fleet.owner_empire_raw(),
                self.owner
            ));
        }
        if let Some(expected) = self.fleet_number
            && number != u16::from(expected)
        {
            return Some(format!(
                "index {} is empire {} fleet {number}, not fleet {expected}",
                self.fleet_idx, self.owner
            ));
        }
        if let Some(coords) = self.coords {
            let at = fleet.current_location_coords_raw();
            let started_at = before
                .iter()
                .find(|start| {
                    start.owner_empire_raw() == self.owner && start.local_slot_word_raw() == number
                })
                .map(FleetRecord::current_location_coords_raw);
            if at != coords && started_at != Some(coords) {
                return Some(format!(
                    "index {} is empire {} fleet {number} at {at:?}, not at {coords:?}",
                    self.fleet_idx, self.owner
                ));
            }
        }
        None
    }
}

pub fn check(observation: &TurnObservation) -> Vec<Violation> {
    let after = observation.after;
    let events = observation.events;
    let mut claims: Vec<(String, FleetClaim)> = Vec::new();
    let mut claim = |event: &dyn std::fmt::Debug, claim: FleetClaim| {
        claims.push((format!("{event:?}"), claim));
    };

    for event in &events.mission_events {
        let mut fleet_claim = FleetClaim::owned(event.fleet_idx, event.owner_empire_raw);
        fleet_claim.coords = event.location_coords;
        claim(event, fleet_claim);
    }
    for event in &events.encounter_disposition_events {
        let (EncounterDispositionEvent::NoEngagement {
            fleet_idx,
            owner_empire_raw,
            coords,
            ..
        }
        | EncounterDispositionEvent::Retreated {
            fleet_idx,
            owner_empire_raw,
            coords,
            ..
        }
        | EncounterDispositionEvent::PursuitFire {
            fleet_idx,
            owner_empire_raw,
            coords,
            ..
        }) = *event;
        claim(event, FleetClaim::at(fleet_idx, owner_empire_raw, coords));
    }
    for event in &events.colonization_events {
        let fleet_claim = match *event {
            ColonizationResolvedEvent::Succeeded {
                fleet_idx,
                planet_idx,
                colonizer_empire_raw,
                ..
            }
            | ColonizationResolvedEvent::BlockedByOwner {
                fleet_idx,
                planet_idx,
                colonizer_empire_raw,
                ..
            } => FleetClaim::at(
                fleet_idx,
                colonizer_empire_raw,
                planet_coords(after, planet_idx),
            ),
            ColonizationResolvedEvent::Aborted {
                fleet_idx,
                colonizer_empire_raw,
                ..
            } => FleetClaim::owned(fleet_idx, colonizer_empire_raw),
        };
        claim(event, fleet_claim);
    }
    for event in &events.salvage_events {
        let (SalvageResolvedEvent::Succeeded {
            fleet_idx,
            owner_empire_raw,
            coords,
            ..
        }
        | SalvageResolvedEvent::Failed {
            fleet_idx,
            owner_empire_raw,
            coords,
            ..
        }) = *event;
        claim(event, FleetClaim::at(fleet_idx, owner_empire_raw, coords));
    }
    for event in &events.join_host_events {
        let (JoinMissionHostEvent::Retargeted {
            fleet_idx,
            owner_empire_raw,
            ..
        }
        | JoinMissionHostEvent::HostDestroyed {
            fleet_idx,
            owner_empire_raw,
            ..
        }) = *event;
        claim(event, FleetClaim::owned(fleet_idx, owner_empire_raw));
    }
    for event in &events.mission_retarget_events {
        let (MissionRetargetEvent::Retargeted {
            fleet_idx,
            reporting_fleet_number,
            owner_empire_raw,
            ..
        }
        | MissionRetargetEvent::Abandoned {
            fleet_idx,
            reporting_fleet_number,
            owner_empire_raw,
            ..
        }) = *event;
        let mut fleet_claim = FleetClaim::owned(fleet_idx, owner_empire_raw);
        fleet_claim.fleet_number = reporting_fleet_number;
        claim(event, fleet_claim);
    }
    for event in &events.invalid_player_state_events {
        if let InvalidPlayerStateEvent::FleetMission {
            fleet_idx,
            owner_empire_raw,
            ..
        }
        | InvalidPlayerStateEvent::FleetInput {
            fleet_idx,
            owner_empire_raw,
            ..
        } = *event
        {
            claim(event, FleetClaim::owned(fleet_idx, owner_empire_raw));
        }
    }
    for event in events
        .fleet_merge_events
        .iter()
        .filter(|event| event.survivor_side)
    {
        let mut fleet_claim = FleetClaim::owned(event.fleet_idx, event.owner_empire_raw);
        fleet_claim.fleet_number = Some(event.host_fleet_number);
        claim(event, fleet_claim);
    }
    for event in &events.planet_intel_events {
        if let Some(fleet_idx) = event.source_fleet_idx {
            claim(event, FleetClaim::owned(fleet_idx, event.viewer_empire_raw));
        }
    }

    claims
        .into_iter()
        .filter_map(|(event, claim)| {
            claim
                .mismatch(&after.fleets.records, &observation.before.fleets.records)
                .map(|detail| Violation::FleetIdentity { event, detail })
        })
        .collect()
}

fn planet_coords(state: &CoreGameData, planet_idx: usize) -> [u8; 2] {
    state.planets.records[planet_idx].coords_raw()
}
