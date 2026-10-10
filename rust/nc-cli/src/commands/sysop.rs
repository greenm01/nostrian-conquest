use std::path::PathBuf;

use crate::commands::setup::{
    init_new_game, init_new_game_from_config, init_new_game_with_seed, init_new_game_with_year,
};
use crate::support::paths::resolve_repo_path;
use crate::usage::print_sysop_usage;
use nc_compat::{
    ensure_classic_auxiliary_files, import_directory_snapshot,
    write_default_database_dat_for_game_data,
};
use nc_data::{CampaignStore, CoreGameData};
use nc_engine::GameStateBuilder;

fn generate_gamestate_from_args(
    program: &str,
    mut args: impl Iterator<Item = String>,
) -> Result<(), Box<dyn std::error::Error>> {
    let Some(target_dir) = args.next().map(|arg| resolve_repo_path(&arg)) else {
        print_sysop_usage(program);
        return Ok(());
    };
    let player_count: u8 = match args.next().and_then(|s| s.parse().ok()) {
        Some(count) if nc_data::is_valid_player_count(count) => count,
        _ => {
            eprintln!("Error: player_count must be 1-25");
            print_sysop_usage(program);
            return Ok(());
        }
    };
    let year: u16 = match args.next().and_then(|s| s.parse().ok()) {
        Some(y) => y,
        None => {
            eprintln!("Error: year must be a valid number");
            print_sysop_usage(program);
            return Ok(());
        }
    };

    let mut homeworld_coords = Vec::new();
    for coord_str in args {
        let parts: Vec<&str> = coord_str.split(':').collect();
        if parts.len() != 2 {
            eprintln!("Error: homeworld coords must be in format x:y");
            print_sysop_usage(program);
            return Ok(());
        }
        let x: u8 = match parts[0].parse() {
            Ok(v) => v,
            Err(_) => {
                eprintln!("Error: invalid x coordinate: {}", parts[0]);
                return Ok(());
            }
        };
        let y: u8 = match parts[1].parse() {
            Ok(v) => v,
            Err(_) => {
                eprintln!("Error: invalid y coordinate: {}", parts[1]);
                return Ok(());
            }
        };
        homeworld_coords.push([x, y]);
    }

    while homeworld_coords.len() < player_count as usize {
        homeworld_coords.push([0, 0]);
    }
    homeworld_coords.truncate(player_count as usize);

    let builder = GameStateBuilder::new()
        .with_player_count(player_count)
        .with_year(year)
        .with_homeworld_coords(homeworld_coords);

    match builder.build_and_save(&target_dir) {
        Ok(()) => {
            let built = CoreGameData::load(&target_dir)?;
            write_default_database_dat_for_game_data(&target_dir, &built)?;
            ensure_classic_auxiliary_files(&target_dir)?;
            let store = CampaignStore::open_default_in_dir(&target_dir)?;
            import_directory_snapshot(&store, &target_dir)?;
            println!("Generated gamestate at: {}", target_dir.display());

            let errors = built.ecmaint_preflight_errors();
            if errors.is_empty() {
                println!("Preflight validation: OK");
            } else {
                println!("Preflight validation errors:");
                for error in errors {
                    println!("  - {}", error);
                }
            }
        }
        Err(e) => eprintln!("Error generating gamestate: {}", e),
    }

    Ok(())
}

fn parse_player_count_value(value: &str) -> Result<u8, String> {
    let player_count: u8 = value
        .parse()
        .map_err(|_| format!("invalid player count: {value}"))?;
    if nc_data::is_valid_player_count(player_count) {
        Ok(player_count)
    } else {
        Err(format!("player_count must be 1-25, got {player_count}"))
    }
}

fn parse_seed_value(value: &str) -> Result<u64, String> {
    value
        .parse()
        .map_err(|_| format!("seed must be a valid unsigned integer, got {value}"))
}

fn parse_year_value(value: &str) -> Result<u16, String> {
    value
        .parse()
        .map_err(|_| format!("year must be a valid unsigned 16-bit integer, got {value}"))
}

