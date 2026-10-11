//! Dashboard application state.

use crate::dashboard::geometry::ScreenGeometry;
use crate::dashboard::table_filter::{TableFilterClause, TableFilterColumn};
use nc_data::FleetDetachSelection;
use nc_data::{
    CampaignStore, CoreGameData, GameStateBuilder, PlanetIntelSnapshot, PlayerActivityState,
    PlayerLifecycleState, ProductionItemKind, QueuedPlayerMail, ReportBlockRow, TurnSubmission,
    WinnerState,
};
use nc_engine::ArmyTransportMode;
use nc_session::startup::{StartupPhase, StartupSequence, StartupSummary};
use std::collections::{BTreeMap, BTreeSet};
use std::time::Instant;

use super::panel_cache::PanelCache;
use crate::dashboard::client_settings::DashClientSettings;
use crate::dashboard::overlays::frame::RelativePopupOrigin;

/// Which panel has keyboard focus.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PanelFocus {
    Map,
    Economy,
    Planets,
    Fleets,
    WarRecord,
    Galaxy,
    Diplomacy,
}

impl PanelFocus {
    pub fn next(self) -> Self {
        match self {
            Self::Map => Self::Economy,
            Self::Economy => Self::Planets,
            Self::Planets => Self::Fleets,
            Self::Fleets => Self::WarRecord,
            Self::WarRecord => Self::Galaxy,
            Self::Galaxy => Self::Diplomacy,
            Self::Diplomacy => Self::Map,
        }
    }

    pub fn prev(self) -> Self {
        match self {
            Self::Map => Self::Diplomacy,
            Self::Economy => Self::Map,
            Self::Planets => Self::Economy,
            Self::Fleets => Self::Planets,
            Self::WarRecord => Self::Fleets,
            Self::Galaxy => Self::WarRecord,
            Self::Diplomacy => Self::Galaxy,
        }
    }
}

/// Which fullscreen overlay is open (if any).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActiveOverlay {
    None,
    PlanetList,
    FleetList,
    IntelDatabase,
    Inbox,
    Diplomacy,
    Help,
}

