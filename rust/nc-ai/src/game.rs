//! A scenario played with every seat's knowledge kept up to date.

use nc_data::{MaintenanceEvents, TurnSubmission, TurnSubmissionReport};
use nc_engine::visible_hazard_intel_from_snapshots;
use nc_harness::{HarnessError, ScenarioPlay};

use crate::memory::SeatMemory;
use crate::observe::{Observation, observe};

/// A [`ScenarioPlay`] with one [`SeatMemory`] per empire. Maintenance routes
/// each empire's fleets around the hazards it knows of, then every seat
/// learns the year's reports addressed to it, as campaign maintenance does.
#[derive(Debug, Clone)]
pub struct SeatedGame {
    pub play: ScenarioPlay,
    pub seats: Vec<SeatMemory>,
}

impl SeatedGame {
    pub fn new(play: ScenarioPlay) -> Self {
        let game_data = &play.built.game_data;
        let seats = (1..=game_data.conquest.player_count)
            .map(|empire| SeatMemory::new(game_data, empire))
            .collect();
        Self { play, seats }
    }

    pub fn submit_turn(
        &mut self,
        submission: &TurnSubmission,
    ) -> Result<TurnSubmissionReport, HarnessError> {
        self.play.submit_turn(submission)
    }

    /// Run one maintenance year and update every seat's memory.
    pub fn run_maintenance(&mut self) -> Result<MaintenanceEvents, HarnessError> {
        let hazards: Vec<_> = self
            .seats
            .iter()
            .map(|seat| {
                visible_hazard_intel_from_snapshots(
                    &self.play.built.game_data,
                    &seat.planet_intel,
                    seat.empire,
                )
            })
            .collect();
        let events = self.play.run_maintenance_with_visible_hazards(&hazards)?;
        for seat in &mut self.seats {
            seat.learn_year(&self.play.built.game_data, &events);
        }
        Ok(events)
    }

    /// What `empire` (1-based) sees now.
    pub fn observe(&self, empire: u8) -> Observation {
        observe(
            &self.play.built.game_data,
            &self.seats[usize::from(empire) - 1],
        )
    }
}
