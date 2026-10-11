//! SQLite save/load round trip of the runtime state a fuzz game reached.

use std::collections::BTreeSet;
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use nc_data::{CampaignStore, ReportBlockRow, default_player_activity_states};
use nc_harness::ScenarioPlay;

use crate::invariants::Violation;

/// Save the campaign the way maintenance does, load it back, and compare.
pub fn check_round_trip(play: &ScenarioPlay, report_rows: &[ReportBlockRow]) -> Vec<Violation> {
    let dir = scratch_dir();
    let result = round_trip(play, report_rows, &dir);
    let _ = fs::remove_dir_all(&dir);
    match result {
        Ok(None) => Vec::new(),
        Ok(Some(detail)) | Err(detail) => vec![Violation::StorageRoundTrip(detail)],
    }
}

fn round_trip(
    play: &ScenarioPlay,
    report_rows: &[ReportBlockRow],
    dir: &std::path::Path,
) -> Result<Option<String>, String> {
    let game_data = &play.built.game_data;
    let player_count = game_data.conquest.player_count;
    fs::create_dir_all(dir).map_err(|err| err.to_string())?;
    let store = CampaignStore::open_default_in_dir(dir).map_err(|err| err.to_string())?;
    store
        .save_runtime_state_structured_with_intel_activity_and_lifecycle(
            game_data.conquest.game_year,
            game_data,
            &BTreeSet::new(),
            report_rows,
            &play.built.queued_mail,
            &[],
            &default_player_activity_states(player_count),
            &play.player_lifecycle_states,
            play.winner_state,
        )
        .map_err(|err| format!("save failed: {err}"))?;
    let loaded = store
        .load_latest_runtime_state()
        .map_err(|err| format!("load failed: {err}"))?
        .ok_or("no snapshot after save")?;
    let lifecycle = store
        .latest_player_lifecycle_states(player_count)
        .map_err(|err| format!("lifecycle load failed: {err}"))?;

    let difference = if loaded.game_data.players != game_data.players {
        Some("player records")
    } else if loaded.game_data.planets != game_data.planets {
        Some("planet records")
    } else if loaded.game_data.fleets != game_data.fleets {
        Some("fleet records")
    } else if loaded.game_data != *game_data {
        Some("game data")
    } else if loaded.report_block_rows != report_rows {
        Some("report rows")
    } else if loaded.queued_mail != play.built.queued_mail {
        Some("queued mail")
    } else if loaded.winner_state != play.winner_state {
        Some("winner state")
    } else if lifecycle != play.player_lifecycle_states {
        Some("player lifecycle")
    } else {
        None
    };
    Ok(difference.map(|what| format!("{what} differ after reload")))
}

fn scratch_dir() -> PathBuf {
    static NEXT: AtomicU64 = AtomicU64::new(0);
    std::env::temp_dir().join(format!(
        "nc-fuzz-store-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ))
}
