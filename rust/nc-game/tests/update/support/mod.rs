#![allow(dead_code)]

pub(crate) use std::collections::{BTreeMap, BTreeSet};
pub(crate) use std::fs;
pub(crate) use std::path::{Path, PathBuf};
pub(crate) use std::sync::atomic::{AtomicU64, Ordering};
pub(crate) use std::time::{SystemTime, UNIX_EPOCH};

pub(crate) use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};
pub(crate) use nc_compat::{decode_report_block_rows, import_directory_snapshot};
pub(crate) use nc_data::{
    CampaignRuntimeState, CampaignStore, CoreGameData, DiplomaticRelation, EmpirePlanetEconomyRow,
    EmpireProductionRankingSort, IntelTier, PlanetIntelSnapshot, ProductionItemKind,
    QueuedPlayerMail, SeatReservation, map_size_for_player_count,
};
pub(crate) use nc_game::app::{
    Action, App, AppConfig, AppOutcome, RuntimeConfig as GameConfig,
    RuntimeSetupOverrides as GameSetupOverrides, apply_action,
};
pub(crate) use nc_game::domains::empire::EmpireAction;
pub(crate) use nc_game::domains::fleet::FleetAction;
pub(crate) use nc_game::domains::fleet::missions::{
    FLEET_MISSION_OPTIONS, FleetMissionRequirement, fleet_record_supports_mission_code,
};
pub(crate) use nc_game::domains::messaging::{MessagingAction, state::InboxFocus};
pub(crate) use nc_game::domains::planet::PlanetAction;
pub(crate) use nc_game::domains::starbase::StarbaseAction;
pub(crate) use nc_game::domains::starmap::StarmapAction;
pub(crate) use nc_game::domains::startup::StartupAction;
pub(crate) use nc_game::model::ClassicLoginState;
pub(crate) use nc_game::screen::help::{MenuHelpTopic, help_lines, menu_help_spec};
pub(crate) use nc_game::screen::layout::COMMAND_LINE_ROW;
pub(crate) use nc_game::screen::table::{TableColumn, fit_table_columns};
pub(crate) use nc_game::screen::{
    CommandMenu, FleetGroupOrderMode, FleetGroupScreen, FleetRow, MessageComposeScreen,
    PlanetBuildMenuView, PlanetBuildOrder, PlanetBuildScreen, PlanetCommissionDraftRow,
    PlanetDatabaseSortMode, PlanetListMode, PlanetListSort, ScreenId, SortDirection,
};
pub(crate) use nc_game::startup::StartupPhase;
pub(crate) use nc_game::terminal::Terminal;
pub(crate) use nc_game::theme;

mod actions;
mod fixtures;
mod render;

pub(crate) use actions::*;
pub(crate) use fixtures::*;
pub(crate) use render::*;
