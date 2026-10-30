//! Positions from a stored campaign, to start games from: a played game's
//! year as its database saved it, so learners can take over the seats of a
//! game people played.

use std::path::Path;

use nc_data::{
    CampaignStore, CampaignStoreError, CoreGameData, PlayerLifecycleState, TurnSubmission,
    name_unnamed_worlds,
};
use nc_harness::{BuiltScenario, ScenarioBuildReport, ScenarioPlay};

use crate::game::SeatedGame;
use crate::memory::SeatMemory;

/// One stored year. A door game saves its players' orders into the year's
/// snapshot as they give them, so `game_data` holds the orders the people
/// gave that year; AI seats order during maintenance, and theirs are not
/// stored.
#[derive(Debug, Clone)]
pub struct StoredPosition {
    pub game_data: CoreGameData,
    pub campaign_seed: u64,
    /// The campaign's first stored year.
    pub start_year: u16,
    pub player_lifecycle_states: Vec<PlayerLifecycleState>,
    /// Each empire's knowledge: the year's stored planet intel. Remembered
    /// contacts are not stored per year, as for an AI seat whose saved
    /// memory is stale.
    pub seats: Vec<SeatMemory>,
}

impl StoredPosition {
    /// The year the position is at.
    pub fn year(&self) -> u16 {
        self.game_data.conquest.game_year
    }

    /// A game played on from here.
    pub fn seated_game(&self) -> SeatedGame {
        let game_data = self.game_data.clone();
        let report = ScenarioBuildReport {
            label: None,
            player_count: game_data.conquest.player_count,
            year: game_data.conquest.game_year,
            planet_records: game_data.planets.len(),
            fleet_records: game_data.fleets.len(),
            queue_mail_count: 0,
            results_blocks: 0,
            message_blocks: 0,
        };
        let mut play = ScenarioPlay::new(BuiltScenario {
            game_data,
            report_block_rows: Vec::new(),
            queued_mail: Vec::new(),
            report,
        })
        .with_campaign_seed(self.campaign_seed);
        play.player_lifecycle_states = self.player_lifecycle_states.clone();
        SeatedGame {
            play,
            seats: self.seats.clone(),
            turns: Vec::new(),
        }
    }
}

/// The campaign at `path` as it stood at the start of `year`.
pub fn load_position(path: &Path, year: u16) -> Result<StoredPosition, CampaignStoreError> {
    let store = CampaignStore::open(path)?;
    let years = store.snapshot_years()?;
    let (Some(&(_, start_year)), Some(&(snapshot_id, _))) =
        (years.first(), years.iter().find(|(_, y)| *y == year))
    else {
        return Err(CampaignStoreError::InvalidState(format!(
            "{} has no snapshot for year {year}",
            path.display()
        )));
    };
    let campaign_seed = store
        .load_latest_runtime_state()?
        .map_or(0, |state| state.campaign_seed);
    let game_data = store.load_snapshot_game_data(snapshot_id)?;
    let players = game_data.conquest.player_count;
    let seats = store
        .load_snapshot_planet_intel_by_viewer(snapshot_id, players)?
        .into_iter()
        .zip(1..=players)
        .map(|(planet_intel, empire)| SeatMemory {
            empire,
            planet_intel,
            contacts: Vec::new(),
        })
        .collect();
    Ok(StoredPosition {
        player_lifecycle_states: store
            .load_snapshot_player_lifecycle_states(snapshot_id, players)?,
        game_data,
        campaign_seed,
        start_year,
        seats,
    })
}

/// The campaign at `path` at the start of `year` as maintenance left it,
/// before anyone gave orders: year − 1 played on with the AI seats' logged
/// turns, its people's orders already in its snapshot. Compared with
/// `year`'s snapshot, it gives the orders people gave in `year`.
///
/// Refused when the campaign has AI seats but year − 1 has no logged turns.
/// shortcut: inactivity autopilot and an AI seat's first join are not
/// replayed; both matter only to the years they happen in.
pub fn opening_position(path: &Path, year: u16) -> Result<StoredPosition, CampaignStoreError> {
    let store = CampaignStore::open(path)?;
    let invalid = |err: String| CampaignStoreError::InvalidState(err);
    let ai_seats: Vec<u8> = store
        .campaign_settings()?
        .map(|settings| settings.ai_seats)
        .unwrap_or_default()
        .iter()
        .map(|seat| seat.player_record_index_1_based as u8)
        .collect();
    let turns = store.load_ai_turns(year - 1)?;
    if !ai_seats.is_empty() && turns.is_empty() {
        return Err(invalid(format!(
            "{} logged no AI turns in {}",
            path.display(),
            year - 1
        )));
    }
    let mut game = load_position(path, year - 1)?.seated_game();
    for (_, turn) in &turns {
        let turn = TurnSubmission::parse_kdl_str(turn).map_err(|err| invalid(err.to_string()))?;
        game.submit_turn(&turn)
            .map_err(|err| invalid(err.to_string()))?;
    }
    game.run_maintenance()
        .map_err(|err| invalid(err.to_string()))?;
    // AI seats name the worlds they took once the year has run.
    for &empire in &ai_seats {
        name_unnamed_worlds(&mut game.play.built.game_data, empire);
    }
    let mut opening = load_position(path, year)?;
    opening.game_data = game.play.built.game_data;
    opening.player_lifecycle_states = game.play.player_lifecycle_states;
    Ok(opening)
}
