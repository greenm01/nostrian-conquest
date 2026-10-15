//! `settings ai-seat` and `settings remove-ai-seat` (t165): seats a learned
//! agent plays in a Rust campaign. `maint` plays them with the agent named
//! by `NC_AI_MODEL`.

use nc_data::{AiDifficulty, AiSeat, CampaignStore};

use crate::{
    is_bbs_campaign_dir, parse_required_dir_flag, parse_required_string_flag,
    parse_required_usize_flag, validate_reservations_against_runtime,
};

pub fn set_ai_seat(args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    let dir = parse_required_dir_flag(args)?;
    let player = parse_required_usize_flag(args, "--player")?;
    let difficulty = AiDifficulty::parse(&parse_required_string_flag(args, "--difficulty")?)?;
    let store = open_rust_campaign(&dir)?;
    let mut settings = store.load_campaign_settings()?;
    settings
        .ai_seats
        .retain(|seat| seat.player_record_index_1_based != player);
    settings.ai_seats.push(AiSeat {
        player_record_index_1_based: player,
        difficulty,
    });
    settings
        .ai_seats
        .sort_by_key(|seat| seat.player_record_index_1_based);
    validate_reservations_against_runtime(&store, &settings)?;
    store.save_campaign_settings(&settings)?;
    println!(
        "Seat {player} in {} is an AI seat ({})",
        dir.display(),
        difficulty.as_str()
    );
    Ok(())
}

pub fn remove_ai_seat(args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    let dir = parse_required_dir_flag(args)?;
    let player = parse_required_usize_flag(args, "--player")?;
    let store = open_rust_campaign(&dir)?;
    let mut settings = store.load_campaign_settings()?;
    settings
        .ai_seats
        .retain(|seat| seat.player_record_index_1_based != player);
    store.save_campaign_settings(&settings)?;
    println!("Seat {player} in {} is no longer an AI seat", dir.display());
    Ok(())
}

fn open_rust_campaign(dir: &std::path::Path) -> Result<CampaignStore, Box<dyn std::error::Error>> {
    if is_bbs_campaign_dir(dir) {
        return Err("AI seats are for Rust campaigns; BBS campaigns do not support them".into());
    }
    Ok(CampaignStore::open_default_in_dir(dir)?)
}
