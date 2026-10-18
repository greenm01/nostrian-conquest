use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use nc_data::{
    CampaignStore, Coords, CoreGameData, FleetDetachSelection, IntelTier, Planet,
    PlanetIntelSnapshot, ProductionItemKind, QueuedPlayerMail, Real48, ReportBlockRow,
    merge_player_intel_from_runtime, present_production_points, recompute_max_speed,
    set_armies_aboard, set_planet_name, set_troop_transports, stardock_slot_kind,
    stored_production_points, troop_transports,
};

use crate::commands::runtime::{
    load_runtime_intel_by_viewer, load_runtime_state_preferring_live_directory,
};
use crate::support::paths::resolve_repo_path;

const DESIRED_OWNED_PLANET_TOTALS: [usize; 12] = [15, 4, 4, 4, 4, 4, 3, 3, 3, 3, 3, 3];

const PLAYER_ONE_PLANET_NAMES: [&str; 15] = [
    "Aurora Prime",
    "Meridian Gate",
    "Forge Delta",
    "Hinterlight",
    "Bastion Reach",
    "Northwatch",
    "Southwatch",
    "Anvil Rest",
    "Lantern",
    "Cobalt Rise",
    "Quiet Current",
    "Harbor Nine",
    "Signal Keep",
    "Pillar",
    "Thornhold",
];

const P1_DETACH_SPECS: [(u16, u16, u16, u16, u16, u16, u16, u8); 24] = [
    (0, 1, 1, 0, 0, 1, 0, 2),
    (0, 1, 0, 0, 0, 1, 0, 3),
    (1, 0, 0, 0, 0, 0, 0, 4),
    (0, 1, 1, 0, 0, 0, 0, 5),
    (1, 1, 0, 0, 0, 0, 0, 6),
    (0, 0, 2, 0, 0, 0, 0, 7),
    (0, 0, 1, 1, 0, 0, 0, 8),
    (0, 1, 0, 1, 0, 0, 0, 9),
    (0, 0, 0, 0, 1, 0, 0, 10),
    (0, 0, 0, 1, 0, 1, 0, 11),
    (0, 0, 0, 1, 0, 0, 1, 12),
    (1, 0, 1, 0, 0, 0, 0, 13),
    (0, 2, 0, 0, 0, 0, 0, 14),
    (0, 1, 0, 0, 1, 0, 0, 15),
    (1, 0, 0, 0, 1, 0, 0, 1),
    (0, 0, 1, 1, 0, 1, 0, 2),
    (0, 1, 1, 0, 0, 1, 0, 3),
    (1, 0, 0, 1, 0, 0, 0, 4),
    (0, 0, 0, 0, 0, 2, 0, 5),
    (0, 0, 0, 0, 0, 1, 1, 6),
    (0, 0, 1, 0, 0, 0, 1, 7),
    (0, 1, 0, 0, 0, 0, 1, 8),
    (1, 0, 0, 0, 0, 1, 0, 9),
    (0, 0, 1, 0, 1, 0, 0, 10),
];

const OTHER_DETACH_SPECS: [(u16, u16, u16, u16, u16, u16, u16, u8); 2] =
    [(0, 1, 0, 0, 0, 0, 0, 3), (0, 0, 1, 0, 0, 0, 0, 5)];

const PLAYER_ONE_STARDOCK_SPECS: [(usize, u8, u16); 10] = [
    (1, 1, 4),
    (1, 2, 2),
    (1, 5, 3),
    (13, 9, 1),
    (14, 9, 1),
    (15, 9, 1),
    (16, 3, 1),
    (16, 5, 2),
    (16, 6, 1),
    (17, 4, 4),
];

const FOREIGN_INTEL_STARDOCK_SPECS: [(usize, u8, u16); 5] =
    [(2, 1, 2), (5, 5, 2), (9, 4, 3), (24, 2, 1), (31, 3, 1)];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Player1TuiStressSeedReport {
    pub planets: usize,
    pub fleets: usize,
    pub report_blocks: usize,
    pub player1_mail: usize,
    pub player1_full_intel: usize,
    pub player1_partial_intel: usize,
    pub commissioned_starbases: usize,
}

