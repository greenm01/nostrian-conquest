use rusqlite::{Connection, params};

use super::{CampaignStore, CampaignStoreError};
use crate::CoreGameData;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlayerActivityState {
    pub player_record_index_1_based: usize,
    pub last_participation_year: u16,
    pub inactivity_autopilot_pending_clear: bool,
}

impl CampaignStore {
    pub fn latest_player_activity_states(
        &self,
        player_count: u8,
    ) -> Result<Vec<PlayerActivityState>, CampaignStoreError> {
        let mut conn = self.connection()?;
        let Some((snapshot_id, _)) = super::metadata::latest_snapshot_id_and_year(&mut conn)?
        else {
            return Ok(default_player_activity_states(player_count));
        };
        load_player_activity_rows(&conn, snapshot_id, player_count)
    }
}

pub(super) fn write_player_activity_rows(
    tx: &rusqlite::Transaction<'_>,
    snapshot_id: i64,
    game_data: &CoreGameData,
    previous_snapshot_id: Option<i64>,
    override_states: Option<&[PlayerActivityState]>,
) -> Result<(), CampaignStoreError> {
    let mut states = if let Some(states) = override_states {
        normalize_activity_states(states, game_data)
    } else if let Some(previous_snapshot_id) = previous_snapshot_id {
        load_player_activity_rows(tx, previous_snapshot_id, game_data.conquest.player_count)?
    } else {
        default_player_activity_states(game_data.conquest.player_count)
    };

    raise_to_last_year_on(&mut states, game_data);
    insert_player_activity_rows(tx, snapshot_id, &states)
}

