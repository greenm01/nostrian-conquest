use std::path::Path;

use crate::{
    CampaignRuntimeState, CampaignStore, CoreGameData, DEFAULT_CAMPAIGN_DB_NAME, PlayerAccessMode,
    PlayerActivityState, PlayerLifecycleState, QueuedPlayerMail, TurnSubmission,
    TurnSubmissionError, TurnSubmissionReport, WinnerState, player_access_mode,
    record_submitted_turn_participation,
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
    submit_to_runtime_state(&store, state, &submission)
}

/// Apply `submission` to loaded runtime state without saving: an empire
/// without normal access is refused, the orders and mail go in, and the
/// player's participation for the year is recorded.
pub(super) fn apply_to_runtime_state(
    submission: &TurnSubmission,
    game_data: &mut CoreGameData,
    queued_mail: &mut Vec<QueuedPlayerMail>,
    player_lifecycle_states: &[PlayerLifecycleState],
    winner_state: WinnerState,
    player_activity_states: &mut [PlayerActivityState],
) -> Result<TurnSubmissionReport, TurnSubmissionError> {
    let player_record_index_1_based = submission.player_record_index_1_based;
    let access_mode = player_access_mode(
        player_record_index_1_based,
        player_lifecycle_states,
        winner_state,
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
    let report = submission.apply_to(game_data, queued_mail)?;
    record_submitted_turn_participation(
        game_data,
        player_record_index_1_based,
        submission.year,
        player_activity_states,
    );
    Ok(report)
}

/// Apply `submission` to the campaign's latest state and save it, in one
/// store transaction (t204), so turns submitted at the same time are all
/// kept. The save is refused with [`CampaignStoreError::StaleSnapshot`] if
/// a newer year has been written since `state` was loaded.
pub(super) fn submit_to_runtime_state(
    store: &CampaignStore,
    state: CampaignRuntimeState,
    submission: &TurnSubmission,
) -> Result<TurnSubmissionReport, TurnSubmissionError> {
    let (report, _) = store.change(state.game_year, |latest| {
        apply_to_runtime_state(
            submission,
            &mut latest.game_data,
            &mut latest.queued_mail,
            &latest.player_lifecycle_states,
            latest.winner_state,
            &mut latest.player_activity_states,
        )
    })?;
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