pub(crate) fn run_seed_player1_tui_stress_args(
    args: Vec<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    let dir = parse_dir_only(args)?;
    let report = seed_player1_tui_stress(&dir)?;
    println!(
        "Seeded player-1 TUI stress runtime state at {}.",
        dir.display()
    );
    println!("  planets={}", report.planets);
    println!("  fleets={}", report.fleets);
    println!("  report_blocks={}", report.report_blocks);
    println!("  player1_mail={}", report.player1_mail);
    println!("  player1_full_intel={}", report.player1_full_intel);
    println!("  player1_partial_intel={}", report.player1_partial_intel);
    println!("  commissioned_starbases={}", report.commissioned_starbases);
    Ok(())
}

fn parse_dir_only(args: Vec<String>) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let mut dir = None;
    let mut remaining = args.into_iter();
    while let Some(arg) = remaining.next() {
        match arg.as_str() {
            "--dir" => {
                let Some(value) = remaining.next() else {
                    return Err("missing path after --dir".into());
                };
                dir = Some(resolve_repo_path(&value));
            }
            other => return Err(format!("unknown harness argument: {other}").into()),
        }
    }
    dir.ok_or_else(|| "harness seed-player1-tui-stress requires --dir <campaign_dir>".into())
}

pub(crate) fn seed_player1_tui_stress(
    dir: &Path,
) -> Result<Player1TuiStressSeedReport, Box<dyn std::error::Error>> {
    let store = CampaignStore::open_default_in_dir(dir)?;
    let mut state = load_runtime_state_preferring_live_directory(dir, &store)?;
    let mut planet_intel_by_viewer = load_runtime_intel_by_viewer(&store, &state.game_data)?;
    let game_year = state.game_year;
    let player_count = state.game_data.conquest.player_count as usize;
    let homeworld_coords = state
        .game_data
        .planets
        .iter()
        .take(player_count)
        .map(|planet| planet.sector.to_array())
        .collect::<Vec<_>>();

    configure_planets(&mut state.game_data)?;
    let player_one_owned_coords = player_owned_planet_coords(&state.game_data, 1);
    configure_player_one_fleets(
        &mut state.game_data,
        &homeworld_coords,
        &player_one_owned_coords,
    )?;
    configure_other_player_fleets(&mut state.game_data, &homeworld_coords)?;
    let commissioned_starbases = commission_player_one_starbases(&mut state.game_data)?;

    for viewer_empire_id in 1..=state.game_data.conquest.player_count {
        let viewer_idx = viewer_empire_id.saturating_sub(1) as usize;
        let previous = planet_intel_by_viewer
            .get(viewer_idx)
            .cloned()
            .unwrap_or_default();
        planet_intel_by_viewer[viewer_idx] = merge_player_intel_from_runtime(
            &state.game_data,
            viewer_empire_id,
            game_year,
            Some(&previous),
            None,
        );
    }
    let intel_summary =
        seed_player_one_intel(&state.game_data, &mut planet_intel_by_viewer, game_year)?;
    replace_player_one_reports(&mut state, game_year);
    replace_player_one_mail(&mut state, game_year);

    store.save_year_with_intel(
        game_year,
        &state.game_data,
        &state.report_block_rows,
        &state.queued_mail,
        &planet_intel_by_viewer,
    )?;

    Ok(Player1TuiStressSeedReport {
        planets: state.game_data.planets.len(),
        fleets: state.game_data.fleets.len(),
        report_blocks: state.report_block_rows.len(),
        player1_mail: state
            .queued_mail
            .iter()
            .filter(|mail| mail.recipient_empire_id == 1 && !mail.recipient_deleted)
            .count(),
        player1_full_intel: intel_summary.full,
        player1_partial_intel: intel_summary.partial,
        commissioned_starbases,
    })
}

#[derive(Debug, Clone)]
struct PlanetPayload {
    name: String,
    potential: u8,
    present: u16,
    stored: u32,
    armies: u8,
    batteries: u8,
}

