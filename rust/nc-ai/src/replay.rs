//! Rebuild a played game from its seeds and the submissions applied to it.
//!
//! The engine is deterministic: the same start state, the same submissions
//! in the same order, and the same campaign seed give the same game. A
//! recorded game ([`crate::SeatedGame::turns`]) therefore stores as its map
//! seed, campaign seed, which seats ran on autopilot, and its turns.

use nc_data::TurnSubmission;
use nc_fuzz::Policy;

use crate::arena::{MatchResult, match_result, seated_game};
use crate::game::SeatedGame;

/// A fresh game on `map_seed` with `campaign_seed`, set up as the batch
/// environment and [`replay`] set it up: `autopilot` flags the seats the
/// engine plays, for up to `years` years.
pub fn match_game(map_seed: u64, campaign_seed: u64, autopilot: &[bool], years: u16) -> SeatedGame {
    let policies = autopilot
        .iter()
        .map(|&auto| {
            if auto {
                Policy::Autopilot
            } else {
                Policy::Aggressor
            }
        })
        .collect();
    seated_game(map_seed, policies, campaign_seed, years)
}

/// Replay up to `years` years of a game on `map_seed` with `campaign_seed`.
/// `autopilot` flags the seats the engine played, which setup must know;
/// `turns` are the recorded submissions in order. Each year applies the
/// turns dated that year, then runs maintenance, until a winner is declared
/// or `years` have passed.
pub fn replay(
    map_seed: u64,
    campaign_seed: u64,
    autopilot: &[bool],
    years: u16,
    turns: &[TurnSubmission],
) -> Result<MatchResult, String> {
    let mut game = match_game(map_seed, campaign_seed, autopilot, years);
    let mut next = 0;
    let mut played = 0;
    while played < years {
        let year = game.play.built.game_data.conquest.game_year;
        while let Some(turn) = turns.get(next).filter(|turn| turn.year == year) {
            game.submit_turn(turn)
                .map_err(|err| format!("turn {next} (year {year}): {err}"))?;
            next += 1;
        }
        game.run_maintenance()
            .map_err(|err| format!("maintenance in year {year}: {err}"))?;
        played += 1;
        if game.play.winner_state.winner_empire.is_some() {
            break;
        }
    }
    if next != turns.len() {
        return Err(format!(
            "{} of {} turns were never applied",
            turns.len() - next,
            turns.len()
        ));
    }
    Ok(match_result(
        &game.play.built.game_data,
        game.play.winner_state.winner_empire,
        played,
    ))
}
