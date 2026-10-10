use std::fs;
use std::path::Path;

use crate::commands::runtime::load_runtime_game_data;
use nc_compat::inspect_classic_messages_dat;
use nc_data::{ConquestDat, CoreGameData, PlanetRecord, SetupDat};
use nc_engine::{FleetEtaEstimate, estimate_fleet_eta, plan_route};

pub(crate) fn inspect_dir(dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let data = load_runtime_game_data(dir)?;

    println!("Directory: {}", dir.display());
    print_header_summary(&data.setup, &data.conquest);
    println!();

    println!("Players:");
    for (idx, record) in data.player.records.iter().enumerate() {
        let campaign_state = data
            .empire_campaign_state((idx + 1) as u8)
            .map(|state| state.as_str())
            .unwrap_or("unknown");
        println!(
            "  slot {}: owner_mode={} assigned_player_flag={} tax={} last_run_year={} autopilot={} reports_pending={} messages_pending={} message_review={:04x}/{:04x} results_review={:04x}/{:04x} results_chain={:04x}/{:04x} campaign_state={} summary={}",
            idx + 1,
            record.owner_mode_raw(),
            record.assigned_player_flag_raw(),
            record.tax_rate(),
            record.last_run_year_raw(),
            record.autopilot_flag(),
            record.classic_reports_pending_flag_raw(),
            record.classic_messages_pending_flag_raw(),
            record.classic_message_review_word_raw(),
            record.classic_message_review_carry_word_raw(),
            record.classic_results_review_word_raw(),
            record.classic_results_review_carry_word_raw(),
            record.classic_results_first_header_raw(),
            record.classic_results_last_header_raw(),
            campaign_state,
            record.ownership_summary()
        );
        println!(
            "    first_base_record_raw={}",
            record.first_base_record_raw()
        );
    }
    println!();

    println!("Planets:");
    for (idx, record) in data.planets.records.iter().enumerate().take(5) {
        let bytes = record.to_bytes();
        println!(
            "  planet {:02}: coords={:?} hdr={:02x?} len={} text='{}' tail58={:02x?} fact_word={:04x} summary='{}'",
            idx + 1,
            record.coords_raw(),
            record.header_bytes(),
            record.string_len(),
            record.status_or_name_summary(),
            &bytes[0x58..0x61],
            u16::from_le_bytes([bytes[0x08], bytes[0x09]]),
            record.derived_summary()
        );
    }
    println!("  ... {} total planet records", data.planets.records.len());

    let homeworld_like = data
        .planets
        .records
        .iter()
        .enumerate()
        .filter(|(_, record)| record.is_named_homeworld_seed())
        .collect::<Vec<_>>();
    if !homeworld_like.is_empty() {
        println!();
        println!("Planet Seeds:");
        for (idx, record) in homeworld_like {
            println!(
                "  planet {:02}: summary='{}' potential={}",
                idx + 1,
                record.derived_summary(),
                record.potential_production_points()
            );
        }
    }

    println!();
    println!("Fleets:");
    let fleet_display_count = data.fleets.records.len().min(16);
    for (idx, record) in data
        .fleets
        .records
        .iter()
        .enumerate()
        .take(fleet_display_count)
    {
        println!(
            "  fleet {:02}: id={} slot={} prev={} next={} cur_spd={} max_spd={} roe={} ships={} loc_raw={:02x?} order={}({}) target_raw={:02x?} summary='{}'",
            idx + 1,
            record.fleet_id(),
            record.local_slot(),
            record.previous_fleet_id(),
            record.next_fleet_id(),
            record.current_speed(),
            record.max_speed(),
            record.rules_of_engagement(),
            record.ship_composition_summary(),
            record.current_location_coords_raw(),
            record.standing_order_kind().as_str(),
            record.standing_order_code_raw(),
            record.standing_order_target_coords_raw(),
            record.standing_order_summary()
        );
        println!("    mission_aux={:02x?}", record.mission_aux_bytes());
    }
    if data.fleets.records.len() > fleet_display_count {
        println!("  ... {} total fleet records", data.fleets.records.len());
    }

    let looks_like_initialized_blocks = !data.fleets.records.is_empty()
        && data.fleets.records.len() % 4 == 0
        && data
            .fleets
            .records
            .chunks_exact(4)
            .all(|group| group.iter().map(|r| r.local_slot()).eq([1, 2, 3, 4]));

    if looks_like_initialized_blocks {
        println!();
        println!("Fleet Groups:");
        for (group_idx, group) in data.fleets.records.chunks_exact(4).enumerate() {
            let home = group[0].current_location_coords_raw();
            println!(
                "  empire block {}: loc_raw={:02x?} target_raw={:02x?}",
                group_idx + 1,
                home,
                group[0].standing_order_target_coords_raw()
            );
            for record in group {
                println!(
                    "    id={} slot={} ships={} max_spd={} order={} summary='{}'",
                    record.fleet_id(),
                    record.local_slot(),
                    record.ship_composition_summary(),
                    record.max_speed(),
                    record.standing_order_kind().as_str(),
                    record.standing_order_summary()
                );
            }
        }
    }

    println!();
    println!("Bases:");
    for (idx, record) in data.bases.records.iter().enumerate() {
        println!(
            "  base {:02}: escort={} id={} owner={} next={} record={} prev={} coords={:02x?} moving={} target={:02x?} target_depth={}",
            idx + 1,
            record.escort_fleet_slot_raw(),
            record.base_id_raw(),
            record.owner_empire_raw(),
            record.next_base_link_raw(),
            record.record_number_raw(),
            record.previous_base_link_raw(),
            record.coords_raw(),
            record.moving_flag_raw(),
            record.target_coords_raw(),
            record.target_depth_raw()
        );
    }

    println!();
    println!("IPBM:");
    for (idx, record) in data.ipbm.records.iter().enumerate() {
        println!(
            "  record {:02}: number={} owner={} next={} record_number={}",
            idx + 1,
            record.ipbm_number_raw(),
            record.owner_empire_raw(),
            record.next_ipbm_link_raw(),
            record.record_number_raw()
        );
    }

    Ok(())
}