fn configure_planets(game_data: &mut CoreGameData) -> Result<(), Box<dyn std::error::Error>> {
    let total_planets = game_data.planets.len();
    let player_count = game_data.conquest.player_count;
    let owners = ownership_sequence_for_players(player_count, total_planets);
    let empire_short_names = game_data
        .players
        .iter()
        .map(|record| {
            let empire = record.empire_name.trim().to_string();
            empire
                .split_whitespace()
                .next()
                .filter(|name| !name.is_empty())
                .unwrap_or("Empire")
                .to_string()
        })
        .collect::<Vec<_>>();
    let mut seen_per_owner = BTreeMap::<u8, usize>::new();

    for (idx, planet) in game_data.planets.iter_mut().enumerate() {
        let owner = owners[idx];
        let ordinal = seen_per_owner.entry(owner).or_insert(0usize);
        let payload = planet_payload(&empire_short_names, owner, idx + 1, *ordinal);
        *ordinal += 1;

        planet.owner.0 = owner;
        planet.years_under_owner = if owner == 0 { 0 } else { 2 };
        set_planet_name(planet, &payload.name);
        planet.potential_production = payload.potential;
        planet.production = Real48::from_f64(f64::from(payload.present));
        planet.stored_goods = Real48::from_f64(f64::from(payload.stored));
        planet.armies = u16::from(payload.armies);
        planet.ground_batteries = u16::from(payload.batteries);
        clear_planet_build_queue(planet);
        clear_planet_stardock(planet);
    }

    apply_player_owned_stardock_specs(game_data, 1, &PLAYER_ONE_STARDOCK_SPECS);
    apply_stardock_specs(game_data, &FOREIGN_INTEL_STARDOCK_SPECS);
    Ok(())
}

fn clear_planet_build_queue(planet: &mut Planet) {
    for slot in 0..10 {
        planet.build_queue[slot].points_owed = 0;
        planet.build_queue[slot].kind = ProductionItemKind::Unknown(0);
    }
}

fn clear_planet_stardock(planet: &mut Planet) {
    for slot in 0..nc_data::STARDOCK_SLOT_COUNT {
        planet.stardock[slot] = 0;
    }
}

fn apply_stardock_specs(game_data: &mut CoreGameData, specs: &[(usize, u8, u16)]) {
    let total_planets = game_data.planets.len();
    for (record_index_1_based, kind, count) in specs {
        let Some(slot) = nc_data::stardock_slot_for_kind(*kind) else {
            continue;
        };
        if *record_index_1_based > total_planets {
            continue;
        }
        let planet = &mut game_data.planets[*record_index_1_based - 1];
        planet.stardock[slot] = u32::from(*count);
    }
}

fn apply_player_owned_stardock_specs(
    game_data: &mut CoreGameData,
    owner_empire_id: u8,
    specs: &[(usize, u8, u16)],
) {
    let owned_planet_records = game_data
        .planets
        .iter()
        .enumerate()
        .filter(|(_, planet)| planet.owner.0 == owner_empire_id)
        .map(|(idx, _)| idx + 1)
        .collect::<Vec<_>>();
    for (owned_ordinal_1_based, kind, count) in specs {
        let Some(&record_index_1_based) = owned_planet_records.get(owned_ordinal_1_based - 1)
        else {
            continue;
        };
        let Some(slot) = nc_data::stardock_slot_for_kind(*kind) else {
            continue;
        };
        let planet = &mut game_data.planets[record_index_1_based - 1];
        planet.stardock[slot] = u32::from(*count);
    }
}

fn ownership_sequence_for_players(player_count: u8, total_planets: usize) -> Vec<u8> {
    let player_count = player_count as usize;
    let mut totals = vec![1usize; player_count];
    let mut remaining = total_planets.saturating_sub(player_count);

    for total in totals.iter_mut().skip(1) {
        if remaining == 0 {
            break;
        }
        *total += 1;
        remaining -= 1;
    }

    let mut progress = true;
    while remaining > 0 && progress {
        progress = false;
        for idx in 0..player_count {
            if totals[idx] < desired_owned_planet_total(idx) {
                totals[idx] += 1;
                remaining -= 1;
                progress = true;
                if remaining == 0 {
                    break;
                }
            }
        }
    }

    let mut owners = (1..=player_count as u8).collect::<Vec<_>>();
    for (idx, total) in totals.into_iter().enumerate() {
        owners.extend(std::iter::repeat_n(
            (idx + 1) as u8,
            total.saturating_sub(1),
        ));
    }
    owners.resize(total_planets, 0);
    owners
}

fn desired_owned_planet_total(player_index_zero_based: usize) -> usize {
    DESIRED_OWNED_PLANET_TOTALS
        .get(player_index_zero_based)
        .copied()
        .unwrap_or(2)
}

