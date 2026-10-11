use std::fs;
use std::path::Path;

use crate::commands::runtime::load_runtime_game_data;
use nc_compat::classic::encode_planet;
use nc_compat::{inspect_classic_messages_dat, load_classic_core_files};
use nc_data::{
    Conquest, CoreGameData, Planet, PlayerMode, Setup, is_named_homeworld_seed, order_summary,
    ownership_summary, planet_derived_summary, ship_composition_summary,
};
use nc_engine::{FleetEtaEstimate, estimate_fleet_eta, plan_route};

pub(crate) fn inspect_dir(dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let data = load_runtime_game_data(dir)?;

    println!("Directory: {}", dir.display());
    print_header_summary(&data.setup, &data.conquest);
    println!();

    println!("Players:");
    for (idx, record) in data.players.iter().enumerate() {
        let campaign_state = data
            .empire_campaign_state((idx + 1) as u8)
            .map(|state| state.as_str())
            .unwrap_or("unknown");
        println!(
            "  slot {}: owner_mode={} assigned_player_flag={} tax={} last_run_year={} autopilot={} reports_pending={} messages_pending={} message_review={:04x}/{:04x} results_review={:04x}/{:04x} results_chain={:04x}/{:04x} campaign_state={} summary={}",
            idx + 1,
            record.mode.to_raw(),
            record.mode.to_raw(),
            record.tax_rate,
            record.last_year_on,
            u8::from(record.autopilot),
            record.messages_last as u8,
            record.messages_first as u8,
            record.messages_first as u16,
            (record.messages_first >> 16) as u16,
            record.messages_last as u16,
            (record.messages_last >> 16) as u16,
            record.results_first as u16,
            record.results_last as u16,
            campaign_state,
            ownership_summary(record)
        );
        println!("    first_base_record_raw={}", record.bases_first.0);
    }
    println!();

    println!("Planets:");
    for (idx, record) in data.planets.iter().enumerate().take(5) {
        let bytes = encode_planet(record);
        println!(
            "  planet {:02}: coords={:?} hdr={:02x?} len={} text='{}' tail58={:02x?} fact_word={:04x} summary='{}'",
            idx + 1,
            record.sector.to_array(),
            [
                record.sector.x,
                record.sector.y,
                record.potential_production
            ],
            record.name.chars().count() as u8,
            record.name.trim(),
            &bytes[0x58..0x61],
            u16::from_le_bytes([bytes[0x08], bytes[0x09]]),
            planet_derived_summary(record)
        );
    }
    println!("  ... {} total planet records", data.planets.len());

    let homeworld_like = data
        .planets
        .iter()
        .enumerate()
        .filter(|(_, record)| is_named_homeworld_seed(record))
        .collect::<Vec<_>>();
    if !homeworld_like.is_empty() {
        println!();
        println!("Planet Seeds:");
        for (idx, record) in homeworld_like {
            println!(
                "  planet {:02}: summary='{}' potential={}",
                idx + 1,
                planet_derived_summary(record),
                u16::from(record.potential_production)
            );
        }
    }

    println!();
    println!("Fleets:");
    let fleet_display_count = data.fleets.len().min(16);
    for (idx, record) in data.fleets.iter().enumerate().take(fleet_display_count) {
        println!(
            "  fleet {:02}: id={} slot={} prev={} next={} cur_spd={} max_spd={} roe={} ships={} loc_raw={:02x?} order={}({}) target_raw={:02x?} summary='{}'",
            idx + 1,
            record.record.0,
            record.fleet_number.0 as u8,
            record.prev.0,
            record.next.0,
            record.speed,
            record.max_speed,
            record.rules_of_engagement,
            ship_composition_summary(record),
            record.sector.to_array(),
            record.order.as_str(),
            record.order.to_raw(),
            record.target.to_array(),
            order_summary(record)
        );
        println!(
            "    mission_aux={:02x?}",
            [record.target_depth, record.mission_target as u8]
        );
    }
    if data.fleets.len() > fleet_display_count {
        println!("  ... {} total fleet records", data.fleets.len());
    }

    let looks_like_initialized_blocks = !data.fleets.is_empty()
        && data.fleets.len() % 4 == 0
        && data.fleets.chunks_exact(4).all(|group| {
            group
                .iter()
                .map(|r| r.fleet_number.0 as u8)
                .eq([1, 2, 3, 4])
        });

    if looks_like_initialized_blocks {
        println!();
        println!("Fleet Groups:");
        for (group_idx, group) in data.fleets.chunks_exact(4).enumerate() {
            let home = group[0].sector.to_array();
            println!(
                "  empire block {}: loc_raw={:02x?} target_raw={:02x?}",
                group_idx + 1,
                home,
                group[0].target.to_array()
            );
            for record in group {
                println!(
                    "    id={} slot={} ships={} max_spd={} order={} summary='{}'",
                    record.record.0,
                    record.fleet_number.0 as u8,
                    ship_composition_summary(record),
                    record.max_speed,
                    record.order.as_str(),
                    order_summary(record)
                );
            }
        }
    }

    println!();
    println!("Bases:");
    for (idx, record) in data.bases.iter().enumerate() {
        println!(
            "  base {:02}: escort={} id={} owner={} next={} record={} prev={} coords={:02x?} moving={} target={:02x?} target_depth={}",
            idx + 1,
            record.escort.0,
            record.base_id.0,
            record.owner.0,
            record.next.0,
            record.record.0,
            record.prev.0,
            record.sector.to_array(),
            u8::from(record.moving),
            record.target.to_array(),
            record.target_depth
        );
    }

    println!();
    println!("IPBM:");
    for (idx, record) in data.ipbms.iter().enumerate() {
        println!(
            "  record {:02}: number={} owner={} next={} record_number={}",
            idx + 1,
            record.ipbm_number,
            record.owner.0,
            record.next.0,
            record.record.0
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
        load_classic_core_files(dir)?
    } else {
        load_runtime_game_data(dir)?
    };
    let Some(fleet) = data.fleets.get(fleet_record_index_1_based - 1) else {
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
    println!("fleet_id={}", fleet.record.0);
    println!(
        "coords={},{}",
        fleet.sector.to_array()[0],
        fleet.sector.to_array()[1]
    );
    println!(
        "target={},{}",
        fleet.target.to_array()[0],
        fleet.target.to_array()[1]
    );
    println!("order={}", fleet.order.as_str());
    println!("order_code={}", fleet.order.to_raw());
    println!("current_speed={}", fleet.speed);
    println!("max_speed={}", fleet.max_speed);
    println!("roe={}", fleet.rules_of_engagement);
    println!(
        "mission_aux={:02x?}",
        [fleet.target_depth, fleet.mission_target as u8]
    );
    println!("x_fraction={:02x?}", fleet.x_fraction.to_bytes());
    println!("depth={:02x?}", fleet.depth.to_bytes());
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
                data.conquest.game_year.saturating_add(years)
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

    for (idx, player) in data.players.iter().enumerate() {
        let slot = idx + 1;
        let handle = player.handle.trim().to_string();
        let empire = player.empire_name.trim().to_string();
        let campaign_state = data
            .empire_campaign_state(slot as u8)
            .map(|state| state.as_str())
            .unwrap_or("unknown");
        let homeworld = login_homeworld_like_planet(&data, slot as u8, player);
        let homeworld_name = homeworld
            .map(|planet| planet.name.clone())
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
            player.messages_last as u8,
            player.messages_first as u8,
            player.messages_first as u16,
            (player.messages_first >> 16) as u16,
            player.messages_last as u16,
            (player.messages_last >> 16) as u16,
            player.results_first as u16,
            player.results_last as u16
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
        String::from_utf8_lossy(setup.version_tag.as_slice())
    );
    println!("SETUP.option_prefix={:02x?}", option_prefix(setup));
    println!(
        "SETUP.com_irqs=[{}, {}, {}, {}]",
        setup.com_irq.first().copied().unwrap_or_default(),
        setup.com_irq.get(1).copied().unwrap_or_default(),
        setup.com_irq.get(2).copied().unwrap_or_default(),
        setup.com_irq.get(3).copied().unwrap_or_default()
    );
    println!(
        "SETUP.com_flow_control=[{}, {}, {}, {}]",
        setup.com_hw_flow_control.first().copied().unwrap_or(false),
        setup.com_hw_flow_control.get(1).copied().unwrap_or(false),
        setup.com_hw_flow_control.get(2).copied().unwrap_or(false),
        setup.com_hw_flow_control.get(3).copied().unwrap_or(false)
    );
    println!("SETUP.snoop_enabled={}", setup.snoop);
    println!("SETUP.local_timeout_enabled={}", setup.local_timeout);
    println!("SETUP.remote_timeout_enabled={}", setup.remote_timeout);
    println!(
        "SETUP.max_time_between_keys_minutes_raw={}",
        u8::try_from(setup.max_key_gap_minutes).unwrap_or(u8::MAX)
    );
    println!(
        "SETUP.minimum_time_granted_minutes_raw={}",
        setup.minimum_time_minutes
    );
    println!(
        "SETUP.purge_after_turns_raw={}",
        u8::try_from(setup.purge_after_turns).unwrap_or(u8::MAX)
    );
    println!(
        "SETUP.autopilot_inactive_turns_raw={}",
        u8::try_from(setup.autopilot_after_turns).unwrap_or(u8::MAX)
    );
    println!("CONQUEST.game_year={}", conquest.game_year);
    println!("CONQUEST.player_count={}", conquest.player_count);
    println!(
        "CONQUEST.maintenance_schedule={:02x?}",
        conquest.maintenance_days.0.map(u8::from)
    );
    let empires = usize::from(conquest.player_count).min(25);
    println!(
        "CONQUEST.previous_production={:?}",
        &conquest.previous_production[..empires]
    );
    println!(
        "CONQUEST.previous_planet_count={:?}",
        &conquest.previous_planet_count[..empires]
    );

    Ok(())
}

/// SETUP.DAT bytes 5..12 as ECUTIL writes them: the COM IRQs, then the
/// COM flow-control flags.
fn option_prefix(setup: &Setup) -> [u8; 8] {
    let mut prefix = [0u8; 8];
    prefix[..4].copy_from_slice(&setup.com_irq);
    for (byte, enabled) in prefix[4..].iter_mut().zip(setup.com_hw_flow_control) {
        *byte = u8::from(enabled);
    }
    prefix
}

fn print_header_summary(setup: &Setup, conquest: &Conquest) {
    println!(
        "SETUP version: {}",
        String::from_utf8_lossy(setup.version_tag.as_slice())
    );
    println!("SETUP option prefix: {:02x?}", option_prefix(setup));
    println!(
        "SETUP COM IRQs: [{}, {}, {}, {}]",
        setup.com_irq.first().copied().unwrap_or_default(),
        setup.com_irq.get(1).copied().unwrap_or_default(),
        setup.com_irq.get(2).copied().unwrap_or_default(),
        setup.com_irq.get(3).copied().unwrap_or_default()
    );
    println!(
        "SETUP COM flow control: [{}, {}, {}, {}]",
        yes_no(setup.com_hw_flow_control.first().copied().unwrap_or(false)),
        yes_no(setup.com_hw_flow_control.get(1).copied().unwrap_or(false)),
        yes_no(setup.com_hw_flow_control.get(2).copied().unwrap_or(false)),
        yes_no(setup.com_hw_flow_control.get(3).copied().unwrap_or(false))
    );
    println!(
        "SETUP snoop enabled: {}",
        if setup.snoop { "yes" } else { "no" }
    );
    println!(
        "SETUP local timeout enabled: {}",
        if setup.local_timeout { "yes" } else { "no" }
    );
    println!(
        "SETUP remote timeout enabled: {}",
        if setup.remote_timeout { "yes" } else { "no" }
    );
    println!(
        "SETUP max time between keys (raw minutes): {}",
        u8::try_from(setup.max_key_gap_minutes).unwrap_or(u8::MAX)
    );
    println!(
        "SETUP minimum time granted (raw minutes): {}",
        setup.minimum_time_minutes
    );
    println!(
        "SETUP purge after turns (raw): {}",
        u8::try_from(setup.purge_after_turns).unwrap_or(u8::MAX)
    );
    println!(
        "SETUP autopilot inactive turns (raw): {}",
        u8::try_from(setup.autopilot_after_turns).unwrap_or(u8::MAX)
    );
    println!("CONQUEST game year: {}", conquest.game_year);
    println!("CONQUEST player count: {}", conquest.player_count);
    println!(
        "CONQUEST maintenance schedule: {:02x?}",
        conquest.maintenance_days.0.map(u8::from)
    );
    let empires = usize::from(conquest.player_count).min(25);
    println!(
        "CONQUEST previous production: {:?}",
        &conquest.previous_production[..empires]
    );
    println!(
        "CONQUEST previous planet count: {:?}",
        &conquest.previous_planet_count[..empires]
    );
}

fn classify_classic_login_state(
    data: &CoreGameData,
    empire_raw: u8,
    caller_alias: &str,
) -> &'static str {
    let Some(player) = data.players.get(empire_raw.saturating_sub(1) as usize) else {
        return "unknown";
    };

    if player.mode != PlayerMode::Human {
        return "first-time-menu";
    }

    let handle = player.handle.trim().to_string();
    if !handle.eq_ignore_ascii_case(caller_alias) {
        return "first-time-menu";
    }

    let Some(homeworld) = login_homeworld_like_planet(data, empire_raw, player) else {
        return "returning-player";
    };

    if is_named_homeworld_seed(homeworld) {
        "matched-preloaded-first-login"
    } else {
        "returning-player"
    }
}

fn homeworld_planet<'a>(data: &'a CoreGameData, player: &nc_data::Player) -> Option<&'a Planet> {
    let index = player.homeworld as usize;
    if index == 0 {
        return None;
    }
    data.planets.get(index - 1)
}

fn login_homeworld_like_planet<'a>(
    data: &'a CoreGameData,
    empire_raw: u8,
    player: &nc_data::Player,
) -> Option<&'a Planet> {
    homeworld_planet(data, player).or_else(|| {
        data.planets
            .iter()
            .find(|planet| planet.owner.0 == empire_raw && is_named_homeworld_seed(planet))
    })
}

fn yes_no(value: bool) -> &'static str {
    if value { "Yes" } else { "No" }
}
