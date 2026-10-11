//! Turn-by-turn play over a built scenario.
//!
//! Turns are submitted through the real turn-submission path, then one
//! maintenance turn runs with player lifecycle and winner state carried
//! across turns, as campaign maintenance does.

use nc_data::{
    MaintenanceEvents, PlayerLifecycleState, TurnSubmission, TurnSubmissionReport, WinnerState,
    default_player_lifecycle_states,
};
use nc_engine::run_maintenance_turn_with_context_seed_and_lifecycle;

use crate::build::BuiltScenario;
use crate::error::HarnessError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScenarioPlay {
    pub built: BuiltScenario,
    pub player_lifecycle_states: Vec<PlayerLifecycleState>,
    pub winner_state: WinnerState,
}

impl ScenarioPlay {
    pub fn new(built: BuiltScenario) -> Self {
        let player_lifecycle_states =
            default_player_lifecycle_states(built.game_data.conquest.player_count);
        Self {
            built,
            player_lifecycle_states,
            winner_state: WinnerState::default(),
        }
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
        if self.winner_state.winner_empire.is_some() {
            return Err(HarnessError::Validation(
                "campaign winner has already been declared; maintenance is frozen".to_string(),
            ));
        }
        run_maintenance_turn_with_context_seed_and_lifecycle(
            &mut self.built.game_data,
            0,
            &[],
            &[],
            &mut self.player_lifecycle_states,
            &mut self.winner_state,
        )
        .map_err(|err| HarnessError::Validation(err.to_string()))
    }
}
