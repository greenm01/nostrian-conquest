use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use crate::dashboard::geometry::ScreenGeometry;
use nc_data::{
    CampaignStore, CoreGameData, Fleet, GameStateBuilder, IntelTier, Order, Planet,
    PlanetIntelSnapshot, PlayerActivityState, PlayerLifecycleState, PlayerWarStatsState,
    QueuedPlayerMail, ReportBlockRow, ReportsPreview, Starbase, WinnerState,
    default_player_lifecycle_states, has_visible_runtime_messages, has_visible_runtime_reports,
};
use nc_nostr::state_sync::{GameState, HostedFleetShips, HostedOwnedFleet, HostedWorldState};

use crate::dashboard::app::state::DashApp;
use crate::dashboard::client_settings;
use crate::dashboard::layout;
use crate::dashboard::theme;
use nc_data::DEFAULT_CAMPAIGN_DB_NAME;

pub struct DashLaunchState {
    pub game_dir: PathBuf,
    pub campaign_store: Option<CampaignStore>,
    pub game_data: CoreGameData,
    pub owned_planet_years: BTreeMap<usize, u16>,
    pub report_block_rows: Vec<ReportBlockRow>,
    pub queued_mail: Vec<QueuedPlayerMail>,
    pub planet_intel_snapshots: Vec<PlanetIntelSnapshot>,
    pub player_activity_states: Vec<PlayerActivityState>,
    pub player_lifecycle_states: Vec<PlayerLifecycleState>,
    pub winner_state: WinnerState,
    pub player_war_stats: PlayerWarStatsState,
    pub player_record_index_1_based: usize,
}

impl DashLaunchState {
    pub fn from_local_dir(game_dir: PathBuf) -> Result<Self, Box<dyn std::error::Error>> {
        validate_local_runtime_dir(&game_dir)?;
        let campaign_store = CampaignStore::open_default_in_dir(&game_dir)?;
        let state = campaign_store
            .load_latest_runtime_state()?
            .ok_or("No runtime snapshots found — run maintenance first.")?;
        let player_record_index_1_based = 1usize;
        let owned_planet_years = campaign_store
            .latest_owned_planet_years_for_empire(player_record_index_1_based as u8)?;
        let planet_intel_snapshots =
            campaign_store.latest_planet_intel_for_viewer(player_record_index_1_based as u8)?;
        let player_war_stats = campaign_store
            .latest_player_war_stats(state.game_data.conquest.player_count)?
            .get(player_record_index_1_based.saturating_sub(1))
            .copied()
            .unwrap_or_else(|| PlayerWarStatsState::for_player(player_record_index_1_based));
        let player_activity_states =
            campaign_store.latest_player_activity_states(state.game_data.conquest.player_count)?;
        let player_lifecycle_states =
            campaign_store.latest_player_lifecycle_states(state.game_data.conquest.player_count)?;

        Ok(Self {
            game_dir,
            campaign_store: Some(campaign_store),
            game_data: state.game_data,
            owned_planet_years,
            report_block_rows: state.report_block_rows,
            queued_mail: state.queued_mail,
            planet_intel_snapshots,
            player_activity_states,
            player_lifecycle_states,
            winner_state: state.winner_state,
            player_war_stats,
            player_record_index_1_based,
        })
    }

