use std::collections::BTreeMap;
use std::path::Path;

use crate::{
    CampaignStore, CoreGameData, DEFAULT_CAMPAIGN_DB_NAME, PlayerStarmapProjection,
    build_player_starmap_projection_from_snapshots,
};

pub const STARMAP_TEXT_FILE_NAME: &str = "starmap.txt";
pub const STARMAP_CSV_FILE_NAME: &str = "starmap.csv";
pub const STARMAP_DETAILS_CSV_FILE_NAME: &str = "starmap-DETAILS.csv";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlayerMapExportFile {
    pub name: &'static str,
    pub contents: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlayerMapExportData {
    pub ascii_export: String,
    pub csv_export: String,
    pub csv_details_export: String,
}

impl PlayerMapExportData {
    pub fn fixed_named_files(&self) -> Vec<PlayerMapExportFile> {
        vec![
            PlayerMapExportFile {
                name: STARMAP_TEXT_FILE_NAME,
                contents: self.ascii_export.clone(),
            },
            PlayerMapExportFile {
                name: STARMAP_CSV_FILE_NAME,
                contents: self.csv_export.clone(),
            },
            PlayerMapExportFile {
                name: STARMAP_DETAILS_CSV_FILE_NAME,
                contents: self.csv_details_export.clone(),
            },
        ]
    }
}

pub fn build_player_map_export_data(
    dir: &Path,
    player_record_index_1_based: usize,
) -> Result<PlayerMapExportData, Box<dyn std::error::Error>> {
    let (store, game_data) = open_latest_game(dir)?;
    let snapshots = store
        .latest_planet_intel_for_viewer(player_record_index_1_based as u8)?
        .into_iter()
        .map(|snapshot| (snapshot.planet_record_index_1_based, snapshot))
        .collect();
    let projection = build_player_starmap_projection_from_snapshots(
        &game_data,
        &snapshots,
        player_record_index_1_based as u8,
    );
    Ok(export_data(&projection))
}

/// The galaxy map every player starts with: where the worlds are, and
/// nothing any empire has learned about them.
pub fn build_galaxy_map_export_data(
    dir: &Path,
) -> Result<PlayerMapExportData, Box<dyn std::error::Error>> {
    let (_, game_data) = open_latest_game(dir)?;
    // A viewer that owns no world and holds no intel; unowned worlds have
    // owner 0, so 0 would count them as the viewer's own.
    let mut projection =
        build_player_starmap_projection_from_snapshots(&game_data, &BTreeMap::new(), u8::MAX);
    projection.viewer_empire_id = 0;
    Ok(export_data(&projection))
}

fn open_latest_game(
    dir: &Path,
) -> Result<(CampaignStore, CoreGameData), Box<dyn std::error::Error>> {
    let store_path = dir.join(DEFAULT_CAMPAIGN_DB_NAME);
    if !store_path.exists() {
        return Err(format!(
            "{} not found; import or create runtime state before using map-export",
            store_path.display()
        )
        .into());
    }

    let store = CampaignStore::open(store_path)?;
    let Some(runtime_state) = store.load_latest_runtime_state()? else {
        return Err(format!(
            "{} has no runtime snapshots; import or create runtime state before using map-export",
            dir.display()
        )
        .into());
    };
    Ok((store, runtime_state.game_data))
}

fn export_data(projection: &PlayerStarmapProjection) -> PlayerMapExportData {
    PlayerMapExportData {
        ascii_export: projection.render_ascii_export(),
        csv_export: projection.render_csv_export(),
        csv_details_export: projection.render_csv_details_export(),
    }
}
