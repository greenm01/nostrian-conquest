mod conquest;
mod state;

use super::{FleetDefectionEvent, MaintenanceEvents};
use crate::CoreGameData;
use nc_data::{
    CampaignOutcome, CampaignOutlook, EmpireEliminationEvent, FleetDestroyedEvent,
    PlanetOwnershipChangeEvent, PlayerLifecycleState, WinnerState,
};

pub(super) fn process_conquest_header(
    game_data: &mut CoreGameData,
    should_accumulate: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    conquest::process_conquest_header(game_data, should_accumulate)
}

pub(super) use state::EmpireTurnStartState;

pub(super) fn capture_empire_turn_start_states(
    game_data: &CoreGameData,
    player_lifecycle_states: &[PlayerLifecycleState],
) -> Vec<EmpireTurnStartState> {
    state::capture_empire_turn_start_states(game_data, player_lifecycle_states)
}

pub(super) fn campaign_outlook(
    game_data: &CoreGameData,
    player_lifecycle_states: &[PlayerLifecycleState],
) -> CampaignOutlook {
    state::campaign_outlook(game_data, player_lifecycle_states)
}

pub(super) fn campaign_outcome(
    game_data: &CoreGameData,
    player_lifecycle_states: &[PlayerLifecycleState],
    winner_state: WinnerState,
) -> CampaignOutcome {
    state::campaign_outcome(game_data, player_lifecycle_states, winner_state)
}

pub(super) fn apply_civil_disorder_fleet_defections(
    game_data: &mut CoreGameData,
    newly_defeated: &[EmpireEliminationEvent],
) -> Result<Vec<FleetDefectionEvent>, Box<dyn std::error::Error>> {
    state::apply_civil_disorder_fleet_defections(game_data, newly_defeated)
}

pub(super) fn apply_stored_diplomatic_escalations(
    game_data: &mut CoreGameData,
    events: &MaintenanceEvents,
) -> Result<(), Box<dyn std::error::Error>> {
    state::apply_stored_diplomatic_escalations(game_data, events)
}

pub(super) fn apply_player_lifecycle_transitions(
    game_data: &mut CoreGameData,
    start_states: &[EmpireTurnStartState],
    ownership_change_events: &[PlanetOwnershipChangeEvent],
    fleet_destroyed_events: &[FleetDestroyedEvent],
    player_lifecycle_states: &mut [PlayerLifecycleState],
    winner_state: &mut WinnerState,
    initial_outlook: CampaignOutlook,
    initial_outcome: CampaignOutcome,
) -> state::CampaignTransitionEvents {
    state::apply_campaign_state_transitions(
        game_data,
        start_states,
        ownership_change_events,
        fleet_destroyed_events,
        player_lifecycle_states,
        winner_state,
        initial_outlook,
        initial_outcome,
    )
}
