use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use std::path::PathBuf;

use nc_data::{
    CampaignStore, CoreGameData, PlanetIntelSnapshot, PlayerAccessMode, PlayerActivityState,
    PlayerLifecycleState, QueuedPlayerMail, ReportBlockRow, WinnerState, player_access_mode,
};

use crate::app::help::PopupHelp;
use crate::app::runtime_config::RuntimeConfig;
use crate::domains::empire::EmpireState;
use crate::domains::fleet::FleetState;
use crate::domains::messaging::MessagingState;
use crate::domains::planet::PlanetState;
use crate::domains::starbase::StarbaseState;
use crate::domains::starmap::StarmapState;
use crate::domains::startup::StartupState;
use crate::model::{MainMenuSummary, PlayerContext};
use crate::reports::{ReportsPreview, has_visible_runtime_messages, has_visible_runtime_reports};
use crate::screen::{
    CommandMenu, EmpireProfileScreen, EmpireStatusScreen, EnemiesScreen, FirstTimeEmpiresScreen,
    FirstTimeIntroScreen, FirstTimeMenuScreen, FleetDetachScreen, FleetEtaScreen, FleetGroupScreen,
    FleetListScreen, FleetMenuScreen, FleetMessageScreen, FleetMissionPickerScreen,
    FleetReviewScreen, FleetSingleOrderScreen, FleetTransferScreen, GeneralMenuScreen,
    MainMenuScreen, MessageComposeScreen, PartialStarmapScreen, PlanetBuildScreen,
    PlanetCommissionScreen, PlanetDatabaseScreen, PlanetInfoScreen, PlanetListScreen,
    PlanetMenuScreen, PlanetTaxScreen, PlanetTransportScreen, RankingsScreen, ReportsScreen,
    ScreenGeometry, ScreenId, StarbaseListScreen, StarbaseMenuScreen, StarbaseReviewScreen,
    StarmapScreen, StartupScreen, ThemePickerScreen,
};
use crate::startup::{StartupSequence, StartupSummary};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppConfig {
    pub game_dir: PathBuf,
    pub player_record_index_1_based: usize,
    pub export_root: Option<PathBuf>,
    pub queue_dir: Option<PathBuf>,
    /// Session time limit in seconds sourced from `--timeout` or a dropfile.
    /// `None` means no limit.
    pub session_timeout_secs: Option<u32>,
    /// Runtime configuration materialized from campaign settings or BBS config.
    pub game_config: RuntimeConfig,
}

pub struct App {
    pub game_dir: PathBuf,
    pub game_name: String,
    pub game_config: RuntimeConfig,
    pub game_data: CoreGameData,
    pub player: PlayerContext,
    pub current_screen: ScreenId,
    pub startup_sequence: StartupSequence,
    pub startup: StartupScreen,
    pub first_time_menu: FirstTimeMenuScreen,
    pub first_time_empires: FirstTimeEmpiresScreen,
    pub first_time_intro: FirstTimeIntroScreen,
    pub theme_picker: ThemePickerScreen,
    pub main_menu: MainMenuScreen,
    pub general_menu: GeneralMenuScreen,
    pub starbase_menu: StarbaseMenuScreen,
    pub starbase_list: StarbaseListScreen,
    pub starbase_review: StarbaseReviewScreen,
    pub fleet_menu: FleetMenuScreen,
    pub fleet_list: FleetListScreen,
    pub fleet_review: FleetReviewScreen,
    pub fleet_order: FleetSingleOrderScreen,
    pub fleet_group: FleetGroupScreen,
    pub fleet_mission_picker: FleetMissionPickerScreen,
    pub fleet_transfer: FleetTransferScreen,
    pub fleet_detach: FleetDetachScreen,
    pub fleet_eta: FleetEtaScreen,
    pub fleet_message: FleetMessageScreen,
    pub planet_menu: PlanetMenuScreen,
    pub planet_commission: PlanetCommissionScreen,
    pub planet_transport: PlanetTransportScreen,
    pub planet_build: PlanetBuildScreen,
    pub planet_list: PlanetListScreen,
    pub planet_tax: PlanetTaxScreen,
    pub starmap: StarmapScreen,
    pub partial_starmap: PartialStarmapScreen,
    pub planet_database: PlanetDatabaseScreen,
    pub planet_info: PlanetInfoScreen,
    pub enemies: EnemiesScreen,
    pub message_compose: MessageComposeScreen,
    pub empire_status: EmpireStatusScreen,
    pub empire_profile: EmpireProfileScreen,
    pub rankings: RankingsScreen,
    pub reports: ReportsScreen,