/// Rewrite snapshot `snapshot_id`'s activity rows if a save of `states`
/// over it would change them (t204).
pub(super) fn write_changed_player_activity_rows(
    tx: &rusqlite::Transaction<'_>,
    snapshot_id: i64,
    game_data: &CoreGameData,
    states: &[PlayerActivityState],
) -> Result<(), CampaignStoreError> {
    let mut states = normalize_activity_states(states, game_data);
    raise_to_last_year_on(&mut states, game_data);
    let stored = tx
        .prepare(
            "SELECT player_record_index, last_participation_year, inactivity_autopilot_pending_clear
             FROM player_activity WHERE snapshot_id = ?1 ORDER BY player_record_index",
        )?
        .query_map(params![snapshot_id], |row| {
            Ok(PlayerActivityState {
                player_record_index_1_based: row.get::<_, i64>(0)? as usize,
                last_participation_year: row.get::<_, i64>(1)? as u16,
                inactivity_autopilot_pending_clear: row.get::<_, i64>(2)? != 0,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    if stored == states {
        return Ok(());
    }
    tx.execute(
        "DELETE FROM player_activity WHERE snapshot_id = ?1",
        [snapshot_id],
    )?;
    insert_player_activity_rows(tx, snapshot_id, &states)
}

/// A player's participation year is at least the last year they ran a turn.
fn raise_to_last_year_on(states: &mut [PlayerActivityState], game_data: &CoreGameData) {
    for state in states {
        let current_last_run_year = game_data
            .players
            .get(state.player_record_index_1_based.saturating_sub(1))
            .map(|player| player.last_year_on)
            .unwrap_or(0);
        if current_last_run_year > state.last_participation_year {
            state.last_participation_year = current_last_run_year;
        }
    }
}

fn insert_player_activity_rows(
    tx: &rusqlite::Transaction<'_>,
    snapshot_id: i64,
    states: &[PlayerActivityState],
) -> Result<(), CampaignStoreError> {
    let mut stmt = tx.prepare(
        "INSERT INTO player_activity(
             snapshot_id,
             player_record_index,
             last_participation_year,
             inactivity_autopilot_pending_clear
         ) VALUES (?1, ?2, ?3, ?4)",
    )?;
    for state in states {
        stmt.execute(params![
            snapshot_id,
            state.player_record_index_1_based as i64,
            i64::from(state.last_participation_year),
            if state.inactivity_autopilot_pending_clear {
                1
            } else {
                0
            },
        ])?;
    }
    Ok(())
}

pub(super) fn load_player_activity_rows(
    conn: &Connection,
    snapshot_id: i64,
    player_count: u8,
) -> Result<Vec<PlayerActivityState>, CampaignStoreError> {
    let mut stmt = conn.prepare(
        "SELECT player_record_index, last_participation_year, inactivity_autopilot_pending_clear
         FROM player_activity
         WHERE snapshot_id = ?1
         ORDER BY player_record_index",
    )?;
    let mut states = stmt
        .query_map(params![snapshot_id], |row| {
            Ok(PlayerActivityState {
                player_record_index_1_based: row.get::<_, i64>(0)? as usize,
                last_participation_year: row.get::<_, i64>(1)? as u16,
                inactivity_autopilot_pending_clear: row.get::<_, i64>(2)? != 0,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

    if states.is_empty() {
        states = derive_player_activity_rows(conn, snapshot_id, player_count)?;
    } else {
        states = normalize_player_activity_count(states, player_count);
    }
    Ok(states)
}

fn derive_player_activity_rows(
    conn: &Connection,
    snapshot_id: i64,
    player_count: u8,
) -> Result<Vec<PlayerActivityState>, CampaignStoreError> {
    let mut stmt = conn.prepare(
        "SELECT record_index, last_year_on
         FROM snapshot_players
         WHERE snapshot_id = ?1
         ORDER BY record_index",
    )?;
    let mut states = default_player_activity_states(player_count);
    for row in stmt.query_map(params![snapshot_id], |row| {
        Ok((row.get::<_, i64>(0)? as usize, row.get::<_, i64>(1)? as u16))
    })? {
        let (player_record_index_1_based, last_run_year) = row?;
        if let Some(state) = states.get_mut(player_record_index_1_based.saturating_sub(1)) {
            state.last_participation_year = last_run_year;
        }
    }
    Ok(states)
}

fn default_player_activity_states(player_count: u8) -> Vec<PlayerActivityState> {
    (1..=player_count as usize)
        .map(|player_record_index_1_based| PlayerActivityState {
            player_record_index_1_based,
            last_participation_year: 0,
            inactivity_autopilot_pending_clear: false,
        })
        .collect()
}

fn normalize_activity_states(
    states: &[PlayerActivityState],
    game_data: &CoreGameData,
) -> Vec<PlayerActivityState> {
    let player_count = game_data.players.len() as u8;
    let mut normalized = default_player_activity_states(player_count);
    for state in states {
        if let Some(slot) = normalized.get_mut(state.player_record_index_1_based.saturating_sub(1))
        {
            *slot = *state;
        }
    }
    for state in &mut normalized {
        let current_last_run_year = game_data
            .players
            .get(state.player_record_index_1_based.saturating_sub(1))
            .map(|player| player.last_year_on)
            .unwrap_or(0);
        if current_last_run_year > state.last_participation_year {
            state.last_participation_year = current_last_run_year;
        }
    }
    normalized
}

fn normalize_player_activity_count(
    mut states: Vec<PlayerActivityState>,
    player_count: u8,
) -> Vec<PlayerActivityState> {
    let expected = player_count as usize;
    states.sort_by_key(|state| state.player_record_index_1_based);
    if states.len() < expected {
        let existing = states
            .iter()
            .map(|state| state.player_record_index_1_based)
            .collect::<std::collections::BTreeSet<_>>();
        for player_record_index_1_based in 1..=expected {
            if !existing.contains(&player_record_index_1_based) {
                states.push(PlayerActivityState {
                    player_record_index_1_based,
                    last_participation_year: 0,
                    inactivity_autopilot_pending_clear: false,
                });
            }
        }
        states.sort_by_key(|state| state.player_record_index_1_based);
    } else if states.len() > expected {
        states.truncate(expected);
    }
    states
}