    pub fn from_hosted_snapshot(snapshot: &GameState) -> Result<Self, Box<dyn std::error::Error>> {
        let player_count = infer_player_count(snapshot);
        let mut game_data = GameStateBuilder::new()
            .with_player_count(player_count)
            .with_year(snapshot.year as u16)
            .build_initialized_baseline()?;

        ensure_planet_capacity(&mut game_data, snapshot);
        set_player_records(&mut game_data, snapshot, player_count);
        set_planets(&mut game_data, snapshot);
        set_bases(&mut game_data, snapshot);
        set_fleets(&mut game_data, snapshot);

        let owned_planet_years = snapshot
            .state
            .owned_planets
            .iter()
            .map(|planet| (planet.planet_index, snapshot.year as u16))
            .collect::<BTreeMap<_, _>>();
        let report_block_rows = snapshot
            .report_blocks
            .iter()
            .map(|block| ReportBlockRow {
                viewer_empire_id: block.viewer_empire_id,
                block_index: block.block_index,
                decoded_text: block.decoded_text.clone(),
                raw_bytes: None,
                recipient_deleted: false,
            })
            .collect::<Vec<_>>();
        let queued_mail = snapshot
            .queued_mail
            .iter()
            .map(|mail| QueuedPlayerMail {
                sender_empire_id: mail.sender_empire_id,
                recipient_empire_id: mail.recipient_empire_id,
                year: mail.year,
                subject: mail.subject.clone(),
                body: mail.body.clone(),
                recipient_deleted: false,
                sender_deleted: false,
                cancelled: false,
            })
            .collect::<Vec<_>>();

        Ok(Self {
            game_dir: PathBuf::from("<hosted>"),
            campaign_store: None,
            game_data,
            owned_planet_years,
            report_block_rows,
            queued_mail,
            planet_intel_snapshots: build_planet_intel_snapshots(snapshot),
            player_activity_states: build_player_activity_states(snapshot, player_count),
            player_lifecycle_states: default_player_lifecycle_states(player_count),
            winner_state: WinnerState::default(),
            player_war_stats: PlayerWarStatsState::for_player(snapshot.player_seat as usize),
            player_record_index_1_based: snapshot.player_seat as usize,
        })
    }

    pub fn into_app(self, geometry: ScreenGeometry) -> Result<DashApp, Box<dyn std::error::Error>> {
        let mut app = DashApp::new(
            self.game_dir,
            self.campaign_store,
            self.game_data,
            self.owned_planet_years,
            self.report_block_rows,
            self.queued_mail,
            self.planet_intel_snapshots,
            self.player_activity_states,
            self.player_lifecycle_states,
            self.winner_state,
            geometry,
            ScreenGeometry::new(0, 0),
            self.player_record_index_1_based,
        );
        app.player_war_stats = self.player_war_stats;
        let client_settings_path = client_settings::settings_path();
        app.client_settings = client_settings::load_client_settings_from(&client_settings_path)?;
        app.client_settings_path = Some(client_settings_path);
        if theme::apply_theme_key(&app.client_settings.theme_key).is_err() {
            theme::apply_default_theme();
        }
        let required = layout::dashboard::required_dashboard_frame(&app);
        app.geometry = required;
        app.frame = required;
        app.is_terminal_too_small = false;
        if app.campaign_store.is_none() {
            app.initialize_hosted_turn_draft();
        }
        app.resize_canvas(geometry.width() as u16, geometry.height() as u16);

        let viewer_id = app.player_record_index_1_based as u8;
        let reports_pending = has_visible_runtime_reports(viewer_id, &app.report_block_rows);
        let messages_pending = has_visible_runtime_messages(
            viewer_id,
            &app.queued_mail,
            app.game_data.conquest.game_year,
        );

        if reports_pending || messages_pending {
            let game_key = if let Some(store) = &app.campaign_store {
                store.path().display().to_string()
            } else {
                app._game_dir.display().to_string()
            };

            let player_record = app
                .game_data
                .players
                .get(app.player_record_index_1_based.saturating_sub(1));
            let results_word = player_record.map(|p| p.messages_last as u16).unwrap_or(0);
            let messages_word = player_record.map(|p| p.messages_first as u16).unwrap_or(0);
            let report_key = format!(
                "{}:{}:{}",
                app.game_data.conquest.game_year, results_word, messages_word
            );

            let already_seen = app
                .client_settings
                .last_seen_report_keys
                .get(&game_key)
                .map(|k| k == &report_key)
                .unwrap_or(false);

            if !already_seen {
                app.popup = crate::dashboard::app::state::ActivePopup::StartupReview;
                app.startup_review.reports = ReportsPreview::from_block_rows(
                    &app.game_data,
                    viewer_id,
                    &app.report_block_rows,
                    &app.queued_mail,
                );
                app.startup_review.mode = if reports_pending {
                    crate::dashboard::app::state::StartupReviewMode::Results
                } else {
                    crate::dashboard::app::state::StartupReviewMode::Messages
                };

                app.client_settings
                    .last_seen_report_keys
                    .insert(game_key, report_key);
                if let Some(path) = &app.client_settings_path {
                    let _ = client_settings::save_client_settings_to(&app.client_settings, path);
                }
            }
        }

        Ok(app)
    }
}

