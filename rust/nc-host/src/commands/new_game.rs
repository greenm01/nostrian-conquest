use nc_data::hosted::GameTier;
use std::path::PathBuf;

use crate::invite::generate_invite_code;

pub fn run(args: &[&str]) -> Result<(), Box<dyn std::error::Error>> {
    if args.is_empty() || args.iter().any(|arg| matches!(*arg, "--help" | "-h")) {
        print_usage();
        return Ok(());
    }

    let mut dir = None;
    let mut players = 4u32;
    let mut name = None;
    let mut seed = None;
    let mut tier = GameTier::League;
    let mut i = 0;
    while i < args.len() {
        match args[i] {
            "--players" => {
                if i + 1 >= args.len() {
                    return Err("missing value for --players".into());
                }
                players = args[i + 1].parse()?;
                i += 2;
            }
            "--name" => {
                if i + 1 >= args.len() {
                    return Err("missing value for --name".into());
                }
                name = Some(args[i + 1].to_string());
                i += 2;
            }
            "--seed" => {
                if i + 1 >= args.len() {
                    return Err("missing value for --seed".into());
                }
                seed = Some(args[i + 1].parse()?);
                i += 2;
            }
            "--tier" => {
                if i + 1 >= args.len() {
                    return Err("missing value for --tier".into());
                }
                tier = GameTier::from_str(args[i + 1])
                    .ok_or("invalid tier: use 'sandbox' or 'league'")?;
                i += 2;
            }
            _ => {
                if dir.is_some() {
                    return Err(format!("unexpected argument: {}", args[i]).into());
                }
                dir = Some(PathBuf::from(args[i]));
                i += 1;
            }
        }
    }

    let dir = dir.ok_or("missing game directory argument")?;
    if !u8::try_from(players).is_ok_and(nc_data::is_valid_player_count) {
        return Err(format!("--players must be in 1..=25, got {players}").into());
    }
    let name = name.unwrap_or_else(|| {
        dir.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("Unnamed Game")
            .to_string()
    });

    create_game(&dir, &name, players, seed, &tier)?;

    println!("Created hosted game:");
    println!("  directory: {}", dir.display());
    println!("  name: {}", name);
    println!("  players: {}", players);
    println!("  tier: {}", tier.as_str());

    Ok(())
}

fn create_game(
    dir: &PathBuf,
    name: &str,
    players: u32,
    seed: Option<u32>,
    tier: &GameTier,
) -> Result<(), Box<dyn std::error::Error>> {
    std::fs::create_dir_all(dir)?;

    let db_path = dir.join("hosted.db");
    let store = nc_data::hosted::HostedStore::create(&db_path)?;

    let game_id = dir
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("game")
        .to_string();

    let now = chrono::Utc::now().timestamp();
    let seed = seed
        .map(u64::from)
        .unwrap_or_else(nc_data::generate_campaign_seed);

    store.connection().execute(
        "INSERT INTO game_metadata (id, name, status, created_at, updated_at, current_year, current_turn, players, recruiting, lobby_visibility, maintenance_enabled, maintenance_interval_minutes, game_tier)
         VALUES (?1, ?2, 'setup', ?3, ?3, 3000, 0, ?4, 'none', 'public', 1, 1440, ?5)",
        rusqlite::params![game_id, name, now, players, tier.as_str()],
    )?;

    let invite_codes = generate_invite_codes(players);
    nc_data::hosted::create_seats(store.connection(), &game_id, &invite_codes)?;
    nc_data::hosted::mark_catalog_dirty(store.connection(), &game_id)?;
    crate::game::state::initialize_runtime_state(dir, &game_id, name, players as u8, 3000, seed)?;

    Ok(())
}

fn generate_invite_codes(player_count: u32) -> Vec<String> {
    let mut existing = std::collections::HashSet::new();
    let mut invite_codes = Vec::with_capacity(player_count as usize);

    for _ in 0..player_count {
        let code = generate_invite_code(&existing);
        existing.insert(code.clone());
        invite_codes.push(code);
    }

    invite_codes
}

fn print_usage() {
    println!("Usage: nc-host new-game <dir> [--players N] [--name \"Name\"] [--seed N]");
    println!();
    println!("Options:");
    println!("  --players N  Number of players (default: 4)");
    println!("  --name      Game name");
    println!("  --seed      Random seed for map generation");
}
