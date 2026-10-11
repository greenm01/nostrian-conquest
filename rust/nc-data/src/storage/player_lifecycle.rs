use rusqlite::{Connection, params};

use super::{CampaignStore, CampaignStoreError};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TerminalOutcome {
    #[default]
    None,
    Defeated,
    LostGame,
    Winner,
}

impl TerminalOutcome {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Defeated => "defeated",
            Self::LostGame => "lost_game",
            Self::Winner => "winner",
        }
    }

    fn from_str(value: &str) -> Self {
        match value {
            "defeated" => Self::Defeated,
            "lost_game" => Self::LostGame,
            "winner" => Self::Winner,
            _ => Self::None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct PlayerLifecycleState {
    pub player_record_index_1_based: usize,
    pub recovery_window_turns_remaining: u8,
    pub terminal_outcome: TerminalOutcome,
    pub terminal_review_consumed: bool,
}

impl PlayerLifecycleState {
    pub fn for_player(player_record_index_1_based: usize) -> Self {
        Self {
            player_record_index_1_based,
            ..Self::default()
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct WinnerState {
    pub winner_empire: Option<u8>,
    pub winner_declared_year: Option<u16>,
}

impl CampaignStore {
    pub fn latest_player_lifecycle_states(
        &self,
        player_count: u8,
    ) -> Result<Vec<PlayerLifecycleState>, CampaignStoreError> {
        let mut conn = self.connection()?;
        let Some((snapshot_id, _)) = super::metadata::latest_snapshot_id_and_year(&mut conn)?
        else {
            return Ok(default_player_lifecycle_states(player_count));
        };
        load_player_lifecycle_rows(&conn, snapshot_id, player_count)
    }

    pub fn winner_state(&self) -> Result<WinnerState, CampaignStoreError> {
        let mut conn = self.connection()?;
        super::metadata::load_winner_state(&mut conn)
    }
}

pub fn default_player_lifecycle_states(player_count: u8) -> Vec<PlayerLifecycleState> {
    (1..=player_count as usize)
        .map(PlayerLifecycleState::for_player)
        .collect()
}

pub(super) fn write_player_lifecycle_rows(
    tx: &rusqlite::Transaction<'_>,
    snapshot_id: i64,
    player_count: u8,
    previous_snapshot_id: Option<i64>,
    override_states: Option<&[PlayerLifecycleState]>,
) -> Result<(), CampaignStoreError> {
    let states = if let Some(states) = override_states {
        normalize_player_lifecycle_states(states, player_count)
    } else if let Some(previous_snapshot_id) = previous_snapshot_id {
        load_player_lifecycle_rows(tx, previous_snapshot_id, player_count)?
    } else {
        default_player_lifecycle_states(player_count)
    };

    let mut stmt = tx.prepare(
        "INSERT INTO player_lifecycle(
             snapshot_id,
             player_record_index,
             recovery_window_turns_remaining,
             terminal_outcome,
             terminal_review_consumed
         ) VALUES (?1, ?2, ?3, ?4, ?5)",
    )?;

    for state in states {
        stmt.execute(params![
            snapshot_id,
            state.player_record_index_1_based as i64,
            i64::from(state.recovery_window_turns_remaining),
            state.terminal_outcome.as_str(),
            i64::from(u8::from(state.terminal_review_consumed)),
        ])?;
    }

    Ok(())
}

fn load_player_lifecycle_rows(
    conn: &Connection,
    snapshot_id: i64,
    player_count: u8,
) -> Result<Vec<PlayerLifecycleState>, CampaignStoreError> {
    let mut stmt = conn.prepare(
        "SELECT player_record_index,
                recovery_window_turns_remaining,
                terminal_outcome,
                terminal_review_consumed
         FROM player_lifecycle
         WHERE snapshot_id = ?1
         ORDER BY player_record_index",
    )?;
    let mut states = stmt
        .query_map(params![snapshot_id], |row| {
            Ok(PlayerLifecycleState {
                player_record_index_1_based: row.get::<_, i64>(0)? as usize,
                recovery_window_turns_remaining: row.get::<_, i64>(1)? as u8,
                terminal_outcome: TerminalOutcome::from_str(&row.get::<_, String>(2)?),
                terminal_review_consumed: row.get::<_, i64>(3)? != 0,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

    if states.is_empty() {
        states = default_player_lifecycle_states(player_count);
    } else {
        states = normalize_player_lifecycle_states(&states, player_count);
    }
    Ok(states)
}

fn normalize_player_lifecycle_states(
    states: &[PlayerLifecycleState],
    player_count: u8,
) -> Vec<PlayerLifecycleState> {
    let expected = player_count as usize;
    let mut normalized = default_player_lifecycle_states(player_count);
    for state in states {
        if let Some(slot) = normalized.get_mut(state.player_record_index_1_based.saturating_sub(1))
        {
            *slot = *state;
        }
    }
    normalized.truncate(expected);
    normalized
}
