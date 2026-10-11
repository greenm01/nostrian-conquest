use std::path::Path;

use nc_data::{CoreGameData, PlayerMode, set_empire_name, set_player_handle};

use crate::commands::runtime::{with_runtime_game_mut, with_runtime_game_mut_and_export};

pub(crate) fn set_player_name(
    dir: &Path,
    player_record_index_1_based: usize,
    handle: &str,
    empire_name: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    with_runtime_game_mut_and_export(dir, |data| {
        let player = data
            .players
            .get_mut(player_record_index_1_based - 1)
            .ok_or_else(|| {
                format!("player record index out of range: {player_record_index_1_based}")
            })?;
        player.mode = PlayerMode::Human;
        set_player_handle(player, handle);
        set_empire_name(player, empire_name);
        player.autopilot = false;
        Ok(())
    })?;

    println!(
        "Player {} renamed: handle='{}' empire='{}'",
        player_record_index_1_based, handle, empire_name
    );
    Ok(())
}

pub(crate) fn join_player(
    dir: &Path,
    player_record_index_1_based: usize,
    caller_alias: &str,
    empire_name: &str,
    homeworld_name: Option<&str>,
) -> Result<(), Box<dyn std::error::Error>> {
    with_runtime_game_mut_and_export(dir, |data| {
        data.join_player(player_record_index_1_based, empire_name)?;
        let player = data
            .players
            .get_mut(player_record_index_1_based - 1)
            .ok_or_else(|| {
                format!("player record index out of range: {player_record_index_1_based}")
            })?;
        set_player_handle(player, caller_alias);
        player.autopilot = false;
        if let Some(homeworld_name) = homeworld_name {
            data.rename_player_homeworld(player_record_index_1_based, homeworld_name)?;
        }
        Ok(())
    })?;

    println!(
        "Joined player {}: caller_alias='{}' empire='{}'{}",
        player_record_index_1_based,
        caller_alias,
        empire_name,
        homeworld_name
            .map(|name| format!(" homeworld='{name}'"))
            .unwrap_or_default()
    );
    Ok(())
}

pub(crate) fn prepare_classic_login(
    dir: &Path,
    player_record_index_1_based: usize,
    caller_alias: &str,
    empire_name_override: Option<&str>,
) -> Result<(), Box<dyn std::error::Error>> {
    let current_empire_name = current_empire_name(dir, player_record_index_1_based)?;
    let empire_name = empire_name_override.unwrap_or(&current_empire_name);

    with_runtime_game_mut_and_export(dir, |data| {
        let player = data
            .players
            .get_mut(player_record_index_1_based - 1)
            .ok_or_else(|| {
                format!("player record index out of range: {player_record_index_1_based}")
            })?;
        player.mode = PlayerMode::Human;
        set_player_handle(player, caller_alias);
        if let Some(empire_name) = empire_name_override {
            set_empire_name(player, empire_name);
        }
        player.autopilot = false;
        Ok(())
    })?;

    println!(
        "Prepared classic login for player {}: caller_alias='{}' empire='{}'",
        player_record_index_1_based, caller_alias, empire_name
    );
    Ok(())
}

fn current_empire_name(
    dir: &Path,
    player_record_index_1_based: usize,
) -> Result<String, Box<dyn std::error::Error>> {
    with_runtime_game_mut(dir, |data: &mut CoreGameData| {
        let player = data
            .players
            .get(player_record_index_1_based - 1)
            .ok_or_else(|| {
                format!("player record index out of range: {player_record_index_1_based}")
            })?;
        Ok(player.empire_name.trim().to_string())
    })
}
