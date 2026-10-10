use nc_compat::{
    ensure_classic_auxiliary_files, import_directory_snapshot_with_seed, save_classic_core_files,
    write_default_database_dat_for_game_data,
};
use nc_data::{CampaignStore, generate_campaign_seed};
use nc_engine::build_seeded_new_game;
use std::fs;
use std::path::Path;

use crate::setup_preset::SetupPresetConfig;
use crate::workspace::seed_classic_runtime_files;

const DEFAULT_NEW_GAME_YEAR: u16 = 3000;

pub(crate) fn init_new_game(
    target: &Path,
    player_count: u8,
) -> Result<(), Box<dyn std::error::Error>> {
    init_new_game_with_year(target, player_count, DEFAULT_NEW_GAME_YEAR)
}

pub(crate) fn init_new_game_with_year(
    target: &Path,
    player_count: u8,
    year: u16,
) -> Result<(), Box<dyn std::error::Error>> {
    init_new_game_with_seed(target, player_count, year, runtime_seed())
}

pub(crate) fn init_new_game_with_seed(
    target: &Path,
    player_count: u8,
    year: u16,
    seed: u64,
) -> Result<(), Box<dyn std::error::Error>> {
    let data = build_seeded_new_game(player_count, year, seed)?;

    fs::create_dir_all(target)?;
    save_classic_core_files(target, &data)?;
    write_default_database_dat_for_game_data(target, &data)?;
    ensure_classic_auxiliary_files(target)?;

    seed_classic_runtime_files(target)?;
    let store = CampaignStore::open_default_in_dir(target)?;
    import_directory_snapshot_with_seed(&store, target, Some(seed))?;

    Ok(())
}

pub(crate) fn init_new_game_from_config(
    target: &Path,
    config_path: &Path,
    player_count_override: Option<u8>,
    year_override: Option<u16>,
    seed_override: Option<u64>,
) -> Result<(), Box<dyn std::error::Error>> {
    let config = SetupPresetConfig::load_kdl(config_path)?;
    let config = if let Some(player_count) = player_count_override {
        config.with_player_count_override(player_count)?
    } else {
        config
    };
    let year = year_override.unwrap_or(DEFAULT_NEW_GAME_YEAR);
    let seed = seed_override.unwrap_or_else(runtime_seed);
    let data = build_seeded_new_game(config.player_count, year, config.seed.unwrap_or(seed))?;

    fs::create_dir_all(target)?;
    save_classic_core_files(target, &data)?;
    write_default_database_dat_for_game_data(target, &data)?;
    ensure_classic_auxiliary_files(target)?;

    seed_classic_runtime_files(target)?;
    let store = CampaignStore::open_default_in_dir(target)?;
    import_directory_snapshot_with_seed(&store, target, Some(seed))?;

    Ok(())
}

fn runtime_seed() -> u64 {
    generate_campaign_seed()
}
