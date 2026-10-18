//! A save is refused once maintenance has written a newer year than the one
//! the caller loaded, so a long-lived session cannot write into a snapshot the
//! store no longer reads.

use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use nc_data::{
    CampaignRuntimeState, CampaignStore, CampaignStoreError, STALE_SESSION_MESSAGE, TurnSubmission,
    TurnSubmissionError,
};
use nc_engine::build_seeded_initialized_game;

static TEMP_DIR_SEQ: AtomicU64 = AtomicU64::new(0);

struct TempStore {
    dir: PathBuf,
    store: CampaignStore,
}

impl Drop for TempStore {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

/// A store holding one year-3000 snapshot of a four-player game.
fn store_at_year_3000() -> TempStore {
    let dir = std::env::temp_dir().join(format!(
        "nc-data-stale-save-{}-{}",
        std::process::id(),
        TEMP_DIR_SEQ.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let store = CampaignStore::open_default_in_dir(&dir).expect("open store");
    let data = build_seeded_initialized_game(4, 3000, 1515).expect("seeded game");
    store
        .save_year(data.conquest.game_year, &data, &[], &[])
        .expect("seed store");
    TempStore { dir, store }
}

fn load(store: &CampaignStore) -> CampaignRuntimeState {
    store
        .load_latest_runtime_state()
        .expect("load runtime state")
        .expect("a snapshot exists")
}

/// One maintenance run as `maint-rust` does it: it loads year Y, advances the
/// game, and saves the new year, passing the year it loaded.
fn run_maintenance(store: &CampaignStore) -> Result<i64, CampaignStoreError> {
    let mut state = load(store);
    nc_engine::run_maintenance_turn(&mut state.game_data).expect("maintenance should run");
    store.save_year(
        state.game_year,
        &state.game_data,
        &state.report_block_rows,
        &state.queued_mail,
    )
}

fn save_as_session(
    store: &CampaignStore,
    session: &CampaignRuntimeState,
) -> Result<i64, CampaignStoreError> {
    store.save_year(
        session.game_year,
        &session.game_data,
        &session.report_block_rows,
        &session.queued_mail,
    )
}

#[test]
fn session_save_after_maintenance_is_refused_and_leaves_the_new_year_untouched() {
    let temp = store_at_year_3000();
    let store = &temp.store;
    let mut session = load(store);
    assert_eq!(session.game_year, 3000);
    let loaded_snapshot = store
        .load_snapshot_game_data(session.snapshot_id)
        .expect("load year 3000 snapshot");

    run_maintenance(store).expect("maintenance saves the new year");
    let after_maintenance = load(store);
    assert_eq!(after_maintenance.game_year, 3001);

    session.game_data.players[0].tax_rate = 77;
    let err = save_as_session(store, &session).expect_err("stale save must be refused");

    assert!(matches!(
        err,
        CampaignStoreError::StaleSnapshot {
            loaded_year: 3000,
            latest_year: 3001
        }
    ));
    assert_eq!(err.to_string(), STALE_SESSION_MESSAGE);
    assert_eq!(load(store), after_maintenance);
    assert_eq!(
        store
            .load_snapshot_game_data(session.snapshot_id)
            .expect("year 3000 snapshot still loads"),
        loaded_snapshot,
        "the refused save must not rewrite the old year either"
    );
}

#[test]
fn session_save_in_the_loaded_year_still_succeeds() {
    let temp = store_at_year_3000();
    let store = &temp.store;
    let mut session = load(store);

    session.game_data.players[0].tax_rate = 77;
    save_as_session(store, &session).expect("save in the loaded year");
    session.game_data.players[0].tax_rate = 78;
    save_as_session(store, &session).expect("a second save in the same year");

    let saved = load(store);
    assert_eq!(saved.game_year, 3000);
    assert_eq!(saved.game_data.players[0].tax_rate, 78);
}

#[test]
fn session_that_loads_after_maintenance_saves_the_new_year() {
    let temp = store_at_year_3000();
    let store = &temp.store;
    run_maintenance(store).expect("maintenance saves the new year");

    let mut session = load(store);
    assert_eq!(session.game_year, 3001);
    session.game_data.players[0].tax_rate = 55;
    save_as_session(store, &session).expect("a fresh session saves year 3001");

    assert_eq!(load(store).game_data.players[0].tax_rate, 55);
}

#[test]
fn maintenance_saves_the_new_year_and_a_second_run_from_the_old_year_is_refused() {
    let temp = store_at_year_3000();
    let store = &temp.store;
    let mut first = load(store);
    let mut second = load(store);
    for state in [&mut first, &mut second] {
        nc_engine::run_maintenance_turn(&mut state.game_data).expect("maintenance should run");
        assert_eq!(state.game_data.conquest.game_year, 3001);
    }

    save_as_session(store, &first).expect("maintenance saves year 3001");
    let saved = load(store);
    assert_eq!(saved.game_year, 3001);

    let err = save_as_session(store, &second).expect_err("a racing maintenance run is refused");
    assert!(matches!(err, CampaignStoreError::StaleSnapshot { .. }));
    assert_eq!(load(store), saved);
}

fn tax_submission(rate: u8) -> TurnSubmission {
    TurnSubmission::parse_kdl_str(&format!("turn player=1 year=3000\ntax rate={rate}\n"))
        .expect("parse turn submission")
}

#[test]
fn submit_turn_against_a_stale_year_is_refused_with_the_player_message() {
    let temp = store_at_year_3000();
    let store = &temp.store;
    let state = load(store);

    run_maintenance(store).expect("maintenance saves the new year");
    let after_maintenance = load(store);

    let err = tax_submission(37)
        .submit_to_runtime_state(store, state)
        .expect_err("a stale submission must be refused");

    assert!(matches!(
        err,
        TurnSubmissionError::Storage(CampaignStoreError::StaleSnapshot { .. })
    ));
    assert_eq!(err.to_string(), STALE_SESSION_MESSAGE);
    assert_eq!(load(store), after_maintenance);
}

#[test]
fn submit_turn_against_the_current_year_saves() {
    let temp = store_at_year_3000();
    let store = &temp.store;

    tax_submission(37)
        .submit_to_runtime_state(store, load(store))
        .expect("submission in the loaded year saves");

    assert_eq!(load(store).game_data.players[0].tax_rate, 37);
}