    pub fleet: FleetState,
    pub planet: PlanetState,
    pub starbase: StarbaseState,
    pub empire: EmpireState,
    pub messaging: MessagingState,
    pub starmap_state: StarmapState,
    pub startup_state: StartupState,

    pub command_return_menu: CommandMenu,
    pub return_screen: Option<ScreenId>,
    pub export_root: PathBuf,
    pub queue_dir: Option<PathBuf>,
    pub screen_geometry: ScreenGeometry,
    pub door_mode: bool,
    pub autopilot: bool,
    pub expert_mode: bool,
    pub snapshot_id: i64,
    pub campaign_seed: u64,
    pub report_block_rows: Vec<ReportBlockRow>,
    pub queued_mail: Vec<QueuedPlayerMail>,
    pub player_activity_states: Vec<PlayerActivityState>,
    pub player_lifecycle_states: Vec<PlayerLifecycleState>,
    pub winner_state: WinnerState,
    pub player_access_mode: PlayerAccessMode,
    pub terminal_notice_lines: Vec<String>,
    pub command_menu_notice: Option<String>,
    pub quit_confirm_open: bool,
    pub popup_help: Option<PopupHelp>,
    pub planet_intel_snapshots: BTreeMap<usize, PlanetIntelSnapshot>,
    pub owned_planet_years: BTreeMap<usize, u16>,
    pub planet_scorch_orders: BTreeSet<usize>,
}

