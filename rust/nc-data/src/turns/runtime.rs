use std::collections::BTreeMap;
use std::path::Path;

use crate::{
    CampaignStore, CampaignStoreError, CoreGameData, DEFAULT_CAMPAIGN_DB_NAME, PlanetIntelSnapshot,
    PlayerAccessMode, TurnSubmission, TurnSubmissionError, TurnSubmissionReport,
    merge_player_intel_from_runtime, player_access_mode, record_submitted_turn_participation,
};

pub(super) fn submit_turn_kdl_file(
    dir: &Path,
    player_record_index_1_based: usize,
    file: &Path,
    check_only: bool,
) -> Result<TurnSubmissionReport, TurnSubmissionError> {
    let submission = TurnSubmission::load_kdl(file)?;
    if submission.player_record_index_1_based != player_record_index_1_based {
        return Err(TurnSubmissionError::Validation(format!(
            "submit-turn player mismatch: CLI requested player {}, file declares player {}",
            player_record_index_1_based, submission.player_record_index_1_based
        )));
    }

    let store = open_existing_store(dir)?;
    let mut state = store
        .load_latest_runtime_state()?
        .ok_or_else(|| TurnSubmissionError::Validation(NO_SNAPSHOT.to_string()))?;
    if check_only {
        return submission.apply_to(&mut state.game_data, &mut state.queued_mail);
    }

    let player_lifecycle_states =
        store.latest_player_lifecycle_states(state.game_data.conquest.player_count())?;
    let access_mode = player_access_mode(
        player_record_index_1_based,
        &player_lifecycle_states,
        state.winner_state,
    );
    if !matches!(access_mode, PlayerAccessMode::Normal) {
        return Err(TurnSubmissionError::Validation(match access_mode {
            PlayerAccessMode::ReviewOnly => {
                "this empire is in review-only mode and cannot submit turns".to_string()
            }
            PlayerAccessMode::SurveyOnly => {
                "the campaign has ended; survey-only access cannot submit turns".to_string()
            }
            PlayerAccessMode::LockedOut => {
                "this empire is locked out and cannot submit turns".to_string()
            }
            PlayerAccessMode::Normal => unreachable!(),
        }));
    }
    let report = submission.apply_to(&mut state.game_data, &mut state.queued_mail)?;
    let planet_intel_by_viewer = load_runtime_intel_by_viewer(&store, &state.game_data)?;
    let mut player_activity_states =
        store.latest_player_activity_states(state.game_data.conquest.player_count())?;
    record_submitted_turn_participation(
        &mut state.game_data,
        player_record_index_1_based,
        submission.year,
        &mut player_activity_states,
    );
    store.save_runtime_state_structured_with_intel_and_activity(
        &state.game_data,
        &state.planet_scorch_orders,
        &state.report_block_rows,
        &state.queued_mail,
        &planet_intel_by_viewer,
        &player_activity_states,
    )?;
    Ok(report)
}

const NO_SNAPSHOT: &str =
    "campaign store has no snapshots; initialize the campaign with nc-sysop first";

/// The campaign store in `dir`, without creating one where none exists.
fn open_existing_store(dir: &Path) -> Result<CampaignStore, TurnSubmissionError> {
    let path = dir.join(DEFAULT_CAMPAIGN_DB_NAME);
    if !path.exists() {
        return Err(TurnSubmissionError::Validation(format!(
            "campaign store not found at {}",
            path.display()
        )));
    }
    Ok(CampaignStore::open(path)?)
}

fn load_runtime_intel_by_viewer(
    campaign_store: &CampaignStore,
    game_data: &CoreGameData,
) -> Result<Vec<BTreeMap<usize, PlanetIntelSnapshot>>, CampaignStoreError> {
    (1..=game_data.conquest.player_count())
        .map(|viewer_empire_id| {
            let previous = campaign_store
                .latest_planet_intel_for_viewer(viewer_empire_id)?
                .into_iter()
                .map(|snapshot| (snapshot.planet_record_index_1_based, snapshot))
                .collect::<BTreeMap<_, _>>();
            Ok(merge_player_intel_from_runtime(
                game_data,
                viewer_empire_id,
                game_data.conquest.game_year(),
                Some(&previous),
                None,
            ))
        })
        .collect()
}
