//! Fleet references captured during a maintenance turn.
//!
//! Maintenance captures fleet indexes (ready-assault sets, movement and
//! mission events, report events) and fleet ids (join-host bookkeeping) and
//! later phases compact `fleets.records` when fleets merge, salvage, or
//! defect. Every compaction goes through `apply_fleet_removal_remap`, whose
//! [`FleetIndexRemap`] must be applied to all references captured so far so
//! they keep naming the same fleet.

use std::collections::HashMap;

use super::combat::{AssaultEvents, FleetBattlePhaseEvents};
use super::{
    ColonizationResolvedEvent, EncounterDispositionEvent, FleetMergeEvent, InvalidPlayerStateEvent,
    JoinMissionHostEvent, MissionEvent, MissionRetargetEvent, MovementEvents, PlanetIntelEvent,
    SalvageResolvedEvent,
};

/// Fleet index stored in report events whose fleet was removed this turn.
///
/// It never resolves to a fleet record, so report lookups fall back to
/// unnamed-fleet text instead of naming whichever fleet shifted into the slot.
pub(super) const REMOVED_FLEET_IDX: usize = usize::MAX;

/// Old-to-new fleet index mapping for one compaction of `fleets.records`.
///
/// Indexes outside the compacted range (including [`REMOVED_FLEET_IDX`]) pass
/// through unchanged, so the default value is the identity remap.
#[derive(Debug, Clone, Default)]
pub(super) struct FleetIndexRemap {
    new_index: Vec<Option<usize>>,
}

impl FleetIndexRemap {
    pub(super) fn from_removed(to_remove: &[bool]) -> Self {
        let mut next = 0;
        let new_index = to_remove
            .iter()
            .map(|&removed| {
                (!removed).then(|| {
                    next += 1;
                    next - 1
                })
            })
            .collect();
        Self { new_index }
    }

    /// Build the remap for a compaction that happened elsewhere, from
    /// per-fleet identity keys captured before and after it. Compaction keeps
    /// record order, so survivors are an ordered subsequence of `before`.
    pub(super) fn from_survivors<K: PartialEq>(before: &[K], after: &[K]) -> Self {
        let mut survivors = after.iter().peekable();
        let to_remove = before
            .iter()
            .map(|key| {
                if survivors.peek() == Some(&key) {
                    survivors.next();
                    false
                } else {
                    true
                }
            })
            .collect::<Vec<_>>();
        Self::from_removed(&to_remove)
    }

    /// New index of a surviving fleet; `None` when the fleet was removed.
    pub(super) fn index(&self, old_idx: usize) -> Option<usize> {
        match self.new_index.get(old_idx) {
            Some(new_idx) => *new_idx,
            None => Some(old_idx),
        }
    }

    /// New 1-based fleet id of a surviving fleet; `None` when it was removed.
    pub(super) fn fleet_id(&self, old_id: u16) -> Option<u16> {
        let old_idx = usize::from(old_id).checked_sub(1)?;
        self.index(old_idx).map(|idx| (idx + 1) as u16)
    }

    /// Remap `idx` in place; returns `false` when its fleet was removed.
    fn retain_index(&self, idx: &mut usize) -> bool {
        match self.index(*idx) {
            Some(new_idx) => {
                *idx = new_idx;
                true
            }
            None => false,
        }
    }

    fn report_index(&self, idx: &mut usize) {
        *idx = self.index(*idx).unwrap_or(REMOVED_FLEET_IDX);
    }

    /// Drop per-fleet entries (indexed by fleet) whose fleet was removed.
    fn compact<T>(&self, per_fleet: &mut Vec<T>) {
        let mut old_idx = 0;
        per_fleet.retain(|_| {
            let keep = self.index(old_idx).is_some();
            old_idx += 1;
            keep
        });
    }

    /// Re-key a map keyed by 1-based fleet id, dropping removed fleets.
    fn rekey_fleet_ids<V>(&self, by_fleet_id: &mut HashMap<u16, V>) {
        *by_fleet_id = std::mem::take(by_fleet_id)
            .into_iter()
            .filter_map(|(id, value)| self.fleet_id(id).map(|new_id| (new_id, value)))
            .collect();
    }

    /// Mission events of removed fleets are dropped, matching salvage removal.
    pub(super) fn remap_mission_events(&self, events: &mut Vec<MissionEvent>) {
        events.retain_mut(|event| self.retain_index(&mut event.fleet_idx));
    }

    pub(super) fn remap_planet_intel_events(&self, events: &mut [PlanetIntelEvent]) {
        for event in events {
            if let Some(idx) = event.source_fleet_idx {
                event.source_fleet_idx = self.index(idx);
            }
        }
    }

