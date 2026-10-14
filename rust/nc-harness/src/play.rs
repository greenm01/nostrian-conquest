//! Turn-by-turn play over a built scenario.
//!
//! Turns are submitted through the real turn-submission path, then one
//! maintenance turn runs with player lifecycle and winner state carried
//! across turns, as campaign maintenance does. The campaign seed drives
//! combat rolls and rogue orders; it defaults to zero.

use nc_data::{
    MaintenanceEvents, PlayerLifecycleState, TurnSubmission, TurnSubmissionReport, WinnerState,
    default_player_lifecycle_states,
};
use nc_engine::{VisibleHazardIntel, run_maintenance_turn_with_context_seed_and_lifecycle};

use crate::build::BuiltScenario;
use crate::error::HarnessError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScenarioPlay {
    pub built: BuiltScenario,
    pub player_lifecycle_states: Vec<PlayerLifecycleState>,
    pub winner_state: WinnerState,
    pub campaign_seed: u64,
}

impl ScenarioPlay {
    pub fn new(built: BuiltScenario) -> Self {
        let player_lifecycle_states =
            default_player_lifecycle_states(built.game_data.conquest.player_count);
        Self {
            built,
            player_lifecycle_states,
            winner_state: WinnerState::default(),
            campaign_seed: 0,
        }
    }

    /// Use `seed` for every maintenance turn, as a campaign's runtime seed.
    pub fn with_campaign_seed(mut self, seed: u64) -> Self {
        self.campaign_seed = seed;
        self
    }

    pub fn submit_turn(
        &mut self,
        submission: &TurnSubmission,
    ) -> Result<TurnSubmissionReport, HarnessError> {
        submission
            .apply_to(&mut self.built.game_data, &mut self.built.queued_mail)
            .map_err(|err| HarnessError::Validation(err.to_string()))
    }

    /// Run one maintenance turn. Maintenance is frozen once a winner is
    /// declared, matching campaign maintenance.
    pub fn run_maintenance(&mut self) -> Result<MaintenanceEvents, HarnessError> {
        self.run_maintenance_with_visible_hazards(&[])
    }

    /// `run_maintenance` with each empire's known hazards, which campaign
    /// maintenance derives from the empires' planet intel for routing.
    pub fn run_maintenance_with_visible_hazards(
        &mut self,
        visible_hazards_by_empire: &[VisibleHazardIntel],
    ) -> Result<MaintenanceEvents, HarnessError> {
        if self.winner_state.winner_empire.is_some() {
            return Err(HarnessError::Validation(
                "campaign winner has already been declared; maintenance is frozen".to_string(),
            ));
        }
        run_maintenance_turn_with_context_seed_and_lifecycle(
            &mut self.built.game_data,
            self.campaign_seed,
            visible_hazards_by_empire,
            &[],
            &mut self.player_lifecycle_states,
            &mut self.winner_state,
        )
        .map_err(|err| HarnessError::Validation(err.to_string()))
    }
}