fn validate_local_runtime_dir(game_dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
    if !game_dir.exists() {
        return Err(format!("game directory not found: {}", game_dir.display()).into());
    }
    if !game_dir.is_dir() {
        return Err(format!("game path is not a directory: {}", game_dir.display()).into());
    }

    let db_path = game_dir.join(DEFAULT_CAMPAIGN_DB_NAME);
    if !db_path.is_file() {
        return Err(format!(
            "missing runtime database: {}. Use a game directory that contains ncgame.db.",
            db_path.display()
        )
        .into());
    }

    Ok(())
}

fn infer_player_count(snapshot: &GameState) -> u8 {
    let mut max_empire = snapshot
        .player_seat
        .max(snapshot.state.starmap.viewer_empire_id as u32);
    for roster_entry in &snapshot.state.roster {
        max_empire = max_empire.max(u32::from(roster_entry.empire_id));
    }
    for relation in &snapshot.state.player.diplomacy {
        max_empire = max_empire.max(u32::from(relation.empire_id));
    }
    for world in &snapshot.state.starmap.worlds {
        if let Some(owner) = world.known_owner_empire_id {
            max_empire = max_empire.max(u32::from(owner));
        }
    }
    max_empire.clamp(1, 25) as u8
}

fn ensure_planet_capacity(game_data: &mut CoreGameData, snapshot: &GameState) {
    let max_index = snapshot
        .state
        .starmap
        .worlds
        .iter()
        .map(|world| world.planet_index)
        .chain(
            snapshot
                .state
                .owned_planets
                .iter()
                .map(|planet| planet.planet_index),
        )
        .max()
        .unwrap_or(0);
    if max_index > game_data.planets.len() {
        game_data.planets.resize_with(max_index, Planet::default);
    }
}

fn set_player_records(game_data: &mut CoreGameData, snapshot: &GameState, player_count: u8) {
    for seat in 1..=player_count as usize {
        if let Some(player) = game_data.players.get_mut(seat - 1) {
            player.mode = nc_data::PlayerMode::from_code(0x01);
            nc_data::set_empire_name(player, &format!("Empire {seat}"));
            player.tax_rate = 50;
            player.last_year_on = snapshot.year as u16;
            player.bases_first.0 = 0;
        }
    }

    for roster_entry in &snapshot.state.roster {
        let empire_id = usize::from(roster_entry.empire_id);
        if empire_id == 0 || empire_id > usize::from(player_count) {
            continue;
        }
        if let Some(player) = game_data.players.get_mut(empire_id - 1) {
            nc_data::set_empire_name(player, &roster_entry.empire_name);
        }
    }

    let viewer_index = snapshot.player_seat as usize;
    if let Some(player) = game_data.players.get_mut(viewer_index.saturating_sub(1)) {
        nc_data::set_empire_name(player, &snapshot.state.player.empire_name);
        if let Some(handle) = snapshot.state.player.handle.as_deref() {
            nc_data::set_player_handle(player, handle);
        }
        player.tax_rate = snapshot.state.player.tax_rate;
        player.homeworld = snapshot
            .state
            .player
            .homeworld_planet_index
            .min(u16::from(u8::MAX)) as u8;
        player.last_year_on = snapshot.state.player.last_run_year;
    }
}

