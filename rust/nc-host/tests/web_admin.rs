use nc_data::CampaignStore;
use nc_host::admin::{Request, execute};
use serde_json::Value;

fn create(root: &std::path::Path, game: &str) -> Value {
    execute(
        root,
        Request::Create {
            game: game.into(),
            name: game.into(),
            players: 4,
            seed: Some(1515),
        },
    )
    .unwrap()
}
fn publish(root: &std::path::Path, game: &str) {
    execute(root, Request::Publish { game: game.into() }).unwrap();
}
fn start(root: &std::path::Path, game: &str) {
    execute(
        root,
        Request::Start {
            game: game.into(),
            first_due: chrono::Utc::now().timestamp() + 3600,
            interval_minutes: 1440,
        },
    )
    .unwrap();
}
#[test]
fn independent_games_atomic_seats_and_lifecycle() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    assert_eq!(create(root, "one")["state"], "draft");
    create(root, "two");
    assert!(
        execute(
            root,
            Request::Join {
                game: "one".into(),
                account: nc_data::AccountIdentity::Discord("123".into()),
                display_name: None,
            }
        )
        .is_err()
    );
    publish(root, "one");
    publish(root, "two");
    for game in ["one", "two"] {
        let join = Request::Join {
            game: game.into(),
            account: nc_data::AccountIdentity::Discord("123".into()),
            display_name: None,
        };
        assert_eq!(execute(root, join.clone()).unwrap()["seat"], 1);
        assert_eq!(execute(root, join).unwrap()["seat"], 1);
    }
    assert!(
        execute(
            root,
            Request::Authorize {
                game: "one".into(),
                account: nc_data::AccountIdentity::Discord("456".into())
            }
        )
        .is_err()
    );
    assert!(
        execute(
            root,
            Request::Inspect {
                game: "../one".into()
            }
        )
        .is_err()
    );
    assert!(
        execute(
            root,
            Request::Create {
                game: "one".into(),
                name: "overwrite".into(),
                players: 4,
                seed: None
            }
        )
        .is_err()
    );
    start(root, "one");
    assert_eq!(
        execute(root, Request::Inspect { game: "two".into() }).unwrap()["state"],
        "recruiting"
    );
    execute(root, Request::Pause { game: "one".into() }).unwrap();
    let store = CampaignStore::open_default_in_dir(&root.join("one")).unwrap();
    let state = store.load_latest_runtime_state().unwrap().unwrap();
    assert!(
        store
            .expecting_snapshot(state.snapshot_id)
            .save_runtime_state_structured(state.game_year, &state.game_data, &[], &[])
            .is_err()
    );
}
#[test]
fn maintenance_retries_restore_and_integrity() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    create(root, "one");
    publish(root, "one");
    start(root, "one");
    let request = Request::Maintain {
        game: "one".into(),
        expected_year: 3000,
        request_id: "first-turn".into(),
    };
    assert_eq!(execute(root, request.clone()).unwrap()["end_year"], 3001);
    assert_eq!(execute(root, request).unwrap()["replayed"], true);
    assert!(
        execute(
            root,
            Request::Maintain {
                game: "one".into(),
                expected_year: 3000,
                request_id: "different".into()
            }
        )
        .is_err()
    );
    let copy = execute(root, Request::Backup { game: "one".into() }).unwrap();
    let path = copy["path"].as_str().unwrap().to_owned();
    assert_eq!(
        execute(root, Request::VerifyBackup { path: path.clone() }).unwrap()["year"],
        3001
    );
    let restore_root = root.join("restore");
    let restored = execute(
        &restore_root,
        Request::Restore {
            path: path.clone(),
            game: "one".into(),
        },
    )
    .unwrap();
    assert_eq!(restored["year"], 3001);
    assert_eq!(restored["state"], "paused");
    assert!(
        execute(
            &restore_root,
            Request::Restore {
                path: path.clone(),
                game: "one".into()
            }
        )
        .is_err()
    );
    std::fs::write(std::path::Path::new(&path).join("ncgame.db"), "corrupt").unwrap();
    assert!(execute(root, Request::VerifyBackup { path }).is_err());
}