fn planet_payload(
    empire_short_names: &[String],
    owner: u8,
    record_index_1_based: usize,
    ordinal_for_owner: usize,
) -> PlanetPayload {
    if owner == 1 {
        let name = player_one_planet_name(ordinal_for_owner).to_string();
        let potential = (160u16 - ((record_index_1_based * 9 + ordinal_for_owner * 7) % 54) as u16)
            .max(78) as u8;
        let present =
            potential as u16 - ((record_index_1_based * 5 + ordinal_for_owner * 3) % 24) as u16;
        return PlanetPayload {
            name,
            potential,
            present: present.max(40),
            stored: (12 + ((record_index_1_based * 11 + ordinal_for_owner * 9) % 70)) as u32,
            armies: (8 + ((record_index_1_based + ordinal_for_owner * 2) % 28)) as u8,
            batteries: (3 + ((record_index_1_based + ordinal_for_owner) % 8)) as u8,
        };
    }

    if owner == 0 {
        return PlanetPayload {
            name: generic_planet_name(
                empire_short_names,
                owner,
                ordinal_for_owner,
                record_index_1_based,
            ),
            potential: (30 + ((record_index_1_based * 7) % 40)) as u8,
            present: 0,
            stored: ((record_index_1_based * 3) % 10) as u32,
            armies: ((record_index_1_based * 2) % 5) as u8,
            batteries: (record_index_1_based % 3) as u8,
        };
    }

    let potential = (60 + ((record_index_1_based * 13 + owner as usize * 5) % 70)) as u8;
    let present = potential as u16 - ((record_index_1_based * 5 + owner as usize * 3) % 32) as u16;
    PlanetPayload {
        name: generic_planet_name(
            empire_short_names,
            owner,
            ordinal_for_owner,
            record_index_1_based,
        ),
        potential,
        present: present.max(18),
        stored: (5 + ((record_index_1_based * 9 + owner as usize * 11) % 55)) as u32,
        armies: (4 + ((record_index_1_based + owner as usize * 2) % 20)) as u8,
        batteries: (1 + ((record_index_1_based + owner as usize) % 6)) as u8,
    }
}

fn player_one_planet_name(index: usize) -> &'static str {
    PLAYER_ONE_PLANET_NAMES
        .get(index)
        .copied()
        .unwrap_or("Aurora Colony")
}

fn generic_planet_name(
    empire_short_names: &[String],
    owner: u8,
    index: usize,
    record_index_1_based: usize,
) -> String {
    if owner == 0 {
        return format!("Frontier {record_index_1_based:02}");
    }
    let short = empire_short_names
        .get(owner as usize - 1)
        .map(String::as_str)
        .unwrap_or("Empire");
    format!("{short} {}", index + 1)
}

fn player_owned_planet_coords(game_data: &CoreGameData, owner: u8) -> Vec<[u8; 2]> {
    game_data
        .planets
        .iter()
        .filter(|planet| planet.owner.0 == owner)
        .map(|planet| planet.sector.to_array())
        .collect()
}

fn configure_player_one_fleets(
    game_data: &mut CoreGameData,
    homeworld_coords: &[[u8; 2]],
    player_one_owned_coords: &[[u8; 2]],
) -> Result<(), Box<dyn std::error::Error>> {
    let home = homeworld_coords[0];
    let p2_home = homeworld_coords.get(1).copied().unwrap_or(home);
    let p3_home = homeworld_coords.get(2).copied().unwrap_or(home);
    let home_fleet_records = [1usize, 2, 3, 4];

    set_fleet_composition(&mut game_data.fleets[0], 30, 18, 24, 36, 18, 0, 10);
    set_fleet_composition(&mut game_data.fleets[1], 4, 0, 2, 6, 6, 0, 0);
    set_fleet_composition(&mut game_data.fleets[2], 2, 0, 1, 4, 4, 4, 0);
    set_fleet_composition(&mut game_data.fleets[3], 6, 4, 6, 8, 2, 0, 2);

    for fleet_record_index_1_based in home_fleet_records {
        set_fleet_location(game_data, fleet_record_index_1_based, home)?;
    }
    set_fleet_location_and_order(game_data, 1, home, 3, 14, p2_home)?;
    set_fleet_location_and_order(game_data, 2, home, 0, 0, home)?;
    set_fleet_location_and_order(game_data, 3, home, 0, 0, home)?;
    set_fleet_location_and_order(game_data, 4, home, 3, 10, p3_home)?;

    let order_cycle = [0u8, 1, 3, 14, 0];
    for (idx, spec) in P1_DETACH_SPECS.iter().enumerate() {
        let (bb, ca, dd, full_tt, empty_tt, scouts, etacs, roe) = *spec;
        let selection = FleetDetachSelection {
            battleships: bb,
            cruisers: ca,
            destroyers: dd,
            full_transports: 0,
            empty_transports: empty_tt + full_tt,
            scouts,
            etacs,
            ..FleetDetachSelection::default()
        };
        let result = game_data.detach_ships_to_new_fleet(1, 1, selection, Some(3), roe)?;
        let fleet_record_index_1_based = result.new_fleet_record_index_1_based;
        let coords = player_one_owned_coords[idx % player_one_owned_coords.len()];
        let target = player_one_owned_coords[(idx + 3) % player_one_owned_coords.len()];
        let order_code = order_cycle[idx % order_cycle.len()];
        let speed = if order_code == 0 {
            0
        } else {
            1 + (idx % 3) as u8
        };
        set_fleet_location(game_data, fleet_record_index_1_based, coords)?;
        set_fleet_location_and_order(
            game_data,
            fleet_record_index_1_based,
            coords,
            speed,
            order_code,
            if order_code == 0 { coords } else { target },
        )?;
    }

    Ok(())
}