pub(crate) fn inspect_messages(dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let path = dir.join("MESSAGES.DAT");
    let bytes = fs::read(&path)?;
    let inspection = inspect_classic_messages_dat(&bytes);

    println!("Directory: {}", dir.display());
    println!("MESSAGES.DAT bytes={}", inspection.byte_len);

    if bytes.is_empty() {
        println!("MESSAGES.DAT is empty");
        return Ok(());
    }

    if let Some(subject) = inspection.subject {
        println!("Subject: {subject}");
    }

    if !inspection.printable_runs.is_empty() {
        println!("Printable runs:");
        for run in inspection.printable_runs {
            println!("  {run}");
        }
    }

    if !inspection.record_previews.is_empty() {
        println!(
            "Possible classic mail records: {} x 40-byte",
            inspection.record_previews.len()
        );
        for record in inspection.record_previews {
            println!(
                "  rec {:02}: {:02x?} text='{}'",
                record.index, record.header_bytes, record.ascii_preview
            );
        }
    } else if let Some(preview) = inspection.raw_preview {
        println!("Raw preview: {}", preview);
    }

    Ok(())
}

pub(crate) fn inspect_fleet_movement(
    dir: &Path,
    fleet_record_index_1_based: usize,
    prefer_live_directory: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let data = if prefer_live_directory {
        CoreGameData::load(dir)?
    } else {
        load_runtime_game_data(dir)?
    };
    let Some(fleet) = data.fleets.records.get(fleet_record_index_1_based - 1) else {
        return Err(format!("fleet record out of range: {fleet_record_index_1_based}").into());
    };
    let eta = estimate_fleet_eta(&data, fleet_record_index_1_based - 1);
    let route = plan_route(&data, fleet_record_index_1_based - 1);
    let route_coords = route
        .as_ref()
        .map(|route| {
            route
                .steps
                .iter()
                .map(|step| format!("{},{}", step.coords[0], step.coords[1]))
                .collect::<Vec<_>>()
                .join(";")
        })
        .unwrap_or_default();

    println!("directory={}", dir.display());
    println!(
        "source={}",
        if prefer_live_directory {
            "live-dir"
        } else {
            "runtime"
        }
    );
    println!("fleet_record={fleet_record_index_1_based}");
    println!("fleet_id={}", fleet.fleet_id());
    println!(
        "coords={},{}",
        fleet.current_location_coords_raw()[0],
        fleet.current_location_coords_raw()[1]
    );
    println!(
        "target={},{}",
        fleet.standing_order_target_coords_raw()[0],
        fleet.standing_order_target_coords_raw()[1]
    );
    println!("order={}", fleet.standing_order_kind().as_str());
    println!("order_code={}", fleet.standing_order_code_raw());
    println!("current_speed={}", fleet.current_speed());
    println!("max_speed={}", fleet.max_speed());
    println!("roe={}", fleet.rules_of_engagement());
    println!("mission_aux={:02x?}", fleet.mission_aux_bytes());
    println!("x_fraction={:02x?}", fleet.x_fraction().to_bytes());
    println!("depth={:02x?}", fleet.depth().to_bytes());
    println!(
        "route_steps={}",
        route.as_ref().map(|route| route.steps.len()).unwrap_or(0)
    );
    println!("route={route_coords}");
    match eta {
        FleetEtaEstimate::Arrived => {
            println!("eta_status=arrived");
            println!("eta_years=0");
        }
        FleetEtaEstimate::Stopped => println!("eta_status=stopped"),
        FleetEtaEstimate::Unreachable => println!("eta_status=unreachable"),
        FleetEtaEstimate::Years(years) => {
            println!("eta_status=years");
            println!("eta_years={years}");
            println!(
                "arrival_year={}",
                data.conquest.game_year().saturating_add(years)
            );
        }
    }

    Ok(())
}