#[test]
fn announcements_follow_committed_turns_and_survive_retries() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    create(root, "one");
    publish(root, "one");
    start(root, "one");
    assert_eq!(
        execute(root, Request::Announcements)
            .unwrap()
            .as_array()
            .unwrap()
            .len(),
        1
    );
    let request = Request::Maintain {
        game: "one".into(),
        expected_year: 3000,
        request_id: "turn-1".into(),
    };
    execute(root, request.clone()).unwrap();
    execute(root, request).unwrap();
    let events = execute(root, Request::Announcements).unwrap();
    assert_eq!(events.as_array().unwrap().len(), 2);
    assert_eq!(events[1]["payload"]["year"], 3001);
    let ack = Request::AcknowledgeAnnouncement {
        game: "one".into(),
        id: "turn-1".into(),
    };
    execute(root, ack.clone()).unwrap();
    execute(root, ack).unwrap();
    assert_eq!(
        execute(root, Request::Announcements)
            .unwrap()
            .as_array()
            .unwrap()
            .len(),
        1
    );
}

#[test]
fn concurrent_claims_cannot_share_a_seat() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    create(root, "one");
    publish(root, "one");
    let barrier = std::sync::Arc::new(std::sync::Barrier::new(8));
    let handles = (0..8)
        .map(|i| {
            let root = root.to_path_buf();
            let barrier = barrier.clone();
            std::thread::spawn(move || {
                barrier.wait();
                execute(
                    &root,
                    Request::Join {
                        game: "one".into(),
                        account: nc_data::AccountIdentity::Discord(format!("100{i}")),
                        display_name: None,
                    },
                )
            })
        })
        .collect::<Vec<_>>();
    let mut seats = handles
        .into_iter()
        .filter_map(|h| h.join().unwrap().ok())
        .map(|v| v["seat"].as_u64().unwrap())
        .collect::<Vec<_>>();
    seats.sort();
    assert_eq!(seats, vec![1, 2, 3, 4]);
}

#[test]
fn committed_turn_recovery_finishes_metadata_without_rerunning_engine() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    create(root, "one");
    publish(root, "one");
    start(root, "one");
    let store = CampaignStore::open_default_in_dir(&root.join("one")).unwrap();
    store.set_hosting_write_blocked(true).unwrap();
    let metadata = rusqlite::Connection::open(root.join("one/hosted.db")).unwrap();
    metadata.execute_batch("UPDATE game_metadata SET status='maintenance'; INSERT INTO web_runs(request_id,expected_year,state,started_at) VALUES ('crashed',3000,'running',0);").unwrap();
    let mut turn = nc_engine::CampaignMaintenance::load(&store).unwrap();
    turn.run_turn().unwrap();
    turn.save(&store.recording_maintenance("crashed", 3000).unwrap())
        .unwrap();
    let before = store.load_latest_runtime_state().unwrap().unwrap();
    let result = execute(
        root,
        Request::Maintain {
            game: "one".into(),
            expected_year: 3000,
            request_id: "crashed".into(),
        },
    )
    .unwrap();
    assert_eq!(result["replayed"], true);
    assert_eq!(store.load_latest_runtime_state().unwrap().unwrap(), before);
    assert!(!store.hosting_writes_blocked().unwrap());
    assert_eq!(
        execute(root, Request::Inspect { game: "one".into() }).unwrap()["state"],
        "running"
    );
}

