use std::path::Path;

use nc_data::{
    CampaignRuntimeState, CampaignStore, CoreGameData, DEFAULT_CAMPAIGN_DB_NAME,
    is_named_homeworld_seed, reset_player_slot_to_baseline, set_empire_name, set_player_handle,
};

/// The game's campaign store. New-game creation seeds it, so a game
/// directory without one is an error.
pub fn open_existing_campaign_store(
    game_dir: &Path,
) -> Result<CampaignStore, Box<dyn std::error::Error>> {
    let path = game_dir.join(DEFAULT_CAMPAIGN_DB_NAME);
    if !path.exists() {
        return Err(format!("campaign store not found at {}", path.display()).into());
    }
    Ok(CampaignStore::open(path)?)
}

/// The latest snapshot in `store`; a store without one is an error.
pub fn load_latest_runtime_state(
    store: &CampaignStore,
) -> Result<CampaignRuntimeState, Box<dyn std::error::Error>> {
    Ok(store
        .load_latest_runtime_state()?
        .ok_or("campaign store has no runtime snapshots")?)
}

pub fn load_runtime_game_data(game_dir: &Path) -> Result<CoreGameData, Box<dyn std::error::Error>> {
    let store = open_existing_campaign_store(game_dir)?;
    Ok(load_latest_runtime_state(&store)?.game_data)
}

pub fn current_runtime_year(game_dir: &Path) -> Result<u16, Box<dyn std::error::Error>> {
    let store = open_existing_campaign_store(game_dir)?;
    Ok(load_latest_runtime_state(&store)?.game_year)
}

pub fn ensure_hosted_player_initialized(
    game_dir: &Path,
    player_seat: u32,
    handle: Option<&str>,
) -> Result<(), Box<dyn std::error::Error>> {
    let player_record_index_1_based =
        usize::try_from(player_seat).map_err(|_| format!("invalid player seat {}", player_seat))?;
    let store = open_existing_campaign_store(game_dir)?;
    let runtime = load_latest_runtime_state(&store)?;

    if !runtime
        .game_data
        .player_slot_is_open_for_first_join(player_record_index_1_based)
    {
        return Ok(());
    }

    // The seat is joined on the latest state (t204), so a player changing
    // the campaign meanwhile keeps their change; a join that lost the race
    // to another session of this seat does nothing.
    store.change(runtime.game_year, |state| {
        let game_data = &mut state.game_data;
        if !game_data.player_slot_is_open_for_first_join(player_record_index_1_based) {
            return Ok(());
        }
        let empire_name = game_data
            .players
            .get(player_record_index_1_based.saturating_sub(1))
            .map(|player| {
                let empire_name = player.empire_name.trim().to_string();
                if empire_name.is_empty() {
                    format!("Seat {}", player_seat)
                } else {
                    empire_name
                }
            })
            .unwrap_or_else(|| format!("Seat {}", player_seat));
        game_data.join_player(player_record_index_1_based, &empire_name)?;
        if let Some(handle) = handle {
            if let Some(player) = game_data
                .players
                .get_mut(player_record_index_1_based.saturating_sub(1))
            {
                set_player_handle(player, handle);
            }
        }
        Ok::<_, Box<dyn std::error::Error>>(())
    })?;
    Ok(())
}

pub fn apply_hosted_first_join_names(
    game_dir: &Path,
    player_seat: u32,
    empire_name: &str,
    homeworld_name: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let player_record_index_1_based =
        usize::try_from(player_seat).map_err(|_| format!("invalid player seat {}", player_seat))?;
    let store = open_existing_campaign_store(game_dir)?;
    let year = load_latest_runtime_state(&store)?.game_year;
    store.change(year, |state| {
        let game_data = &mut state.game_data;
        let player = game_data
            .players
            .get(player_record_index_1_based.saturating_sub(1))
            .ok_or_else(|| format!("missing player record for seat {}", player_seat))?;
        let empire_pending = player.empire_name.trim() == "In Civil Disorder";
        let homeworld_pending = player
            .homeworld
            .checked_sub(1)
            .and_then(|idx| game_data.planets.get(idx as usize))
            .is_some_and(is_named_homeworld_seed);

        if !empire_pending && !homeworld_pending {
            return Err("first-join naming is no longer available".into());
        }

        if let Some(player) = game_data
            .players
            .get_mut(player_record_index_1_based.saturating_sub(1))
        {
            set_empire_name(player, empire_name);
        }
        game_data.rename_player_homeworld(player_record_index_1_based, homeworld_name)?;
        Ok::<_, Box<dyn std::error::Error>>(())
    })?;
    Ok(())
}

pub fn reset_hosted_player_to_baseline(
    game_dir: &Path,
    player_seat: u32,
) -> Result<(), Box<dyn std::error::Error>> {
    let player_record_index_1_based =
        usize::try_from(player_seat).map_err(|_| format!("invalid player seat {}", player_seat))?;
    let store = open_existing_campaign_store(game_dir)?;
    let year = load_latest_runtime_state(&store)?.game_year;
    store.change(year, |state| {
        reset_player_slot_to_baseline(&mut state.game_data, player_record_index_1_based)?;
        Ok::<_, Box<dyn std::error::Error>>(())
    })?;
    Ok(())
}
