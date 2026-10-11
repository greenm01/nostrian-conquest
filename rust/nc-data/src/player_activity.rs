use crate::{CoreGameData, PlayerActivityState};

pub const DEFAULT_INACTIVITY_AUTOPILOT_AFTER_TURNS: u8 = 3;

pub fn default_player_activity_states(player_count: u8) -> Vec<PlayerActivityState> {
    (1..=player_count as usize)
        .map(|player_record_index_1_based| PlayerActivityState {
            player_record_index_1_based,
            last_participation_year: 0,
            inactivity_autopilot_pending_clear: false,
        })
        .collect()
}

pub fn apply_inactivity_autopilot_policy(
    game_data: &mut CoreGameData,
    threshold: u8,
    player_activity_states: &mut [PlayerActivityState],
) {
    if threshold == 0 {
        return;
    }
    let current_year = game_data.conquest.game_year();
    for state in player_activity_states {
        let Some(player) = game_data
            .players
            .get_mut(state.player_record_index_1_based.saturating_sub(1))
        else {
            continue;
        };
        if !player.is_active_human_player() {
            continue;
        }
        let missed_turns = current_year.saturating_sub(state.last_participation_year);
        if missed_turns < u16::from(threshold) {
            continue;
        }
        if player.autopilot_flag() == 0 {
            player.set_autopilot_flag(1);
            state.inactivity_autopilot_pending_clear = true;
        }
    }
}

pub fn record_interactive_participation(
    game_data: &mut CoreGameData,
    player_record_index_1_based: usize,
    player_activity_states: &mut [PlayerActivityState],
) {
    let current_year = game_data.conquest.game_year();
    if let Some(player) = game_data
        .players
        .get_mut(player_record_index_1_based.saturating_sub(1))
    {
        player.set_last_run_year_raw(current_year);
        if let Some(state) =
            player_activity_states.get_mut(player_record_index_1_based.saturating_sub(1))
        {
            state.last_participation_year = current_year;
            if state.inactivity_autopilot_pending_clear {
                player.set_autopilot_flag(0);
                state.inactivity_autopilot_pending_clear = false;
            }
        }
    }
}

pub fn record_submitted_turn_participation(
    game_data: &mut CoreGameData,
    player_record_index_1_based: usize,
    year: u16,
    player_activity_states: &mut [PlayerActivityState],
) {
    let Some(state) = player_activity_states.get_mut(player_record_index_1_based.saturating_sub(1))
    else {
        return;
    };
    state.last_participation_year = year;
    if state.inactivity_autopilot_pending_clear {
        if let Some(player) = game_data
            .players
            .get_mut(player_record_index_1_based.saturating_sub(1))
        {
            player.set_autopilot_flag(0);
        }
        state.inactivity_autopilot_pending_clear = false;
    }
}

pub fn clear_inactivity_autopilot_pending(
    player_record_index_1_based: usize,
    player_activity_states: &mut [PlayerActivityState],
) {
    if let Some(state) =
        player_activity_states.get_mut(player_record_index_1_based.saturating_sub(1))
    {
        state.inactivity_autopilot_pending_clear = false;
    }
}