fn set_planets(game_data: &mut CoreGameData, snapshot: &GameState) {
    for world in &snapshot.state.starmap.worlds {
        let Some(record) = game_data
            .planets
            .get_mut(world.planet_index.saturating_sub(1))
        else {
            continue;
        };
        record.sector = nc_data::Coords::from_array(world.coords);
        if let Some(name) = world.known_name.as_deref() {
            nc_data::set_planet_name(record, name);
        }
        if let Some(owner) = world.known_owner_empire_id {
            record.owner.0 = owner;
        }
        if let Some(potential) = world.known_potential_production {
            record.potential_production = potential.min(u16::from(u8::MAX)) as u8;
        }
        if let Some(current) = world.known_current_production {
            record.production = nc_data::Real48::from_f64(f64::from(u16::from(current)));
        }
        if let Some(stored) = world.known_stored_points {
            record.stored_goods = nc_data::Real48::from_f64(f64::from(u32::from(stored)));
        }
        if let Some(armies) = world.known_armies {
            record.armies = u16::from(armies);
        }
        if let Some(batteries) = world.known_ground_batteries {
            record.ground_batteries = u16::from(batteries);
        }
    }

    for planet in &snapshot.state.owned_planets {
        let Some(record) = game_data
            .planets
            .get_mut(planet.planet_index.saturating_sub(1))
        else {
            continue;
        };
        record.sector = nc_data::Coords::from_array(planet.coords);
        nc_data::set_planet_name(record, &planet.name);
        record.owner.0 = snapshot.player_seat as u8;
        record.potential_production = planet.potential_production.min(u16::from(u8::MAX)) as u8;
        record.production =
            nc_data::Real48::from_f64(f64::from(u16::from(planet.current_production)));
        record.stored_goods = nc_data::Real48::from_f64(f64::from(u32::from(planet.stored_points)));
        record.armies = u16::from(planet.armies);
        record.ground_batteries = u16::from(planet.ground_batteries);
        for dock in &planet.stardock {
            if let Some(slot) = nc_data::stardock_slot_for_kind(production_kind_code(&dock.kind)) {
                record.stardock[slot] = u32::from(dock.count);
            }
        }
    }
}

fn set_bases(game_data: &mut CoreGameData, snapshot: &GameState) {
    let mut next_base_id = 1u16;
    let mut records: Vec<Starbase> = Vec::new();
    for planet in &snapshot.state.owned_planets {
        for slot in 0..planet.starbase_count.max(1) {
            if slot >= planet.starbase_count {
                break;
            }
            let mut base = Starbase::default();
            base.base_id.0 = next_base_id;
            base.owner.0 = snapshot.player_seat as u8;
            base.record.0 = next_base_id;
            if let Some(previous) = records.last_mut() {
                previous.next.0 = next_base_id;
                base.prev.0 = previous.record.0;
            }
            base.sector = nc_data::Coords::from_array(planet.coords);
            nc_data::set_starbase_at_rest(&mut base);
            base.target = nc_data::Coords::from_array(planet.coords);
            base.target_depth = 1;
            records.push(base);
            next_base_id = next_base_id.saturating_add(1);
        }
    }
    game_data.bases = records;
    game_data.relink_base_chain(snapshot.player_seat as u8);
    game_data.sync_last_base_records();
}

fn set_fleets(game_data: &mut CoreGameData, snapshot: &GameState) {
    let count = snapshot.state.owned_fleets.len().max(1);
    game_data.fleets.resize_with(count, Fleet::default);
    for (idx, fleet) in snapshot.state.owned_fleets.iter().enumerate() {
        let Some(record) = game_data.fleets.get_mut(idx) else {
            continue;
        };
        *record = build_fleet_record(snapshot.player_seat as u8, fleet);
    }
}

