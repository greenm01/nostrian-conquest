//! The hosted databases open under one policy (t205): many processes can
//! create and open `hosted.db` at once, and the file is left in WAL mode.

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
fn many_sessions_open_a_new_hosted_db_at_once() {
    let dir = TempDir(
        std::env::temp_dir().join(format!("nc-data-hosted-connections-{}", std::process::id())),
    );
    std::fs::create_dir_all(&dir.0).expect("temp dir");
    let path = dir.0.join("hosted.db");

    let barrier = Arc::new(Barrier::new(8));
    let opens: Vec<_> = (0..8)
        .map(|_| {
            let (path, barrier) = (path.clone(), barrier.clone());
            std::thread::spawn(move || {
                barrier.wait();
                HostedStore::create(&path).map(|_| ())
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
    let mode: String = conn
        .query_row("PRAGMA journal_mode", [], |row| row.get(0))
        .expect("journal mode");
    assert_eq!(mode, "wal");
    let foreign_keys: bool = conn
        .query_row("PRAGMA foreign_keys", [], |row| row.get(0))
        .expect("foreign keys");
    assert!(foreign_keys);
}
