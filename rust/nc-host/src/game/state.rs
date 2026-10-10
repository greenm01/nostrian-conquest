use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use nc_data::{
    CampaignSettings, CampaignStore, CoreGameData, DEFAULT_CAMPAIGN_DB_NAME, DiplomaticRelation,
    PlanetIntelSnapshot, PlayerRecord, ProductionItemKind, QueuedPlayerMail, ReportBlockRow,
    STARDOCK_SLOT_COUNT, active_starbase_count_at, build_player_starmap_projection_from_snapshots,
    load_mail_queue, merge_player_intel_from_runtime,
};
use nc_nostr::state_sync::{
    GameState, HostedDiplomacyState, HostedFleetShips, HostedOwnedFleet, HostedOwnedPlanet,
    HostedPlayerRosterEntry, HostedPlayerState, HostedQueuedMail, HostedReportBlock,
    HostedStardockSlot, HostedStarmapState, HostedStatePayload, HostedWorldState,
    compute_state_hash,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GameRuntime {
    pub game_id: String,
}

pub fn initialize_runtime_state(
    game_dir: &Path,
    slug: &str,
    game_name: &str,
    player_count: u8,
    year: u16,
    seed: u64,
) -> Result<CoreGameData, Box<dyn std::error::Error>> {
    let game_data = nc_engine::build_seeded_new_game(player_count, year, seed)?;
    game_data.save(game_dir)?;

    let campaign_store = CampaignStore::open_default_in_dir(game_dir)?;
    let intel_by_viewer = (1..=player_count)
        .map(|viewer_empire_id| {
            merge_player_intel_from_runtime(&game_data, viewer_empire_id, year, None, None)
        })
        .collect::<Vec<_>>();
    campaign_store.save_runtime_state_structured_with_intel(
        &game_data,
        &BTreeSet::new(),
        &[],
        &[],
        &intel_by_viewer,
    )?;
    campaign_store.save_campaign_settings(&CampaignSettings::new(slug, game_name))?;

    Ok(game_data)
}

pub fn build_game_state_payload(
    game_dir: &Path,
    game_id: &str,
    player_seat: u32,
) -> Result<GameState, Box<dyn std::error::Error>> {
    let viewer_empire_id = u8::try_from(player_seat)
        .map_err(|_| format!("seat {} is out of range for runtime snapshot", player_seat))?;
    let snapshot = load_player_runtime_snapshot(game_dir, viewer_empire_id)?;
    let player = snapshot
        .game_data
        .player
        .records
        .get(viewer_empire_id as usize - 1)
        .ok_or_else(|| format!("missing player record for seat {}", player_seat))?;

    let starmap = build_player_starmap_projection_from_snapshots(
        &snapshot.game_data,
        &snapshot.intel,
        viewer_empire_id,
    );
    let state = HostedStatePayload {
        player: player_state(player, &snapshot.game_data, viewer_empire_id),
        roster: roster_state(&snapshot.game_data, viewer_empire_id),
        starmap: starmap_state(&starmap),
        owned_planets: owned_planets_state(&snapshot.game_data, viewer_empire_id),
        owned_fleets: owned_fleets_state(&snapshot.game_data, viewer_empire_id),
    };
    let queued_mail = visible_queued_mail(&snapshot.queued_mail, viewer_empire_id);
    let report_blocks = visible_report_blocks(&snapshot.report_block_rows, viewer_empire_id);

    let mut result = GameState {
        game_id: game_id.to_string(),
        turn: snapshot.turn,
        year: u32::from(snapshot.game_year),
        player_seat,
        player_name: display_player_name(player, player_seat),
        state_hash: String::new(),
        state,
        queued_mail,
        report_blocks,
    };
    result.state_hash = compute_state_hash(&result)?;
    Ok(result)
}

struct PlayerRuntimeSnapshot {
    game_data: CoreGameData,
    game_year: u16,
    turn: u32,
    queued_mail: Vec<QueuedPlayerMail>,
    report_block_rows: Vec<ReportBlockRow>,
    intel: BTreeMap<usize, PlanetIntelSnapshot>,
}

fn load_player_runtime_snapshot(
    game_dir: &Path,
    viewer_empire_id: u8,
) -> Result<PlayerRuntimeSnapshot, Box<dyn std::error::Error>> {
    let runtime_db_path = game_dir.join(DEFAULT_CAMPAIGN_DB_NAME);
    if runtime_db_path.exists() {
        let store = CampaignStore::open(&runtime_db_path)?;
        if let Some(runtime) = store.load_latest_runtime_state()? {
            let intel = store
                .latest_planet_intel_for_viewer(viewer_empire_id)?
                .into_iter()
                .map(|snapshot| (snapshot.planet_record_index_1_based, snapshot))
                .collect::<BTreeMap<_, _>>();
            let intel = if intel.is_empty() {
                merge_player_intel_from_runtime(
                    &runtime.game_data,
                    viewer_empire_id,
                    runtime.game_year,
                    None,
                    None,
                )
            } else {
                intel
            };
            return Ok(PlayerRuntimeSnapshot {
                turn: runtime.game_data.conquest.game_year().saturating_sub(3000) as u32,
                game_year: runtime.game_year,
                game_data: runtime.game_data,
                queued_mail: runtime.queued_mail,
                report_block_rows: runtime.report_block_rows,
                intel,
            });
        }
    }

    let game_data = CoreGameData::load(game_dir)?;
    let game_year = game_data.conquest.game_year();
    let intel =
        merge_player_intel_from_runtime(&game_data, viewer_empire_id, game_year, None, None);
    let queued_mail = load_mail_queue(game_dir).unwrap_or_default();

    Ok(PlayerRuntimeSnapshot {
        turn: game_year.saturating_sub(3000) as u32,
        game_year,
        game_data,
        queued_mail,
        report_block_rows: Vec::new(),
        intel,
    })
}

fn player_state(
    player: &PlayerRecord,
    game_data: &CoreGameData,
    viewer_empire_id: u8,
) -> HostedPlayerState {
    let player_count = game_data.conquest.player_count();
    let diplomacy = (1..=player_count)
        .filter(|empire_id| *empire_id != viewer_empire_id)
        .filter_map(|empire_id| {
            player
                .diplomatic_relation_toward(empire_id)
                .map(|relation| HostedDiplomacyState {
                    empire_id,
                    relation: diplomacy_label(relation).to_string(),
                })
        })
        .collect::<Vec<_>>();

    HostedPlayerState {
        seat: viewer_empire_id,
        empire_name: display_player_name(player, u32::from(viewer_empire_id)),
        handle: blank_to_null(player.assigned_player_handle_summary()),
        mode: player_mode_label(player).to_string(),
        tax_rate: player.tax_rate(),
        planet_count: player.planet_count_raw(),
        starbase_count: player.starbase_count_raw().min(u16::from(u8::MAX)) as u8,
        homeworld_planet_index: u16::from(player.homeworld_planet_index_1_based_raw()),
        last_run_year: player.last_run_year_raw(),
        diplomacy,
    }
}

fn roster_state(game_data: &CoreGameData, viewer_empire_id: u8) -> Vec<HostedPlayerRosterEntry> {
    let player_count = usize::from(game_data.conquest.player_count());
    game_data
        .player
        .records
        .iter()
        .take(player_count)
        .enumerate()
        .map(|(idx, player)| {
            let empire_id = (idx + 1) as u8;
            HostedPlayerRosterEntry {
                empire_id,
                empire_name: display_player_name(player, u32::from(empire_id)),
                is_self: empire_id == viewer_empire_id,
            }
        })
        .collect()
}

fn starmap_state(projection: &nc_data::PlayerStarmapProjection) -> HostedStarmapState {
    let worlds = projection
        .worlds
        .iter()
        .map(|world| HostedWorldState {
            planet_index: world.planet_record_index_1_based,
            coords: world.coords,
            intel_tier: world.intel_tier.as_str().to_string(),
            known_name: world.known_name.clone(),
            known_owner_empire_id: world.known_owner_empire_id,
            known_owner_empire_name: world.known_owner_empire_name.clone(),
            known_potential_production: world.known_potential_production,
            known_armies: world.known_armies,
            known_ground_batteries: world.known_ground_batteries,
            known_starbase_count: world.known_starbase_count,
            known_current_production: world.known_current_production,
            known_stored_points: world.known_stored_points,
            known_docked_summary: world.known_docked_summary.clone(),
            known_orbit_summary: world.known_orbit_summary.clone(),
        })
        .collect::<Vec<_>>();

    HostedStarmapState {
        map_width: projection.map_width,
        map_height: projection.map_height,
        viewer_empire_id: projection.viewer_empire_id,
        year: projection.year,
        worlds,
    }
}

fn owned_planets_state(game_data: &CoreGameData, viewer_empire_id: u8) -> Vec<HostedOwnedPlanet> {
    game_data
        .planets
        .records
        .iter()
        .enumerate()
        .filter(|(_, planet)| planet.owner_empire_slot_raw() == viewer_empire_id)
        .map(|(planet_index, planet)| {
            let stardock = (0..STARDOCK_SLOT_COUNT)
                .filter_map(|slot| {
                    let count = planet.stardock_count_raw(slot);
                    (count > 0).then(|| HostedStardockSlot {
                        slot: slot + 1,
                        kind: production_kind_label(planet.stardock_item_kind_current_known(slot))
                            .to_string(),
                        count,
                    })
                })
                .collect::<Vec<_>>();
            HostedOwnedPlanet {
                planet_index: planet_index + 1,
                name: planet.status_or_name_summary(),
                coords: planet.coords_raw(),
                potential_production: planet.potential_production_points(),
                current_production: planet
                    .present_production_points()
                    .unwrap_or(0)
                    .min(u16::from(u8::MAX)) as u8,
                stored_points: planet.stored_production_points().min(u32::from(u16::MAX)) as u16,
                armies: planet.army_count_raw(),
                ground_batteries: planet.ground_batteries_raw(),
                starbase_count: active_starbase_count_at(game_data, planet.coords_raw()),
                stardock,
            }
        })
        .collect()
}

fn owned_fleets_state(game_data: &CoreGameData, viewer_empire_id: u8) -> Vec<HostedOwnedFleet> {
    game_data
        .fleets
        .records
        .iter()
        .filter(|fleet| fleet.owner_empire_raw() == viewer_empire_id && fleet.has_any_force())
        .map(|fleet| HostedOwnedFleet {
            fleet_id: fleet.fleet_id(),
            local_slot: fleet.local_slot(),
            coords: fleet.current_location_coords_raw(),
            target_coords: fleet.standing_order_target_coords_raw(),
            order: fleet.standing_order_kind().as_str().to_string(),
            order_summary: fleet.standing_order_summary(),
            rules_of_engagement: fleet.rules_of_engagement(),
            current_speed: fleet.current_speed(),
            max_speed: fleet.max_speed(),
            ships: HostedFleetShips {
                scout: fleet.scout_count(),
                battleship: fleet.battleship_count(),
                cruiser: fleet.cruiser_count(),
                destroyer: fleet.destroyer_count(),
                transport: fleet.troop_transport_count(),
                army: fleet.army_count(),
                etac: fleet.etac_count(),
                total_starships: fleet.total_starships().min(u32::from(u16::MAX)) as u16,
                summary: fleet.ship_composition_table_summary(),
            },
        })
        .collect()
}

fn visible_queued_mail(
    queued_mail: &[QueuedPlayerMail],
    viewer_empire_id: u8,
) -> Vec<HostedQueuedMail> {
    queued_mail
        .iter()
        .filter(|mail| mail.is_visible_to_recipient(viewer_empire_id))
        .map(|mail| HostedQueuedMail {
            sender_empire_id: mail.sender_empire_id,
            recipient_empire_id: mail.recipient_empire_id,
            year: mail.year,
            subject: mail.subject.clone(),
            body: mail.body.clone(),
        })
        .collect()
}

fn visible_report_blocks(
    report_block_rows: &[ReportBlockRow],
    viewer_empire_id: u8,
) -> Vec<HostedReportBlock> {
    report_block_rows
        .iter()
        .filter(|row| !row.recipient_deleted && row.is_visible_to_viewer(viewer_empire_id))
        .map(|row| HostedReportBlock {
            viewer_empire_id: row.viewer_empire_id,
            block_index: row.block_index,
            decoded_text: row.decoded_text.clone(),
        })
        .collect()
}

fn display_player_name(player: &PlayerRecord, player_seat: u32) -> String {
    let empire_name = player.controlled_empire_name_summary();
    if !empire_name.is_empty() {
        empire_name
    } else {
        format!("Seat {}", player_seat)
    }
}

fn player_mode_label(player: &PlayerRecord) -> &'static str {
    if player.is_active_human_player() {
        "active"
    } else if player.is_rogue_player() {
        "rogue"
    } else {
        "civil_disorder"
    }
}

fn diplomacy_label(relation: DiplomaticRelation) -> &'static str {
    match relation {
        DiplomaticRelation::Neutral => "neutral",
        DiplomaticRelation::Enemy => "enemy",
    }
}

fn production_kind_label(kind: ProductionItemKind) -> &'static str {
    match kind {
        ProductionItemKind::Destroyer => "destroyer",
        ProductionItemKind::Cruiser => "cruiser",
        ProductionItemKind::Battleship => "battleship",
        ProductionItemKind::Scout => "scout",
        ProductionItemKind::Transport => "transport",
        ProductionItemKind::Etac => "etac",
        ProductionItemKind::GroundBattery => "ground_battery",
        ProductionItemKind::Army => "army",
        ProductionItemKind::Starbase => "starbase",
        ProductionItemKind::Unknown(_) => "unknown",
    }
}

fn blank_to_null(value: String) -> Option<String> {
    let trimmed = value.trim();
    (!trimmed.is_empty()).then(|| trimmed.to_string())
}
