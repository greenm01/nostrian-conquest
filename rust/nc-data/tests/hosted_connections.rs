//! The hosted databases open under one policy (t205): many processes can
//! open and upgrade `hosted.db` at once, and the file is left in WAL mode.

use std::path::PathBuf;
use std::sync::{Arc, Barrier};

use nc_data::hosted::HostedStore;

struct TempDir(PathBuf);

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[test]
fn many_sessions_upgrade_an_older_hosted_db_at_once() {
    let dir = TempDir(
        std::env::temp_dir().join(format!("nc-data-hosted-connections-{}", std::process::id())),
    );
    std::fs::create_dir_all(&dir.0).expect("temp dir");
    let path = dir.0.join("hosted.db");
    // A file from before `seats.claimed_year` was added.
    HostedStore::create(&path)
        .expect("create")
        .connection()
        .execute_batch("ALTER TABLE seats DROP COLUMN claimed_year")
        .expect("make the file older");

    let barrier = Arc::new(Barrier::new(8));
    let opens: Vec<_> = (0..8)
        .map(|_| {
            let (path, barrier) = (path.clone(), barrier.clone());
            std::thread::spawn(move || {
                barrier.wait();
                HostedStore::open(&path).map(|_| ())
            })
        })
        .collect();
    for open in opens {
        open.join()
            .expect("thread")
            .expect("every session opens the file");
    }

    let store = HostedStore::open(&path).expect("open");
    let conn = store.connection();
    let claimed_year: bool = conn
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM pragma_table_info('seats') WHERE name = 'claimed_year')",
            [],
            |row| row.get(0),
        )
        .expect("columns");
    assert!(claimed_year, "the upgrade added the column");
    let mode: String = conn
        .query_row("PRAGMA journal_mode", [], |row| row.get(0))
        .expect("journal mode");
    assert_eq!(mode, "wal");
    let foreign_keys: bool = conn
        .query_row("PRAGMA foreign_keys", [], |row| row.get(0))
        .expect("foreign keys");
    assert!(foreign_keys);
}