pub(crate) fn inspect_classic_login(
    dir: &Path,
    caller_alias: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let data = load_runtime_game_data(dir)?;

    println!("Directory: {}", dir.display());
    println!("Caller alias: {caller_alias}");
    println!("Classic login compatibility:");

    for (idx, player) in data.player.records.iter().enumerate() {
        let slot = idx + 1;
        let handle = player.assigned_player_handle_summary();
        let empire = player.controlled_empire_name_summary();
        let campaign_state = data
            .empire_campaign_state(slot as u8)
            .map(|state| state.as_str())
            .unwrap_or("unknown");
        let homeworld = login_homeworld_like_planet(&data, slot as u8, player);
        let homeworld_name = homeworld
            .map(PlanetRecord::planet_name)
            .unwrap_or_else(|| "<unknown>".to_string());
        let classification = classify_classic_login_state(&data, slot as u8, caller_alias);

        println!(
            "  slot {}: classification={} handle='{}' empire='{}' campaign_state={} homeworld='{}' reports_pending={} messages_pending={} message_review={:04x}/{:04x} results_review={:04x}/{:04x} results_chain={:04x}/{:04x}",
            slot,
            classification,
            handle,
            empire,
            campaign_state,
            homeworld_name,
            player.classic_reports_pending_flag_raw(),
            player.classic_messages_pending_flag_raw(),
            player.classic_message_review_word_raw(),
            player.classic_message_review_carry_word_raw(),
            player.classic_results_review_word_raw(),
            player.classic_results_review_carry_word_raw(),
            player.classic_results_first_header_raw(),
            player.classic_results_last_header_raw()
        );
    }

    Ok(())
}

pub(crate) fn dump_headers(dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let game_data = load_runtime_game_data(dir)?;
    let setup = &game_data.setup;
    let conquest = &game_data.conquest;

    println!("Directory: {}", dir.display());
    println!(
        "SETUP.version={}",
        String::from_utf8_lossy(setup.version_tag())
    );
    println!("SETUP.option_prefix={:02x?}", setup.option_prefix());
    println!(
        "SETUP.com_irqs=[{}, {}, {}, {}]",
        setup.com_irq_raw(0).unwrap_or_default(),
        setup.com_irq_raw(1).unwrap_or_default(),
        setup.com_irq_raw(2).unwrap_or_default(),
        setup.com_irq_raw(3).unwrap_or_default()
    );
    println!(
        "SETUP.com_flow_control=[{}, {}, {}, {}]",
        setup.com_hardware_flow_control_enabled(0).unwrap_or(false),
        setup.com_hardware_flow_control_enabled(1).unwrap_or(false),
        setup.com_hardware_flow_control_enabled(2).unwrap_or(false),
        setup.com_hardware_flow_control_enabled(3).unwrap_or(false)
    );
    println!("SETUP.snoop_enabled={}", setup.snoop_enabled());
    println!(
        "SETUP.local_timeout_enabled={}",
        setup.local_timeout_enabled()
    );
    println!(
        "SETUP.remote_timeout_enabled={}",
        setup.remote_timeout_enabled()
    );
    println!(
        "SETUP.max_time_between_keys_minutes_raw={}",
        setup.max_time_between_keys_minutes_raw()
    );
    println!(
        "SETUP.minimum_time_granted_minutes_raw={}",
        setup.minimum_time_granted_minutes_raw()
    );
    println!(
        "SETUP.purge_after_turns_raw={}",
        setup.purge_after_turns_raw()
    );
    println!(
        "SETUP.autopilot_inactive_turns_raw={}",
        setup.autopilot_inactive_turns_raw()
    );
    println!("CONQUEST.game_year={}", conquest.game_year());
    println!("CONQUEST.player_count={}", conquest.player_count());
    println!(
        "CONQUEST.player_config_word={:04x}",
        conquest.player_config_word()
    );
    println!(
        "CONQUEST.maintenance_schedule={:02x?}",
        conquest.maintenance_schedule_bytes()
    );
    println!("CONQUEST.header_len={}", conquest.control_header().len());
    println!("CONQUEST.header_words={:04x?}", conquest.header_words());

    Ok(())
}

