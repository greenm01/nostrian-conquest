use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use nc_compat::{import_directory_snapshot, load_classic_core_files, save_classic_core_files};
use nc_data::{
    CampaignRuntimeState, CampaignStore, CoreGameData, DEFAULT_CAMPAIGN_DB_NAME,
    PlanetIntelSnapshot,
};

pub(crate) fn with_runtime_game_mut<T, F>(
    dir: &Path,
    mutate: F,
) -> Result<T, Box<dyn std::error::Error>>
where
    F: FnOnce(&mut CoreGameData) -> Result<T, Box<dyn std::error::Error>>,
{
    with_runtime_state_mut(dir, |state| mutate(&mut state.game_data))
}

pub(crate) fn with_runtime_state_mut<T, F>(
    dir: &Path,
    mutate: F,
) -> Result<T, Box<dyn std::error::Error>>
where
    F: FnOnce(&mut CampaignRuntimeState) -> Result<T, Box<dyn std::error::Error>>,
{
    let store = CampaignStore::open_default_in_dir(dir)?;
    let mut state = load_runtime_state_preferring_live_directory(dir, &store)?;
    let mut planet_intel_by_viewer = load_runtime_intel_by_viewer(&store, &state.game_data)?;
    let result = mutate(&mut state)?;
    for viewer_empire_id in 1..=state.game_data.conquest.player_count() {
        let viewer_idx = viewer_empire_id.saturating_sub(1) as usize;
        let previous = planet_intel_by_viewer
            .get(viewer_idx)
            .cloned()
            .unwrap_or_default();
        planet_intel_by_viewer[viewer_idx] = nc_data::merge_player_intel_from_runtime(
            &state.game_data,
            viewer_empire_id,
            state.game_data.conquest.game_year(),
            Some(&previous),
            None,
        );
    }
    store.save_runtime_state_structured_with_intel(
        &state.game_data,
        &state.planet_scorch_orders,
        &state.report_block_rows,
        &state.queued_mail,
        &planet_intel_by_viewer,
    )?;
    Ok(result)
}

pub(crate) fn load_runtime_state_preferring_live_directory(
    dir: &Path,
    store: &CampaignStore,
) -> Result<CampaignRuntimeState, Box<dyn std::error::Error>> {
    if let Some(state) = store.load_latest_runtime_state()? {
        return Ok(state);
    }

    import_directory_snapshot(store, dir)?;
    Ok(store
        .load_latest_runtime_state()?
        .ok_or("campaign store has no snapshots after importing directory")?)
}

pub(crate) fn load_runtime_game_data(
    dir: &Path,
) -> Result<CoreGameData, Box<dyn std::error::Error>> {
    let store_path = dir.join(DEFAULT_CAMPAIGN_DB_NAME);
    if store_path.exists() {
        let store = CampaignStore::open(store_path)?;
        if let Some(state) = store.load_latest_runtime_state()? {
            return Ok(state.game_data);
        }
    }

    Ok(load_classic_core_files(dir)?)
}

pub(crate) fn export_runtime_snapshot_in_place(
    dir: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    crate::commands::storage::export_latest_db_snapshot(dir, dir)?;
    Ok(())
}

/// Write the store's core files over `dir`, except CONQUEST.DAT: a replayable
/// scenario keeps the replay context's CONQUEST.DAT as copied.
pub(crate) fn export_runtime_core_projection_in_place(
    dir: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let game_data = load_runtime_game_data(dir)?;
    let conquest_path = dir.join("CONQUEST.DAT");
    let replay_conquest = fs::read(&conquest_path)?;
    save_classic_core_files(dir, &game_data)?;
    fs::write(conquest_path, replay_conquest)?;
    Ok(())
}

pub(crate) fn with_runtime_game_mut_and_export<T, F>(
    dir: &Path,
    mutate: F,
) -> Result<T, Box<dyn std::error::Error>>
where
    F: FnOnce(&mut CoreGameData) -> Result<T, Box<dyn std::error::Error>>,
{
    with_runtime_game_mut(dir, mutate)
}

pub(crate) fn load_runtime_intel_by_viewer(
    campaign_store: &CampaignStore,
    game_data: &CoreGameData,
) -> Result<Vec<BTreeMap<usize, PlanetIntelSnapshot>>, Box<dyn std::error::Error>> {
    (1..=game_data.conquest.player_count())
        .map(|viewer_empire_id| {
            Ok(campaign_store
                .latest_planet_intel_for_viewer(viewer_empire_id)?
                .into_iter()
                .map(|snapshot| (snapshot.planet_record_index_1_based, snapshot))
                .collect::<BTreeMap<_, _>>())
        })
        .collect()
}

pub(crate) fn with_runtime_game_mut_and_export_core<T, F>(
    dir: &Path,
    mutate: F,
) -> Result<T, Box<dyn std::error::Error>>
where
    F: FnOnce(&mut CoreGameData) -> Result<T, Box<dyn std::error::Error>>,
{
    with_runtime_game_mut(dir, mutate)
}