fn build_fleet_record(viewer_empire_id: u8, fleet: &HostedOwnedFleet) -> Fleet {
    let mut record = Fleet::default();
    record.fleet_number.0 = u16::from(fleet.local_slot);
    record.owner.0 = viewer_empire_id;
    record.record.0 = u16::from(fleet.fleet_id);
    record.max_speed = fleet.max_speed;
    record.speed = fleet.current_speed;
    record.sector = nc_data::Coords::from_array(fleet.coords);
    record.order = order_from_label(&fleet.order);
    record.target = nc_data::Coords::from_array(fleet.target_coords);
    record.rules_of_engagement = fleet.rules_of_engagement;
    apply_ship_counts(&mut record, &fleet.ships);
    record
}

fn apply_ship_counts(record: &mut Fleet, ships: &HostedFleetShips) {
    record.ships.scouts = ships.scout;
    record.ships.battleships = ships.battleship;
    record.ships.cruisers = ships.cruiser;
    record.ships.destroyers = ships.destroyer;
    nc_data::set_troop_transports(record, ships.transport);
    record.ships.etacs = ships.etac;
    nc_data::set_armies_aboard(record, ships.army);
}

fn build_planet_intel_snapshots(snapshot: &GameState) -> Vec<PlanetIntelSnapshot> {
    let viewer_empire_id = snapshot.player_seat as u8;
    let mut rows = snapshot
        .state
        .starmap
        .worlds
        .iter()
        .map(|world| intel_snapshot_from_world(snapshot.year as u16, viewer_empire_id, world))
        .collect::<Vec<_>>();
    for planet in &snapshot.state.owned_planets {
        let owned = PlanetIntelSnapshot {
            planet_record_index_1_based: planet.planet_index,
            intel_tier: IntelTier::Owned,
            compat_is_orbit_seed: false,
            last_intel_year: Some(snapshot.year as u16),
            seen_year: Some(snapshot.year as u16),
            scout_year: Some(snapshot.year as u16),
            known_name: Some(planet.name.clone()),
            known_owner_empire_id: Some(viewer_empire_id),
            known_potential_production: Some(planet.potential_production),
            known_armies: Some(planet.armies),
            known_ground_batteries: Some(planet.ground_batteries),
            known_starbase_count: Some(planet.starbase_count),
            known_current_production: Some(planet.current_production),
            known_stored_points: Some(planet.stored_points),
            known_docked_summary: None,
            known_orbit_summary: None,
            compat_word_1e: None,
        };
        if let Some(existing) = rows
            .iter_mut()
            .find(|existing| existing.planet_record_index_1_based == planet.planet_index)
        {
            *existing = owned;
        } else {
            rows.push(owned);
        }
    }
    rows.sort_by_key(|row| row.planet_record_index_1_based);
    rows
}

fn intel_snapshot_from_world(
    year: u16,
    viewer_empire_id: u8,
    world: &HostedWorldState,
) -> PlanetIntelSnapshot {
    PlanetIntelSnapshot {
        planet_record_index_1_based: world.planet_index,
        intel_tier: intel_tier_from_label(viewer_empire_id, world),
        compat_is_orbit_seed: false,
        last_intel_year: Some(year),
        seen_year: Some(year),
        scout_year: Some(year),
        known_name: world.known_name.clone(),
        known_owner_empire_id: world.known_owner_empire_id,
        known_potential_production: world.known_potential_production,
        known_armies: world.known_armies,
        known_ground_batteries: world.known_ground_batteries,
        known_starbase_count: world.known_starbase_count,
        known_current_production: world.known_current_production,
        known_stored_points: world.known_stored_points,
        known_docked_summary: world.known_docked_summary.clone(),
        known_orbit_summary: world.known_orbit_summary.clone(),
        compat_word_1e: None,
    }
}

fn build_player_activity_states(
    snapshot: &GameState,
    player_count: u8,
) -> Vec<PlayerActivityState> {
    (1..=player_count as usize)
        .map(|player_record_index_1_based| PlayerActivityState {
            player_record_index_1_based,
            last_participation_year: if player_record_index_1_based == snapshot.player_seat as usize
            {
                snapshot.year as u16
            } else {
                0
            },
            inactivity_autopilot_pending_clear: false,
        })
        .collect()
}

