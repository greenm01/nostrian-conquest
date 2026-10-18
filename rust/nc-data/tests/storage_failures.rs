//! When the campaign file cannot be written, a change fails and the
//! campaign is left as it was; a damaged file is reported before anyone
//! plays on it (t205).

use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;

use nc_data::{CampaignStore, CampaignStoreError};
use nc_engine::build_seeded_initialized_game;

struct TempDir(PathBuf);

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn campaign(name: &str) -> (TempDir, CampaignStore) {
    let dir = TempDir(std::env::temp_dir().join(format!(
        "nc-data-storage-failures-{name}-{}",
        std::process::id()
    )));
    let _ = std::fs::remove_dir_all(&dir.0);
    std::fs::create_dir_all(&dir.0).expect("temp dir");
    let store = CampaignStore::open_default_in_dir(&dir.0).expect("open store");
    let data = build_seeded_initialized_game(4, 3000, 1515).expect("seeded game");
    store.save_year(3000, &data, &[], &[]).expect("seed store");
    (dir, store)
}

fn set_mode(path: &std::path::Path, mode: u32) {
    if path.exists() {
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(mode)).expect("chmod");
    }
}

#[test]
fn a_change_that_cannot_be_written_fails_and_changes_nothing() {
    let (dir, store) = campaign("unwritable");
    let before = store.load_latest_runtime_state().expect("load");
    let files = ["ncgame.db", "ncgame.db-wal", "ncgame.db-shm"].map(|name| dir.0.join(name));
    for file in &files {
        set_mode(file, 0o444);
    }
    set_mode(&dir.0, 0o555);
    let changed = store.change(3000, |state| {
        state.game_data.players[0].tax_rate = 7;
        Ok::<_, CampaignStoreError>(())
    });
    set_mode(&dir.0, 0o755);
    for file in &files {
        set_mode(file, 0o644);
    }
    assert!(changed.is_err(), "a change to an unwritable file must fail");
    assert_eq!(store.load_latest_runtime_state().expect("load"), before);
}

#[test]
fn a_damaged_campaign_file_is_reported() {
    let (dir, store) = campaign("damaged");
    let path = store.path().to_path_buf();
    drop(store);
    // Checkpoint the WAL into the file, then overwrite part of a table page.
    rusqlite::Connection::open(&path)
        .expect("open")
        .query_row("PRAGMA wal_checkpoint(TRUNCATE)", [], |_| Ok(()))
        .expect("checkpoint");
    let mut bytes = std::fs::read(&path).expect("read");
    let page = 4096;
    for byte in &mut bytes[page * 3..page * 3 + 512] {
        *byte = 0xA5;
    }
    std::fs::write(&path, bytes).expect("damage");
    let checked =
        CampaignStore::open_default_in_dir(&dir.0).and_then(|store| store.check_integrity());
    assert!(checked.is_err(), "the damage must be reported");
}
