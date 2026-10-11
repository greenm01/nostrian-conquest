use std::collections::BTreeMap;
use std::path::Path;

use nc_data::{
    CampaignSettings, CampaignStore, CoreGameData, DiplomaticRelation, PlanetIntelSnapshot, Player,
    PlayerMode, ProductionItemKind, QueuedPlayerMail, ReportBlockRow, STARDOCK_SLOT_COUNT,
    active_starbase_count_at, build_player_starmap_projection_from_snapshots,
    diplomatic_relation_toward, has_any_force, merge_player_intel_from_runtime, order_summary,
    present_production_points, ship_composition_table_summary, stardock_slot_kind, starship_count,
    stored_production_points, troop_transports,
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

    let campaign_store = CampaignStore::open_default_in_dir(game_dir)?;
    let intel_by_viewer = (1..=player_count)
        .map(|viewer_empire_id| {
            merge_player_intel_from_runtime(&game_data, viewer_empire_id, year, None, None)
        })
        .collect::<Vec<_>>();
    campaign_store.save_runtime_state_structured_with_intel(
        year,
        &game_data,
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
        .players
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
    let queued_mail =
        visible_queued_mail(&snapshot.queued_mail, viewer_empire_id, snapshot.game_year);
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
    let store = crate::support::runtime::open_existing_campaign_store(game_dir)?;
    let runtime = crate::support::runtime::load_latest_runtime_state(&store)?;
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
    Ok(PlayerRuntimeSnapshot {
        turn: crate::game::turns::turn_for_year(runtime.game_data.conquest.game_year),
        game_year: runtime.game_year,
        game_data: runtime.game_data,
        queued_mail: runtime.queued_mail,
        report_block_rows: runtime.report_block_rows,
        intel,
    })
}

fn player_state(
    player: &Player,
    game_data: &CoreGameData,
    viewer_empire_id: u8,
) -> HostedPlayerState {
    let player_count = game_data.conquest.player_count;
    let diplomacy = (1..=player_count)
        .filter(|empire_id| *empire_id != viewer_empire_id)
        .filter_map(|empire_id| {
            diplomatic_relation_toward(player, empire_id).map(|relation| HostedDiplomacyState {
                empire_id,
                relation: diplomacy_label(relation).to_string(),
            })
        })
        .collect::<Vec<_>>();

    HostedPlayerState {
        seat: viewer_empire_id,
        empire_name: display_player_name(player, u32::from(viewer_empire_id)),
        handle: blank_to_null(player.handle.trim().to_string()),
        mode: player_mode_label(player).to_string(),
        tax_rate: player.tax_rate,
        planet_count: player.planet_count,
        starbase_count: game_data
            .bases
            .iter()
            .filter(|base| base.base_id.0 != 0 && base.owner.0 == viewer_empire_id)
            .count()
            .min(usize::from(u8::MAX)) as u8,
        homeworld_planet_index: u16::from(player.homeworld),
        last_run_year: player.last_year_on,
        diplomacy,
    }
}

fn roster_state(game_data: &CoreGameData, viewer_empire_id: u8) -> Vec<HostedPlayerRosterEntry> {
    let player_count = usize::from(game_data.conquest.player_count);
    game_data
        .players
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
        .iter()
        .enumerate()
        .filter(|(_, planet)| planet.owner.0 == viewer_empire_id)
        .map(|(planet_index, planet)| {
            let stardock = (0..STARDOCK_SLOT_COUNT)
                .filter_map(|slot| {
                    let count = u16::try_from(planet.stardock[slot]).unwrap_or(u16::MAX);
                    (count > 0).then(|| HostedStardockSlot {
                        slot: slot + 1,
                        kind: production_kind_label(stardock_slot_kind(planet, slot)).to_string(),
                        count,
                    })
                })
                .collect::<Vec<_>>();
            HostedOwnedPlanet {
                planet_index: planet_index + 1,
                name: planet.name.trim().to_string(),
                coords: planet.sector.to_array(),
                potential_production: u16::from(planet.potential_production),
                current_production: present_production_points(planet).min(u16::from(u8::MAX)) as u8,
                stored_points: stored_production_points(planet).min(u32::from(u16::MAX)) as u16,
                armies: u8::try_from(planet.armies).unwrap_or(u8::MAX),
                ground_batteries: u8::try_from(planet.ground_batteries).unwrap_or(u8::MAX),
                starbase_count: active_starbase_count_at(game_data, planet.sector.to_array()),
                stardock,
            }
        })
        .collect()
}

fn owned_fleets_state(game_data: &CoreGameData, viewer_empire_id: u8) -> Vec<HostedOwnedFleet> {
    game_data
        .fleets
        .iter()
        .filter(|fleet| fleet.owner.0 == viewer_empire_id && has_any_force(fleet))
        .map(|fleet| HostedOwnedFleet {
            fleet_id: fleet.record.0,
            local_slot: fleet.fleet_number.0 as u8,
            coords: fleet.sector.to_array(),
            target_coords: fleet.target.to_array(),
            order: fleet.order.as_str().to_string(),
            order_summary: order_summary(fleet),
            rules_of_engagement: fleet.rules_of_engagement,
            current_speed: fleet.speed,
            max_speed: fleet.max_speed,
            ships: HostedFleetShips {
                scout: fleet.ships.scouts,
                battleship: fleet.ships.battleships,
                cruiser: fleet.ships.cruisers,
                destroyer: fleet.ships.destroyers,
                transport: troop_transports(fleet),
                army: fleet.ships.loaded_transports,
                etac: fleet.ships.etacs,
                total_starships: starship_count(fleet).min(u32::from(u16::MAX)) as u16,
                summary: ship_composition_table_summary(fleet),
            },
        })
        .collect()
}

fn visible_queued_mail(
    queued_mail: &[QueuedPlayerMail],
    viewer_empire_id: u8,
    current_year: u16,
) -> Vec<HostedQueuedMail> {
    queued_mail
        .iter()
        .filter(|mail| mail.is_visible_to_recipient(viewer_empire_id, current_year))
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

fn display_player_name(player: &Player, player_seat: u32) -> String {
    let empire_name = player.empire_name.trim().to_string();
    if !empire_name.is_empty() {
        empire_name
    } else {
        format!("Seat {}", player_seat)
    }
}

fn player_mode_label(player: &Player) -> &'static str {
    if player.mode == PlayerMode::Human {
        "active"
    } else if player.mode == PlayerMode::Rogue {
        "rogue"
    } else {
        "civil_disorder"
    }
}

fn diplomacy_label(relation: DiplomaticRelation) -> &'static str {
    match relation {
        DiplomaticRelation::Neutral => "neutral",
        DiplomaticRelation::Enemy => "enemy",
        DiplomaticRelation::Unknown(_) => "unknown",
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