impl ActiveOverlay {
    pub const fn is_draggable(self) -> bool {
        match self {
            Self::PlanetList
            | Self::FleetList
            | Self::IntelDatabase
            | Self::Inbox
            | Self::Diplomacy
            | Self::Help => true,
            Self::None => false,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActivePopup {
    None,
    QuitConfirm,
    InboxMessageConfirm { action: InboxMessageConfirmAction },
    TaxPrompt,
    PlanetDetail { planet_record_index_1_based: usize },
    OwnedPlanet { planet_record_index_1_based: usize },
    FleetDetail { fleet_record_index_1_based: usize },
    StartupReview,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InboxMessageConfirmAction {
    Send,
    Discard,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DashboardExitRequest {
    QuitClient,
    ReturnToLobby,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OwnedPlanetPopupMode {
    Browse,
    CommissionSelect,
    CommissionResult,
    MassCommissionConfirm,
    MassCommissionReport,
    TransportFleetSelect { mode: ArmyTransportMode },
    TransportQuantity { mode: ArmyTransportMode },
    ScorchConfirm1,
    ScorchConfirm2,
    ScorchConfirm3,
}

#[derive(Debug, Clone)]
pub struct OwnedPlanetPopupState {
    pub mode: OwnedPlanetPopupMode,
    pub input: String,
    pub default: String,
    pub status: Option<String>,
    pub transport_selected_fleet_record_index_1_based: Option<usize>,
    pub transport_selected_fleet_number: Option<u16>,
    pub transport_available_qty: u16,
    pub report_lines: Vec<String>,
}

impl Default for OwnedPlanetPopupState {
    fn default() -> Self {
        Self {
            mode: OwnedPlanetPopupMode::Browse,
            input: String::new(),
            default: String::new(),
            status: None,
            transport_selected_fleet_record_index_1_based: None,
            transport_selected_fleet_number: None,
            transport_available_qty: 0,
            report_lines: Vec::new(),
        }
    }
}

impl OwnedPlanetPopupState {
    pub fn reset(&mut self) {
        *self = Self::default();
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StartupReviewMode {
    Results,
    Messages,
}

#[derive(Debug, Clone)]
pub struct StartupReviewPopupState {
    pub mode: StartupReviewMode,
    pub results_block: usize,
    pub results_scroll: usize,
    pub results_nonstop: bool,
    pub messages_block: usize,
    pub messages_scroll: usize,
    pub messages_nonstop: bool,
    pub reports: nc_data::ReportsPreview,
}

impl Default for StartupReviewPopupState {
    fn default() -> Self {
        Self {
            mode: StartupReviewMode::Results,
            results_block: 0,
            results_scroll: 0,
            results_nonstop: false,
            messages_block: 0,
            messages_scroll: 0,
            messages_nonstop: false,
            reports: nc_data::ReportsPreview {
                results_lines: Vec::new(),
                message_lines: Vec::new(),
                result_blocks: Vec::new(),
                message_blocks: Vec::new(),
            },
        }
    }
}

impl StartupReviewPopupState {
    pub fn reset(&mut self) {
        *self = Self::default();
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MapViewMode {
    Readable,
    Fill,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActiveMouseGesture {
    None,
    DraggingOverlay {
        grab_col_offset: usize,
        grab_row_offset: usize,
    },
    DraggingPopup {
        grab_col_offset: usize,
        grab_row_offset: usize,
    },
    DraggingStarmap {
        anchor_col: u16,
        anchor_row: u16,
        start_x_min: u8,
        start_y_min: u8,
        cell_width: u16,
        cell_height: u16,
        drag_occurred: bool,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HelpContext {
    Global,
    OwnedPlanetPopup,
    PlanetList,
    PlanetListSort,
    PlanetListFilter,
    PlanetBuildSpecify,
    PlanetBuildQuantity,
    PromptInput,
    FleetList,
    FleetListSort,
    FleetListFilter,
    FleetMissionPicker,
    FleetOrderInput,
    StarbaseMove,
    IntelDatabase,
    IntelDatabaseSort,
    IntelDatabaseFilter,
    Inbox,
    InboxCompose,
    Diplomacy,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InboxFocus {
    List,
    Preview,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InboxFilter {
    All,
    Reports,
    Messages,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InboxPromptMode {
    None,
    ComposeRecipient,
    ComposeSubject,
    ComposeBody,
    ComposeConfirm,
    Outbox,
}

impl InboxFilter {
    pub const fn label(self) -> &'static str {
        match self {
            Self::All => "All",
            Self::Reports => "Reports",
            Self::Messages => "Messages",
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct ListOverlayState {
    pub selected: usize,
    pub scroll: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortDirection {
    Asc,
    Desc,
}

impl SortDirection {
    pub const fn toggle(self) -> Self {
        match self {
            Self::Asc => Self::Desc,
            Self::Desc => Self::Asc,
        }
    }

    #[cfg(test)]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Asc => "ASC",
            Self::Desc => "DESC",
        }
    }

    pub const fn title_label(self) -> &'static str {
        match self {
            Self::Asc => "ASCENDING",
            Self::Desc => "DESCENDING",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlanetOverlaySort {
    Location,
    PlanetName,
    MaxProduction,
    CurrentProduction,
    Treasury,
    Budget,
    Revenue,
    Growth,
    BuildQueue,
    Stardock,
    Starbase,
    Armies,
    Batteries,
}

pub const fn default_planet_overlay_sort_direction(sort: PlanetOverlaySort) -> SortDirection {
    match sort {
        PlanetOverlaySort::Location | PlanetOverlaySort::PlanetName => SortDirection::Asc,
        PlanetOverlaySort::MaxProduction
        | PlanetOverlaySort::CurrentProduction
        | PlanetOverlaySort::Treasury
        | PlanetOverlaySort::Budget
        | PlanetOverlaySort::Revenue
        | PlanetOverlaySort::Growth
        | PlanetOverlaySort::BuildQueue
        | PlanetOverlaySort::Stardock
        | PlanetOverlaySort::Starbase
        | PlanetOverlaySort::Armies
        | PlanetOverlaySort::Batteries => SortDirection::Desc,
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlanetOverlayFilter {
    All,
    Range { anchor: [u8; 2], radius: u8 },
    Starbase,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlanetOverlayPromptMode {
    None,
    SortMenu,
    FilterMenu,
    FilterValueInput,
    BuildSpecify,
    BuildQuantity,
    CommissionSelect,
    MassCommissionConfirm,
    TransportFleetSelect { mode: ArmyTransportMode },
    TransportQuantity { mode: ArmyTransportMode },
}

#[derive(Debug, Clone)]
pub struct PlanetOverlayPromptFrame {
    pub mode: PlanetOverlayPromptMode,
    pub prompt_input: String,
    pub prompt_default: String,
    pub prompt_status: Option<String>,
    pub pending_range_anchor: Option<[u8; 2]>,
}

#[derive(Debug, Clone)]
pub struct PlanetOverlayState {
    pub selected: usize,
    pub scroll: usize,
    pub jump_input: String,
    pub footer_notice: Option<String>,
    pub sort: PlanetOverlaySort,
    pub sort_direction: SortDirection,
    pub filter: PlanetOverlayFilter,
    pub filter_clause: Option<TableFilterClause>,
    pub pending_filter_column: Option<TableFilterColumn>,
    pub prompt_mode: PlanetOverlayPromptMode,
    pub prompt_input: String,
    pub prompt_default: String,
    pub prompt_status: Option<String>,
    pub pending_range_anchor: Option<[u8; 2]>,
    pub prompt_stack: Vec<PlanetOverlayPromptFrame>,
    pub build_planet_record_index_1_based: Option<usize>,
    pub build_unit_status: Option<String>,
    pub build_selected_kind: Option<ProductionItemKind>,
    pub build_unit_input: String,
    pub build_quantity_input: String,
    pub build_quantity_status: Option<String>,
    pub action_planet_record_index_1_based: Option<usize>,
    pub transport_selected_fleet_record_index_1_based: Option<usize>,
    pub transport_selected_fleet_number: Option<u16>,
    pub transport_available_qty: u16,
}

impl Default for PlanetOverlayState {
    fn default() -> Self {
        Self {
            selected: 0,
            scroll: 0,
            jump_input: String::new(),
            footer_notice: None,
            sort: PlanetOverlaySort::CurrentProduction,
            sort_direction: default_planet_overlay_sort_direction(
                PlanetOverlaySort::CurrentProduction,
            ),
            filter: PlanetOverlayFilter::All,
            filter_clause: None,
            pending_filter_column: None,
            prompt_mode: PlanetOverlayPromptMode::None,
            prompt_input: String::new(),
            prompt_default: String::new(),
            prompt_status: None,
            pending_range_anchor: None,
            prompt_stack: Vec::new(),
            build_planet_record_index_1_based: None,
            build_unit_status: None,
            build_selected_kind: None,
            build_unit_input: String::new(),
            build_quantity_input: String::new(),
            build_quantity_status: None,
            action_planet_record_index_1_based: None,
            transport_selected_fleet_record_index_1_based: None,
            transport_selected_fleet_number: None,
            transport_available_qty: 0,
        }
    }
}

impl PlanetOverlayState {
    pub fn open_prompt(&mut self, mode: PlanetOverlayPromptMode) {
        if self.prompt_mode != PlanetOverlayPromptMode::None {
            self.prompt_stack.push(PlanetOverlayPromptFrame {
                mode: self.prompt_mode,
                prompt_input: self.prompt_input.clone(),
                prompt_default: self.prompt_default.clone(),
                prompt_status: self.prompt_status.clone(),
                pending_range_anchor: self.pending_range_anchor,
            });
        }
        self.prompt_mode = mode;
    }

    pub fn close_prompt(&mut self) {
        if let Some(frame) = self.prompt_stack.pop() {
            self.prompt_mode = frame.mode;
            self.prompt_input = frame.prompt_input;
            self.prompt_default = frame.prompt_default;
            self.prompt_status = frame.prompt_status;
            self.pending_range_anchor = frame.pending_range_anchor;
            return;
        }
        self.clear_prompt();
    }

    pub fn clear_prompt(&mut self) {
        self.prompt_mode = PlanetOverlayPromptMode::None;
        self.prompt_input.clear();
        self.prompt_default.clear();
        self.prompt_status = None;
        self.pending_range_anchor = None;
        self.pending_filter_column = None;
        self.prompt_stack.clear();
        self.action_planet_record_index_1_based = None;
        self.transport_selected_fleet_record_index_1_based = None;
        self.transport_selected_fleet_number = None;
        self.transport_available_qty = 0;
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FleetOverlayRowKey {
    Fleet(usize),
    Starbase(usize),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FleetOverlaySort {
    Id,
    Selected,
    Location,
    Order,
    Target,
    Speed,
    Eta,
    Roe,
    Armies,
    Strength,
}

pub const fn default_fleet_overlay_sort_direction(sort: FleetOverlaySort) -> SortDirection {
    match sort {
        FleetOverlaySort::Location
        | FleetOverlaySort::Order
        | FleetOverlaySort::Target
        | FleetOverlaySort::Eta => SortDirection::Asc,
        FleetOverlaySort::Id
        | FleetOverlaySort::Selected
        | FleetOverlaySort::Speed
        | FleetOverlaySort::Roe
        | FleetOverlaySort::Armies
        | FleetOverlaySort::Strength => SortDirection::Desc,
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FleetOverlayFilter {
    All,
    Holding,
    Combat,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FleetOverlayChangeField {
    Roe,
    Id,
    Speed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FleetOverlayTransferClass {
    Battleships,
    Cruisers,
    Destroyers,
    FullTransports,
    EmptyTransports,
    Scouts,
    Etacs,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FleetOverlayTransferMode {
    ChoosingClass,
    EnteringQuantity(FleetOverlayTransferClass),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FleetOverlayPromptMode {
    None,
    SortMenu,
    FilterMenu,
    FilterValueInput,
    ChangeField,
    ChangeValue,
    MergeHost,
    MergeConfirm,
    TransferHost,
    TransferStage,
    MissionPicker,
    OrderTarget,
    OrderTargetX,
    OrderTargetY,
    /// ECGAME's Seek Home question for a fleet already in its nearest system.
    SeekNextClosest,
    OrderConfirm,
    StarbaseMoveDecision,
    StarbaseMoveDestination,
    StarbaseHaltConfirm,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FleetOrderScope {
    None,
    SingleFleet,
    Group,
    StarbaseMove,
}

#[derive(Debug, Clone)]
pub struct FleetOverlayState {
    pub selected: usize,
    pub scroll: usize,
    pub jump_input: String,
    pub sort: FleetOverlaySort,
    pub sort_direction: SortDirection,
    pub filter: FleetOverlayFilter,
    pub filter_clause: Option<TableFilterClause>,
    pub pending_filter_column: Option<TableFilterColumn>,
    pub filter_prompt_input: String,
    pub filter_prompt_default: String,
    pub filter_prompt_status: Option<String>,
    pub location_filter: Option<[u8; 2]>,
    pub selected_fleet_record_indexes: BTreeSet<usize>,
    pub prompt_mode: FleetOverlayPromptMode,
    pub aux_input: String,
    pub aux_default: String,
    pub aux_status: Option<String>,
    pub change_field: Option<FleetOverlayChangeField>,
    pub transfer_mode: FleetOverlayTransferMode,
    pub transfer_donor_record_index_1_based: Option<usize>,
    pub transfer_host_record_index_1_based: Option<usize>,
    pub transfer_selection: FleetDetachSelection,
    pub order_scope: FleetOrderScope,
    pub active_row_key: Option<FleetOverlayRowKey>,
    pub mission_picker_input: String,
    pub mission_picker_cursor: usize,
    pub mission_picker_status: Option<String>,
    pub order_mission_code: Option<u8>,
    pub order_status: Option<String>,
    pub order_input: String,
    pub order_target_x_input: String,
    pub order_target_y_input: String,
    pub order_confirm_input: String,
    pub starbase_move_input: String,
    pub starbase_move_status: Option<String>,
    pub prompt_stack: Vec<FleetOverlayPromptMode>,
}

impl Default for FleetOverlayState {
    fn default() -> Self {
        Self {
            selected: 0,
            scroll: 0,
            jump_input: String::new(),
            sort: FleetOverlaySort::Id,
            sort_direction: default_fleet_overlay_sort_direction(FleetOverlaySort::Id),
            filter: FleetOverlayFilter::All,
            filter_clause: None,
            pending_filter_column: None,
            filter_prompt_input: String::new(),
            filter_prompt_default: String::new(),
            filter_prompt_status: None,
            location_filter: None,
            selected_fleet_record_indexes: BTreeSet::new(),
            prompt_mode: FleetOverlayPromptMode::None,
            aux_input: String::new(),
            aux_default: String::new(),
            aux_status: None,
            change_field: None,
            transfer_mode: FleetOverlayTransferMode::ChoosingClass,
            transfer_donor_record_index_1_based: None,
            transfer_host_record_index_1_based: None,
            transfer_selection: FleetDetachSelection::default(),
            order_scope: FleetOrderScope::None,
            active_row_key: None,
            mission_picker_input: String::new(),
            mission_picker_cursor: 0,
            mission_picker_status: None,
            order_mission_code: None,
            order_status: None,
            order_input: String::new(),
            order_target_x_input: String::new(),
            order_target_y_input: String::new(),
            order_confirm_input: String::new(),
            starbase_move_input: String::new(),
            starbase_move_status: None,
            prompt_stack: Vec::new(),
        }
    }
}

impl FleetOverlayState {
    pub fn open_prompt(&mut self, mode: FleetOverlayPromptMode) {
        if self.prompt_mode != FleetOverlayPromptMode::None {
            self.prompt_stack.push(self.prompt_mode);
        }
        self.prompt_mode = mode;
    }

    pub fn close_prompt(&mut self) {
        self.prompt_mode = self
            .prompt_stack
            .pop()
            .unwrap_or(FleetOverlayPromptMode::None);
    }

    pub fn clear_prompt(&mut self) {
        self.prompt_mode = FleetOverlayPromptMode::None;
        self.pending_filter_column = None;
        self.filter_prompt_input.clear();
        self.filter_prompt_default.clear();
        self.filter_prompt_status = None;
        self.aux_input.clear();
        self.aux_default.clear();
        self.aux_status = None;
        self.change_field = None;
        self.transfer_mode = FleetOverlayTransferMode::ChoosingClass;
        self.transfer_donor_record_index_1_based = None;
        self.transfer_host_record_index_1_based = None;
        self.transfer_selection = FleetDetachSelection::default();
        self.prompt_stack.clear();
    }

    pub fn clear_group_selection(&mut self) {
        self.selected_fleet_record_indexes.clear();
    }

    pub fn clear_transient_location_filter(&mut self) {
        self.location_filter = None;
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IntelOverlaySort {
    Location,
    Range([u8; 2]),
    PlanetName,
    Owner,
    MaxProduction,
    YearSeen,
    Armies,
    Batteries,
    Starbases,
    CurrentProduction,
    Treasury,
    ScoutYear,
}

pub const fn default_intel_overlay_sort_direction(sort: IntelOverlaySort) -> SortDirection {
    match sort {
        IntelOverlaySort::Location
        | IntelOverlaySort::Range(_)
        | IntelOverlaySort::PlanetName
        | IntelOverlaySort::Owner => SortDirection::Asc,
        IntelOverlaySort::MaxProduction
        | IntelOverlaySort::YearSeen
        | IntelOverlaySort::Armies
        | IntelOverlaySort::Batteries
        | IntelOverlaySort::Starbases
        | IntelOverlaySort::CurrentProduction
        | IntelOverlaySort::Treasury
        | IntelOverlaySort::ScoutYear => SortDirection::Desc,
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IntelOverlayFilter {
    All,
    Empire(u8),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IntelOverlayPromptMode {
    None,
    SortMenu,
    SortRangeInput,
    FilterMenu,
    FilterValueInput,
}

#[derive(Debug, Clone)]
pub struct IntelOverlayPromptFrame {
    pub mode: IntelOverlayPromptMode,
    pub prompt_input: String,
    pub prompt_default: String,
    pub prompt_status: Option<String>,
    pub pending_range_anchor: Option<[u8; 2]>,
}

#[derive(Debug, Clone)]
pub struct IntelOverlayState {
    pub selected: usize,
    pub scroll: usize,
    pub jump_input: String,
    pub sort: IntelOverlaySort,
    pub sort_direction: SortDirection,
    pub filter: IntelOverlayFilter,
    pub filter_clause: Option<TableFilterClause>,
    pub pending_filter_column: Option<TableFilterColumn>,
    pub prompt_mode: IntelOverlayPromptMode,
    pub prompt_input: String,
    pub prompt_default: String,
    pub prompt_status: Option<String>,
    pub pending_range_anchor: Option<[u8; 2]>,
    pub prompt_stack: Vec<IntelOverlayPromptFrame>,
}

impl Default for IntelOverlayState {
    fn default() -> Self {
        Self {
            selected: 0,
            scroll: 0,
            jump_input: String::new(),
            sort: IntelOverlaySort::Location,
            sort_direction: default_intel_overlay_sort_direction(IntelOverlaySort::Location),
            filter: IntelOverlayFilter::All,
            filter_clause: None,
            pending_filter_column: None,
            prompt_mode: IntelOverlayPromptMode::None,
            prompt_input: String::new(),
            prompt_default: String::new(),
            prompt_status: None,
            pending_range_anchor: None,
            prompt_stack: Vec::new(),
        }
    }
}

impl IntelOverlayState {
    pub fn open_prompt(&mut self, mode: IntelOverlayPromptMode) {
        if self.prompt_mode != IntelOverlayPromptMode::None {
            self.prompt_stack.push(IntelOverlayPromptFrame {
                mode: self.prompt_mode,
                prompt_input: self.prompt_input.clone(),
                prompt_default: self.prompt_default.clone(),
                prompt_status: self.prompt_status.clone(),
                pending_range_anchor: self.pending_range_anchor,
            });
        }
        self.prompt_mode = mode;
    }

    pub fn close_prompt(&mut self) {
        if let Some(frame) = self.prompt_stack.pop() {
            self.prompt_mode = frame.mode;
            self.prompt_input = frame.prompt_input;
            self.prompt_default = frame.prompt_default;
            self.prompt_status = frame.prompt_status;
            self.pending_range_anchor = frame.pending_range_anchor;
            return;
        }
        self.clear_prompt();
    }

    pub fn clear_prompt(&mut self) {
        self.prompt_mode = IntelOverlayPromptMode::None;
        self.prompt_input.clear();
        self.prompt_default.clear();
        self.prompt_status = None;
        self.pending_range_anchor = None;
        self.pending_filter_column = None;
        self.prompt_stack.clear();
    }
}

#[derive(Debug, Clone)]
pub struct InboxOverlayState {
    pub selected: usize,
    pub scroll: usize,
    pub preview_scroll: usize,
    pub focus: InboxFocus,
    pub filter: InboxFilter,
    pub current_year_only: bool,
    pub delete_confirm: bool,
    pub jump_input: String,
    pub prompt_mode: InboxPromptMode,
    pub prompt_input: String,
    pub prompt_default: String,
    pub prompt_status: Option<String>,
    pub compose_recipient_empire: Option<u8>,
    pub compose_recipient_selected: usize,
    pub compose_recipient_scroll: usize,
    pub compose_subject: String,
    pub compose_body: String,
    pub compose_body_cursor_row: usize,
    pub compose_body_cursor_col: usize,
    pub outbox_selected: usize,
    pub outbox_scroll: usize,
    pub outbox_preview_scroll: usize,
    pub outbox_focus: InboxFocus,
}

impl Default for InboxOverlayState {
    fn default() -> Self {
        Self {
            selected: 0,
            scroll: 0,
            preview_scroll: 0,
            focus: InboxFocus::List,
            filter: InboxFilter::All,
            current_year_only: false,
            delete_confirm: false,
            jump_input: String::new(),
            prompt_mode: InboxPromptMode::None,
            prompt_input: String::new(),
            prompt_default: String::new(),
            prompt_status: None,
            compose_recipient_empire: None,
            compose_recipient_selected: 0,
            compose_recipient_scroll: 0,
            compose_subject: String::new(),
            compose_body: String::new(),
            compose_body_cursor_row: 0,
            compose_body_cursor_col: 0,
            outbox_selected: 0,
            outbox_scroll: 0,
            outbox_preview_scroll: 0,
            outbox_focus: InboxFocus::List,
        }
    }
}

/// Dashboard application state.
pub struct DashApp {
    pub _game_dir: std::path::PathBuf,
    pub campaign_store: Option<CampaignStore>,
    pub hosted_turn_draft: Option<TurnSubmission>,
    pub game_data: CoreGameData,
    pub owned_planet_years: BTreeMap<usize, u16>,
    pub planet_scorch_orders: BTreeSet<usize>,
    pub report_block_rows: Vec<ReportBlockRow>,
    pub queued_mail: Vec<QueuedPlayerMail>,
    pub planet_intel_snapshots: Vec<PlanetIntelSnapshot>,
    pub player_activity_states: Vec<PlayerActivityState>,
    pub player_lifecycle_states: Vec<PlayerLifecycleState>,
    pub winner_state: WinnerState,
    pub player_war_stats: nc_data::PlayerWarStatsState,
    /// Full terminal dimensions (canvas).
    pub geometry: ScreenGeometry,
    /// Dashboard frame dimensions (map + panels + borders).
    pub frame: ScreenGeometry,
    pub player_record_index_1_based: usize,

    // Startup flow
    pub _startup_sequence: StartupSequence,
    pub _startup_phase: StartupPhase,

    // Dashboard navigation
    pub focus: PanelFocus,
    pub overlay: ActiveOverlay,
    pub popup: ActivePopup,
    pub help_return_overlay: ActiveOverlay,
    pub overlay_position: Option<RelativePopupOrigin>,
    pub popup_position: Option<RelativePopupOrigin>,
    pub help_return_overlay_position: Option<RelativePopupOrigin>,
    pub mouse_gesture: ActiveMouseGesture,
    pub help_context: HelpContext,

    // Starmap crosshair (1-based sector coords)
    pub crosshair_x: u8,
    pub crosshair_y: u8,
    pub map_view_mode: MapViewMode,
    pub starmap_viewport_x_min: u8,
    pub starmap_viewport_y_min: u8,
    pub last_starmap_cell_dims: Option<(u16, u16)>,
    pub map_coord_input: String,
    pub client_settings: DashClientSettings,
    pub client_settings_path: Option<std::path::PathBuf>,

    // Panel scroll positions
    pub diplomacy_scroll: usize,

    // Overlay-local state
    pub planet_overlay: PlanetOverlayState,
    pub fleet_overlay: FleetOverlayState,
    pub intel_overlay: IntelOverlayState,
    pub diplomacy_overlay: ListOverlayState,
    pub inbox_overlay: InboxOverlayState,
    pub owned_planet_popup: OwnedPlanetPopupState,
    pub startup_review: StartupReviewPopupState,
    pub tax_prompt_input: String,
    pub tax_prompt_status: Option<String>,

    pub is_terminal_too_small: bool,
    pub should_quit: bool,
    pub exit_request: Option<DashboardExitRequest>,
    pub quit_confirm_return_popup: ActivePopup,
    pub quit_confirm_return_popup_position: Option<RelativePopupOrigin>,
    pub command_line_toast_message: Option<String>,
    pub command_line_toast_deadline: Option<Instant>,

    pub(crate) game_data_revision: u64,
    pub(crate) panel_cache: std::cell::RefCell<PanelCache>,
    pub(crate) starmap_projection_cache:
        std::cell::RefCell<Option<super::panel_cache::CachedStarmapProjection>>,
    pub(crate) sector_detail_cache:
        std::cell::RefCell<Option<super::panel_cache::CachedSectorDetails>>,
}

impl Clone for DashApp {
    fn clone(&self) -> Self {
        Self {
            _game_dir: self._game_dir.clone(),
            campaign_store: self.campaign_store.clone(),
            hosted_turn_draft: self.hosted_turn_draft.clone(),
            game_data: self.game_data.clone(),
            owned_planet_years: self.owned_planet_years.clone(),
            planet_scorch_orders: self.planet_scorch_orders.clone(),
            report_block_rows: self.report_block_rows.clone(),
            queued_mail: self.queued_mail.clone(),
            planet_intel_snapshots: self.planet_intel_snapshots.clone(),
            player_activity_states: self.player_activity_states.clone(),
            player_lifecycle_states: self.player_lifecycle_states.clone(),
            winner_state: self.winner_state.clone(),
            player_war_stats: self.player_war_stats,
            geometry: self.geometry,
            frame: self.frame,
            player_record_index_1_based: self.player_record_index_1_based,
            _startup_sequence: self._startup_sequence.clone(),
            _startup_phase: self._startup_phase,
            focus: self.focus,
            overlay: self.overlay,
            popup: self.popup,
            help_return_overlay: self.help_return_overlay,
            overlay_position: self.overlay_position,
            popup_position: self.popup_position,
            help_return_overlay_position: self.help_return_overlay_position,
            mouse_gesture: self.mouse_gesture,
            help_context: self.help_context,
            crosshair_x: self.crosshair_x,
            crosshair_y: self.crosshair_y,
            map_view_mode: self.map_view_mode,
            starmap_viewport_x_min: self.starmap_viewport_x_min,
            starmap_viewport_y_min: self.starmap_viewport_y_min,
            last_starmap_cell_dims: self.last_starmap_cell_dims,
            map_coord_input: self.map_coord_input.clone(),
            client_settings: self.client_settings.clone(),
            client_settings_path: self.client_settings_path.clone(),
            diplomacy_scroll: self.diplomacy_scroll,
            planet_overlay: self.planet_overlay.clone(),
            fleet_overlay: self.fleet_overlay.clone(),
            intel_overlay: self.intel_overlay.clone(),
            diplomacy_overlay: self.diplomacy_overlay.clone(),
            inbox_overlay: self.inbox_overlay.clone(),
            owned_planet_popup: self.owned_planet_popup.clone(),
            startup_review: self.startup_review.clone(),
            tax_prompt_input: self.tax_prompt_input.clone(),
            tax_prompt_status: self.tax_prompt_status.clone(),
            is_terminal_too_small: self.is_terminal_too_small,
            should_quit: self.should_quit,
            exit_request: self.exit_request,
            quit_confirm_return_popup: self.quit_confirm_return_popup,
            quit_confirm_return_popup_position: self.quit_confirm_return_popup_position,
            command_line_toast_message: self.command_line_toast_message.clone(),
            command_line_toast_deadline: self.command_line_toast_deadline,
            game_data_revision: self.game_data_revision,
            panel_cache: std::cell::RefCell::new(PanelCache::default()),
            starmap_projection_cache: std::cell::RefCell::new(None),
            sector_detail_cache: std::cell::RefCell::new(None),
        }
    }
}

impl DashApp {
    pub fn new(
        game_dir: std::path::PathBuf,
        campaign_store: Option<CampaignStore>,
        game_data: CoreGameData,
        owned_planet_years: BTreeMap<usize, u16>,
        planet_scorch_orders: BTreeSet<usize>,
        report_block_rows: Vec<ReportBlockRow>,
        queued_mail: Vec<QueuedPlayerMail>,
        planet_intel_snapshots: Vec<PlanetIntelSnapshot>,
        player_activity_states: Vec<PlayerActivityState>,
        player_lifecycle_states: Vec<PlayerLifecycleState>,
        winner_state: WinnerState,
        geometry: ScreenGeometry,
        frame: ScreenGeometry,
        player_record_index_1_based: usize,
    ) -> Self {
        let startup_summary = StartupSummary {
            game_year: game_data.conquest.game_year(),
            show_login_review: false,
            pending_results: false,
            pending_messages: false,
            results_line_count: 0,
            message_line_count: 0,
        };
        let startup_sequence = StartupSequence::new(&startup_summary);
        let [crosshair_x, crosshair_y] =
            initial_crosshair_coords(&game_data, player_record_index_1_based);
        Self {
            _game_dir: game_dir,
            campaign_store,
            hosted_turn_draft: None,
            game_data,
            owned_planet_years,
            planet_scorch_orders,
            report_block_rows,
            queued_mail,
            planet_intel_snapshots,
            player_activity_states,
            player_lifecycle_states,
            winner_state,
            player_war_stats: nc_data::PlayerWarStatsState::for_player(player_record_index_1_based),
            geometry,
            frame,
            player_record_index_1_based,
            _startup_phase: StartupPhase::Complete,
            _startup_sequence: startup_sequence,
            focus: PanelFocus::Map,
            overlay: ActiveOverlay::None,
            popup: ActivePopup::None,
            help_return_overlay: ActiveOverlay::None,
            overlay_position: None,
            popup_position: None,
            help_return_overlay_position: None,
            mouse_gesture: ActiveMouseGesture::None,
            help_context: HelpContext::Global,
            crosshair_x,
            crosshair_y,
            map_view_mode: MapViewMode::Readable,
            starmap_viewport_x_min: 0,
            starmap_viewport_y_min: 0,
            last_starmap_cell_dims: None,
            map_coord_input: String::new(),
            client_settings: DashClientSettings::default(),
            client_settings_path: None,
            diplomacy_scroll: 0,
            planet_overlay: PlanetOverlayState::default(),
            fleet_overlay: FleetOverlayState::default(),
            intel_overlay: IntelOverlayState::default(),
            diplomacy_overlay: ListOverlayState::default(),
            inbox_overlay: InboxOverlayState::default(),
            owned_planet_popup: OwnedPlanetPopupState::default(),
            startup_review: StartupReviewPopupState::default(),
            tax_prompt_input: String::new(),
            tax_prompt_status: None,
            is_terminal_too_small: false,
            should_quit: false,
            exit_request: None,
            quit_confirm_return_popup: ActivePopup::None,
            quit_confirm_return_popup_position: None,
            command_line_toast_message: None,
            command_line_toast_deadline: None,
            game_data_revision: 0,
            panel_cache: std::cell::RefCell::new(PanelCache::default()),
            starmap_projection_cache: std::cell::RefCell::new(None),
            sector_detail_cache: std::cell::RefCell::new(None),
        }
    }

    #[cfg(test)]
    pub fn new_for_tests(
        game_dir: std::path::PathBuf,
        game_data: CoreGameData,
        owned_planet_years: BTreeMap<usize, u16>,
        planet_scorch_orders: BTreeSet<usize>,
        report_block_rows: Vec<ReportBlockRow>,
        queued_mail: Vec<QueuedPlayerMail>,
        planet_intel_snapshots: Vec<PlanetIntelSnapshot>,
        geometry: ScreenGeometry,
        frame: ScreenGeometry,
        player_record_index_1_based: usize,
    ) -> Self {
        Self::new(
            game_dir,
            None,
            game_data,
            owned_planet_years,
            planet_scorch_orders,
            report_block_rows,
            queued_mail,
            planet_intel_snapshots,
            Vec::new(),
            Vec::new(),
            WinnerState::default(),
            geometry,
            frame,
            player_record_index_1_based,
        )
    }

    #[doc(hidden)]
    pub fn new_for_repro(geometry: ScreenGeometry, frame: ScreenGeometry) -> Self {
        Self::new(
            std::path::PathBuf::from("."),
            None,
            GameStateBuilder::new()
                .with_player_count(4)
                .build_initialized_baseline()
                .expect("baseline"),
            BTreeMap::new(),
            BTreeSet::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            WinnerState::default(),
            geometry,
            frame,
            1,
        )
    }

    pub fn overlay_position_for(&self, overlay: ActiveOverlay) -> Option<RelativePopupOrigin> {
        if self.overlay == overlay {
            self.overlay_position
        } else if self.overlay == ActiveOverlay::Help && self.help_return_overlay == overlay {
            self.help_return_overlay_position
        } else {
            None
        }
    }

    pub fn popup_position_for(&self, popup: ActivePopup) -> Option<RelativePopupOrigin> {
        (self.popup == popup)
            .then_some(self.popup_position)
            .flatten()
    }

    pub fn is_hosted_mode(&self) -> bool {
        self.hosted_turn_draft.is_some()
    }

    pub fn hosted_turn_text(&self) -> Option<String> {
        self.hosted_turn_draft
            .as_ref()
            .filter(|submission| {
                submission.tax_rate.is_some()
                    || !submission.diplomacy.is_empty()
                    || !submission.planets.is_empty()
                    || !submission.fleets.is_empty()
                    || !submission.messages.is_empty()
            })
            .map(TurnSubmission::to_kdl_string)
    }
}

pub(crate) fn initial_crosshair_coords(
    game_data: &CoreGameData,
    player_record_index_1_based: usize,
) -> [u8; 2] {
    if let Some(coords) = game_data
        .player_homeworld_seed_coords_current_known()
        .get(player_record_index_1_based.saturating_sub(1))
        .and_then(|coords| *coords)
    {
        return coords;
    }
    let Some(player) = game_data
        .players
        .get(player_record_index_1_based.saturating_sub(1))
    else {
        return [1, 1];
    };
    let homeworld_index = player.homeworld_planet_index_1_based_raw() as usize;
    game_data
        .planets
        .get(homeworld_index.saturating_sub(1))
        .map(|planet| planet.coords_raw())
        .unwrap_or([1, 1])
}

#[cfg(test)]
mod tests {
    use super::{DashApp, PanelFocus};
    use crate::dashboard::geometry::ScreenGeometry;
    use nc_data::GameStateBuilder;
    use std::collections::{BTreeMap, BTreeSet};
    use std::path::PathBuf;

    #[test]
    fn focus_cycle_skips_removed_reports_panel() {
        assert_eq!(PanelFocus::Diplomacy.next(), PanelFocus::Map);
        assert_eq!(PanelFocus::Map.prev(), PanelFocus::Diplomacy);
        assert_eq!(PanelFocus::Fleets.next(), PanelFocus::WarRecord);
        assert_eq!(PanelFocus::Galaxy.prev(), PanelFocus::WarRecord);
    }

    #[test]
    fn new_app_starts_crosshair_on_player_homeworld() {
        let app = DashApp::new_for_tests(
            PathBuf::from("."),
            GameStateBuilder::new()
                .with_player_count(4)
                .build_initialized_baseline()
                .expect("baseline"),
            BTreeMap::new(),
            BTreeSet::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            ScreenGeometry::new(160, 45),
            ScreenGeometry::new(0, 0),
            1,
        );
        let coords = app.game_data.player_homeworld_seed_coords_current_known()[0]
            .expect("player one homeworld seed");

        assert_eq!([app.crosshair_x, app.crosshair_y], coords);
    }
}
