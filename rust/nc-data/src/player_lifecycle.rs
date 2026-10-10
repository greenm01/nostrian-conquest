use crate::{
    CoreGameData, PlayerActivityState, PlayerLifecycleState, PlayerRecord, TerminalOutcome,
    WinnerState,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PublicEmpireStatus {
    Active,
    Mia,
    Defeated,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayerAccessMode {
    Normal,
    ReviewOnly,
    SurveyOnly,
    LockedOut,
}

pub fn empire_has_recovery_path(game_data: &CoreGameData, empire_raw: u8) -> bool {
    let unowned_planet_exists = game_data
        .planets
        .records
        .iter()
        .any(|planet| planet.owner_empire_slot_raw() == 0);

    game_data.fleets.records.iter().any(|fleet| {
        if fleet.owner_empire_raw() != empire_raw {
            return false;
        }
        (fleet.troop_transport_count() > 0 && fleet.army_count() > 0)
            || (unowned_planet_exists && fleet.etac_count() > 0)
    })
}

/// Joined and rogue empires both contest the campaign and can be eliminated;
/// only civil disorder (0x00) is out of it.
pub fn is_contesting_empire(player: &PlayerRecord) -> bool {
    player.owner_mode_raw() != 0x00
}

/// A contesting empire still contends for the throne while it holds a world,
/// is inside its recovery window, or can still take a world. Missing
/// lifecycle states count as a fresh empire's.
pub fn is_campaign_contender(
    game_data: &CoreGameData,
    empire_raw: u8,
    player_lifecycle_states: &[PlayerLifecycleState],
) -> bool {
    let Some(player) = game_data
        .player
        .records
        .get(empire_raw.saturating_sub(1) as usize)
    else {
        return false;
    };
    let lifecycle = player_lifecycle_states
        .get(empire_raw.saturating_sub(1) as usize)
        .copied()
        .unwrap_or_else(|| PlayerLifecycleState::for_player(empire_raw as usize));
    if !matches!(lifecycle.terminal_outcome, TerminalOutcome::None) {
        return false;
    }
    let owns_any_planets = game_data
        .planets
        .records
        .iter()
        .any(|planet| planet.owner_empire_slot_raw() == empire_raw);
    is_contesting_empire(player)
        && (owns_any_planets
            || lifecycle.recovery_window_turns_remaining > 0
            || empire_has_recovery_path(game_data, empire_raw))
}

pub fn default_player_lifecycle_states(player_count: u8) -> Vec<PlayerLifecycleState> {
    (1..=player_count as usize)
        .map(PlayerLifecycleState::for_player)
        .collect()
}

pub fn player_public_status(
    game_data: &CoreGameData,
    player_record_index_1_based: usize,
    player_activity_states: &[PlayerActivityState],
    player_lifecycle_states: &[PlayerLifecycleState],
) -> PublicEmpireStatus {
    let lifecycle = player_lifecycle_states
        .get(player_record_index_1_based.saturating_sub(1))
        .copied()
        .unwrap_or_else(|| PlayerLifecycleState::for_player(player_record_index_1_based));
    if matches!(
        lifecycle.terminal_outcome,
        TerminalOutcome::Defeated | TerminalOutcome::LostGame
    ) {
        return PublicEmpireStatus::Defeated;
    }

    let activity = player_activity_states
        .get(player_record_index_1_based.saturating_sub(1))
        .copied()
        .unwrap_or(PlayerActivityState {
            player_record_index_1_based,
            last_participation_year: 0,
            inactivity_autopilot_pending_clear: false,
        });
    let autopilot_active = game_data
        .player
        .records
        .get(player_record_index_1_based.saturating_sub(1))
        .map(|player| player.autopilot_flag() != 0)
        .unwrap_or(false);
    if autopilot_active && activity.inactivity_autopilot_pending_clear {
        PublicEmpireStatus::Mia
    } else {
        PublicEmpireStatus::Active
    }
}

pub fn player_access_mode(
    player_record_index_1_based: usize,
    player_lifecycle_states: &[PlayerLifecycleState],
    winner_state: WinnerState,
) -> PlayerAccessMode {
    let lifecycle = player_lifecycle_states
        .get(player_record_index_1_based.saturating_sub(1))
        .copied()
        .unwrap_or_else(|| PlayerLifecycleState::for_player(player_record_index_1_based));
    match lifecycle.terminal_outcome {
        TerminalOutcome::Winner => PlayerAccessMode::SurveyOnly,
        TerminalOutcome::Defeated | TerminalOutcome::LostGame => {
            if lifecycle.terminal_review_consumed {
                PlayerAccessMode::LockedOut
            } else {
                PlayerAccessMode::ReviewOnly
            }
        }
        TerminalOutcome::None => {
            if winner_state.winner_empire_raw.is_some() {
                PlayerAccessMode::LockedOut
            } else {
                PlayerAccessMode::Normal
            }
        }
    }
}
