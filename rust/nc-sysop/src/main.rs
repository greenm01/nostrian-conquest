mod ai_seats;
mod check;
mod map_export;
mod usage;

use std::env;
use std::path::{Path, PathBuf};

use nc_ai_onnx::{CampaignAi, model_from_env};
use nc_data::{CampaignSettings, CampaignStore, SeatReservation, generate_campaign_seed};
use nc_engine::{CampaignMaintenance, build_seeded_new_game};

#[derive(Clone)]
struct ParsedArgs {
    log_file: Option<PathBuf>,
    log_level: nc_log::LogLevel,
    args: Vec<String>,
}

fn main() {
    if let Err(err) = run() {
        tracing::error!(error = %err, "nc-sysop command failed");
        eprintln!("error: {err}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let parsed = parse_args(env::args().skip(1))?;
    let mut args = parsed.args.clone().into_iter();
    let Some(cmd) = args.next() else {
        usage::print_usage();
        return Ok(());
    };
    let rest = args.collect::<Vec<_>>();

    match cmd.as_str() {
        "--help" | "-h" | "help" => {
            usage::print_usage();
            Ok(())
        }
        "new-game" => {
            init_logging(&parsed, false)?;
            if rest.iter().any(|arg| arg == "--help" || arg == "-h") {
                usage::print_new_game_usage();
                return Ok(());
            }
            tracing::info!("running nc-sysop new-game");
            run_new_game(&rest)
        }
        "maint" => {
            init_logging(&parsed, false)?;
            if rest.iter().any(|arg| arg == "--help" || arg == "-h") {
                usage::print_maint_usage();
                return Ok(());
            }
            tracing::info!("running nc-sysop maint");
            run_maint(&rest)
        }
        "settings" => {
            init_logging(&parsed, false)?;
            if rest.iter().any(|arg| arg == "--help" || arg == "-h") {
                usage::print_settings_usage();
                return Ok(());
            }
            tracing::info!("running nc-sysop settings");
            run_settings(&rest)
        }
        "map-export" => {
            init_logging(&parsed, false)?;
            if rest.iter().any(|arg| arg == "--help" || arg == "-h") {
                usage::print_map_export_usage();
                return Ok(());
            }
            tracing::info!("running nc-sysop map-export");
            let dir = parse_required_dir_flag(&rest)?;
            let out_dir = parse_optional_path_flag_anywhere(&rest, "--out-dir")?
                .unwrap_or_else(|| dir.join("exports"));
            if rest
                .iter()
                .any(|arg| arg == "--player" || arg.starts_with("--player="))
            {
                let player = parse_required_usize_flag(&rest, "--player")?;
                map_export::export_player_map(&dir, player, &out_dir)
            } else {
                map_export::export_galaxy_map(&dir, &out_dir)
            }
        }
        "check" => {
            init_logging(&parsed, false)?;
            if rest.iter().any(|arg| arg == "--help" || arg == "-h") {
                usage::print_check_usage();
                return Ok(());
            }
            tracing::info!("running nc-sysop check");
            check::check_campaign(&parse_required_dir_flag(&rest)?)
        }
        "economy-report" => {
            init_logging(&parsed, false)?;
            if rest.iter().any(|arg| arg == "--help" || arg == "-h") {
                usage::print_economy_report_usage();
                return Ok(());
            }
            tracing::info!("running nc-sysop economy-report");
            let dir = parse_required_dir_flag(&rest)?;
            let player = parse_required_usize_flag(&rest, "--player")?;
            let Some(runtime) =
                CampaignStore::open_default_in_dir(&dir)?.load_latest_runtime_state()?
            else {
                return Err(format!("{} has no campaign", dir.display()).into());
            };
            let report = runtime
                .game_data
                .economy_report_text(player)
                .ok_or_else(|| format!("--player must be 1-{}", runtime.game_data.players.len()))?;
            print!("{report}");
            Ok(())
        }
        _ => {
            usage::print_usage();
            Err(format!("unknown subcommand: {cmd}").into())
        }
    }
}

fn run_new_game(args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    let mut bbs_mode = false;
    let mut target_dir = None;
    let mut player_count = 4u8;
    let mut seed = None;
    let mut game_name = None;
    let mut idx = 0;
    while idx < args.len() {
        match args[idx].as_str() {
            "--bbs" => {
                bbs_mode = true;
                idx += 1;
            }
            "--players" | "-p" => {
                let Some(value) = args.get(idx + 1) else {
                    return Err("missing value for --players".into());
                };
                player_count = parse_player_count(value)?;
                idx += 2;
            }
            "--seed" => {
                let Some(value) = args.get(idx + 1) else {
                    return Err("missing value for --seed".into());
                };
                seed = Some(value.parse::<u64>()?);
                idx += 2;
            }
            "--name" => {
                let Some(value) = args.get(idx + 1) else {
                    return Err("missing value for --name".into());
                };
                game_name = Some(value.to_string());
                idx += 2;
            }
            value if !value.starts_with('-') => {
                if target_dir.is_some() {
                    return Err(format!("unexpected argument: {value}").into());
                }
                target_dir = Some(resolve_repo_path(value));
                idx += 1;
            }
            other => {
                return Err(format!("unexpected argument: {other}").into());
            }
        }
    }
    let Some(target_dir) = target_dir else {
        usage::print_new_game_usage();
        return Err("missing target_dir for new-game".into());
    };

    let bbs_config_path = target_dir.join("config.kdl");
    if bbs_mode {
        if game_name.is_some() {
            return Err(
                "new-game --bbs reads game_name from config.kdl; do not pass --name".into(),
            );
        }
        if !bbs_config_path.exists() {
            return Err(format!(
                "{} requires an existing config.kdl in the target directory",
                target_dir.display()
            )
            .into());
        }
    } else {
        std::fs::create_dir_all(&target_dir)?;
        if bbs_config_path.exists() {
            return Err(format!(
                "{} contains config.kdl; use 'nc-sysop new-game --bbs {}' for BBS campaigns",
                target_dir.display(),
                target_dir.display()
            )
            .into());
        }
    }

    let slug = slug_from_dir(&target_dir)?;
    let game_name = if bbs_mode {
        humanize_slug(&slug)
    } else {
        game_name.unwrap_or_else(|| humanize_slug(&slug))
    };
    let existing_entries = std::fs::read_dir(&target_dir)?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .collect::<Vec<_>>();
    if bbs_mode {
        if existing_entries.iter().any(|path| path != &bbs_config_path) {
            return Err(format!(
                "{} must contain only config.kdl before creating a BBS campaign",
                target_dir.display()
            )
            .into());
        }
    } else if existing_entries
        .iter()
        .any(|path| path.file_name().and_then(|name| name.to_str()) != Some("ncgame.db"))
    {
        return Err(format!(
            "{} must be empty before creating a DB-only campaign",
            target_dir.display()
        )
        .into());
    }

    let store = CampaignStore::open_default_in_dir(&target_dir)?;
    if store.has_snapshots()? {
        return Err(format!("{} already contains a campaign", target_dir.display()).into());
    }

    let seed = seed.unwrap_or_else(generate_campaign_seed);
    let game_data = build_seeded_new_game(player_count, 3000, seed)?;
    store.save_year_with_intel_and_seed(
        game_data.conquest.game_year,
        &game_data,
        &[],
        &[],
        &[],
        Some(seed),
    )?;
    let settings = CampaignSettings::new(&slug, &game_name);
    store.save_campaign_settings(&settings)?;
    if bbs_mode {
        nc_data::sync_game_config(&target_dir, &store)?;
    }
    let settings = store.load_campaign_settings()?;
    nc_data::write_game_config(&target_dir, &store, &settings)?;
    println!(
        "Initialized new game at: {} (name={}, players={}, year=3000, seed={})",
        target_dir.display(),
        settings.game_name,
        player_count,
        seed
    );
    Ok(())
}

fn run_maint(args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    let Some(dir_arg) = args.first() else {
        usage::print_maint_usage();
        return Err("missing dir for maint".into());
    };
    let dir = resolve_repo_path(dir_arg);
    let turns = match args.get(1) {
        Some(value) => value.parse::<u16>()?,
        None => 1,
    };
    if args.len() > 2 {
        return Err("usage: nc-sysop maint <dir> [turns]".into());
    }
    run_maintenance_for_dir(&dir, turns, false)
}

fn run_settings(args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    let Some(cmd) = args.first().map(String::as_str) else {
        usage::print_settings_usage();
        return Ok(());
    };
    match cmd {
        "show" => {
            let dir = parse_required_dir_flag(&args[1..])?;
            let store = CampaignStore::open_default_in_dir(&dir)?;
            nc_data::sync_game_config(&dir, &store)?;
            if let Some(runtime) = store.load_latest_runtime_state()? {
                println!("players={}", runtime.game_data.players.len());
            }
            let settings = store.load_campaign_settings()?;
            println!("slug={}", settings.slug);
            println!("game_name={}", settings.game_name);
            println!("default_theme_key={}", settings.default_theme_key);
            println!("snoop={}", settings.snoop_enabled);
            println!(
                "session_max_idle_minutes={}",
                settings.session_max_idle_minutes
            );
            println!(
                "session_minimum_time_minutes={}",
                settings.session_minimum_time_minutes
            );
            println!("session_local_timeout={}", settings.session_local_timeout);
            println!("session_remote_timeout={}", settings.session_remote_timeout);
            println!(
                "inactivity_autopilot_after_turns={}",
                settings.inactivity_autopilot_after_turns
            );
            println!("maintenance_enabled={}", settings.maintenance_enabled);
            println!(
                "maintenance_interval_minutes={}",
                settings.maintenance_interval_minutes
            );
            println!(
                "maintenance_next_due_unix_seconds={}",
                settings
                    .maintenance_next_due_unix_seconds
                    .map(|value| value.to_string())
                    .unwrap_or_default()
            );
            for reservation in settings.reservations {
                println!(
                    "reservation seat={} alias={}",
                    reservation.player_record_index_1_based, reservation.alias
                );
            }
            for seat in settings.ai_seats {
                println!(
                    "ai_seat seat={} difficulty={} personality={}",
                    seat.player_record_index_1_based,
                    seat.difficulty.as_str(),
                    seat.personality
                );
            }
            Ok(())
        }
        "set" => {
            let dir = parse_required_dir_flag(&args[1..])?;
            let store = CampaignStore::open_default_in_dir(&dir)?;
            nc_data::sync_game_config(&dir, &store)?;
            let mut settings = store.load_campaign_settings()?;
            let mut idx = 1;
            while idx < args.len() {
                match args[idx].as_str() {
                    "--dir" => idx += 2,
                    "--game-name" => {
                        settings.game_name = args_value(args, &mut idx, "--game-name")?;
                    }
                    "--theme-key" => {
                        settings.default_theme_key = args_value(args, &mut idx, "--theme-key")?;
                    }
                    "--snoop" => {
                        settings.snoop_enabled =
                            parse_on_off(&args_value(args, &mut idx, "--snoop")?)?;
                    }
                    "--session-max-idle" => {
                        settings.session_max_idle_minutes =
                            args_value(args, &mut idx, "--session-max-idle")?.parse::<u8>()?;
                    }
                    "--session-minimum-time" => {
                        settings.session_minimum_time_minutes =
                            args_value(args, &mut idx, "--session-minimum-time")?.parse::<u8>()?;
                    }
                    "--session-local-timeout" => {
                        settings.session_local_timeout =
                            parse_on_off(&args_value(args, &mut idx, "--session-local-timeout")?)?;
                    }
                    "--session-remote-timeout" => {
                        settings.session_remote_timeout =
                            parse_on_off(&args_value(args, &mut idx, "--session-remote-timeout")?)?;
                    }
                    "--inactivity-autopilot-after" => {
                        settings.inactivity_autopilot_after_turns =
                            args_value(args, &mut idx, "--inactivity-autopilot-after")?
                                .parse::<u8>()?;
                    }
                    "--maintenance-enabled" => {
                        settings.maintenance_enabled =
                            parse_on_off(&args_value(args, &mut idx, "--maintenance-enabled")?)?;
                    }
                    "--maintenance-interval-minutes" => {
                        settings.maintenance_interval_minutes =
                            args_value(args, &mut idx, "--maintenance-interval-minutes")?
                                .parse::<u32>()?;
                    }
                    "--maintenance-next-due" => {
                        let value = args_value(args, &mut idx, "--maintenance-next-due")?;
                        settings.maintenance_next_due_unix_seconds = if value.trim().is_empty() {
                            None
                        } else {
                            Some(value.parse::<u64>()?)
                        };
                    }
                    other => return Err(format!("unexpected argument: {other}").into()),
                }
            }
            save_settings(&dir, &store, &settings)?;
            println!("Updated settings for {}", dir.display());
            Ok(())
        }
        "reserve" => {
            let dir = parse_required_dir_flag(&args[1..])?;
            let player = parse_required_usize_flag(&args[1..], "--player")?;
            let alias = parse_required_string_flag(&args[1..], "--alias")?;
            let store = CampaignStore::open_default_in_dir(&dir)?;
            nc_data::sync_game_config(&dir, &store)?;
            let mut settings = store.load_campaign_settings()?;
            settings
                .reservations
                .retain(|reservation| reservation.player_record_index_1_based != player);
            settings.reservations.push(SeatReservation {
                player_record_index_1_based: player,
                alias,
            });
            validate_reservations_against_runtime(&store, &settings)?;
            save_settings(&dir, &store, &settings)?;
            println!("Reserved seat {} in {}", player, dir.display());
            Ok(())
        }
        "ai-seat" => ai_seats::set_ai_seat(&args[1..]),
        "remove-ai-seat" => ai_seats::remove_ai_seat(&args[1..]),
        "unreserve" => {
            let dir = parse_required_dir_flag(&args[1..])?;
            let player = parse_required_usize_flag(&args[1..], "--player")?;
            let store = CampaignStore::open_default_in_dir(&dir)?;
            nc_data::sync_game_config(&dir, &store)?;
            let mut settings = store.load_campaign_settings()?;
            settings
                .reservations
                .retain(|reservation| reservation.player_record_index_1_based != player);
            save_settings(&dir, &store, &settings)?;
            println!(
                "Removed reservation for seat {} in {}",
                player,
                dir.display()
            );
            Ok(())
        }
        other => Err(format!("unknown settings subcommand: {other}").into()),
    }
}

fn run_maintenance_for_dir(
    dir: &Path,
    turns: u16,
    update_schedule: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    println!(
        "Running Rust maintenance on: {} ({} turn{})",
        dir.display(),
        turns,
        if turns == 1 { "" } else { "s" }
    );
    let store = CampaignStore::open_default_in_dir(dir)?;
    nc_data::sync_game_config(dir, &store)?;
    // Players' changes are refused as paused until the run ends (t205).
    let run = store.begin_maintenance()?;
    if run.took_over() {
        eprintln!("warning: an earlier maintenance run stopped before finishing; running again");
    }
    let mut maintenance = CampaignMaintenance::load(&store)?;
    let start_year = maintenance.start_year();
    let mut ai = CampaignAi::load(&store, &maintenance, model_from_env().as_deref())?;
    for (player, saved) in ai.iter().flat_map(CampaignAi::stale_seats) {
        eprintln!(
            "warning: AI seat {player}'s saved memory is from year {saved}, not {start_year}; \
             it starts again from its stored intel"
        );
    }

    for turn in 1..=turns {
        if let Some(ai) = &ai {
            ai.play(&mut maintenance)?;
        }
        maintenance.run_turn()?;
        if let Some(ai) = &mut ai {
            ai.learn(&maintenance);
        }
        println!(
            "  Turn {}: year {}",
            turn, maintenance.game_data.conquest.game_year
        );
    }

    let end_year = maintenance.game_data.conquest.game_year;
    maintenance.save(run.store())?;
    if let Some(ai) = &ai {
        ai.save(&store, end_year)?;
    }
    drop(run);

    if update_schedule {
        let mut settings = store.load_campaign_settings()?;
        if settings.maintenance_enabled {
            settings.maintenance_next_due_unix_seconds = Some(unix_now().saturating_add(
                u64::from(settings.maintenance_interval_minutes).saturating_mul(60),
            ));
            store.save_campaign_settings(&settings)?;
        }
    }

    println!("  Year advanced: {} -> {}", start_year, end_year);
    println!("Rust maintenance complete.");
    Ok(())
}

fn validate_reservations_against_runtime(
    store: &CampaignStore,
    settings: &CampaignSettings,
) -> Result<(), Box<dyn std::error::Error>> {
    let runtime_state = store
        .load_latest_runtime_state()?
        .ok_or("campaign store has no snapshots; initialize the campaign with nc-sysop first")?;
    settings.validate_reservations_for_player_count(runtime_state.game_data.players.len())?;
    Ok(())
}

/// Save `settings` to the store and, when the game has a `config.kdl`, to
/// the file too, so the next sync from the file keeps the change (t218).
fn save_settings(
    dir: &Path,
    store: &CampaignStore,
    settings: &CampaignSettings,
) -> Result<(), Box<dyn std::error::Error>> {
    store.save_campaign_settings(settings)?;
    if is_bbs_campaign_dir(dir) {
        nc_data::write_game_config(dir, store, settings)?;
    }
    Ok(())
}

fn is_bbs_campaign_dir(dir: &Path) -> bool {
    dir.join("config.kdl").exists()
}

fn slug_from_dir(dir: &Path) -> Result<String, Box<dyn std::error::Error>> {
    let slug = dir
        .file_name()
        .and_then(|name| name.to_str())
        .filter(|name| !name.is_empty())
        .ok_or_else(|| format!("cannot derive slug from {}", dir.display()))?
        .to_string();
    if !slug
        .chars()
        .all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '-')
    {
        return Err(format!(
            "game slug '{}' must use only lowercase ascii letters, digits, and dashes",
            slug
        )
        .into());
    }
    Ok(slug)
}

fn humanize_slug(slug: &str) -> String {
    slug.split('-')
        .filter(|part| !part.is_empty())
        .map(|part| {
            let mut chars = part.chars();
            match chars.next() {
                Some(first) => {
                    let mut word = String::new();
                    word.extend(first.to_uppercase());
                    word.push_str(chars.as_str());
                    word
                }
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn parse_player_count(value: &str) -> Result<u8, Box<dyn std::error::Error>> {
    let player_count = value.parse::<u8>()?;
    if !nc_data::is_valid_player_count(player_count) {
        return Err(format!("player_count must be 1-25, got {player_count}").into());
    }
    Ok(player_count)
}

fn parse_on_off(value: &str) -> Result<bool, Box<dyn std::error::Error>> {
    match value.trim().to_ascii_lowercase().as_str() {
        "on" | "true" | "1" => Ok(true),
        "off" | "false" | "0" => Ok(false),
        other => Err(format!("expected on/off, got {other}").into()),
    }
}

fn parse_required_dir_flag(args: &[String]) -> Result<PathBuf, Box<dyn std::error::Error>> {
    parse_optional_path_flag_anywhere(args, "--dir")?
        .ok_or_else(|| "missing value for --dir".into())
}

fn parse_required_string_flag(
    args: &[String],
    flag: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    let mut i = 0;
    while i < args.len() {
        let arg = &args[i];
        if arg == flag {
            let Some(next) = args.get(i + 1) else {
                return Err(format!("missing value for {flag}").into());
            };
            return Ok(next.to_string());
        }
        if let Some(value) = arg.strip_prefix(&format!("{flag}=")) {
            return Ok(value.to_string());
        }
        i += 1;
    }
    Err(format!("missing value for {flag}").into())
}

fn parse_required_usize_flag(
    args: &[String],
    flag: &str,
) -> Result<usize, Box<dyn std::error::Error>> {
    Ok(parse_required_string_flag(args, flag)?.parse::<usize>()?)
}

fn args_value(
    args: &[String],
    idx: &mut usize,
    flag: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    let Some(value) = args.get(*idx + 1) else {
        return Err(format!("missing value for {flag}").into());
    };
    *idx += 2;
    Ok(value.to_string())
}

fn parse_optional_path_flag_anywhere(
    args: &[String],
    flag: &str,
) -> Result<Option<PathBuf>, Box<dyn std::error::Error>> {
    let mut i = 0;
    while i < args.len() {
        let arg = &args[i];
        if arg == flag {
            let Some(next) = args.get(i + 1) else {
                return Err(format!("missing value for {flag}").into());
            };
            return Ok(Some(PathBuf::from(next)));
        }
        if let Some(value) = arg.strip_prefix(&format!("{flag}=")) {
            return Ok(Some(PathBuf::from(value)));
        }
        i += 1;
    }
    Ok(None)
}

fn resolve_repo_path(arg: &str) -> PathBuf {
    let path = PathBuf::from(arg);
    if path.is_absolute() {
        path
    } else if path.exists() {
        path
    } else {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join(path)
    }
}

fn init_logging(
    parsed: &ParsedArgs,
    default_to_stderr: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(log_file) = &parsed.log_file {
        nc_log::init_file_logging(log_file, parsed.log_level)?;
        tracing::info!(
            log_file = %log_file.display(),
            level = ?parsed.log_level,
            "nc-sysop logging initialized"
        );
    } else if default_to_stderr {
        nc_log::init_stderr_logging(parsed.log_level)?;
        tracing::info!(level = ?parsed.log_level, "nc-sysop stderr logging initialized");
    }
    Ok(())
}

fn parse_args(
    args: impl IntoIterator<Item = String>,
) -> Result<ParsedArgs, Box<dyn std::error::Error>> {
    let mut log_file = None;
    let mut log_level = nc_log::LogLevel::Info;
    let mut remaining = Vec::new();
    let mut iter = args.into_iter();
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--log-file" => {
                let Some(value) = iter.next() else {
                    return Err("missing value for --log-file".into());
                };
                log_file = Some(PathBuf::from(value));
            }
            "--log-level" => {
                let Some(value) = iter.next() else {
                    return Err("missing value for --log-level".into());
                };
                log_level = nc_log::LogLevel::parse(&value)?;
            }
            _ => {
                remaining.push(arg);
                remaining.extend(iter);
                break;
            }
        }
    }
    Ok(ParsedArgs {
        log_file,
        log_level,
        args: remaining,
    })
}

fn unix_now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}
