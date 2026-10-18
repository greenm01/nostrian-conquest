//! A change writes only the rows it alters (t204, phase 2), and leaves the
//! store exactly as rewriting the whole year would; maintenance still sees a
//! change made during its run.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use nc_data::{
    CampaignStore, CampaignStoreError, CommittedChange, FleetDetachSelection, QueuedPlayerMail,
    ReportBlockRow, RuntimeChange, delete_report_block_for_viewer,
};
use nc_engine::build_seeded_initialized_game;

static TEMP_DIR_SEQ: AtomicU64 = AtomicU64::new(0);

struct TempDir(PathBuf);

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn temp_dir() -> TempDir {
    let dir = std::env::temp_dir().join(format!(
        "nc-data-row-changes-{}-{}",
        std::process::id(),
        TEMP_DIR_SEQ.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::create_dir_all(&dir).expect("create temp dir");
    TempDir(dir)
}

/// A year-3000 four-player game with stocked stardocks, a broadcast report,
/// one report per empire and some mail.
fn campaign(dir: &Path) -> CampaignStore {
    let store = CampaignStore::open_default_in_dir(dir).expect("open store");
    let mut data = build_seeded_initialized_game(4, 3000, 1515).expect("seeded game");
    for empire in 1..=4 {
        let home = data
            .planets
            .iter()
            .position(|planet| planet.owner.0 == empire)
            .expect("home world");
        data.planets[home].stardock.destroyers = 30;
        data.planets[home].stardock.starbases = 2;
    }
    let reports: Vec<ReportBlockRow> = (0..=4)
        .map(|viewer| ReportBlockRow {
            viewer_empire_id: viewer,
            block_index: usize::from(viewer),
            decoded_text: format!("report for {viewer}"),
            raw_bytes: None,
            recipient_deleted: false,
        })
        .collect();
    store
        .save_runtime_state_structured(3000, &data, &reports, &[mail(1, 2), mail(3, 4)])
        .expect("seed store");
    store
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

/// A copy of `store`'s campaign in `dir`, taken through SQLite so the WAL is
/// included.
fn copy(store: &CampaignStore, dir: &Path) -> CampaignStore {
    let target = dir.join(nc_data::DEFAULT_CAMPAIGN_DB_NAME);
    rusqlite::Connection::open(store.path())
        .expect("open source")
        .execute("VACUUM INTO ?1", [target.to_str().expect("utf-8 path")])
        .expect("copy campaign");
    CampaignStore::open(target).expect("open copy")
}

/// One random player change, as plain data, so the same change can run
/// against two stores.
#[derive(Clone, Copy, Debug)]
enum Step {
    Commission(u8),
    Detach(u8, usize),
    Roe(u8, usize, u8),
    Tax(u8, u8),
    Mail(u8, u8),
    DeleteReport(u8),
    CancelMail(usize),
    Handle(u8, usize),
    Participate(u8),
}

fn apply(step: Step, state: &mut RuntimeChange) -> Result<(), Box<dyn std::error::Error>> {
    let own = |state: &RuntimeChange, empire: u8| -> Vec<usize> {
        state
            .game_data
            .fleets
            .iter()
            .enumerate()
            .filter(|(_, fleet)| fleet.owner.0 == empire)
            .map(|(idx, _)| idx + 1)
            .collect()
    };
    match step {
        Step::Commission(empire) => {
            state
                .game_data
                .auto_commission_all_stardock_units(usize::from(empire))?;
        }
        Step::Detach(empire, at) => {
            let donors: Vec<usize> = own(state, empire)
                .into_iter()
                .filter(|&record| state.game_data.fleets[record - 1].ships.destroyers > 1)
                .collect();
            if let Some(&donor) = donors.get(at % donors.len().max(1)) {
                state.game_data.detach_ships_to_new_fleet(
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
        }
        Step::Roe(empire, at, roe) => {
            let fleets = own(state, empire);
            if let Some(&record) = fleets.get(at % fleets.len().max(1)) {
                let _ =
                    state
                        .game_data
                        .set_fleet_rules_of_engagement(usize::from(empire), record, roe);
            }
        }
        Step::Tax(empire, rate) => {
            state
                .game_data
                .set_player_tax_rate(usize::from(empire), rate)?;
        }
        Step::Mail(sender, recipient) => state.queued_mail.push(mail(sender, recipient)),
        Step::DeleteReport(viewer) => {
            let players = state.game_data.players.clone();
            if let Some(idx) = state
                .report_block_rows
                .iter()
                .position(|row| row.is_visible_to_viewer(viewer) && !row.recipient_deleted)
            {
                delete_report_block_for_viewer(&mut state.report_block_rows, &players, idx, viewer);
            }
        }
        Step::CancelMail(at) => {
            let count = state.queued_mail.len();
            if count > 0 {
                state.queued_mail[at % count].cancel();
            }
        }
        Step::Handle(empire, k) => nc_data::set_player_handle(
            &mut state.game_data.players[usize::from(empire) - 1],
            &format!("h{empire}-{k}"),
        ),
        Step::Participate(empire) => nc_data::record_interactive_participation(
            &mut state.game_data,
            usize::from(empire),
            &mut state.player_activity_states,
        ),
    }
    Ok(())
}

struct Rng(u64);

impl Rng {
    fn below(&mut self, n: usize) -> usize {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        (self.0 % n as u64) as usize
    }

    fn step(&mut self) -> Step {
        let empire = self.below(4) as u8 + 1;
        let at = self.below(1000);
        match self.below(9) {
            0 => Step::Commission(empire),
            1 => Step::Detach(empire, at),
            2 => Step::Roe(empire, at, self.below(11) as u8),
            3 => Step::Tax(empire, self.below(101) as u8),
            4 => Step::Mail(empire, empire % 4 + 1),
            5 => Step::DeleteReport(empire),
            6 => Step::CancelMail(at),
            7 => Step::Handle(empire, at),
            _ => Step::Participate(empire),
        }
    }
}

/// Every stored row, table by table, with snapshot IDs left out (a rewrite
/// mints new ones) and the bookkeeping counters dropped.
fn dump(store: &CampaignStore) -> Vec<(String, Vec<String>)> {
    let conn = rusqlite::Connection::open(store.path()).expect("open");
    let mut tables: Vec<String> = conn
        .prepare("SELECT name FROM sqlite_master WHERE type = 'table' AND name != 'snapshots'")
        .expect("list tables")
        .query_map([], |row| row.get(0))
        .expect("query tables")
        .collect::<Result<_, _>>()
        .expect("table names");
    tables.sort();
    tables
        .into_iter()
        .map(|table| {
            let mut stmt = conn
                .prepare(&format!("SELECT * FROM {table}"))
                .expect("select");
            let columns: Vec<String> = stmt.column_names().iter().map(|c| c.to_string()).collect();
            let mut rows: Vec<String> = stmt
                .query_map([], |row| {
                    Ok(columns
                        .iter()
                        .enumerate()
                        .filter(|(_, name)| name.as_str() != "snapshot_id")
                        .map(|(idx, name)| {
                            let value = match row.get::<_, rusqlite::types::Value>(idx)? {
                                rusqlite::types::Value::Text(text) => text,
                                other => format!("{other:?}"),
                            };
                            Ok(format!("{name}={value}"))
                        })
                        .collect::<Result<Vec<_>, rusqlite::Error>>()?
                        .join(","))
                })
                .expect("rows")
                .collect::<Result<_, _>>()
                .expect("row values");
            if table == "campaign_metadata" {
                rows.retain(|row| {
                    !row.starts_with("key=revision,") && !row.starts_with("key=snapshot_sequence,")
                });
            }
            rows.sort();
            (table, rows)
        })
        .collect()
}

/// The first table whose rows differ, with the rows only one side holds.
fn assert_same_rows(rows: &CampaignStore, year: &CampaignStore, context: &str) {
    for ((table, by_rows), (_, by_year)) in dump(rows).into_iter().zip(dump(year)) {
        let only_rows: Vec<_> = by_rows
            .iter()
            .filter(|row| !by_year.contains(row))
            .collect();
        let only_year: Vec<_> = by_year
            .iter()
            .filter(|row| !by_rows.contains(row))
            .collect();
        assert!(
            only_rows.is_empty() && only_year.is_empty() && by_rows.len() == by_year.len(),
            "{context}: table {table} differs\nonly written by rows: {only_rows:#?}\nonly written by the year rewrite: {only_year:#?}"
        );
    }
}

#[test]
fn row_changes_store_exactly_what_rewriting_the_year_stores() {
    for seed in 1..=4u64 {
        let (rows_dir, year_dir) = (temp_dir(), temp_dir());
        let by_rows = campaign(&rows_dir.0);
        let by_year = copy(&by_rows, &year_dir.0);
        assert_same_rows(&by_rows, &by_year, "the copy");
        let mut rng = Rng(0x2545_f491_4f6c_dd1d ^ seed);
        for n in 0..60 {
            let step = rng.step();
            let rows: Result<((), CommittedChange), Box<dyn std::error::Error>> =
                by_rows.change(3000, |state| apply(step, state));
            let year: Result<((), CommittedChange), Box<dyn std::error::Error>> =
                by_year.change_rewriting_the_year(3000, |state| apply(step, state));
            assert_eq!(rows.is_ok(), year.is_ok(), "seed {seed} step {n} {step:?}");
            if n % 10 == 9 {
                assert_same_rows(
                    &by_rows,
                    &by_year,
                    &format!("seed {seed} after step {n} {step:?}"),
                );
            }
        }
        assert_same_rows(&by_rows, &by_year, &format!("seed {seed} at the end"));
    }
}

/// Count every row written, through triggers on each table.
fn count_writes(store: &CampaignStore) {
    let conn = rusqlite::Connection::open(store.path()).expect("open");
    conn.execute_batch("CREATE TABLE IF NOT EXISTS t204_writes(t TEXT)")
        .expect("counter");
    let tables: Vec<String> = conn
        .prepare("SELECT name FROM sqlite_master WHERE type = 'table' AND name != 't204_writes'")
        .expect("list tables")
        .query_map([], |row| row.get(0))
        .expect("query tables")
        .collect::<Result<_, _>>()
        .expect("table names");
    for table in tables {
        for event in ["INSERT", "UPDATE", "DELETE"] {
            conn.execute_batch(&format!(
                "CREATE TRIGGER t204_{table}_{event} AFTER {event} ON {table}
                 BEGIN INSERT INTO t204_writes VALUES ('{table}'); END;"
            ))
            .expect("trigger");
        }
    }
}

/// The rows written since the last call, in all and by table.
fn writes(store: &CampaignStore) -> (i64, Vec<(String, i64)>) {
    let conn = rusqlite::Connection::open(store.path()).expect("open");
    let n = conn
        .query_row("SELECT COUNT(*) FROM t204_writes", [], |row| row.get(0))
        .expect("count");
    let by_table: Vec<(String, i64)> = conn
        .prepare("SELECT t, COUNT(*) FROM t204_writes GROUP BY t ORDER BY 2 DESC")
        .expect("group")
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
        .expect("rows")
        .collect::<Result<_, _>>()
        .expect("counts");
    conn.execute("DELETE FROM t204_writes", []).expect("reset");
    (n, by_table)
}

#[test]
fn a_tax_change_writes_a_few_rows_not_the_whole_year() {
    let dir = temp_dir();
    let store = campaign(&dir.0);
    count_writes(&store);
    let tax = |state: &mut RuntimeChange| {
        state.game_data.set_player_tax_rate(1, 40)?;
        Ok::<_, Box<dyn std::error::Error>>(())
    };
    store.change(3000, tax).expect("row change");
    let (by_rows, rows_tables) = writes(&store);
    store
        .change_rewriting_the_year(3000, tax)
        .expect("year rewrite");
    let (by_year, _) = writes(&store);
    assert!(
        by_rows < 10 && by_rows * 20 < by_year,
        "a tax change wrote {by_rows} rows ({rows_tables:?}); rewriting the year writes {by_year}"
    );
}

/// Maintenance loads a year, a player's change commits, and maintenance's
/// save is refused, though the change wrote its rows in place.
#[test]
fn maintenance_refuses_to_save_over_a_change_made_during_its_run() {
    let dir = temp_dir();
    let store = campaign(&dir.0);
    let mut run = nc_engine::CampaignMaintenance::load(&store).expect("load the year");
    store
        .change(3000, |state| {
            state.game_data.set_player_tax_rate(2, 33)?;
            Ok::<_, Box<dyn std::error::Error>>(())
        })
        .expect("a player's change");
    run.run_turn().expect("run the year");
    let refused = run.save(&store).expect_err("maintenance must not save");
    assert!(
        matches!(
            refused.downcast_ref::<CampaignStoreError>(),
            Some(CampaignStoreError::ConcurrentChange { .. })
        ),
        "{refused}"
    );
    let stored = store
        .load_latest_runtime_state()
        .expect("load")
        .expect("a snapshot");
    assert_eq!(stored.game_year, 3000);
    assert_eq!(stored.game_data.players[1].tax_rate, 33);
}
