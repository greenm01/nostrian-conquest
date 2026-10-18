//! Players of one campaign change it at the same time (t204): each change
//! runs on the latest state inside one transaction, so none is refused or
//! lost, and records two players create at once never share a slot.

use std::collections::BTreeSet;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use nc_data::{
    CampaignStore, CampaignStoreError, CoreGameData, FleetDetachSelection, QueuedPlayerMail,
    ReportBlockRow, RuntimeChange, delete_report_block_for_viewer,
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

fn home_index(game_data: &CoreGameData, empire: u8) -> usize {
    game_data
        .planets
        .iter()
        .position(|planet| planet.owner.0 == empire)
        .expect("every empire owns a world")
}

/// A year-3000 four-player game whose first two empires each have two
/// destroyers and a starbase waiting in their home stardock, and a broadcast
/// report and one report for empire 2.
fn campaign() -> TempStore {
    let dir = std::env::temp_dir().join(format!(
        "nc-data-concurrent-{}-{}",
        std::process::id(),
        TEMP_DIR_SEQ.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let store = CampaignStore::open_default_in_dir(&dir).expect("open store");
    let mut data = build_seeded_initialized_game(4, 3000, 1515).expect("seeded game");
    for empire in [1, 2] {
        let home = home_index(&data, empire);
        data.planets[home].stardock.destroyers = 2;
        data.planets[home].stardock.starbases = 1;
    }
    let report = |viewer_empire_id, block_index| ReportBlockRow {
        viewer_empire_id,
        block_index,
        decoded_text: format!("report {block_index}"),
        raw_bytes: None,
        recipient_deleted: false,
    };
    store
        .save_runtime_state_structured(
            data.conquest.game_year,
            &data,
            &[report(0, 0), report(2, 1)],
            &[],
        )
        .expect("seed store");
    TempStore { dir, store }
}

fn mail(sender_empire_id: u8, recipient_empire_id: u8) -> QueuedPlayerMail {
    QueuedPlayerMail {
        sender_empire_id,
        recipient_empire_id,
        year: 3000,
        subject: format!("from {sender_empire_id}"),
        body: "hello".to_string(),
        recipient_deleted: false,
        sender_deleted: false,
        cancelled: false,
    }
}

/// Fleet IDs are distinct, each empire's fleet chain reaches once every
/// fleet created since `start`, and its base chain is whole. (A seeded game
/// leaves `fleets_first` at 0, so its starting fleets are on no chain.)
fn assert_chains_whole(game_data: &CoreGameData, start: &CoreGameData) {
    let ids: BTreeSet<_> = game_data
        .fleets
        .iter()
        .map(|fleet| fleet.record.0)
        .collect();
    assert_eq!(ids.len(), game_data.fleets.len(), "fleet IDs are distinct");
    for (idx, player) in game_data.players.iter().enumerate() {
        let empire = idx as u8 + 1;
        let mut reached = BTreeSet::new();
        let mut id = player.fleets_first.0;
        while id != 0 {
            let fleet = game_data
                .fleets
                .iter()
                .find(|fleet| fleet.owner.0 == empire && fleet.record.0 == id)
                .unwrap_or_else(|| panic!("empire {empire}'s chain names missing fleet {id}"));
            assert!(reached.insert(id), "empire {empire}'s chain loops at {id}");
            id = fleet.next.0;
        }
        let owned = |game_data: &CoreGameData| -> BTreeSet<_> {
            game_data
                .fleets
                .iter()
                .filter(|fleet| fleet.owner.0 == empire)
                .map(|fleet| fleet.record.0)
                .collect()
        };
        let created: BTreeSet<_> = owned(game_data)
            .difference(&owned(start))
            .copied()
            .collect();
        assert_eq!(reached, created, "empire {empire}'s fleet chain");
    }
    assert_eq!(
        game_data.ecmaint_structural_preflight_errors(),
        Vec::<String>::new()
    );
    assert_eq!(game_data.unchained_base_errors(), Vec::<String>::new());
}

fn change(
    store: &CampaignStore,
    apply: impl FnOnce(&mut RuntimeChange) -> Result<(), Box<dyn std::error::Error>>,
) -> RuntimeChange {
    store
        .change(3000, apply)
        .expect("the change commits")
        .1
        .state
}

#[test]
fn two_players_interleaving_changes_keep_every_change() {
    let temp = campaign();
    // Two sessions, as two players' terminals open the same campaign file.
    let first = CampaignStore::open(temp.store.path()).expect("first session");
    let second = CampaignStore::open(temp.store.path()).expect("second session");
    let start = temp
        .store
        .load_latest_runtime_state()
        .expect("load")
        .expect("a snapshot")
        .game_data;
    let fleets = |game_data: &CoreGameData, empire: u8| {
        game_data
            .fleets
            .iter()
            .filter(|fleet| fleet.owner.0 == empire)
            .count()
    };

    change(&first, |state| {
        state.game_data.auto_commission_all_stardock_units(1)?;
        Ok(())
    });
    change(&second, |state| {
        state.game_data.auto_commission_all_stardock_units(2)?;
        Ok(())
    });
    change(&first, |state| {
        let donor = state
            .game_data
            .fleets
            .iter()
            .rposition(|fleet| fleet.owner.0 == 1 && fleet.ships.destroyers == 2)
            .expect("the commissioned fleet");
        state.game_data.detach_ships_to_new_fleet(
            1,
            donor + 1,
            FleetDetachSelection {
                destroyers: 1,
                ..FleetDetachSelection::default()
            },
            None,
            0,
        )?;
        Ok(())
    });
    change(&second, |state| {
        state.queued_mail.push(mail(2, 1));
        let players = state.game_data.players.clone();
        let own = state
            .report_block_rows
            .iter()
            .position(|row| row.viewer_empire_id == 2)
            .expect("empire 2's report");
        delete_report_block_for_viewer(&mut state.report_block_rows, &players, own, 2);
        Ok(())
    });
    let last = change(&first, |state| {
        state.queued_mail.push(mail(1, 2));
        Ok(())
    });

    let stored = temp
        .store
        .load_latest_runtime_state()
        .expect("load")
        .expect("a snapshot");
    assert_eq!(stored.game_data, last.game_data);
    let game_data = &stored.game_data;
    assert_eq!(
        fleets(game_data, 1),
        fleets(&start, 1) + 2,
        "empire 1 commissioned and split"
    );
    assert_eq!(
        fleets(game_data, 2),
        fleets(&start, 2) + 1,
        "empire 2 commissioned"
    );
    for empire in [1, 2] {
        let home = home_index(game_data, empire);
        assert_eq!(game_data.planets[home].stardock.destroyers, 0);
        assert_eq!(
            game_data
                .bases
                .iter()
                .filter(|base| base.owner.0 == empire)
                .count(),
            1,
            "empire {empire}'s starbase"
        );
    }
    assert_chains_whole(game_data, &start);
    assert_eq!(
        stored
            .queued_mail
            .iter()
            .map(|m| (m.sender_empire_id, m.recipient_empire_id))
            .collect::<Vec<_>>(),
        [(2, 1), (1, 2)]
    );
    assert!(
        stored
            .report_block_rows
            .iter()
            .any(|row| row.viewer_empire_id == 2 && row.recipient_deleted)
    );
}

#[test]
fn a_failed_change_writes_nothing() {
    let temp = campaign();
    let before = temp.store.load_latest_runtime_state().expect("load");
    let failed: Result<((), _), CampaignStoreError> = temp.store.change(3000, |state| {
        state.queued_mail.push(mail(1, 2));
        Err(CampaignStoreError::InvalidState("refused".into()))
    });
    assert!(failed.is_err());
    assert_eq!(
        temp.store.load_latest_runtime_state().expect("load"),
        before
    );
}

#[test]
fn a_change_from_a_year_maintenance_has_closed_is_refused() {
    let temp = campaign();
    let mut state = temp
        .store
        .load_latest_runtime_state()
        .expect("load")
        .expect("a snapshot");
    nc_engine::run_maintenance_turn(&mut state.game_data).expect("maintenance should run");
    temp.store
        .save_runtime_state_structured(
            3000,
            &state.game_data,
            &state.report_block_rows,
            &state.queued_mail,
        )
        .expect("maintenance saves year 3001");
    let refused: Result<((), _), CampaignStoreError> = temp.store.change(3000, |_| Ok(()));
    assert!(matches!(
        refused,
        Err(CampaignStoreError::StaleSnapshot {
            loaded_year: 3000,
            latest_year: 3001
        })
    ));
}

/// A saved campaign is in WAL mode, and replacing a year's snapshot removes
/// the replaced snapshot's rows, so a campaign does not grow with every
/// change.
#[test]
fn changes_use_wal_and_leave_no_rows_of_replaced_snapshots() {
    let temp = campaign();
    for _ in 0..3 {
        change(&temp.store, |_| Ok(()));
    }
    let conn = rusqlite::Connection::open(temp.store.path()).expect("open");
    let mode: String = conn
        .query_row("PRAGMA journal_mode", [], |row| row.get(0))
        .expect("journal mode");
    assert_eq!(mode, "wal", "a saved campaign is in WAL mode");
    let orphans: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM snapshot_fleets
             WHERE snapshot_id NOT IN (SELECT id FROM snapshots)",
            [],
            |row| row.get(0),
        )
        .expect("count");
    assert_eq!(orphans, 0);
}

/// Players' changes arriving at the same instant wait for one another
/// instead of failing on the database lock.
#[test]
fn simultaneous_changes_from_many_sessions_all_commit() {
    let temp = campaign();
    let path = temp.store.path().to_path_buf();
    let threads: Vec<_> = (1..=4u8)
        .map(|empire| {
            let path = path.clone();
            std::thread::spawn(move || {
                let store = CampaignStore::open(&path).expect("session");
                for _ in 0..10 {
                    change(&store, |state| {
                        state.queued_mail.push(mail(empire, empire % 4 + 1));
                        Ok(())
                    });
                }
            })
        })
        .collect();
    for thread in threads {
        thread.join().expect("every session's changes commit");
    }
    let stored = temp
        .store
        .load_latest_runtime_state()
        .expect("load")
        .expect("a snapshot");
    for empire in 1..=4u8 {
        assert_eq!(
            stored
                .queued_mail
                .iter()
                .filter(|m| m.sender_empire_id == empire)
                .count(),
            10,
            "empire {empire}'s messages"
        );
    }
}

/// A small xorshift, so each session's random play is repeatable.
struct Rng(u64);

impl Rng {
    fn below(&mut self, n: usize) -> usize {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        (self.0 % n as u64) as usize
    }
}

/// Four sessions play at once, each giving its own empire random orders:
/// commissioning ships, splitting fleets, setting rules of engagement and
/// sending mail. Every change commits, every message is kept, the fleet
/// and base chains stay whole, and maintenance then runs.
#[test]
fn random_play_from_many_sessions_loses_nothing() {
    let temp = campaign();
    let mut stocked = temp
        .store
        .load_latest_runtime_state()
        .expect("load")
        .expect("a snapshot");
    for empire in 1..=4 {
        let home = home_index(&stocked.game_data, empire);
        stocked.game_data.planets[home].stardock.destroyers = 40;
        stocked.game_data.planets[home].stardock.starbases = 2;
    }
    temp.store
        .save_runtime_state_structured(
            3000,
            &stocked.game_data,
            &stocked.report_block_rows,
            &stocked.queued_mail,
        )
        .expect("stock the stardocks");
    let start = stocked.game_data;
    let path = temp.store.path().to_path_buf();
    let threads: Vec<_> = (1..=4u8)
        .map(|empire| {
            let path = path.clone();
            std::thread::spawn(move || {
                let store = CampaignStore::open(&path).expect("session");
                let mut rng = Rng(0x9e37_79b9_7f4a_7c15 ^ u64::from(empire));
                let mut sent = 0;
                for _ in 0..40 {
                    let pick = rng.below(4);
                    let roe = rng.below(11) as u8;
                    let at = rng.below(1000);
                    change(&store, |state| {
                        let game_data = &mut state.game_data;
                        let own: Vec<usize> = game_data
                            .fleets
                            .iter()
                            .enumerate()
                            .filter(|(_, fleet)| fleet.owner.0 == empire)
                            .map(|(idx, _)| idx + 1)
                            .collect();
                        match pick {
                            0 => {
                                game_data
                                    .auto_commission_all_stardock_units(usize::from(empire))?;
                            }
                            1 => {
                                let Some(&donor) = own
                                    .iter()
                                    .filter(|&&record| {
                                        game_data.fleets[record - 1].ships.destroyers > 1
                                    })
                                    .nth(at % own.len().max(1))
                                else {
                                    return Ok(());
                                };
                                game_data.detach_ships_to_new_fleet(
                                    usize::from(empire),
                                    donor,
                                    FleetDetachSelection {
                                        destroyers: 1,
                                        ..FleetDetachSelection::default()
                                    },
                                    None,
                                    0,
                                )?;
                            }
                            2 => {
                                if let Some(&record) = own.get(at % own.len().max(1)) {
                                    let _ = game_data.set_fleet_rules_of_engagement(
                                        usize::from(empire),
                                        record,
                                        roe,
                                    );
                                }
                            }
                            _ => state.queued_mail.push(mail(empire, empire % 4 + 1)),
                        }
                        Ok(())
                    });
                    if pick == 3 {
                        sent += 1;
                    }
                }
                sent
            })
        })
        .collect();
    let sent: Vec<usize> = threads
        .into_iter()
        .map(|thread| thread.join().expect("every session's changes commit"))
        .collect();

    let stored = temp
        .store
        .load_latest_runtime_state()
        .expect("load")
        .expect("a snapshot");
    for (idx, &count) in sent.iter().enumerate() {
        let empire = idx as u8 + 1;
        assert_eq!(
            stored
                .queued_mail
                .iter()
                .filter(|m| m.sender_empire_id == empire)
                .count(),
            count,
            "empire {empire}'s messages"
        );
    }
    assert_chains_whole(&stored.game_data, &start);
    let mut after = stored.game_data.clone();
    nc_engine::run_maintenance_turn(&mut after).expect("maintenance runs on the result");
}

/// Maintenance runs while players keep changing the campaign, as the host
/// runs it: player writes are blocked, the year is run and saved with its
/// receipt, and writes resume. A change made while writes are blocked is
/// refused and retried; one from the closed year is refused. Every change
/// committed before the year closed is in the new year.
#[test]
fn maintenance_amid_changes_loses_no_committed_change() {
    let temp = campaign();
    let path = temp.store.path().to_path_buf();
    let committing = std::sync::Arc::new(AtomicU64::new(0));
    let writers: Vec<_> = (1..=4u8)
        .map(|empire| {
            let path = path.clone();
            let committing = committing.clone();
            std::thread::spawn(move || {
                let store = CampaignStore::open(&path).expect("session");
                let mut last = None;
                // Change until maintenance closes the year, so the year is
                // always closed while every session is still changing it.
                for k in 0..100_000 {
                    let handle = format!("p{empire}-{k}");
                    let changed: Result<((), _), CampaignStoreError> =
                        store.change(3000, |state| {
                            nc_data::set_player_handle(
                                &mut state.game_data.players[usize::from(empire) - 1],
                                &handle,
                            );
                            Ok(())
                        });
                    match changed {
                        Ok(_) => {
                            if last.replace(handle).is_none() {
                                committing.fetch_add(1, Ordering::SeqCst);
                            }
                        }
                        Err(CampaignStoreError::HostingPaused) => {
                            std::thread::sleep(std::time::Duration::from_millis(5));
                        }
                        Err(CampaignStoreError::StaleSnapshot { .. }) => return last,
                        Err(err) => panic!("empire {empire}'s change failed: {err}"),
                    }
                }
                panic!("empire {empire}'s session never saw maintenance close the year")
            })
        })
        .collect();
    // Close the year once every session has committed a change.
    while committing.load(Ordering::SeqCst) < 4 {
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    let store = CampaignStore::open(&path).expect("maintenance store");
    store.set_hosting_write_blocked(true).expect("block writes");
    let mut run = nc_engine::CampaignMaintenance::load(&store).expect("load the year");
    run.run_turn().expect("run the year");
    run.save(
        &store
            .recording_maintenance("t204-stress", 3000)
            .expect("receipt"),
    )
    .expect("maintenance saves the new year");
    store
        .set_hosting_write_blocked(false)
        .expect("resume writes");
    let last: Vec<Option<String>> = writers
        .into_iter()
        .map(|writer| writer.join().expect("writer"))
        .collect();

    let stored = temp
        .store
        .load_latest_runtime_state()
        .expect("load")
        .expect("a snapshot");
    assert_eq!(stored.game_year, 3001);
    for (idx, last) in last.iter().enumerate() {
        let handle = last.as_deref().expect("every session committed a change");
        assert!(
            nc_data::player_handle_matches(&stored.game_data.players[idx], handle),
            "empire {}'s last committed change survives maintenance",
            idx + 1
        );
    }
}