#[test]
fn simultaneous_snapshot_writers_get_one_winner_and_one_conflict() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    create(root, "one");
    publish(root, "one");
    let store = CampaignStore::open_default_in_dir(&root.join("one")).unwrap();
    let before = store.load_latest_runtime_state().unwrap().unwrap();
    let barrier = std::sync::Arc::new(std::sync::Barrier::new(2));
    let handles = (0..2)
        .map(|player| {
            let store = store.clone();
            let mut state = before.clone();
            let barrier = barrier.clone();
            std::thread::spawn(move || {
                state.game_data.players[player].autopilot =
                    !state.game_data.players[player].autopilot;
                barrier.wait();
                store
                    .expecting_snapshot(state.snapshot_id)
                    .save_runtime_state_structured(
                        state.game_year,
                        &state.game_data,
                        &state.report_block_rows,
                        &state.queued_mail,
                    )
            })
        })
        .collect::<Vec<_>>();
    let results = handles
        .into_iter()
        .map(|h| h.join().unwrap())
        .collect::<Vec<_>>();
    assert_eq!(results.iter().filter(|r| r.is_ok()).count(), 1);
    assert_eq!(
        results
            .iter()
            .filter(|r| matches!(
                r,
                Err(nc_data::CampaignStoreError::ConcurrentSnapshot { .. })
            ))
            .count(),
        1
    );
    let after = store.load_latest_runtime_state().unwrap().unwrap();
    assert_eq!(
        (0..2)
            .filter(
                |i| after.game_data.players[*i].autopilot != before.game_data.players[*i].autopilot
            )
            .count(),
        1
    );
}

#[test]
fn daily_backup_is_once_per_utc_date_and_keeps_manual_copies() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    create(root, "one");
    let manual = execute(root, Request::Backup { game: "one".into() }).unwrap();
    let first = execute(root, Request::DailyBackups).unwrap();
    assert_eq!(first.as_array().unwrap().len(), 1);
    assert_eq!(first[0]["manifest"]["kind"], "daily");
    assert_eq!(
        execute(root, Request::DailyBackups)
            .unwrap()
            .as_array()
            .unwrap()
            .len(),
        0
    );
    assert!(std::path::Path::new(manual["path"].as_str().unwrap()).exists());
    assert!(
        CampaignStore::open_default_in_dir(&root.join("one"))
            .unwrap()
            .hosting_writes_blocked()
            .unwrap()
    );
}

#[test]
fn maintenance_cannot_overwrite_a_newer_player_snapshot() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    create(root, "one");
    publish(root, "one");
    let store = CampaignStore::open_default_in_dir(&root.join("one")).unwrap();
    let mut turn = nc_engine::CampaignMaintenance::load(&store).unwrap();
    turn.run_turn().unwrap();
    let mut player = store.load_latest_runtime_state().unwrap().unwrap();
    player.game_data.players[0].autopilot = !player.game_data.players[0].autopilot;
    store
        .expecting_snapshot(player.snapshot_id)
        .save_runtime_state_structured(
            player.game_year,
            &player.game_data,
            &player.report_block_rows,
            &player.queued_mail,
        )
        .unwrap();
    let before = store.load_latest_runtime_state().unwrap().unwrap();
    assert!(turn.save(&store).is_err());
    assert_eq!(store.load_latest_runtime_state().unwrap().unwrap(), before);
}

#[test]
fn daily_retention_keeps_thirty_copies_without_pruning_manual_backup() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    create(root, "one");
    let manual = execute(root, Request::Backup { game: "one".into() }).unwrap();
    let conn = rusqlite::Connection::open(root.join("one/hosted.db")).unwrap();
    for n in 0..31 {
        let path = root.join(".backups").join(format!("old-{n}"));
        std::fs::create_dir(&path).unwrap();
        conn.execute(
            "INSERT INTO web_backups(path,at,kind) VALUES (?1,?2,'daily')",
            rusqlite::params![path.to_string_lossy(), n],
        )
        .unwrap();
    }
    execute(root, Request::DailyBackups).unwrap();
    let count: u32 = conn
        .query_row(
            "SELECT COUNT(*) FROM web_backups WHERE kind='daily'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(count, 30);
    assert!(!root.join(".backups/old-0").exists());
    assert!(!root.join(".backups/old-1").exists());
    assert!(std::path::Path::new(manual["path"].as_str().unwrap()).exists());
}