fn configure_other_player_fleets(
    game_data: &mut CoreGameData,
    homeworld_coords: &[[u8; 2]],
) -> Result<(), Box<dyn std::error::Error>> {
    for player in 2..=game_data.conquest.player_count as usize {
        let donor_record_index_1_based = (player - 1) * 4 + 1;
        let home = homeworld_coords[player - 1];
        let scouts = 4 + (player as u16 % 3);
        let battleships = 3 + (player as u16 % 2);
        let cruisers = 6 + (player as u16 % 4);
        let destroyers = 8 + (player as u16 % 5);
        let transports = 2 + (player as u16 % 3);
        let etacs = player as u16 % 2;
        set_fleet_composition(
            &mut game_data.fleets[donor_record_index_1_based - 1],
            scouts,
            battleships,
            cruisers,
            destroyers,
            transports,
            0,
            etacs,
        );
        set_fleet_location(game_data, donor_record_index_1_based, home)?;
        set_fleet_location_and_order(game_data, donor_record_index_1_based, home, 0, 0, home)?;

        for (idx, spec) in OTHER_DETACH_SPECS.iter().enumerate() {
            let (bb, ca, dd, full_tt, empty_tt, scouts, etacs, roe) = *spec;
            let selection = FleetDetachSelection {
                battleships: bb,
                cruisers: ca,
                destroyers: dd,
                full_transports: full_tt,
                empty_transports: empty_tt,
                scouts,
                etacs,
                ..FleetDetachSelection::default()
            };
            let result = game_data.detach_ships_to_new_fleet(
                player,
                donor_record_index_1_based,
                selection,
                Some(2),
                roe,
            )?;
            let fleet_record_index_1_based = result.new_fleet_record_index_1_based;
            let order_code = if idx % 2 == 0 { 0 } else { 3 };
            let speed = if order_code == 0 { 0 } else { 2 };
            set_fleet_location(game_data, fleet_record_index_1_based, home)?;
            set_fleet_location_and_order(
                game_data,
                fleet_record_index_1_based,
                home,
                speed,
                order_code,
                home,
            )?;
        }
    }

    Ok(())
}

fn set_fleet_composition(
    fleet: &mut nc_data::Fleet,
    scouts: u16,
    battleships: u16,
    cruisers: u16,
    destroyers: u16,
    transports: u16,
    armies_loaded: u16,
    etacs: u16,
) {
    fleet.ships.scouts = scouts;
    fleet.ships.battleships = battleships;
    fleet.ships.cruisers = cruisers;
    fleet.ships.destroyers = destroyers;
    set_troop_transports(fleet, transports);
    set_armies_aboard(fleet, armies_loaded);
    fleet.ships.etacs = etacs;
    recompute_max_speed(fleet);
    if fleet.speed > fleet.max_speed {
        fleet.speed = fleet.max_speed;
    }
}

fn set_fleet_location(
    game_data: &mut CoreGameData,
    fleet_record_index_1_based: usize,
    coords: [u8; 2],
) -> Result<(), Box<dyn std::error::Error>> {
    let Some(fleet) = game_data.fleets.get_mut(fleet_record_index_1_based - 1) else {
        return Err(
            format!("fleet record index out of range: {fleet_record_index_1_based}").into(),
        );
    };
    fleet.sector = Coords::from_array(coords);
    Ok(())
}

