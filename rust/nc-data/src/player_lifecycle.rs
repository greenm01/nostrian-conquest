use crate::names::set_empire_name;
use crate::{
    CoreGameData, Player, PlayerActivityState, PlayerLifecycleState, PlayerMode, TerminalOutcome,
    WinnerState, troop_transports,
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

pub fn empire_has_recovery_path(game_data: &CoreGameData, empire: u8) -> bool {
    let unowned_planet_exists = game_data.planets.iter().any(|planet| planet.owner.0 == 0);

    game_data.fleets.iter().any(|fleet| {
        if fleet.owner.0 != empire {
            return false;
        }
        (troop_transports(fleet) > 0 && fleet.ships.loaded_transports > 0)
            || (unowned_planet_exists && fleet.ships.etacs > 0)
    })
}

const CIVIL_DISORDER_NAME: &str = "In Civil Disorder";
const ROGUE_NAME: &str = "Rogues";

/// Returns the empire to civil disorder as ECUTIL and ECMAINT do: mode 0, a
/// blank handle, and the name "In Civil Disorder". Every other field stays.
pub fn set_civil_disorder(player: &mut Player) {
    player.mode = PlayerMode::CivilDisorder;
    player.handle.clear();
    set_empire_name(player, CIVIL_DISORDER_NAME);
}

/// Turns a civil-disorder empire into a rogue as ECMAINT's orders pass does
/// (`24a3:0c2a`): mode 0xFF, the name "In Civil Disorder" becomes "Rogues"
/// (any other name stays), and the handle goes blank. The stances stay as
/// they are. `keep_handle` leaves the handle for a former player who is
/// still owed a final review, which finds the empire by it.
pub fn set_rogue(player: &mut Player, keep_handle: bool) {
    player.mode = PlayerMode::Rogue;
    if !keep_handle {
        player.handle.clear();
    }
    if player.empire_name == CIVIL_DISORDER_NAME {
        set_empire_name(player, ROGUE_NAME);
    }
}

/// Joined and rogue empires both contest the campaign; only civil disorder
/// (0x00) is out of it. A joined empire with no way back is eliminated.
/// ECMAINT keeps playing a rogue empire after its last world falls, so a
/// rogue one is never eliminated.
pub fn is_contesting_empire(player: &Player) -> bool {
    player.mode.code() != 0x00
}

/// A contesting empire still contends for the throne while it holds a world,
/// is inside its recovery window, or can still take a world. A rogue empire
/// contends only while it holds a world: without one it patrols
/// (`24a3:0dbc`) and never takes another. Missing lifecycle states count as
/// a fresh empire's.
pub fn is_campaign_contender(
    game_data: &CoreGameData,
    empire: u8,
    player_lifecycle_states: &[PlayerLifecycleState],
) -> bool {
    let Some(player) = game_data.players.get(empire.saturating_sub(1) as usize) else {
        return false;
    };
    let lifecycle = player_lifecycle_states
        .get(empire.saturating_sub(1) as usize)
        .copied()
        .unwrap_or_else(|| PlayerLifecycleState::for_player(empire as usize));
    if !matches!(lifecycle.terminal_outcome, TerminalOutcome::None) {
        return false;
    }
    let owns_any_planets = game_data
        .planets
        .iter()
        .any(|planet| planet.owner.0 == empire);
    is_contesting_empire(player)
        && (owns_any_planets
            || (player.mode != PlayerMode::Rogue
                && (lifecycle.recovery_window_turns_remaining > 0
                    || empire_has_recovery_path(game_data, empire))))
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
        .players
        .get(player_record_index_1_based.saturating_sub(1))
        .map(|player| player.autopilot)
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
            if winner_state.winner_empire.is_some() {
                PlayerAccessMode::LockedOut
            } else {
                PlayerAccessMode::Normal
            }
        }
    }
}
