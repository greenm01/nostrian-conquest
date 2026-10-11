use std::path::Path;

use nc_compat::{export_latest_snapshot_to_dir, import_directory_snapshot};
use nc_data::{CampaignStore, CampaignStoreError, DEFAULT_CAMPAIGN_DB_NAME, crippled_hull_count};

use crate::workspace::{overlay_classic_runtime_files, seed_classic_runtime_files};

pub fn import_directory_to_db(dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let store = CampaignStore::open_default_in_dir(dir)?;
    let snapshot_id = match import_directory_snapshot(&store, dir) {
        Err(CampaignStoreError::StaleSnapshot {
            loaded_year,
            latest_year,
        }) => {
            return Err(format!(
                "Refusing to import: the store already holds year {latest_year}, newer than \
                 the directory's year {loaded_year}. Import into a fresh store or export the \
                 store first."
            )
            .into());
        }
        result => result?,
    };
    println!(
        "Imported {} into {} as snapshot {}.",
        dir.display(),
        store.path().display(),
        snapshot_id
    );
    Ok(())
}

pub fn export_latest_db_snapshot(
    source_dir: &Path,
    target_dir: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let store = CampaignStore::open(source_dir.join(DEFAULT_CAMPAIGN_DB_NAME))?;
    overlay_classic_runtime_files(source_dir, target_dir)?;
    seed_classic_runtime_files(target_dir)?;
    let year = export_latest_snapshot_to_dir(&store, target_dir)?;
    if year == 0 {
        println!("No snapshots found in {}.", store.path().display());
    } else {
        println!(
            "Exported year {} from {} to {}.",
            year,
            store.path().display(),
            target_dir.display()
        );
        let crippled = crippled_hull_count(&store.load_latest_runtime_game_data()?);
        if crippled > 0 {
            println!(
                "{crippled} crippled hull(s) were written as whole hulls: classic files have no \
                 recovery state, so it is lost in the export (the campaign store keeps it)."
            );
        }
    }
    Ok(())
}