impl App {
    pub fn load(config: AppConfig) -> Result<Self, Box<dyn std::error::Error>> {
        let game_dir = config.game_dir.clone();
        let game_name = config.game_config.game_name.clone();
        crate::theme::initialize_from_game_dir(&game_dir, config.game_config.theme.clone())?;
        let export_root = config
            .export_root
            .clone()
            .unwrap_or_else(|| game_dir.join("exports"));
        let campaign_store = CampaignStore::open_default_in_dir(&game_dir)?;
        let runtime_state = campaign_store.load_latest_runtime_state()?.ok_or(
            "campaign store has no snapshots; initialize the campaign with nc-sysop first",
        )?;
        let snapshot_id = runtime_state.snapshot_id;
        let campaign_seed = runtime_state.campaign_seed;
        let report_block_rows = runtime_state.report_block_rows;
        let queued_mail = runtime_state.queued_mail;
        let planet_scorch_orders = runtime_state.planet_scorch_orders;
        let winner_state = runtime_state.winner_state;

        // Apply campaign-setting overrides to game_data. Only save a new
        // snapshot if any field actually changed, to avoid churn on clean
        // starts.
        let (game_data, snapshot_id) = apply_game_config_overrides(
            runtime_state.game_data,
            &config.game_config,
            &campaign_store,
            snapshot_id,
            &planet_scorch_orders,
            &report_block_rows,
            &queued_mail,
        )?;

        let reports = ReportsPreview::from_block_rows(
            &game_data,
            config.player_record_index_1_based as u8,
            &report_block_rows,
            &queued_mail,
        );
        let player = PlayerContext::from_game_data(&game_data, config.player_record_index_1_based)?;
        apply_player_theme_preference(&campaign_store, &game_dir, &player)?;
        let planet_intel_snapshots = campaign_store
            .latest_planet_intel_for_viewer(config.player_record_index_1_based as u8)?
            .into_iter()
            .map(|snapshot| (snapshot.planet_record_index_1_based, snapshot))
            .collect::<BTreeMap<_, _>>();
        let player_activity_states =
            campaign_store.latest_player_activity_states(game_data.conquest.player_count)?;
        let player_lifecycle_states =
            campaign_store.latest_player_lifecycle_states(game_data.conquest.player_count)?;
        let player_access_mode = player_access_mode(
            config.player_record_index_1_based,
            &player_lifecycle_states,
            winner_state,
        );
        if matches!(player_access_mode, PlayerAccessMode::LockedOut) {
            return Err("This empire has been defeated and is no longer allowed to log in.".into());
        }
        let mut player = player;
        player.apply_access_mode_override(player_access_mode);
        let owned_planet_years = campaign_store
            .latest_owned_planet_years_for_empire(config.player_record_index_1_based as u8)?;
        let main_menu_summary = MainMenuSummary::from_game_data(
            &game_data,
            config.player_record_index_1_based,
            has_visible_runtime_reports(
                config.player_record_index_1_based as u8,
                &report_block_rows,
            ),
            has_visible_runtime_messages(
                config.player_record_index_1_based as u8,
                &queued_mail,
                game_data.conquest.game_year,
            ),
        );
        let startup_summary = StartupSummary::from_reports(
            main_menu_summary.game_year,
            player.classic_login_state,
            main_menu_summary.pending_results,
            main_menu_summary.pending_messages,
            &reports,
        );
        let startup_sequence = StartupSequence::new(&startup_summary.to_shared());
        let mut startup_state = StartupState::default();
        startup_state.reserved_seat_alias = config
            .game_config
            .reservation_for_player(config.player_record_index_1_based)
            .map(|reservation| reservation.alias.clone());
        Ok(Self {
            game_dir,
            game_name,
            game_config: config.game_config.clone(),
            game_data,
            player,
            current_screen: ScreenId::Startup(startup_sequence.current()),
            startup_sequence,
            startup: StartupScreen::new(startup_summary, reports.clone()),
            first_time_menu: FirstTimeMenuScreen::new(),
            first_time_empires: FirstTimeEmpiresScreen::new(),
            first_time_intro: FirstTimeIntroScreen::new(),
            theme_picker: ThemePickerScreen::new(),
            main_menu: MainMenuScreen::new(),
            general_menu: GeneralMenuScreen::new(),
            starbase_menu: StarbaseMenuScreen::new(),
            starbase_list: StarbaseListScreen::new(),
            starbase_review: StarbaseReviewScreen::new(),
            fleet_menu: FleetMenuScreen::new(),
            fleet_list: FleetListScreen::new(),
            fleet_review: FleetReviewScreen::new(),
            fleet_order: FleetSingleOrderScreen::new(),
            fleet_group: FleetGroupScreen::new(),
            fleet_mission_picker: FleetMissionPickerScreen::new(),
            fleet_transfer: FleetTransferScreen::new(),
            fleet_detach: FleetDetachScreen::new(),
            fleet_eta: FleetEtaScreen::new(),
            fleet_message: FleetMessageScreen::new(),
            planet_menu: PlanetMenuScreen::new(),
            planet_commission: PlanetCommissionScreen::new(),
            planet_transport: PlanetTransportScreen::new(),
            planet_build: PlanetBuildScreen::new(),
            planet_list: PlanetListScreen::new(),
            planet_tax: PlanetTaxScreen::new(),
            starmap: StarmapScreen::new(),
            partial_starmap: PartialStarmapScreen::new(),
            planet_database: PlanetDatabaseScreen::new(),
            planet_info: PlanetInfoScreen::new(),
            enemies: EnemiesScreen::new(),
            message_compose: MessageComposeScreen::new(),
            empire_status: EmpireStatusScreen::new(),
            empire_profile: EmpireProfileScreen::new(),
            rankings: RankingsScreen::new(),
            reports: ReportsScreen::new(),

            fleet: FleetState::default(),
            planet: PlanetState::new(campaign_store, planet_intel_snapshots.clone()),
            starbase: StarbaseState::default(),
            empire: EmpireState::default(),
            messaging: MessagingState::default(),
            starmap_state: StarmapState {
                partial_center: [8, 2],
                ..Default::default()
            },
            startup_state,

            command_return_menu: CommandMenu::General,
            return_screen: None,
            export_root,
            queue_dir: config.queue_dir,
            screen_geometry: ScreenGeometry::local_default(),
            door_mode: false,
            autopilot: false,
            expert_mode: false,
            snapshot_id,
            campaign_seed,
            report_block_rows,
            queued_mail,
            player_activity_states,
            player_lifecycle_states,
            winner_state,
            player_access_mode,
            terminal_notice_lines: Vec::new(),
            command_menu_notice: None,
            quit_confirm_open: false,
            popup_help: None,
            planet_intel_snapshots,
            owned_planet_years,
            planet_scorch_orders,
        })
    }
}

