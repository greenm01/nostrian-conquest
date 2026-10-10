mod arrivals;
mod guard;
mod salvage;
mod starbase;
mod stepper;

use super::{FleetRemovalRemapInfo, MovementEvents};
use crate::{CoreGameData, Order, VisibleHazardIntel};
use arrivals::{handle_fleet_arrival, handle_hostile_target_entry};
pub(super) use starbase::BaseStep;
pub(super) use stepper::TickEnd;
use stepper::{FleetTrack, is_hostile_order, step_fleet, test_arrival};

/// Every fleet's movement through one maintenance year, advanced a tick at a
/// time by the encounter loop (`navigation::motion` for the step).
/// Resolved orders run their arrival handling in the tick they resolve; a
/// hostile order that passes its strike test is reported to the encounter
/// loop, which resolves the strike in that tick.
///
/// Events index the fleet list as it stood at the start of the year;
/// `finish` removes salvaged fleets and returns the remap for that removal.
pub(super) struct YearMovement<'a> {
    visible_hazards_by_empire: &'a [VisibleHazardIntel],
    tracks: Vec<FleetTrack>,
    start: Vec<[u8; 2]>,
    entered_target: Vec<bool>,
    to_remove: Vec<bool>,
    events: MovementEvents,
}

impl<'a> YearMovement<'a> {
    pub(super) fn new(
        game_data: &CoreGameData,
        visible_hazards_by_empire: &'a [VisibleHazardIntel],
    ) -> Self {
        let fleet_count = game_data.fleets.records.len();
        Self {
            visible_hazards_by_empire,
            tracks: vec![FleetTrack::default(); fleet_count],
            start: game_data
                .fleets
                .records
                .iter()
                .map(|fleet| fleet.current_location_coords_raw())
                .collect(),
            entered_target: vec![false; fleet_count],
            to_remove: vec![false; fleet_count],
            events: MovementEvents {
                fleet_travel_speeds: vec![0; fleet_count],
                ..MovementEvents::default()
            },
        }
    }

    /// Step fleet `fleet_idx` one tick. Returns whether it moved.
    pub(super) fn step(&mut self, game_data: &mut CoreGameData, fleet_idx: usize) -> bool {
        step_fleet(
            game_data,
            &mut self.tracks[fleet_idx],
            fleet_idx,
            self.visible_hazards_by_empire,
        )
    }

    /// Run the order's arrival test and, when it passes, the arrival action.
    /// Returns how the tick ended for the order; a `Strike` is the caller's
    /// to resolve.
    pub(super) fn dispatch(
        &mut self,
        game_data: &mut CoreGameData,
        fleet_idx: usize,
        tick: u8,
    ) -> Result<TickEnd, Box<dyn std::error::Error>> {
        let (target, order_kind, owner_empire, sector) = {
            let fleet = &game_data.fleets.records[fleet_idx];
            (
                fleet.standing_order_target_coords_raw(),
                fleet.standing_order_kind(),
                fleet.owner_empire_raw(),
                fleet.current_location_coords_raw(),
            )
        };
        if order_kind == Order::HoldPosition {
            // Classic's Hold action zeroes the speed, so a holding fleet's
            // visibility loses its speed term after its first tick.
            game_data.fleets.records[fleet_idx].set_current_speed(0);
            return Ok(TickEnd::Underway);
        }
        let marks = EventMarks::of(&self.events);
        if order_kind == Order::GuardStarbase {
            if self.tracks[fleet_idx].stepped() {
                self.dispatch_guard(game_data, fleet_idx, tick);
            }
            return Ok(TickEnd::Underway);
        }
        if is_hostile_order(order_kind)
            && !self.entered_target[fleet_idx]
            && self.start[fleet_idx] != target
            && sector == target
        {
            self.entered_target[fleet_idx] = true;
            handle_hostile_target_entry(game_data, &mut self.events, fleet_idx, order_kind, target);
        }
        let end = test_arrival(game_data, &mut self.tracks[fleet_idx], fleet_idx, tick);
        if end == TickEnd::Resolved {
            handle_fleet_arrival(
                game_data,
                &mut self.events,
                &mut self.to_remove,
                fleet_idx,
                order_kind,
                owner_empire,
                target,
            )?;
        }
        marks.stamp(&mut self.events, tick);
        Ok(end)
    }

    /// The fleet left the game this tick (salvaged).
    pub(super) fn removed(&self, fleet_idx: usize) -> bool {
        self.to_remove[fleet_idx]
    }

    /// The order and target depth flag the encounter rules see.
    pub(super) fn effective_order(
        &self,
        game_data: &CoreGameData,
        fleet_idx: usize,
    ) -> (Order, u8) {
        self.tracks[fleet_idx].effective_order(&game_data.fleets.records[fleet_idx])
    }

    /// The record's position was changed outside movement.
    pub(super) fn reload(&mut self, fleet_idx: usize) {
        self.tracks[fleet_idx].reload();
    }

    /// Undo fleet `fleet_idx`'s step this tick.
    pub(super) fn undo_step(&mut self, game_data: &mut CoreGameData, fleet_idx: usize) {
        self.tracks[fleet_idx].undo_step(&mut game_data.fleets.records[fleet_idx]);
    }

    pub(super) fn finish(
        mut self,
        game_data: &mut CoreGameData,
    ) -> (MovementEvents, FleetRemovalRemapInfo) {
        for (idx, track) in self.tracks.iter().enumerate() {
            track.finish(&mut game_data.fleets.records[idx]);
            self.events.fleet_travel_speeds[idx] = track.travel_speed;
        }
        let remap_info = if self.to_remove.iter().any(|remove| *remove) {
            super::apply_fleet_removal_remap(game_data, &self.to_remove)
        } else {
            FleetRemovalRemapInfo::default()
        };
        (self.events, remap_info)
    }
}

/// Lengths of the event lists before a dispatch, so its events get the
/// tick as their stardate.
struct EventMarks {
    colonizations: usize,
    missions: usize,
    intel: usize,
    observations: usize,
    salvage: usize,
    escalations: usize,
}

impl EventMarks {
    fn of(events: &MovementEvents) -> Self {
        Self {
            colonizations: events.colonization_events.len(),
            missions: events.mission_events.len(),
            intel: events.planet_intel_events.len(),
            observations: events.pending_observation_events.len(),
            salvage: events.salvage_events.len(),
            escalations: events.diplomatic_escalation_events.len(),
        }
    }

    fn stamp(self, events: &mut MovementEvents, tick: u8) {
        for event in &mut events.colonization_events[self.colonizations..] {
            event.stardate_week = Some(tick);
        }
        for event in &mut events.mission_events[self.missions..] {
            event.stardate_week = Some(tick);
        }
        for event in &mut events.planet_intel_events[self.intel..] {
            event.stardate_week = Some(tick);
        }
        for event in &mut events.pending_observation_events[self.observations..] {
            event.stardate_week = Some(tick);
            if let Some(intel) = event.intel_event.as_mut() {
                intel.stardate_week = Some(tick);
            }
        }
        for event in &mut events.salvage_events[self.salvage..] {
            match event {
                nc_data::SalvageResolvedEvent::Succeeded { stardate_week, .. }
                | nc_data::SalvageResolvedEvent::Failed { stardate_week, .. } => {
                    *stardate_week = Some(tick)
                }
            }
        }
        for event in &mut events.diplomatic_escalation_events[self.escalations..] {
            event.stardate_week = Some(tick);
        }
    }
}

pub(super) fn complete_view_world(fleet: &mut nc_data::FleetRecord) -> bool {
    arrivals::complete_view_world(fleet)
}