fn set_fleet_location_and_order(
    game_data: &mut CoreGameData,
    fleet_record_index_1_based: usize,
    coords: [u8; 2],
    speed: u8,
    order_code: u8,
    target: [u8; 2],
) -> Result<(), Box<dyn std::error::Error>> {
    set_fleet_location(game_data, fleet_record_index_1_based, coords)?;
    game_data.set_fleet_order(
        fleet_record_index_1_based,
        speed,
        order_code,
        target,
        None,
        None,
    )?;
    Ok(())
}

fn replace_player_one_reports(state: &mut nc_data::CampaignRuntimeState, year: u16) {
    let previous_year = year.saturating_sub(1);
    let blocks = [
        format!(
            "YEAR {year} STRATEGIC REVIEW\n\
Fleet Command reports a heavy traffic year. Aurora border patrols are now spread across three active fronts.\n\
Planet Database refresh complete. Scout traffic from Y{previous_year} has been merged into the current intelligence net."
        ),
        "EMPIRE STATUS DIGEST\n\
Tax receipts remain stable at current rates.\n\
Multiple colonies now carry enough stored production to exercise commission, tax, and transport screens without empty-state shortcuts."
            .to_string(),
        "STARDOCK SUMMARY\n\
Aurora Prime, Meridian Gate, and Forge Delta each have non-trivial units waiting in stardock.\n\
Several starbase chassis were held back for local commissioning drills."
            .to_string(),
        "FLEET TRAFFIC REPORT\n\
Rapid-response groups now span idle, moving, escort, and survey roles.\n\
Several formations have mixed ship classes specifically to stress list and review output."
            .to_string(),
        "SCOUT INTELLIGENCE DIGEST\n\
Recent sweeps produced mixed-quality data on foreign worlds.\n\
Some entries are stale, some are partial, and some include full docked and orbit summaries."
            .to_string(),
        "TRANSPORT COMMAND NOTE\n\
Empty troop transports are orbiting Aurora Prime for immediate army loading tests.\n\
Loaded transports are staged nearby so unload paths are also ready."
            .to_string(),
        "STARBASE CONTROL REVIEW\n\
Player 1 now maintains active starbases on several colonies.\n\
Guard, review, and control surfaces should all open with live data."
            .to_string(),
        "MESSAGE TRAFFIC SUMMARY\n\
Diplomatic and logistics mail has been queued for Player 1 only.\n\
Unread message counts should be high enough to exercise list scrolling and detail review."
            .to_string(),
    ];

    state.report_block_rows = blocks
        .into_iter()
        .enumerate()
        .map(|(block_index, decoded_text)| ReportBlockRow {
            viewer_empire_id: 0,
            block_index,
            decoded_text,
            raw_bytes: None,
            recipient_deleted: false,
        })
        .collect();
}

fn replace_player_one_mail(state: &mut nc_data::CampaignRuntimeState, year: u16) {
    state
        .queued_mail
        .retain(|mail| mail.recipient_empire_id != 1);
    state.queued_mail.extend([
        queued_mail(2, year, "Border Watch", "Heavy drive wakes near the eastern lane. Keep scouts rotating through the corridor."),
        queued_mail(3, year, "Trade Offer", "If you hold the line this year, Vela merchants can move spare industrial stock through Aurora space."),
        queued_mail(4, year, "Intercept", "Recovered fragment mentions a transport convoy with minimal escort near the southern quadrant."),
        queued_mail(5, year, "Status Check", "Your last fleet inventory looked healthy. Confirm whether Meridian Gate still has spare destroyers in dock."),
        queued_mail(6, year, "Survey Packet", "Attached scout digest lists three colonies with fresh orbit and stardock observations."),
        queued_mail(7, year, "Standing Orders", "Keep troop transports empty until army transfer drills are complete."),
        queued_mail(8, year, "Route Advice", "The shorter lane is hotter. Use the longer western curve if you want the safer approach."),
        queued_mail(9, year, "Intel Exchange", "We can confirm one rival colony still shows only partial industrial output in the last sweep."),
        queued_mail(10, year, "Dockyard Alert", "Starbase frame delivery complete. Commission at your discretion once escorts are ready."),
        queued_mail(11, year, "Fleet Note", "Mixed escort groups are easier to track if you keep their orders staggered."),
        queued_mail(12, year, "Observer Report", "Several neutral systems remain only lightly classified. They are useful for stale-intel testing."),
    ]);
}