fn print_header_summary(setup: &SetupDat, conquest: &ConquestDat) {
    println!(
        "SETUP version: {}",
        String::from_utf8_lossy(setup.version_tag())
    );
    println!("SETUP option prefix: {:02x?}", setup.option_prefix());
    println!(
        "SETUP COM IRQs: [{}, {}, {}, {}]",
        setup.com_irq_raw(0).unwrap_or_default(),
        setup.com_irq_raw(1).unwrap_or_default(),
        setup.com_irq_raw(2).unwrap_or_default(),
        setup.com_irq_raw(3).unwrap_or_default()
    );
    println!(
        "SETUP COM flow control: [{}, {}, {}, {}]",
        yes_no(setup.com_hardware_flow_control_enabled(0).unwrap_or(false)),
        yes_no(setup.com_hardware_flow_control_enabled(1).unwrap_or(false)),
        yes_no(setup.com_hardware_flow_control_enabled(2).unwrap_or(false)),
        yes_no(setup.com_hardware_flow_control_enabled(3).unwrap_or(false))
    );
    println!(
        "SETUP snoop enabled: {}",
        if setup.snoop_enabled() { "yes" } else { "no" }
    );
    println!(
        "SETUP local timeout enabled: {}",
        if setup.local_timeout_enabled() {
            "yes"
        } else {
            "no"
        }
    );
    println!(
        "SETUP remote timeout enabled: {}",
        if setup.remote_timeout_enabled() {
            "yes"
        } else {
            "no"
        }
    );
    println!(
        "SETUP max time between keys (raw minutes): {}",
        setup.max_time_between_keys_minutes_raw()
    );
    println!(
        "SETUP minimum time granted (raw minutes): {}",
        setup.minimum_time_granted_minutes_raw()
    );
    println!(
        "SETUP purge after turns (raw): {}",
        setup.purge_after_turns_raw()
    );
    println!(
        "SETUP autopilot inactive turns (raw): {}",
        setup.autopilot_inactive_turns_raw()
    );
    println!("CONQUEST game year: {}", conquest.game_year());
    println!("CONQUEST player count: {}", conquest.player_count());
    println!(
        "CONQUEST maintenance schedule: {:02x?}",
        conquest.maintenance_schedule_bytes()
    );
    println!("CONQUEST header bytes: {}", conquest.control_header().len());
    println!(
        "CONQUEST first header words: {:04x?}",
        &conquest.header_words()[..8]
    );
}

fn classify_classic_login_state(
    data: &CoreGameData,
    empire_raw: u8,
    caller_alias: &str,
) -> &'static str {
    let Some(player) = data
        .player
        .records
        .get(empire_raw.saturating_sub(1) as usize)
    else {
        return "unknown";
    };

    if !player.is_active_human_player() {
        return "first-time-menu";
    }

    let handle = player.assigned_player_handle_summary();
    if !handle.eq_ignore_ascii_case(caller_alias) {
        return "first-time-menu";
    }

    let Some(homeworld) = login_homeworld_like_planet(data, empire_raw, player) else {
        return "returning-player";
    };

    if homeworld.is_named_homeworld_seed() {
        "matched-preloaded-first-login"
    } else {
        "returning-player"
    }
}

fn homeworld_planet<'a>(
    data: &'a CoreGameData,
    player: &nc_data::PlayerRecord,
) -> Option<&'a PlanetRecord> {
    let index = player.homeworld_planet_index_1_based_raw() as usize;
    if index == 0 {
        return None;
    }
    data.planets.records.get(index - 1)
}

fn login_homeworld_like_planet<'a>(
    data: &'a CoreGameData,
    empire_raw: u8,
    player: &nc_data::PlayerRecord,
) -> Option<&'a PlanetRecord> {
    homeworld_planet(data, player).or_else(|| {
        data.planets.records.iter().find(|planet| {
            planet.owner_empire_slot_raw() == empire_raw && planet.is_named_homeworld_seed()
        })
    })
}

fn yes_no(value: bool) -> &'static str {
    if value { "Yes" } else { "No" }
}
