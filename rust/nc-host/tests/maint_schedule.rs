//! The scheduled trigger: the serve loop's poll finds due games, arms the
//! ones never scheduled, and the game worker runs a due year and moves the
//! next due time one interval past now.

mod common;

use common::{create_test_game_in_root, seed_runtime_snapshot};
use nc_data::CampaignStore;
use nc_data::hosted::{self, HostedStore};
use nc_host::game::effects::GameEffects;
use nc_host::game::worker::GameWorker;
use nc_host::supervisor::scheduler::{Scheduler, dispatch_due_maintenance};
use nc_host::supervisor::worker_registry::WorkerRegistry;
use std::path::Path;
use std::time::Duration;

const PLAYER: &str = "schedule-player-one";

fn set_next_due(store: &HostedStore, game_id: &str, enabled: bool, next_due: Option<i64>) {
    let mut settings = hosted::get_settings(store.connection(), game_id).expect("settings");
    settings.maintenance_enabled = enabled;
    settings.maintenance_next_due_unix_seconds = next_due;
    hosted::update_settings(store.connection(), game_id, &settings).expect("update settings");
}

fn next_due(store: &HostedStore, game_id: &str) -> Option<i64> {
    hosted::get_settings(store.connection(), game_id)
        .expect("settings")
        .maintenance_next_due_unix_seconds
}

fn runtime_year(game_dir: &Path) -> u16 {
    CampaignStore::open_default_in_dir(game_dir)
        .expect("open campaign store")
        .load_latest_runtime_state()
        .expect("load runtime state")
        .expect("runtime snapshot should exist")
        .game_year
}

#[test]
fn scheduler_arms_a_game_that_was_never_scheduled_instead_of_running_it() {
    let temp = tempfile::tempdir().expect("temp dir");
    let (_dir, store) = create_test_game_in_root(temp.path(), "never-scheduled", 4);
    assert_eq!(next_due(&store, "never-scheduled"), None);
    let before = chrono::Utc::now().timestamp();

    let due = Scheduler::new(temp.path().to_path_buf()).check_due_games();

    assert!(due.is_empty());
    let armed = next_due(&store, "never-scheduled").expect("armed");
    assert!(armed >= before + 1440 * 60);
    assert!(armed <= chrono::Utc::now().timestamp() + 1440 * 60);
}

#[test]
fn scheduler_reports_only_enabled_games_past_their_due_time() {
    let temp = tempfile::tempdir().expect("temp dir");
    let now = chrono::Utc::now().timestamp();
    let (_overdue_dir, overdue) = create_test_game_in_root(temp.path(), "overdue", 4);
    set_next_due(&overdue, "overdue", true, Some(now - 5));
    let (_later_dir, later) = create_test_game_in_root(temp.path(), "later", 4);
    set_next_due(&later, "later", true, Some(now + 3600));
    let (_off_dir, off) = create_test_game_in_root(temp.path(), "switched-off", 4);
    set_next_due(&off, "switched-off", false, Some(now - 5));

    let due = Scheduler::new(temp.path().to_path_buf()).check_due_games();

    assert_eq!(due, vec!["overdue".to_string()]);
}

#[tokio::test]
async fn worker_runs_a_due_year_and_moves_the_next_due_time_one_interval_on() {
    let temp = tempfile::tempdir().expect("temp dir");
    let game_id = "worker-due";
    let (game_dir, store) = create_test_game_in_root(temp.path(), game_id, 4);
    seed_runtime_snapshot(&game_dir, game_id, "Due", 4, &[], &[]);
    hosted::claim_seat(store.connection(), game_id, 1, PLAYER, 3000).expect("claim");
    let now = chrono::Utc::now().timestamp();
    set_next_due(&store, game_id, true, Some(now - 5));
    let worker = GameWorker::new(game_id.to_string(), game_dir.join("hosted.db"));

    worker
        .handle_effect(GameEffects::RunMaintenance {
            game_id: game_id.to_string(),
        })
        .await;

    assert_eq!(runtime_year(&game_dir), 3001);
    let rescheduled = next_due(&store, game_id).expect("rescheduled");
    assert!(rescheduled >= now + 1440 * 60);
    let synced = hosted::get_pending(store.connection(), game_id, 50)
        .expect("pending outbox")
        .into_iter()
        .filter(|item| item.kind == 30520 && item.pubkey == PLAYER)
        .count();
    assert_eq!(synced, 1);
}

#[tokio::test]
async fn a_failed_run_is_still_rescheduled_for_the_next_interval() {
    let temp = tempfile::tempdir().expect("temp dir");
    let game_id = "worker-broken";
    // No campaign store: the run fails before any year.
    let (game_dir, store) = create_test_game_in_root(temp.path(), game_id, 4);
    let now = chrono::Utc::now().timestamp();
    set_next_due(&store, game_id, true, Some(now - 5));
    let worker = GameWorker::new(game_id.to_string(), game_dir.join("hosted.db"));

    worker
        .handle_effect(GameEffects::RunMaintenance {
            game_id: game_id.to_string(),
        })
        .await;

    assert!(next_due(&store, game_id).expect("rescheduled") >= now + 1440 * 60);
}

#[tokio::test]
async fn the_poll_hands_a_due_game_to_its_worker() {
    let temp = tempfile::tempdir().expect("temp dir");
    let game_id = "poll-due";
    let (game_dir, store) = create_test_game_in_root(temp.path(), game_id, 4);
    seed_runtime_snapshot(&game_dir, game_id, "Poll", 4, &[], &[]);
    set_next_due(
        &store,
        game_id,
        true,
        Some(chrono::Utc::now().timestamp() - 5),
    );
    let scheduler = Scheduler::new(temp.path().to_path_buf());
    let registry = WorkerRegistry::new(temp.path().to_path_buf());

    dispatch_due_maintenance(&scheduler, &registry).await;

    let deadline = tokio::time::Instant::now() + Duration::from_secs(30);
    while runtime_year(&game_dir) == 3000 {
        assert!(
            tokio::time::Instant::now() < deadline,
            "the worker should have run the due year"
        );
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    assert_eq!(runtime_year(&game_dir), 3001);
    assert_eq!(registry.list_games().await, vec![game_id.to_string()]);
}