fn queued_mail(sender_empire_id: u8, year: u16, subject: &str, body: &str) -> QueuedPlayerMail {
    QueuedPlayerMail {
        sender_empire_id,
        recipient_empire_id: 1,
        year,
        subject: subject.to_string(),
        body: body.to_string(),
        recipient_deleted: false,
        sender_deleted: false,
        cancelled: false,
    }
}

fn commission_player_one_starbases(
    game_data: &mut CoreGameData,
) -> Result<usize, Box<dyn std::error::Error>> {
    let owned_planets = game_data
        .planets
        .iter()
        .enumerate()
        .filter(|(_, planet)| planet.owner.0 == 1)
        .map(|(idx, _)| idx + 1)
        .collect::<Vec<_>>();

    let mut commissioned = 0usize;
    for planet_index_1_based in owned_planets {
        let starbase_slots = {
            let planet = &game_data.planets[planet_index_1_based - 1];
            (0..nc_data::STARDOCK_SLOT_COUNT)
                .filter(|slot| {
                    stardock_slot_kind(planet, *slot).code() == 9
                        && u16::try_from(planet.stardock[*slot]).unwrap_or(u16::MAX) > 0
                })
                .collect::<Vec<_>>()
        };
        for slot in starbase_slots {
            game_data.commission_planet_stardock_slot(1, planet_index_1_based, slot)?;
            commissioned += 1;
        }
    }
    Ok(commissioned)
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct IntelSeedSummary {
    full: usize,
    partial: usize,
}

fn seed_player_one_intel(
    game_data: &CoreGameData,
    planet_intel_by_viewer: &mut [BTreeMap<usize, PlanetIntelSnapshot>],
    game_year: u16,
) -> Result<IntelSeedSummary, Box<dyn std::error::Error>> {
    let Some(player_one_intel) = planet_intel_by_viewer.get_mut(0) else {
        return Err("campaign has no player 1 intel row".into());
    };

    let foreign_owned = game_data
        .planets
        .iter()
        .enumerate()
        .filter(|(_, planet)| {
            let owner = planet.owner.0;
            owner >= 2 && owner <= game_data.conquest.player_count
        })
        .map(|(idx, planet)| (idx + 1, planet))
        .collect::<Vec<_>>();

    let mut summary = IntelSeedSummary::default();
    for (idx, (planet_record_index_1_based, planet)) in foreign_owned.into_iter().enumerate() {
        let stale_offset = (idx % 4) as u16 + 1;
        let observed_year = game_year.saturating_sub(stale_offset);
        let snapshot = if idx % 3 == 0 {
            summary.partial += 1;
            partial_snapshot(planet_record_index_1_based, planet, observed_year)
        } else {
            summary.full += 1;
            full_snapshot(
                game_data,
                planet_record_index_1_based,
                planet,
                observed_year,
            )
        };
        player_one_intel.insert(planet_record_index_1_based, snapshot);
    }

    Ok(summary)
}

fn partial_snapshot(
    planet_record_index_1_based: usize,
    planet: &nc_data::Planet,
    intel_year: u16,
) -> PlanetIntelSnapshot {
    PlanetIntelSnapshot {
        planet_record_index_1_based,
        intel_tier: IntelTier::Partial,
        compat_is_orbit_seed: false,
        last_intel_year: Some(intel_year),
        seen_year: Some(intel_year),
        scout_year: None,
        known_name: Some(planet.name.trim().to_string()),
        known_owner_empire_id: Some(planet.owner.0),
        known_potential_production: Some(u16::from(planet.potential_production)),
        known_armies: None,
        known_ground_batteries: None,
        known_starbase_count: None,
        known_current_production: None,
        known_stored_points: None,
        known_docked_summary: None,
        known_orbit_summary: None,
        compat_word_1e: None,
    }
}

fn full_snapshot(
    game_data: &CoreGameData,
    planet_record_index_1_based: usize,
    planet: &nc_data::Planet,
    intel_year: u16,
) -> PlanetIntelSnapshot {
    PlanetIntelSnapshot {
        planet_record_index_1_based,
        intel_tier: IntelTier::Full,
        compat_is_orbit_seed: false,
        last_intel_year: Some(intel_year),
        seen_year: Some(intel_year),
        scout_year: Some(intel_year),
        known_name: Some(planet.name.trim().to_string()),
        known_owner_empire_id: Some(planet.owner.0),
        known_potential_production: Some(u16::from(planet.potential_production)),
        known_armies: Some(u8::try_from(planet.armies).unwrap_or(u8::MAX)),
        known_ground_batteries: Some(u8::try_from(planet.ground_batteries).unwrap_or(u8::MAX)),
        known_starbase_count: Some(nc_data::active_starbase_count_at(
            game_data,
            planet.sector.to_array(),
        )),
        known_current_production: Some(present_production_points(planet))
            .map(|value| value.min(u16::from(u8::MAX)) as u8),
        known_stored_points: Some(stored_production_points(planet).min(u32::from(u16::MAX)) as u16),
        known_docked_summary: Some(format_stardock_summary(planet)),
        known_orbit_summary: Some(format_orbit_summary(game_data, planet.sector.to_array())),
        compat_word_1e: Some(0x23),
    }
}

fn format_stardock_summary(planet: &nc_data::Planet) -> String {
    let mut parts = Vec::new();
    for slot in 0..nc_data::STARDOCK_SLOT_COUNT {
        let count = u32::from(u16::try_from(planet.stardock[slot]).unwrap_or(u16::MAX));
        if count == 0 {
            continue;
        }
        let kind = stardock_slot_kind(planet, slot);
        parts.push(format!("{} {}", count, unit_label(kind, count)));
    }
    if parts.is_empty() {
        "Nothing".to_string()
    } else {
        parts.join(", ")
    }
}

fn format_orbit_summary(game_data: &CoreGameData, coords: [u8; 2]) -> String {
    let fleet_count = game_data
        .fleets
        .iter()
        .filter(|fleet| fleet.sector.to_array() == coords && fleet_has_any_force(fleet))
        .count();
    let starbase_count = game_data
        .bases
        .iter()
        .filter(|base| base.sector.to_array() == coords && base.base_id.0 != 0)
        .count();
    let mut parts = Vec::new();
    if fleet_count > 0 {
        parts.push(format!(
            "{} {}",
            fleet_count,
            if fleet_count == 1 { "fleet" } else { "fleets" }
        ));
    }
    if starbase_count > 0 {
        parts.push(format!(
            "{} {}",
            starbase_count,
            if starbase_count == 1 {
                "starbase"
            } else {
                "starbases"
            }
        ));
    }
    if parts.is_empty() {
        "Nothing".to_string()
    } else {
        parts.join(", ")
    }
}

fn fleet_has_any_force(fleet: &nc_data::Fleet) -> bool {
    fleet.ships.scouts > 0
        || fleet.ships.battleships > 0
        || fleet.ships.cruisers > 0
        || fleet.ships.destroyers > 0
        || troop_transports(fleet) > 0
        || fleet.ships.loaded_transports > 0
        || fleet.ships.etacs > 0
}

fn unit_label(kind: ProductionItemKind, count: u32) -> &'static str {
    match kind {
        ProductionItemKind::Destroyer => {
            if count == 1 {
                "destroyer"
            } else {
                "destroyers"
            }
        }
        ProductionItemKind::Cruiser => {
            if count == 1 {
                "cruiser"
            } else {
                "cruisers"
            }
        }
        ProductionItemKind::Battleship => {
            if count == 1 {
                "battleship"
            } else {
                "battleships"
            }
        }
        ProductionItemKind::Scout => {
            if count == 1 {
                "scout"
            } else {
                "scouts"
            }
        }
        ProductionItemKind::Transport => {
            if count == 1 {
                "troop transport"
            } else {
                "troop transports"
            }
        }
        ProductionItemKind::Etac => {
            if count == 1 {
                "ETAC"
            } else {
                "ETACs"
            }
        }
        ProductionItemKind::Army => {
            if count == 1 {
                "army"
            } else {
                "armies"
            }
        }
        ProductionItemKind::GroundBattery => {
            if count == 1 {
                "ground battery"
            } else {
                "ground batteries"
            }
        }
        ProductionItemKind::Starbase => {
            if count == 1 {
                "starbase"
            } else {
                "starbases"
            }
        }
        ProductionItemKind::Unknown(_) => {
            if count == 1 {
                "unit"
            } else {
                "units"
            }
        }
    }
}