fn intel_tier_from_label(viewer_empire_id: u8, world: &HostedWorldState) -> IntelTier {
    if world.known_owner_empire_id == Some(viewer_empire_id) {
        IntelTier::Owned
    } else {
        match world.intel_tier.as_str() {
            "owned" => IntelTier::Owned,
            "full" => IntelTier::Full,
            "partial" => IntelTier::Partial,
            _ => IntelTier::Unknown,
        }
    }
}

fn production_kind_code(kind: &str) -> u8 {
    match kind {
        "destroyer" => 1,
        "cruiser" => 2,
        "battleship" => 3,
        "scout" => 4,
        "transport" => 5,
        "etac" => 6,
        "ground_battery" => 7,
        "army" => 8,
        "starbase" => 9,
        _ => 0,
    }
}

fn order_from_label(label: &str) -> Order {
    match label {
        "hold" => Order::HoldPosition,
        "move" => Order::MoveOnly,
        "seek_home" => Order::SeekHome,
        "patrol" => Order::PatrolSector,
        "guard_starbase" => Order::GuardStarbase,
        "guard_blockade" => Order::GuardBlockadeWorld,
        "bombard" => Order::BombardWorld,
        "invade" => Order::InvadeWorld,
        "blitz" => Order::BlitzWorld,
        "view" => Order::ViewWorld,
        "scout_sector" => Order::ScoutSector,
        "scout_system" => Order::ScoutSolarSystem,
        "colonize" => Order::ColonizeWorld,
        "join_fleet" => Order::JoinAnotherFleet,
        "rendezvous" => Order::RendezvousSector,
        "salvage" => Order::Salvage,
        _ => Order::Unknown(0),
    }
}

#[cfg(test)]
mod tests {
    use super::DashLaunchState;
    use nc_nostr::state_sync::{
        GameState, HostedPlayerRosterEntry, HostedPlayerState, HostedStarmapState,
        HostedStatePayload,
    };

    #[test]
    fn hosted_snapshot_applies_roster_empire_names_to_player_records() {
        let snapshot = GameState {
            game_id: "friday-night".to_string(),
            turn: 4,
            year: 3004,
            player_seat: 1,
            player_name: "Terran Union".to_string(),
            state_hash: "abc123".to_string(),
            state: HostedStatePayload {
                player: HostedPlayerState {
                    seat: 1,
                    empire_name: "Terran Union".to_string(),
                    handle: Some("captain".to_string()),
                    mode: "active".to_string(),
                    tax_rate: 33,
                    planet_count: 0,
                    starbase_count: 0,
                    homeworld_planet_index: 1,
                    last_run_year: 3004,
                    diplomacy: Vec::new(),
                },
                roster: vec![
                    HostedPlayerRosterEntry {
                        empire_id: 1,
                        empire_name: "Terran Union".to_string(),
                        is_self: true,
                    },
                    HostedPlayerRosterEntry {
                        empire_id: 2,
                        empire_name: "Rigel Empire".to_string(),
                        is_self: false,
                    },
                ],
                starmap: HostedStarmapState {
                    map_width: 18,
                    map_height: 18,
                    viewer_empire_id: 1,
                    year: 3004,
                    worlds: Vec::new(),
                },
                owned_planets: Vec::new(),
                owned_fleets: Vec::new(),
            },
            queued_mail: Vec::new(),
            report_blocks: Vec::new(),
        };

        let launch = DashLaunchState::from_hosted_snapshot(&snapshot).expect("hosted launch");

        assert_eq!(launch.game_data.conquest.player_count, 2);
        assert_eq!(
            launch.game_data.players[1].empire_name.trim().to_string(),
            "Rigel Empire"
        );
    }
}
