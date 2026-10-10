mod usage;

use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::path::{Path, PathBuf};

use nc_data::{
    BbsGameConfig, CampaignSettings, CampaignStore, MaintenanceEvents, PlanetIntelSnapshot,
    SeatReservation, apply_inactivity_autopilot_policy,
    apply_maintenance_events_to_player_war_stats, carry_unread_report_blocks,
    generate_campaign_seed, latest_planet_intel_grants_for_viewer, merge_player_intel_from_runtime,
};
use nc_engine::{
    VisibleHazardIntel, apply_results_reviewable_flags, build_results_report_blocks,
    build_seeded_new_game, run_maintenance_turn_with_context_seed_and_lifecycle,
    visible_hazard_intel_from_snapshots,
};

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
        if game_name.is_some() || args.iter().any(|arg| arg == "--players" || arg == "-p") {
            return Err(
                "new-game --bbs reads players from config.kdl; do not pass --name or --players"
                    .into(),
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

    let (player_count, seed) = if bbs_mode {
        let config = BbsGameConfig::load_kdl(&bbs_config_path)?;
        (config.players, seed.unwrap_or_else(generate_campaign_seed))
    } else {
        (player_count, seed.unwrap_or_else(generate_campaign_seed))
    };
    let game_data = build_seeded_new_game(player_count, 3000, seed)?;
    store.save_runtime_state_structured(&game_data, &BTreeSet::new(), &[], &[])?;
    store.save_campaign_settings(&CampaignSettings::new(&slug, &game_name))?;
    println!(
        "Initialized new game at: {} (name={}, players={}, year=3000, seed={})",
        target_dir.display(),
        game_name,
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
            if is_bbs_campaign_dir(&dir) {
                let config = load_bbs_game_config(&dir)?;
                println!("mode=bbs");
                println!("players={}", config.players);
                for reservation in config.reservations {
                    println!(
                        "reservation seat={} alias={}",
                        reservation.player_record_index_1_based, reservation.alias
                    );
                }
                return Ok(());
            }
            let store = CampaignStore::open_default_in_dir(&dir)?;
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
                "inactivity_purge_after_turns={}",
                settings.inactivity_purge_after_turns
            );
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
            Ok(())
        }
        "set" => {
            let dir = parse_required_dir_flag(&args[1..])?;
            if is_bbs_campaign_dir(&dir) {
                return Err(
                    "BBS campaigns do not support 'settings set'; edit config.kdl directly and use settings reserve/unreserve for reservations"
                        .into(),
                );
            }
            let store = CampaignStore::open_default_in_dir(&dir)?;
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
                    "--inactivity-purge-after" => {
                        settings.inactivity_purge_after_turns =
                            args_value(args, &mut idx, "--inactivity-purge-after")?
                                .parse::<u8>()?;
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
            store.save_campaign_settings(&settings)?;
            println!("Updated settings for {}", dir.display());
            Ok(())
        }
        "reserve" => {
            let dir = parse_required_dir_flag(&args[1..])?;
            let player = parse_required_usize_flag(&args[1..], "--player")?;
            let alias = parse_required_string_flag(&args[1..], "--alias")?;
            if is_bbs_campaign_dir(&dir) {
                let store = CampaignStore::open_default_in_dir(&dir)?;
                let mut config = load_bbs_game_config(&dir)?;
                config
                    .reservations
                    .retain(|reservation| reservation.player_record_index_1_based != player);
                config.reservations.push(SeatReservation {
                    player_record_index_1_based: player,
                    alias,
                });
                validate_bbs_reservations_against_runtime(&store, &config)?;
                save_bbs_game_config(&dir, &config)?;
                println!("Reserved seat {} in {}", player, dir.display());
                return Ok(());
            }
            let store = CampaignStore::open_default_in_dir(&dir)?;
            let mut settings = store.load_campaign_settings()?;
            settings
                .reservations
                .retain(|reservation| reservation.player_record_index_1_based != player);
            settings.reservations.push(SeatReservation {
                player_record_index_1_based: player,
                alias,
            });
            validate_reservations_against_runtime(&store, &settings)?;
            store.save_campaign_settings(&settings)?;
            println!("Reserved seat {} in {}", player, dir.display());
            Ok(())
        }
        "unreserve" => {
            let dir = parse_required_dir_flag(&args[1..])?;
            let player = parse_required_usize_flag(&args[1..], "--player")?;
            if is_bbs_campaign_dir(&dir) {
                let mut config = load_bbs_game_config(&dir)?;
                config
                    .reservations
                    .retain(|reservation| reservation.player_record_index_1_based != player);
                save_bbs_game_config(&dir, &config)?;
                println!(
                    "Removed reservation for seat {} in {}",
                    player,
                    dir.display()
                );
                return Ok(());
            }
            let store = CampaignStore::open_default_in_dir(&dir)?;
            let mut settings = store.load_campaign_settings()?;
            settings
                .reservations
                .retain(|reservation| reservation.player_record_index_1_based != player);
            store.save_campaign_settings(&settings)?;
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
    let runtime_state = store
        .load_latest_runtime_state()?
        .ok_or("campaign store has no snapshots; initialize the campaign with nc-sysop first")?;
    let mut game_data = runtime_state.game_data;
    let campaign_seed = runtime_state.campaign_seed;
    let mut winner_state = runtime_state.winner_state;
    let start_year = game_data.conquest.game_year();
    let mut planet_intel_by_viewer = store.load_snapshot_planet_intel_by_viewer(
        runtime_state.snapshot_id,
        game_data.conquest.player_count(),
    )?;
    let mut player_activity_states =
        store.latest_player_activity_states(game_data.conquest.player_count())?;
    let mut player_lifecycle_states =
        store.latest_player_lifecycle_states(game_data.conquest.player_count())?;
    let mut player_war_stats = store.latest_player_war_stats(game_data.conquest.player_count())?;
    let mut all_events = MaintenanceEvents::default();

    if winner_state.winner_empire_raw.is_some() {
        return Err("campaign winner has already been declared; maintenance is frozen".into());
    }

    for turn in 1..=turns {
        let inactivity_threshold = game_data.setup.autopilot_inactive_turns_raw();
        apply_inactivity_autopilot_policy(
            &mut game_data,
            inactivity_threshold,
            &mut player_activity_states,
        );
        let visible_hazards = visible_hazards_from_snapshots(&game_data, &planet_intel_by_viewer);
        let events = if visible_hazards.is_empty() {
            run_maintenance_turn_with_context_seed_and_lifecycle(
                &mut game_data,
                campaign_seed,
                &[],
                &[],
                &mut player_lifecycle_states,
                &mut winner_state,
            )?
        } else {
            run_maintenance_turn_with_context_seed_and_lifecycle(
                &mut game_data,
                campaign_seed,
                &visible_hazards,
                &[],
                &mut player_lifecycle_states,
                &mut winner_state,
            )?
        };
        let planet_intel_grants_by_viewer = (1..=game_data.conquest.player_count())
            .map(|viewer_empire_id| {
                latest_planet_intel_grants_for_viewer(&events, viewer_empire_id)
            })
            .collect::<Vec<_>>();
        extend_maintenance_events(&mut all_events, events);
        for viewer_empire_id in 1..=game_data.conquest.player_count() {
            let viewer_idx = viewer_empire_id.saturating_sub(1) as usize;
            let previous = planet_intel_by_viewer
                .get(viewer_idx)
                .cloned()
                .unwrap_or_default();
            planet_intel_by_viewer[viewer_idx] = merge_player_intel_from_runtime(
                &game_data,
                viewer_empire_id,
                game_data.conquest.game_year(),
                Some(&previous),
                planet_intel_grants_by_viewer.get(viewer_idx),
            );
        }
        println!("  Turn {}: year {}", turn, game_data.conquest.game_year());
    }

    let report_block_rows = carry_unread_report_blocks(
        &runtime_state.report_block_rows,
        build_results_report_blocks(&game_data, &all_events),
    );
    apply_results_reviewable_flags(&mut game_data, &report_block_rows);
    apply_maintenance_events_to_player_war_stats(&mut player_war_stats, &all_events);
    store.save_runtime_state_structured_with_intel_activity_lifecycle_and_war_stats(
        &game_data,
        &runtime_state.planet_scorch_orders,
        &report_block_rows,
        &runtime_state.queued_mail,
        &planet_intel_by_viewer,
        &player_activity_states,
        &player_lifecycle_states,
        &player_war_stats,
        winner_state,
    )?;

    if update_schedule {
        let mut settings = store.load_campaign_settings()?;
        if settings.maintenance_enabled {
            settings.maintenance_next_due_unix_seconds = Some(unix_now().saturating_add(
                u64::from(settings.maintenance_interval_minutes).saturating_mul(60),
            ));
            store.save_campaign_settings(&settings)?;
        }
    }

    println!(
        "  Year advanced: {} -> {}",
        start_year,
        game_data.conquest.game_year()
    );
    println!("Rust maintenance complete.");
    Ok(())
}

fn extend_maintenance_events(all_events: &mut MaintenanceEvents, events: MaintenanceEvents) {
    all_events.bombard_events.extend(events.bombard_events);
    all_events
        .planet_intel_events
        .extend(events.planet_intel_events);
    all_events
        .ownership_change_events
        .extend(events.ownership_change_events);
    all_events
        .fleet_battle_events
        .extend(events.fleet_battle_events);
    all_events
        .fleet_destroyed_events
        .extend(events.fleet_destroyed_events);
    all_events
        .starbase_destroyed_events
        .extend(events.starbase_destroyed_events);
    all_events
        .assault_report_events
        .extend(events.assault_report_events);
    all_events
        .scout_contact_events
        .extend(events.scout_contact_events);
    all_events
        .encounter_disposition_events
        .extend(events.encounter_disposition_events);
    all_events
        .invalid_player_state_events
        .extend(events.invalid_player_state_events);
    all_events
        .fleet_merge_events
        .extend(events.fleet_merge_events);
    all_events.join_host_events.extend(events.join_host_events);
    all_events
        .mission_retarget_events
        .extend(events.mission_retarget_events);
    all_events
        .colonization_events
        .extend(events.colonization_events);
    all_events.mission_events.extend(events.mission_events);
    all_events.salvage_events.extend(events.salvage_events);
    all_events
        .diplomatic_escalation_events
        .extend(events.diplomatic_escalation_events);
    all_events
        .civil_disorder_events
        .extend(events.civil_disorder_events);
    all_events
        .campaign_outlook_events
        .extend(events.campaign_outlook_events);
    all_events
        .campaign_outcome_events
        .extend(events.campaign_outcome_events);
    all_events
        .game_victory_notice_events
        .extend(events.game_victory_notice_events);
    all_events
        .empire_elimination_events
        .extend(events.empire_elimination_events);
    all_events
        .fleet_defection_events
        .extend(events.fleet_defection_events);
}

fn visible_hazards_from_snapshots(
    game_data: &nc_data::CoreGameData,
    planet_intel_by_viewer: &[BTreeMap<usize, PlanetIntelSnapshot>],
) -> Vec<VisibleHazardIntel> {
    (1..=game_data.conquest.player_count() as usize)
        .map(|viewer_idx| {
            let empty = BTreeMap::new();
            visible_hazard_intel_from_snapshots(
                game_data,
                planet_intel_by_viewer.get(viewer_idx - 1).unwrap_or(&empty),
                viewer_idx as u8,
            )
        })
        .collect()
}

fn validate_reservations_against_runtime(
    store: &CampaignStore,
    settings: &CampaignSettings,
) -> Result<(), Box<dyn std::error::Error>> {
    let runtime_state = store
        .load_latest_runtime_state()?
        .ok_or("campaign store has no snapshots; initialize the campaign with nc-sysop first")?;
    settings
        .validate_reservations_for_player_count(runtime_state.game_data.player.records.len())?;
    Ok(())
}

fn validate_bbs_reservations_against_runtime(
    store: &CampaignStore,
    config: &BbsGameConfig,
) -> Result<(), Box<dyn std::error::Error>> {
    let runtime_state = store
        .load_latest_runtime_state()?
        .ok_or("campaign store has no snapshots; initialize the campaign with nc-sysop first")?;
    config.validate_reservations_for_player_count(runtime_state.game_data.player.records.len())?;
    Ok(())
}

fn is_bbs_campaign_dir(dir: &Path) -> bool {
    dir.join("config.kdl").exists()
}

fn load_bbs_game_config(dir: &Path) -> Result<BbsGameConfig, Box<dyn std::error::Error>> {
    Ok(BbsGameConfig::load_kdl(&dir.join("config.kdl"))?)
}

fn save_bbs_game_config(
    dir: &Path,
    config: &BbsGameConfig,
) -> Result<(), Box<dyn std::error::Error>> {
    config.save_kdl(&dir.join("config.kdl"))?;
    Ok(())
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