fn parse_new_game_options(
    args: &[String],
    allow_config: bool,
) -> Result<(Option<u8>, Option<PathBuf>, Option<u64>, Option<u16>), String> {
    let mut player_count = None;
    let mut config_path = None;
    let mut seed = None;
    let mut year = None;
    let mut idx = 0;
    while idx < args.len() {
        match args[idx].as_str() {
            "--players" | "-p" => {
                let Some(value) = args.get(idx + 1) else {
                    return Err("missing value for --players".to_string());
                };
                player_count = Some(parse_player_count_value(value)?);
                idx += 2;
            }
            "--config" => {
                if !allow_config {
                    return Err(
                        "--config is only supported on the internal nc-cli setup preset path"
                            .to_string(),
                    );
                }
                let Some(value) = args.get(idx + 1) else {
                    return Err("missing value for --config".to_string());
                };
                config_path = Some(resolve_repo_path(value));
                idx += 2;
            }
            "--seed" => {
                let Some(value) = args.get(idx + 1) else {
                    return Err("missing value for --seed".to_string());
                };
                seed = Some(parse_seed_value(value)?);
                idx += 2;
            }
            "--year" => {
                let Some(value) = args.get(idx + 1) else {
                    return Err("missing value for --year".to_string());
                };
                year = Some(parse_year_value(value)?);
                idx += 2;
            }
            _ => {
                return Err(if allow_config {
                    "usage: sysop new-game <target_dir> [--players N] [--year N] [--seed N] [--config path]"
                        .to_string()
                } else {
                    "usage: sysop new-game <target_dir> [--players N] [--year N] [--seed N]"
                        .to_string()
                });
            }
        }
    }

    Ok((Some(player_count.unwrap_or(4)), config_path, seed, year))
}

pub fn run_sysop_args(
    program: &str,
    mut args: impl Iterator<Item = String>,
) -> Result<(), Box<dyn std::error::Error>> {
    let allow_config = program.starts_with("nc-cli");
    let Some(cmd) = args.next() else {
        print_sysop_usage(program);
        return Ok(());
    };

    match cmd.as_str() {
        "generate-gamestate" if allow_config => generate_gamestate_from_args(program, args)?,
        "new-game" => {
            let Some(target_dir) = args.next().map(|arg| resolve_repo_path(&arg)) else {
                print_sysop_usage(program);
                return Ok(());
            };
            let remaining = args.collect::<Vec<_>>();
            let (player_count, config_path, seed, year) =
                match parse_new_game_options(&remaining, allow_config) {
                    Ok(options) => options,
                    Err(message) => {
                        eprintln!("Error: {message}");
                        print_sysop_usage(program);
                        return Err(message.into());
                    }
                };
            if let Some(config_path) = config_path {
                init_new_game_from_config(&target_dir, &config_path, player_count, year, seed)?;
                println!(
                    "Initialized new game at: {} (config={}{}{}{}{}{}{})",
                    target_dir.display(),
                    config_path.display(),
                    if player_count.is_some() {
                        ", players="
                    } else {
                        ""
                    },
                    player_count
                        .map(|count| count.to_string())
                        .unwrap_or_default(),
                    if year.is_some() { ", year=" } else { "" },
                    year.map(|value| value.to_string()).unwrap_or_default(),
                    if seed.is_some() { ", seed=" } else { "" },
                    seed.map(|value| value.to_string()).unwrap_or_default()
                );
            } else {
                let player_count = player_count.expect("player count should be present");
                if let Some(seed) = seed {
                    init_new_game_with_seed(&target_dir, player_count, year.unwrap_or(3000), seed)?;
                } else {
                    if let Some(year) = year {
                        init_new_game_with_year(&target_dir, player_count, year)?;
                    } else {
                        init_new_game(&target_dir, player_count)?;
                    }
                }
                println!(
                    "Initialized new game at: {} (players={}{}{}{}{})",
                    target_dir.display(),
                    player_count,
                    if year.is_some() { ", year=" } else { "" },
                    year.map(|value| value.to_string()).unwrap_or_default(),
                    if seed.is_some() { ", seed=" } else { "" },
                    seed.map(|value| value.to_string()).unwrap_or_default()
                );
            }
        }
        _ => print_sysop_usage(program),
    }

    Ok(())
}