fn apply_player_theme_preference(
    campaign_store: &CampaignStore,
    game_dir: &Path,
    player: &PlayerContext,
) -> Result<(), Box<dyn std::error::Error>> {
    if !player.is_joined {
        return Ok(());
    }
    let Some(theme_key) = campaign_store.player_theme_preference(player.record_index_1_based)?
    else {
        return Ok(());
    };
    let entries = crate::theme::discover_theme_entries(game_dir)?;
    if let Some(entry) = entries.into_iter().find(|entry| entry.key == theme_key) {
        if crate::theme::apply_theme_entry(&entry).is_ok() {
            return Ok(());
        }
    }
    crate::theme::apply_default_theme();
    campaign_store.set_player_theme_preference(player.record_index_1_based, "tokyo_night")?;
    Ok(())
}

/// Apply runtime operational settings to `game_data.setup`.
///
/// If any Setup byte changed, save a new snapshot so the engine always
/// sees the current config on the next run.  Returns the (possibly updated)
/// `CoreGameData` and the active snapshot id (new if saved, original if not).
fn apply_game_config_overrides(
    mut game_data: CoreGameData,
    cfg: &RuntimeConfig,
    store: &CampaignStore,
    current_snapshot_id: i64,
    planet_scorch_orders: &BTreeSet<usize>,
    report_block_rows: &[ReportBlockRow],
    queued_mail: &[QueuedPlayerMail],
) -> Result<(CoreGameData, i64), Box<dyn std::error::Error>> {
    let s = &mut game_data.setup;
    let mut changed = false;

    macro_rules! apply {
        ($field:ident, $value:expr) => {
            if s.$field != $value {
                s.$field = $value;
                changed = true;
            }
        };
    }

    if let Some(value) = cfg.setup_overrides.snoop_enabled {
        apply!(snoop, value);
    }
    if let Some(value) = cfg.setup_overrides.session_local_timeout {
        apply!(local_timeout, value);
    }
    if let Some(value) = cfg.setup_overrides.session_remote_timeout {
        apply!(remote_timeout, value);
    }
    if let Some(value) = cfg.setup_overrides.session_max_idle_minutes {
        apply!(max_key_gap_minutes, u16::from(value));
    }
    if let Some(value) = cfg.setup_overrides.session_minimum_time_minutes {
        apply!(minimum_time_minutes, value);
    }
    if let Some(value) = cfg.setup_overrides.inactivity_autopilot_after_turns {
        apply!(autopilot_after_turns, u16::from(value));
    }

    let snapshot_id = if changed {
        store.save_runtime_state_structured(
            &game_data,
            planet_scorch_orders,
            report_block_rows,
            queued_mail,
        )?
    } else {
        current_snapshot_id
    };

    Ok((game_data, snapshot_id))
}