    pub(super) fn remap_colonization_events(&self, events: &mut [ColonizationResolvedEvent]) {
        for event in events {
            match event {
                ColonizationResolvedEvent::Succeeded { fleet_idx, .. }
                | ColonizationResolvedEvent::BlockedByOwner { fleet_idx, .. }
                | ColonizationResolvedEvent::Aborted { fleet_idx, .. } => {
                    self.report_index(fleet_idx)
                }
            }
        }
    }

    pub(super) fn remap_encounter_disposition_events(
        &self,
        events: &mut [EncounterDispositionEvent],
    ) {
        for event in events {
            match event {
                EncounterDispositionEvent::NoEngagement { fleet_idx, .. }
                | EncounterDispositionEvent::Retreated { fleet_idx, .. }
                | EncounterDispositionEvent::PursuitFire { fleet_idx, .. } => {
                    self.report_index(fleet_idx)
                }
            }
        }
    }

    pub(super) fn remap_fleet_battle_phase_events(&self, events: &mut FleetBattlePhaseEvents) {
        self.remap_mission_events(&mut events.mission_events);
        self.remap_planet_intel_events(&mut events.planet_intel_events);
        self.remap_encounter_disposition_events(&mut events.encounter_disposition_events);
    }

    fn remap_movement_events(&self, events: &mut MovementEvents) {
        events
            .colonization_events
            .retain_mut(|event| self.retain_index(&mut event.fleet_idx));
        self.remap_planet_intel_events(&mut events.planet_intel_events);
        events.pending_observation_events.retain_mut(|event| {
            if let Some(intel_event) = event.intel_event.as_mut() {
                self.remap_planet_intel_events(std::slice::from_mut(intel_event));
            }
            self.retain_index(&mut event.fleet_idx)
        });
        self.remap_mission_events(&mut events.mission_events);
        for event in &mut events.salvage_events {
            let SalvageResolvedEvent::Succeeded { fleet_idx, .. } = event;
            self.report_index(fleet_idx);
        }
        self.compact(&mut events.fleet_travel_speeds);
    }
}

/// Fleet references that stay live across the turn's fleet compactions.
#[derive(Debug, Default)]
pub(super) struct TurnFleetRefs {
    /// Destroyed join-host fleet numbers keyed by the joining fleet's id.
    pub destroyed_join_host_fleet_numbers: HashMap<u16, u8>,
    pub merge_events: Vec<FleetMergeEvent>,
    pub join_host_events: Vec<JoinMissionHostEvent>,
    pub mission_retarget_events: Vec<MissionRetargetEvent>,
    pub invalid_player_state_events: Vec<InvalidPlayerStateEvent>,
    pub movement_events: MovementEvents,
    /// Sightings, decisions, and battles from the year's ticks.
    pub encounter_events: FleetBattlePhaseEvents,
    /// Planetary strikes from the year's ticks.
    pub assault_events: AssaultEvents,
}

impl TurnFleetRefs {
    pub(super) fn remap(&mut self, remap: &FleetIndexRemap) {
        remap.rekey_fleet_ids(&mut self.destroyed_join_host_fleet_numbers);
        // The absorbed side keeps the index the merged-away record had when it
        // merged; only the surviving fleet's index follows later compactions.
        for event in self
            .merge_events
            .iter_mut()
            .filter(|event| event.survivor_side)
        {
            remap.report_index(&mut event.fleet_idx);
        }
        for event in &mut self.join_host_events {
            match event {
                JoinMissionHostEvent::Retargeted { fleet_idx, .. }
                | JoinMissionHostEvent::HostDestroyed { fleet_idx, .. } => {
                    remap.report_index(fleet_idx)
                }
            }
        }
        for event in &mut self.mission_retarget_events {
            match event {
                MissionRetargetEvent::Retargeted { fleet_idx, .. }
                | MissionRetargetEvent::Abandoned { fleet_idx, .. } => {
                    remap.report_index(fleet_idx)
                }
            }
        }
        for event in &mut self.invalid_player_state_events {
            match event {
                InvalidPlayerStateEvent::FleetMission { fleet_idx, .. }
                | InvalidPlayerStateEvent::FleetInput { fleet_idx, .. } => {
                    remap.report_index(fleet_idx)
                }
                InvalidPlayerStateEvent::PlanetInput { .. }
                | InvalidPlayerStateEvent::PlayerTaxRate { .. }
                | InvalidPlayerStateEvent::DiplomacyInput { .. } => {}
            }
        }
        remap.remap_movement_events(&mut self.movement_events);
        remap.remap_fleet_battle_phase_events(&mut self.encounter_events);
        remap.remap_mission_events(&mut self.assault_events.mission_events);
        remap.remap_planet_intel_events(&mut self.assault_events.planet_intel_events);
    }
}
