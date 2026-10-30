//! AI seats (t165): which seats a learned agent plays, how hard, and what
//! each one remembers between maintenance runs.
//!
//! A seat's memory (`nc_ai::SeatMemory`) holds more than the stored records
//! do: the foreign forces its reports named, and what its own attacks
//! revealed. The store keeps it as text the AI layer writes, one row per
//! seat, dated by the year it was learned up to.

use rusqlite::{OptionalExtension, params};

use super::{CampaignStore, CampaignStoreError};

/// How hard an AI seat plays.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AiDifficulty {
    Easy,
    Normal,
    Hard,
}

impl AiDifficulty {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Easy => "easy",
            Self::Normal => "normal",
            Self::Hard => "hard",
        }
    }

    pub fn parse(text: &str) -> Result<Self, CampaignStoreError> {
        match text.trim().to_ascii_lowercase().as_str() {
            "easy" => Ok(Self::Easy),
            "normal" => Ok(Self::Normal),
            "hard" => Ok(Self::Hard),
            other => Err(CampaignStoreError::InvalidState(format!(
                "AI difficulty must be easy, normal or hard, got {other:?}"
            ))),
        }
    }
}

/// A seat a learned agent plays.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AiSeat {
    pub player_record_index_1_based: usize,
    pub difficulty: AiDifficulty,
    /// The seat's style (t167), one of `nc_ai::PERSONALITIES`.
    pub personality: String,
}

/// The personality an AI seat plays unless the sysop names one.
pub const DEFAULT_AI_PERSONALITY: &str = "land-grab";

pub(super) fn load_ai_seats_conn(
    conn: &rusqlite::Connection,
) -> Result<Vec<AiSeat>, CampaignStoreError> {
    let mut stmt = conn.prepare(
        "SELECT player_record_index, difficulty, personality
         FROM ai_seats
         ORDER BY player_record_index ASC",
    )?;
    let rows = stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, i64>(0)? as usize,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
            ))
        })?
        .collect::<Result<Vec<_>, _>>()?;
    rows.into_iter()
        .map(|(player, difficulty, personality)| {
            Ok(AiSeat {
                player_record_index_1_based: player,
                difficulty: AiDifficulty::parse(&difficulty)?,
                personality,
            })
        })
        .collect()
}

pub(super) fn save_ai_seats_tx(
    tx: &rusqlite::Transaction<'_>,
    seats: &[AiSeat],
) -> Result<(), CampaignStoreError> {
    tx.execute("DELETE FROM ai_seats", [])?;
    for seat in seats {
        tx.execute(
            "INSERT INTO ai_seats(player_record_index, difficulty, personality)
             VALUES (?1, ?2, ?3)",
            params![
                seat.player_record_index_1_based as i64,
                seat.difficulty.as_str(),
                seat.personality
            ],
        )?;
    }
    Ok(())
}

impl CampaignStore {
    /// The memory saved for AI seat `player`, and the year it was learned
    /// up to.
    pub fn load_ai_seat_memory(
        &self,
        player_record_index_1_based: usize,
    ) -> Result<Option<(u16, String)>, CampaignStoreError> {
        let conn = self.connection()?;
        Ok(conn
            .query_row(
                "SELECT game_year, memory FROM ai_seat_memory WHERE player_record_index = ?1",
                params![player_record_index_1_based as i64],
                |row| Ok((row.get::<_, i64>(0)? as u16, row.get::<_, String>(1)?)),
            )
            .optional()?)
    }

    /// Replace the AI seats' turns of every year in `turns`, which holds
    /// `(game_year, player, turn KDL)` in the order they were applied. With
    /// the year's stored snapshot, they replay its maintenance exactly.
    pub fn save_ai_turns(&self, turns: &[(u16, usize, String)]) -> Result<(), CampaignStoreError> {
        let mut conn = self.connection()?;
        let tx = conn.transaction()?;
        let mut years: Vec<u16> = turns.iter().map(|(year, ..)| *year).collect();
        years.dedup();
        for year in years {
            tx.execute(
                "DELETE FROM ai_turns WHERE game_year = ?1",
                params![i64::from(year)],
            )?;
            for (seq, (_, player, turn)) in turns.iter().filter(|t| t.0 == year).enumerate() {
                tx.execute(
                    "INSERT INTO ai_turns(game_year, seq, player_record_index, turn)
                     VALUES (?1, ?2, ?3, ?4)",
                    params![i64::from(year), seq as i64, *player as i64, turn],
                )?;
            }
        }
        tx.commit()?;
        Ok(())
    }

    /// The AI seats' turns of `game_year` as `(player, turn KDL)`, in the
    /// order they were applied.
    pub fn load_ai_turns(
        &self,
        game_year: u16,
    ) -> Result<Vec<(usize, String)>, CampaignStoreError> {
        let conn = self.connection()?;
        let mut stmt = conn.prepare(
            "SELECT player_record_index, turn FROM ai_turns WHERE game_year = ?1 ORDER BY seq",
        )?;
        let turns = stmt
            .query_map(params![i64::from(game_year)], |row| {
                Ok((row.get::<_, i64>(0)? as usize, row.get(1)?))
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(turns)
    }

    /// Replace AI seat `player`'s memory, learned up to `game_year`.
    pub fn save_ai_seat_memory(
        &self,
        player_record_index_1_based: usize,
        game_year: u16,
        memory: &str,
    ) -> Result<(), CampaignStoreError> {
        let conn = self.connection()?;
        conn.execute(
            "INSERT INTO ai_seat_memory(player_record_index, game_year, memory)
             VALUES (?1, ?2, ?3)
             ON CONFLICT(player_record_index) DO UPDATE SET
                 game_year = excluded.game_year,
                 memory = excluded.memory",
            params![
                player_record_index_1_based as i64,
                i64::from(game_year),
                memory
            ],
        )?;
        Ok(())
    }
}
