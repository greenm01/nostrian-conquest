//! Maintenance outside the host blocks players' changes while it runs
//! (t205), and recovers the block a crashed run left.

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
        "nc-data-maintenance-run-{name}-{}",
        std::process::id()
    )));
    std::fs::create_dir_all(&dir.0).expect("temp dir");
    let store = CampaignStore::open_default_in_dir(&dir.0).expect("open store");
    let data = build_seeded_initialized_game(4, 3000, 1515).expect("seeded game");
    store.save_year(3000, &data, &[], &[]).expect("seed store");
    (dir, store)
}

fn tax(store: &CampaignStore) -> Result<(), CampaignStoreError> {
    store
        .change(3000, |state| {
            state.game_data.players[0].tax_rate = 41;
            Ok::<_, CampaignStoreError>(())
        })
        .map(|_| ())
}

#[test]
fn a_run_blocks_changes_admits_its_own_save_and_lifts_the_block() {
    let (_dir, store) = campaign("blocks");
    let run = store.begin_maintenance().expect("begin");
    assert!(!run.took_over());
    assert!(matches!(
        tax(&store),
        Err(CampaignStoreError::HostingPaused)
    ));
    assert!(
        matches!(
            store.begin_maintenance(),
            Err(CampaignStoreError::InvalidState(_))
        ),
        "one run at a time"
    );
    let mut year = nc_engine::CampaignMaintenance::load(&store).expect("load");
    year.run_turn().expect("run");
    year.save(run.store())
        .expect("the run's own save passes its block");
    drop(run);
    assert_eq!(
        store.latest_snapshot_metadata().expect("meta").map(|m| m.1),
        Some(3001)
    );
    assert!(matches!(
        tax(&store),
        Err(CampaignStoreError::StaleSnapshot { .. })
    ));
    store
        .change(3001, |state| {
            state.game_data.players[0].tax_rate = 41;
            Ok::<_, CampaignStoreError>(())
        })
        .expect("changes resume in the new year");
}

/// A run that crashed left its block; the next run takes it over, and
/// players play again once that run ends.
#[test]
fn the_next_run_takes_over_the_block_of_one_that_crashed() {
    let (dir, store) = campaign("crashed");
    let marker = rusqlite::Connection::open(store.path()).expect("open");
    marker
        .execute(
            "INSERT INTO campaign_metadata(key,int_value) VALUES ('maintenance_running',1)",
            [],
        )
        .expect("leave a crashed run's block");
    assert!(matches!(
        tax(&store),
        Err(CampaignStoreError::HostingPaused)
    ));
    let run = store.begin_maintenance().expect("begin");
    assert!(run.took_over());
    drop(run);
    tax(&store).expect("players play again");
    drop(dir);
}
