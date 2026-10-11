use std::collections::BTreeMap;
use std::path::Path;

use nc_data::{
    CampaignRuntimeState, CampaignStore, CoreGameData, DEFAULT_CAMPAIGN_DB_NAME,
    PlanetIntelSnapshot, is_named_homeworld_seed, merge_player_intel_from_runtime,
    reset_player_slot_to_baseline, set_empire_name, set_player_handle,
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

    let mut game_data = runtime.game_data.clone();
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

    save_runtime_state_with_recomputed_intel(&store, runtime, game_data)?;

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
    let runtime = load_latest_runtime_state(&store)?;
    let mut game_data = runtime.game_data.clone();
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

    save_runtime_state_with_recomputed_intel(&store, runtime, game_data)?;

    Ok(())
}

pub fn reset_hosted_player_to_baseline(
    game_dir: &Path,
    player_seat: u32,
) -> Result<(), Box<dyn std::error::Error>> {
    let player_record_index_1_based =
        usize::try_from(player_seat).map_err(|_| format!("invalid player seat {}", player_seat))?;
    let store = open_existing_campaign_store(game_dir)?;
    let runtime = load_latest_runtime_state(&store)?;
    let mut game_data = runtime.game_data.clone();
    reset_player_slot_to_baseline(&mut game_data, player_record_index_1_based)?;
    save_runtime_state_with_recomputed_intel(&store, runtime, game_data)?;
    Ok(())
}

fn save_runtime_state_with_recomputed_intel(
    store: &CampaignStore,
    runtime: CampaignRuntimeState,
    game_data: CoreGameData,
) -> Result<(), Box<dyn std::error::Error>> {
    let player_count = game_data.conquest.player_count;

    let planet_intel_by_viewer = (1..=player_count)
        .map(|viewer_empire_id| {
            let previous = store
                .latest_planet_intel_for_viewer(viewer_empire_id)?
                .into_iter()
                .map(|snapshot| (snapshot.planet_record_index_1_based, snapshot))
                .collect::<BTreeMap<usize, PlanetIntelSnapshot>>();
            Ok(merge_player_intel_from_runtime(
                &game_data,
                viewer_empire_id,
                game_data.conquest.game_year,
                Some(&previous),
                None,
            ))
        })
        .collect::<Result<Vec<_>, nc_data::CampaignStoreError>>()?;
    let player_activity_states = store.latest_player_activity_states(player_count)?;
    let player_lifecycle_states = store.latest_player_lifecycle_states(player_count)?;
    let player_war_stats = store.latest_player_war_stats(player_count)?;

    store.save_runtime_state_structured_with_intel_activity_lifecycle_and_war_stats(
        runtime.game_year,
        &game_data,
        &runtime.planet_scorch_orders,
        &runtime.report_block_rows,
        &runtime.queued_mail,
        &planet_intel_by_viewer,
        &player_activity_states,
        &player_lifecycle_states,
        &player_war_stats,
        runtime.winner_state,
    )?;

    Ok(())
}
