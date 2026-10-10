use super::manip::{fleet_eta_label, fleet_list_eta_label};
use super::orders::resolve_yes_no_input;
use crate::app::helpers::{
    is_coordinate_input_char, resolve_default_coords_input, sync_scroll_to_cursor,
};
use crate::app::state::App;
use crate::domains::fleet::FleetAction;
use crate::domains::fleet::state::{FleetChangeField, FleetCommandContext, FleetMenuPromptMode};
use crate::screen::layout::PromptFeedback;
use crate::screen::{
    CommandMenu, FleetEtaMode, FleetListFilter, FleetListFilterPromptMode, FleetListSort, FleetRow,
    PlanetTransportMode, ScreenId, SortDirection,
};
use nc_data::{FleetRecord, Order};
use nc_engine::{
    FleetTargetInputKind, SelectedFleetRef, fleet_target_input_kind, fleet_target_status_line,
};
use nc_ui::table_filter::{
    ColumnCodeParseError, FilterKind, TableFilterClause, TableFilterColumn, TableFilterPredicate,
    format_column_code_error, is_filter_column_char, is_filter_value_char, parse_column_code,
    parse_filter_clause,
};
use std::cmp::{Ordering, Reverse};

fn fleet_strength_key(fleet: &FleetRecord) -> (u16, u16, u16, u16, u16, u16, Reverse<u16>) {
    (
        fleet.battleship_count(),
        fleet.cruiser_count(),
        fleet.destroyer_count(),
        fleet.troop_transport_count(),
        fleet.scout_count(),
        fleet.etac_count(),
        Reverse(fleet.local_slot_word_raw()),
    )
}

fn fleet_combat_only_strength_key(fleet: &FleetRecord) -> (u16, u16, u16, Reverse<u16>) {
    (
        fleet.battleship_count(),
        fleet.cruiser_count(),
        fleet.destroyer_count(),
        Reverse(fleet.local_slot_word_raw()),
    )
}

fn fleet_is_idle_hold(fleet: &FleetRecord) -> bool {
    fleet.standing_order_kind() == Order::HoldPosition
}

fn fleet_is_combat_only_fallback_candidate(fleet: &FleetRecord) -> bool {
    (fleet.destroyer_count() > 0 || fleet.cruiser_count() > 0 || fleet.battleship_count() > 0)
        && fleet.scout_count() == 0
        && fleet.etac_count() == 0
}

fn fleet_matches_filter(row: &FleetRow, filter: FleetListFilter) -> bool {
    let order = Order::from_raw(row.order_code);
    match filter {
        FleetListFilter::All => true,
        FleetListFilter::Holding => order == Order::HoldPosition,
        FleetListFilter::Moving => matches!(
            order,
            Order::MoveOnly
                | Order::SeekHome
                | Order::PatrolSector
                | Order::ViewWorld
                | Order::ScoutSector
                | Order::ScoutSolarSystem
                | Order::ColonizeWorld
                | Order::JoinAnotherFleet
                | Order::RendezvousSector
                | Order::Salvage
        ),
        FleetListFilter::Combat => matches!(
            order,
            Order::GuardStarbase
                | Order::GuardBlockadeWorld
                | Order::BombardWorld
                | Order::InvadeWorld
                | Order::BlitzWorld
        ),
    }
}

const FLEET_FILTER_COLUMNS: &[TableFilterColumn] = &[
    TableFilterColumn {
        code: "id",
        label: "Fleet ID",
        aliases: &["fleet", "fleetid"],
        kind: FilterKind::Number,
    },
    TableFilterColumn {
        code: "loc",
        label: "Location",
        aliases: &["coord", "coordinates"],
        kind: FilterKind::Coord,
    },
    TableFilterColumn {
        code: "ord",
        label: "Order",
        aliases: &[],
        kind: FilterKind::Text,
    },
    TableFilterColumn {
        code: "tar",
        label: "Target",
        aliases: &["destination"],
        kind: FilterKind::Coord,
    },
    TableFilterColumn {
        code: "spd",
        label: "Speed",
        aliases: &[],
        kind: FilterKind::Number,
    },
    TableFilterColumn {
        code: "eta",
        label: "ETA",
        aliases: &["arrival"],
        kind: FilterKind::Text,
    },
    TableFilterColumn {
        code: "roe",
        label: "ROE",
        aliases: &["rules", "engagement"],
        kind: FilterKind::Number,
    },
    TableFilterColumn {
        code: "ars",
        label: "Armies",
        aliases: &[],
        kind: FilterKind::Number,
    },
    TableFilterColumn {
        code: "shi",
        label: "Ships",
        aliases: &["forces"],
        kind: FilterKind::Text,
    },
    TableFilterColumn {
        code: "sel",
        label: "Selected",
        aliases: &["marked"],
        kind: FilterKind::Bool,
    },
];

fn fleet_matches_clause(
    row: &FleetRow,
    clause: &TableFilterClause,
    selected_fleet_record_indexes: &std::collections::BTreeSet<usize>,
) -> bool {
    match clause.column.code {
        "id" => clause
            .predicate
            .matches_number(Some(i64::from(row.fleet_number))),
        "loc" => clause.predicate.matches_coord(row.coords),
        "ord" => match &clause.predicate {
            TableFilterPredicate::TextContains(value) if value == "holding" => {
                fleet_matches_filter(row, FleetListFilter::Holding)
            }
            TableFilterPredicate::TextContains(value) if value == "moving" => {
                fleet_matches_filter(row, FleetListFilter::Moving)
            }
            TableFilterPredicate::TextContains(value) if value == "combat" => {
                fleet_matches_filter(row, FleetListFilter::Combat)
            }
            predicate => predicate.matches_text(Some(&row.order_label)),
        },
        "tar" => clause.predicate.matches_coord(row.target_coords),
        "spd" => clause
            .predicate
            .matches_number(Some(i64::from(row.current_speed))),
        "eta" => clause.predicate.matches_text(Some(&row.list_eta_label)),
        "roe" => clause
            .predicate
            .matches_number(Some(i64::from(row.rules_of_engagement))),
        "ars" => clause
            .predicate
            .matches_number(Some(i64::from(row.loaded_armies))),
        "shi" => clause.predicate.matches_text(Some(&row.composition_label)),
        "sel" => clause
            .predicate
            .matches_bool(selected_fleet_record_indexes.contains(&row.fleet_record_index_1_based)),
        _ => true,
    }
}

fn fleet_eta_sort_key(label: &str) -> (u8, u16) {
    match label {
        "0" => (0, 0),
        "S" => (1, 0),
        "X" => (3, 0),
        _ => label
            .parse::<u16>()
            .map(|value| (0, value))
            .unwrap_or((2, 0)),
    }
}

const fn fleet_list_sort_code(sort: FleetListSort) -> &'static str {
    match sort {
        FleetListSort::Id => "id",
        FleetListSort::Selected => "sel",
        FleetListSort::Location => "loc",
        FleetListSort::Order => "ord",
        FleetListSort::Target => "tar",
        FleetListSort::Speed => "spd",
        FleetListSort::Eta => "eta",
        FleetListSort::Roe => "roe",
        FleetListSort::Armies => "ars",
        FleetListSort::Strength => "shi",
    }
}

fn fleet_list_sort_from_column(column: TableFilterColumn) -> Option<FleetListSort> {
    match column.code {
        "id" => Some(FleetListSort::Id),
        "sel" => Some(FleetListSort::Selected),
        "loc" => Some(FleetListSort::Location),
        "ord" => Some(FleetListSort::Order),
        "tar" => Some(FleetListSort::Target),
        "spd" => Some(FleetListSort::Speed),
        "eta" => Some(FleetListSort::Eta),
        "roe" => Some(FleetListSort::Roe),
        "ars" => Some(FleetListSort::Armies),
        "shi" => Some(FleetListSort::Strength),
        _ => None,
    }
}

const fn default_fleet_list_sort_direction(sort: FleetListSort) -> SortDirection {
    match sort {
        FleetListSort::Id => SortDirection::Desc,
        FleetListSort::Selected => SortDirection::Desc,
        FleetListSort::Location => SortDirection::Asc,
        FleetListSort::Order => SortDirection::Asc,
        FleetListSort::Target => SortDirection::Asc,
        FleetListSort::Speed => SortDirection::Desc,
        FleetListSort::Eta => SortDirection::Asc,
        FleetListSort::Roe => SortDirection::Desc,
        FleetListSort::Armies => SortDirection::Desc,
        FleetListSort::Strength => SortDirection::Desc,
    }
}

fn apply_sort_direction(direction: SortDirection, ordering: Ordering) -> Ordering {
    match direction {
        SortDirection::Asc => ordering,
        SortDirection::Desc => ordering.reverse(),
    }
}

impl App {
    fn active_owned_fleet_records(&self) -> impl Iterator<Item = (usize, &FleetRecord)> + '_ {
        self.game_data
            .fleets
            .records
            .iter()
            .enumerate()
            .filter(|(_, fleet)| {
                fleet.owner_empire_raw() as usize == self.player.record_index_1_based
                    && fleet.has_any_force()
            })
    }

    fn active_owned_fleets(&self) -> impl Iterator<Item = &FleetRecord> + '_ {
        self.active_owned_fleet_records().map(|(_, fleet)| fleet)
    }

    fn fleet_list_visible_rows(&self) -> usize {
        crate::domains::fleet::screens::fleet::fleet_list_visible_rows(self.screen_geometry)
    }

    fn fleet_group_visible_rows(&self) -> usize {
        crate::domains::fleet::screens::fleet::fleet_visible_rows(self.screen_geometry)
    }

    pub fn open_fleet_menu(&mut self) {
        self.clear_command_menu_notice();
        self.clear_fleet_menu_prompt();
        self.clear_checked_fleet_selection();
        self.fleet.command_context = FleetCommandContext::Menu;
        self.fleet.order_return_to_menu = false;
        self.fleet.list_dismiss_message = None;
        self.fleet.dismiss_message = None;
        self.current_screen = ScreenId::FleetMenu;
    }

    pub(crate) fn clear_fleet_menu_prompt(&mut self) {
        self.fleet.menu_prompt_mode = None;
        self.fleet.menu_prompt_input.clear();
        self.fleet.menu_prompt_status = None;
        self.fleet.menu_prompt_default_value.clear();
        self.fleet.menu_prompt_context_fleet_record_index_1_based = None;
        self.fleet.menu_prompt_change_field = None;
    }

    fn clear_fleet_list_input(&mut self) {
        self.fleet.list_input.clear();
        self.fleet.list_status = None;
    }

    pub(crate) fn clear_checked_fleet_selection(&mut self) {
        self.fleet.group_selected_fleets.clear();
    }

    pub(crate) fn checked_fleet_rows(&self) -> Vec<FleetRow> {
        self.fleet_rows()
            .into_iter()
            .filter(|row| {
                self.fleet
                    .group_selected_fleets
                    .contains(&row.fleet_record_index_1_based)
            })
            .collect()
    }

    pub(crate) fn checked_fleet_refs(&self) -> Vec<SelectedFleetRef> {
        self.checked_fleet_rows()
            .into_iter()
            .map(|row| SelectedFleetRef {
                fleet_record_index_1_based: row.fleet_record_index_1_based,
                fleet_number: row.fleet_number,
                coords: row.coords,
            })
            .collect()
    }

    pub(crate) fn fleet_list_has_checked_selection(&self) -> bool {
        self.current_screen == ScreenId::FleetList && !self.fleet.group_selected_fleets.is_empty()
    }

    pub(crate) fn clear_fleet_list_dismiss_message(&mut self) {
        self.fleet.list_dismiss_message = None;
    }

    pub(crate) fn fleet_context_screen(&self) -> ScreenId {
        match self.fleet.command_context {
            FleetCommandContext::Menu => ScreenId::FleetMenu,
            FleetCommandContext::List => ScreenId::FleetList,
        }
    }

    fn clamp_fleet_list_cursor_for_len(&mut self, total: usize) {
        if total == 0 {
            self.fleet.cursor = 0;
            self.fleet.scroll_offset = 0;
            return;
        }
        self.fleet.cursor = self.fleet.cursor.min(total - 1);
        let visible_rows = self.fleet_list_visible_rows();
        sync_scroll_to_cursor(
            &mut self.fleet.scroll_offset,
            self.fleet.cursor,
            visible_rows,
        );
    }

    pub(crate) fn normalize_fleet_list_selection(&mut self) {
        self.clamp_fleet_list_cursor_for_len(self.fleet_list_rows().len());
    }

    pub(crate) fn fleet_selected_list_row(&mut self) -> Result<FleetRow, String> {
        let ScreenId::FleetList = self.current_screen else {
            return Err("Fleet list is not active.".to_string());
        };
        let rows = self.fleet_list_rows();
        if rows.is_empty() {
            return Err("You have no active fleets.".to_string());
        }
        let input = self.fleet.list_input.trim();
        if !input.is_empty() {
            let fleet_number = input
                .parse::<u16>()
                .map_err(|_| "Enter a fleet number from the table.".to_string())?;
            return rows
                .into_iter()
                .find(|row| row.fleet_number == fleet_number)
                .ok_or_else(|| format!("Fleet #{fleet_number} is not in your fleet list."));
        }
        self.clamp_fleet_list_cursor_for_len(rows.len());
        rows.get(self.fleet.cursor)
            .cloned()
            .ok_or_else(|| "You have no active fleets.".to_string())
    }

    pub(crate) fn dismiss_fleet_message(&mut self) {
        if self.fleet.list_dismiss_message.is_some() {
            self.fleet.list_dismiss_message = None;
            self.current_screen = ScreenId::FleetList;
            return;
        }
        self.fleet.dismiss_message = None;
        self.current_screen = self.fleet_context_screen();
    }

    pub(crate) fn show_fleet_list_dismiss_message(&mut self, message: impl Into<String>) {
        self.clear_command_menu_notice();
        self.current_screen = ScreenId::FleetList;
        self.fleet.list_status = None;
        self.fleet.dismiss_message = None;
        self.fleet.list_dismiss_message = Some(message.into());
    }

    pub(crate) fn dismiss_fleet_list_filter_prompt_notice(&mut self) {
        self.fleet.list_filter_prompt_dismiss_message = None;
    }

    pub(crate) fn show_fleet_context_notice(&mut self, message: impl Into<String>) {
        let message = message.into();
        match self.fleet.command_context {
            FleetCommandContext::Menu => {
                self.clear_fleet_menu_prompt();
                self.show_command_menu_notice(CommandMenu::Fleet, message);
            }
            FleetCommandContext::List => {
                self.clear_fleet_menu_prompt();
                self.show_fleet_list_dismiss_message(message);
            }
        }
    }

    pub(crate) fn show_fleet_context_success(
        &mut self,
        message: impl Into<String>,
        visible_in_fleet_list: bool,
    ) {
        if visible_in_fleet_list && self.fleet.command_context == FleetCommandContext::List {
            self.clear_fleet_menu_prompt();
            self.clear_command_menu_notice();
            self.current_screen = ScreenId::FleetList;
            self.fleet.list_status = None;
            self.fleet.list_dismiss_message = None;
            self.fleet.dismiss_message = None;
            return;
        }
        self.show_fleet_context_notice(message);
    }

    pub(crate) fn show_fleet_prompt_feedback(
        &mut self,
        feedback: PromptFeedback,
    ) -> Option<PromptFeedback> {
        if self.current_screen == ScreenId::FleetList
            && self.inline_fleet_menu_prompt_active_on_current_screen()
        {
            self.show_fleet_list_dismiss_message(match &feedback {
                PromptFeedback::Notice(value) => value.clone(),
                PromptFeedback::Error(value) => format!("Error: {value}"),
                PromptFeedback::Warning(value) => format!("Warning: {value}"),
            });
            return None;
        }
        Some(feedback)
    }

    pub(crate) fn strongest_owned_fleet_number(&self) -> Option<u16> {
        self.active_owned_fleets()
            .max_by_key(|fleet| fleet_strength_key(fleet))
            .map(|fleet| fleet.local_slot_word_raw())
    }

    pub(crate) fn largest_owned_fleet_number_by_ship_total(&self) -> Option<u16> {
        self.active_owned_fleets()
            .max_by_key(|fleet| {
                (
                    u32::from(fleet.battleship_count())
                        + u32::from(fleet.cruiser_count())
                        + u32::from(fleet.destroyer_count())
                        + u32::from(fleet.troop_transport_count())
                        + u32::from(fleet.scout_count())
                        + u32::from(fleet.etac_count()),
                    Reverse(fleet.local_slot_word_raw()),
                )
            })
            .map(|fleet| fleet.local_slot_word_raw())
    }

    pub(crate) fn remember_newly_commissioned_fleet_record(
        &mut self,
        fleet_record_index_1_based: usize,
    ) {
        self.fleet
            .recently_commissioned_fleet_records_mru
            .retain(|idx| *idx != fleet_record_index_1_based);
        self.fleet
            .recently_commissioned_fleet_records_mru
            .insert(0, fleet_record_index_1_based);
    }

    fn prune_recently_commissioned_fleet_records(&mut self) {
        self.fleet
            .recently_commissioned_fleet_records_mru
            .retain(|record_index_1_based| {
                self.game_data
                    .fleets
                    .records
                    .get(*record_index_1_based - 1)
                    .is_some_and(|fleet| {
                        fleet.owner_empire_raw() as usize == self.player.record_index_1_based
                            && fleet.has_any_force()
                            && fleet_is_idle_hold(fleet)
                    })
            });
    }

    pub(crate) fn order_prompt_default_fleet_number(&mut self) -> Option<u16> {
        self.prune_recently_commissioned_fleet_records();
        let owned = self.active_owned_fleets().collect::<Vec<_>>();

        if let Some(fleet_number) = self
            .fleet
            .recently_commissioned_fleet_records_mru
            .iter()
            .find_map(|record_index_1_based| {
                self.game_data
                    .fleets
                    .records
                    .get(*record_index_1_based - 1)
                    .map(FleetRecord::local_slot_word_raw)
            })
        {
            return Some(fleet_number);
        }

        let idle_etacs = owned
            .iter()
            .copied()
            .filter(|fleet| fleet_is_idle_hold(fleet) && fleet.etac_count() > 0)
            .collect::<Vec<_>>();
        if let Some(fleet) = idle_etacs
            .into_iter()
            .max_by_key(|fleet| fleet_strength_key(fleet))
        {
            return Some(fleet.local_slot_word_raw());
        }

        let idle_hold = owned
            .iter()
            .copied()
            .filter(|fleet| fleet_is_idle_hold(fleet))
            .collect::<Vec<_>>();
        if let Some(fleet) = idle_hold
            .into_iter()
            .max_by_key(|fleet| fleet_strength_key(fleet))
        {
            return Some(fleet.local_slot_word_raw());
        }

        let fallback_combat = owned
            .iter()
            .copied()
            .filter(|fleet| fleet_is_combat_only_fallback_candidate(fleet))
            .collect::<Vec<_>>();
        if let Some(fleet) = fallback_combat
            .into_iter()
            .max_by_key(|fleet| fleet_combat_only_strength_key(fleet))
        {
            return Some(fleet.local_slot_word_raw());
        }

        owned
            .into_iter()
            .max_by_key(|fleet| fleet_strength_key(fleet))
            .map(|fleet| fleet.local_slot_word_raw())
    }

    pub(crate) fn fleet_menu_prompt_label(&self) -> Option<String> {
        let mode = self.fleet.menu_prompt_mode?;
        Some(match mode {
            FleetMenuPromptMode::Review => "Review Fleet # ".to_string(),
            FleetMenuPromptMode::Order => "Order Fleet # ".to_string(),
            FleetMenuPromptMode::ChangeFleet => "Change Fleet # ".to_string(),
            FleetMenuPromptMode::ChangeField => {
                if self.fleet_list_has_checked_selection() {
                    "Change checked fleets <R>OE or <S>peed ".to_string()
                } else {
                    "Change <R>OE, <I>D, or <S>peed ".to_string()
                }
            }
            FleetMenuPromptMode::ChangeValue => match self.fleet.menu_prompt_change_field {
                Some(FleetChangeField::Roe) => "New ROE ".to_string(),
                Some(FleetChangeField::Id) => "New Fleet ID ".to_string(),
                Some(FleetChangeField::Speed) => "New Speed ".to_string(),
                None => "New Value ".to_string(),
            },
            FleetMenuPromptMode::EtaFleet => "ETA Fleet # ".to_string(),
            FleetMenuPromptMode::DetachFleet => "Detach Fleet # ".to_string(),
            FleetMenuPromptMode::MergeSource => "Merge Fleet # ".to_string(),
            FleetMenuPromptMode::MergeHost => "Into Fleet # ".to_string(),
            FleetMenuPromptMode::MergeCheckedConfirm => "Merge checked fleets? [Y]/N ".to_string(),
            FleetMenuPromptMode::TransferDonor => "Transfer From Fleet # ".to_string(),
            FleetMenuPromptMode::TransferHost => "Transfer To Fleet # ".to_string(),
            FleetMenuPromptMode::TransportFleet(PlanetTransportMode::Load) => {
                "Load Fleet # ".to_string()
            }
            FleetMenuPromptMode::TransportFleet(PlanetTransportMode::Unload) => {
                "Unload Fleet # ".to_string()
            }
            FleetMenuPromptMode::TransportQuantity(PlanetTransportMode::Load) => {
                "How many armies to load? ".to_string()
            }
            FleetMenuPromptMode::TransportQuantity(PlanetTransportMode::Unload) => {
                "How many armies to unload? ".to_string()
            }
        })
    }

    pub(crate) fn open_fleet_menu_prompt(
        &mut self,
        mode: FleetMenuPromptMode,
        default_value: impl Into<String>,
    ) {
        if matches!(
            mode,
            FleetMenuPromptMode::Review
                | FleetMenuPromptMode::Order
                | FleetMenuPromptMode::ChangeFleet
                | FleetMenuPromptMode::EtaFleet
                | FleetMenuPromptMode::DetachFleet
                | FleetMenuPromptMode::MergeSource
                | FleetMenuPromptMode::TransferDonor
                | FleetMenuPromptMode::TransportFleet(_)
        ) && self.fleet_rows().is_empty()
        {
            self.show_command_menu_notice(CommandMenu::Fleet, "You have no active fleets.");
            return;
        }
        self.clear_command_menu_notice();
        self.current_screen = self.fleet_context_screen();
        self.fleet.list_dismiss_message = None;
        self.fleet.menu_prompt_mode = Some(mode);
        self.fleet.menu_prompt_input.clear();
        self.fleet.menu_prompt_status = None;
        self.fleet.menu_prompt_default_value = default_value.into();
    }

    pub fn open_fleet_review_prompt(&mut self) {
        self.open_fleet_menu_prompt(
            FleetMenuPromptMode::Review,
            self.strongest_owned_fleet_number()
                .map(|value| value.to_string())
                .unwrap_or_default(),
        );
    }

    pub fn open_fleet_change_prompt(&mut self) {
        if self.current_screen == ScreenId::FleetList {
            if self.fleet_list_has_checked_selection() {
                self.fleet.command_context = FleetCommandContext::List;
                self.clear_command_menu_notice();
                self.clear_fleet_list_input();
                self.clear_fleet_list_dismiss_message();
                self.fleet.menu_prompt_context_fleet_record_index_1_based = None;
                self.fleet.menu_prompt_change_field = None;
                self.fleet.menu_prompt_mode = Some(FleetMenuPromptMode::ChangeField);
                self.fleet.menu_prompt_input.clear();
                self.fleet.menu_prompt_status = None;
                self.fleet.menu_prompt_default_value = "R".to_string();
                return;
            }
            match self.fleet_selected_list_row() {
                Ok(row) => {
                    self.fleet.command_context = FleetCommandContext::List;
                    self.clear_checked_fleet_selection();
                    self.clear_command_menu_notice();
                    self.clear_fleet_list_input();
                    self.clear_fleet_list_dismiss_message();
                    self.fleet.menu_prompt_context_fleet_record_index_1_based =
                        Some(row.fleet_record_index_1_based);
                    self.fleet.menu_prompt_change_field = None;
                    self.fleet.menu_prompt_mode = Some(FleetMenuPromptMode::ChangeField);
                    self.fleet.menu_prompt_input.clear();
                    self.fleet.menu_prompt_status = None;
                    self.fleet.menu_prompt_default_value = "R".to_string();
                    return;
                }
                Err(err) => {
                    let message = if err == "You have no active fleets." {
                        err
                    } else {
                        "Fleet unavailable".to_string()
                    };
                    self.show_fleet_list_dismiss_message(message);
                    return;
                }
            }
        }
        self.fleet.command_context = FleetCommandContext::Menu;
        self.open_fleet_menu_prompt(
            FleetMenuPromptMode::ChangeFleet,
            self.strongest_owned_fleet_number()
                .map(|value| value.to_string())
                .unwrap_or_default(),
        );
    }

    pub(crate) fn inline_fleet_menu_prompt_active_on_current_screen(&self) -> bool {
        matches!(
            self.current_screen,
            ScreenId::FleetMenu | ScreenId::FleetList
        ) && self.fleet.menu_prompt_mode.is_some()
    }

    pub(crate) fn enforce_valid_fleet_filter(&mut self) {
        if self.fleet.list_filter == FleetListFilter::All && self.fleet.list_filter_clause.is_none()
        {
            return;
        }
        if !self.fleet_list_rows().is_empty() {
            return;
        }

        let previous_filter = self.fleet.list_filter;
        let previous_clause = self.fleet.list_filter_clause.clone();
        self.fleet.list_filter = FleetListFilter::All;
        self.fleet.list_filter_clause = None;
        if self.fleet_list_rows().is_empty() {
            self.fleet.list_filter = previous_filter;
            self.fleet.list_filter_clause = previous_clause;
            return;
        }

        self.clear_checked_fleet_selection();
        self.fleet.cursor = 0;
        self.fleet.scroll_offset = 0;
    }

    pub fn open_fleet_list(&mut self) {
        if self.fleet_rows().is_empty() {
            self.show_command_menu_notice(CommandMenu::Fleet, "You have no active fleets.");
            return;
        }
        self.clear_command_menu_notice();
        self.clear_fleet_menu_prompt();
        self.clear_checked_fleet_selection();
        self.fleet.command_context = FleetCommandContext::List;
        self.fleet.dismiss_message = None;
        self.fleet.list_dismiss_message = None;
        self.clear_fleet_list_input();
        self.fleet.scroll_offset = 0;
        self.fleet.cursor = 0;
        self.current_screen = ScreenId::FleetList;
    }

    pub fn open_fleet_list_filter_prompt(&mut self) {
        if self.fleet_rows().is_empty() {
            self.show_command_menu_notice(CommandMenu::Fleet, "You have no active fleets.");
            return;
        }
        self.clear_command_menu_notice();
        self.clear_fleet_menu_prompt();
        self.clear_checked_fleet_selection();
        self.fleet.list_filter_prompt_mode = FleetListFilterPromptMode::Column;
        self.fleet.list_filter_prompt_input.clear();
        self.fleet.list_filter_prompt_default_value = "all".to_string();
        self.fleet.list_filter_prompt_status = None;
        self.fleet.list_filter_prompt_dismiss_message = None;
        self.fleet.list_filter_pending_column = None;
        self.current_screen = ScreenId::FleetListFilterPrompt;
    }

    pub fn open_fleet_list_sort_prompt(&mut self) {
        self.enforce_valid_fleet_filter();
        if self.fleet_list_rows().is_empty() {
            self.show_command_menu_notice(CommandMenu::Fleet, "You have no active fleets.");
            return;
        }
        self.clear_command_menu_notice();
        self.clear_fleet_menu_prompt();
        self.fleet.list_filter_prompt_mode = FleetListFilterPromptMode::Column;
        self.fleet.list_filter_prompt_input.clear();
        self.fleet.list_filter_prompt_default_value =
            fleet_list_sort_code(self.fleet.list_sort).to_string();
        self.fleet.list_filter_prompt_status = None;
        self.fleet.list_filter_prompt_dismiss_message = None;
        self.fleet.list_filter_pending_column = None;
        self.current_screen = ScreenId::FleetListSortPrompt;
    }

    pub fn close_fleet_list_prompt(&mut self) {
        self.fleet.list_filter_prompt_mode = FleetListFilterPromptMode::Column;
        self.fleet.list_filter_prompt_input.clear();
        self.fleet.list_filter_prompt_default_value.clear();
        self.fleet.list_filter_prompt_status = None;
        self.fleet.list_filter_prompt_dismiss_message = None;
        self.fleet.list_filter_pending_column = None;
        self.current_screen = ScreenId::FleetList;
    }

    pub fn submit_fleet_list_filter(&mut self, filter: FleetListFilter) {
        self.fleet.list_filter_clause = None;
        self.clear_checked_fleet_selection();
        let selected_record = self
            .fleet_list_rows()
            .get(self.fleet.cursor)
            .map(|row| row.fleet_record_index_1_based);
        let previous = self.fleet.list_filter;
        self.fleet.list_filter = filter;
        self.current_screen = ScreenId::FleetList;
        let rows = self.fleet_list_rows();
        if rows.is_empty() {
            self.fleet.list_filter = FleetListFilter::All;
            if previous == FleetListFilter::All {
                self.fleet.cursor = 0;
                self.fleet.scroll_offset = 0;
            } else {
                let full_rows = self.fleet_list_rows();
                self.fleet.cursor = selected_record
                    .and_then(|record| {
                        full_rows
                            .iter()
                            .position(|row| row.fleet_record_index_1_based == record)
                    })
                    .unwrap_or(0);
                let visible_rows = self.fleet_list_visible_rows();
                sync_scroll_to_cursor(
                    &mut self.fleet.scroll_offset,
                    self.fleet.cursor,
                    visible_rows,
                );
            }
            return;
        }
        self.fleet.cursor = selected_record
            .and_then(|record| {
                rows.iter()
                    .position(|row| row.fleet_record_index_1_based == record)
            })
            .unwrap_or(0);
        let visible_rows = self.fleet_list_visible_rows();
        sync_scroll_to_cursor(
            &mut self.fleet.scroll_offset,
            self.fleet.cursor,
            visible_rows,
        );
    }

    pub fn submit_fleet_list_filter_prompt(&mut self) {
        if self.current_screen != ScreenId::FleetListFilterPrompt {
            return;
        }
        match self.fleet.list_filter_prompt_mode {
            FleetListFilterPromptMode::Column => {
                let raw = if self.fleet.list_filter_prompt_input.trim().is_empty() {
                    self.fleet.list_filter_prompt_default_value.trim()
                } else {
                    self.fleet.list_filter_prompt_input.trim()
                };
                if raw.eq_ignore_ascii_case("a") || raw.eq_ignore_ascii_case("all") {
                    self.apply_fleet_filter_clause(None);
                    return;
                }
                match parse_column_code(FLEET_FILTER_COLUMNS, raw) {
                    Ok(column) => {
                        self.fleet.list_filter_pending_column = Some(column);
                        self.fleet.list_filter_prompt_mode = FleetListFilterPromptMode::Value;
                        self.fleet.list_filter_prompt_input.clear();
                        self.fleet.list_filter_prompt_default_value =
                            self.fleet_filter_default_value_for(column);
                        self.fleet.list_filter_prompt_status = None;
                        self.fleet.list_filter_prompt_dismiss_message = None;
                    }
                    Err(ColumnCodeParseError::Ambiguous(codes)) => {
                        self.fleet.list_filter_prompt_input.clear();
                        self.fleet.list_filter_prompt_status = Some(format!(
                            " {}",
                            format_column_code_error(&ColumnCodeParseError::Ambiguous(codes))
                        ));
                        self.fleet.list_filter_prompt_dismiss_message = None;
                    }
                    Err(ColumnCodeParseError::Unknown) => {
                        self.fleet.list_filter_prompt_input.clear();
                        self.fleet.list_filter_prompt_status = None;
                        self.fleet.list_filter_prompt_dismiss_message =
                            Some("Enter a valid column name/code or ALL".to_string());
                    }
                }
            }
            FleetListFilterPromptMode::Value => {
                let Some(column) = self.fleet.list_filter_pending_column else {
                    self.fleet.list_filter_prompt_mode = FleetListFilterPromptMode::Column;
                    self.fleet.list_filter_prompt_status =
                        Some("Enter a column code first.".to_string());
                    return;
                };
                let raw = if self.fleet.list_filter_prompt_input.trim().is_empty() {
                    self.fleet.list_filter_prompt_default_value.trim()
                } else {
                    self.fleet.list_filter_prompt_input.trim()
                };
                match parse_filter_clause(column, raw) {
                    Ok(clause) => self.apply_fleet_filter_clause(Some(clause)),
                    Err(err) => {
                        self.fleet.list_filter_prompt_status = Some(err);
                        self.fleet.list_filter_prompt_dismiss_message = None;
                    }
                }
            }
        }
    }

    fn apply_fleet_filter_clause(&mut self, clause: Option<TableFilterClause>) {
        let selected_record = self
            .fleet_list_rows()
            .get(self.fleet.cursor)
            .map(|row| row.fleet_record_index_1_based);
        self.fleet.list_filter = FleetListFilter::All;
        self.fleet.list_filter_clause = clause;
        self.fleet.list_filter_prompt_mode = FleetListFilterPromptMode::Column;
        self.fleet.list_filter_prompt_input.clear();
        self.fleet.list_filter_prompt_default_value.clear();
        self.fleet.list_filter_prompt_status = None;
        self.fleet.list_filter_prompt_dismiss_message = None;
        self.fleet.list_filter_pending_column = None;
        self.current_screen = ScreenId::FleetList;
        let rows = self.fleet_list_rows();
        if rows.is_empty() {
            self.fleet.list_filter_clause = None;
        }
        let rows = self.fleet_list_rows();
        self.fleet.cursor = selected_record
            .and_then(|record| {
                rows.iter()
                    .position(|row| row.fleet_record_index_1_based == record)
            })
            .unwrap_or(0);
        let visible_rows = self.fleet_list_visible_rows();
        sync_scroll_to_cursor(
            &mut self.fleet.scroll_offset,
            self.fleet.cursor,
            visible_rows,
        );
    }

    pub fn submit_fleet_list_sort(&mut self, sort: FleetListSort) {
        let selected_record = self
            .fleet_list_rows()
            .get(self.fleet.cursor)
            .map(|row| row.fleet_record_index_1_based);
        if self.fleet.list_sort == sort {
            self.fleet.list_sort_direction = self.fleet.list_sort_direction.toggle();
        } else {
            self.fleet.list_sort = sort;
            self.fleet.list_sort_direction = default_fleet_list_sort_direction(sort);
        }
        self.fleet.list_filter_prompt_mode = FleetListFilterPromptMode::Column;
        self.fleet.list_filter_prompt_input.clear();
        self.fleet.list_filter_prompt_default_value.clear();
        self.fleet.list_filter_prompt_status = None;
        self.fleet.list_filter_prompt_dismiss_message = None;
        self.fleet.list_filter_pending_column = None;
        self.current_screen = ScreenId::FleetList;
        let rows = self.fleet_list_rows();
        if rows.is_empty() {
            self.fleet.cursor = 0;
            self.fleet.scroll_offset = 0;
            return;
        }
        self.fleet.cursor = selected_record
            .and_then(|record| {
                rows.iter()
                    .position(|row| row.fleet_record_index_1_based == record)
            })
            .unwrap_or(0);
        let visible_rows = self.fleet_list_visible_rows();
        sync_scroll_to_cursor(
            &mut self.fleet.scroll_offset,
            self.fleet.cursor,
            visible_rows,
        );
    }

    pub fn submit_fleet_list_sort_prompt(&mut self) {
        if self.current_screen != ScreenId::FleetListSortPrompt {
            return;
        }
        let raw = if self.fleet.list_filter_prompt_input.trim().is_empty() {
            self.fleet.list_filter_prompt_default_value.trim()
        } else {
            self.fleet.list_filter_prompt_input.trim()
        };
        match parse_column_code(FLEET_FILTER_COLUMNS, raw) {
            Ok(column) => match fleet_list_sort_from_column(column) {
                Some(sort) => self.submit_fleet_list_sort(sort),
                None => {
                    self.fleet.list_filter_prompt_input.clear();
                    self.fleet.list_filter_prompt_status = None;
                    self.fleet.list_filter_prompt_dismiss_message =
                        Some("Enter a valid sort column.".to_string());
                }
            },
            Err(ColumnCodeParseError::Ambiguous(codes)) => {
                self.fleet.list_filter_prompt_input.clear();
                self.fleet.list_filter_prompt_status = Some(format!(
                    " {}",
                    format_column_code_error(&ColumnCodeParseError::Ambiguous(codes))
                ));
                self.fleet.list_filter_prompt_dismiss_message = None;
            }
            Err(ColumnCodeParseError::Unknown) => {
                self.fleet.list_filter_prompt_input.clear();
                self.fleet.list_filter_prompt_status = None;
                self.fleet.list_filter_prompt_dismiss_message =
                    Some("Enter a valid sort column.".to_string());
            }
        }
    }

    pub fn open_fleet_review(&mut self) {
        let review_return_to_list = self.current_screen == ScreenId::FleetList;
        let rows = if review_return_to_list {
            self.fleet_list_rows()
        } else {
            self.fleet_rows()
        };
        let total = rows.len();
        if total == 0 {
            self.show_command_menu_notice(CommandMenu::Fleet, "You have no active fleets.");
            return;
        }
        self.clear_command_menu_notice();
        self.clear_fleet_menu_prompt();
        if review_return_to_list && !self.fleet.list_input.trim().is_empty() {
            let fleet_number = match self.fleet.list_input.trim().parse::<u16>() {
                Ok(value) => value,
                Err(_) => {
                    self.fleet.list_status =
                        Some("Enter a fleet number from the table.".to_string());
                    return;
                }
            };
            let Some(index) = rows.iter().position(|row| row.fleet_number == fleet_number) else {
                self.fleet.list_status =
                    Some(format!("Fleet #{fleet_number} is not in your fleet list."));
                return;
            };
            self.fleet.cursor = index;
        }
        self.clear_fleet_list_input();
        self.fleet.review_return_to_list = review_return_to_list;
        self.fleet.review_index = self.fleet.cursor.min(total - 1);
        self.current_screen = ScreenId::FleetReview;
    }

    pub fn close_fleet_review(&mut self) {
        if self.current_screen != ScreenId::FleetReview {
            return;
        }
        let total = self.fleet_review_rows().len();
        if total == 0 {
            self.open_fleet_menu();
            return;
        }
        self.fleet.cursor = self.fleet.review_index.min(total - 1);
        if self.fleet.review_return_to_list {
            let visible_rows = self.fleet_list_visible_rows();
            sync_scroll_to_cursor(
                &mut self.fleet.scroll_offset,
                self.fleet.cursor,
                visible_rows,
            );
            self.current_screen = ScreenId::FleetList;
        } else {
            self.current_screen = ScreenId::FleetMenu;
        }
    }

    pub fn open_fleet_eta(&mut self) {
        if self.current_screen == ScreenId::FleetList {
            match self.fleet_selected_list_row() {
                Ok(row) => {
                    self.fleet.command_context = FleetCommandContext::List;
                    self.open_fleet_eta_with_selected_record(row.fleet_record_index_1_based);
                    return;
                }
                Err(err) => {
                    let message = if err == "You have no active fleets." {
                        err
                    } else {
                        "Fleet unavailable".to_string()
                    };
                    self.show_fleet_list_dismiss_message(message);
                    return;
                }
            }
        }
        self.fleet.command_context = FleetCommandContext::Menu;
        self.fleet.eta_fleet_record_index_1_based = None;
        self.fleet.eta_status = None;
        self.fleet.eta_destination_input.clear();
        self.fleet.eta_include_system_input.clear();
        self.open_fleet_menu_prompt(
            FleetMenuPromptMode::EtaFleet,
            self.strongest_owned_fleet_number()
                .map(|value| value.to_string())
                .unwrap_or_default(),
        );
    }

    /// Close the ETA screen and return to the calling context (fleet list or menu).
    pub fn close_fleet_eta(&mut self) {
        self.fleet.eta_status = None;
        self.fleet.eta_fleet_record_index_1_based = None;
        self.fleet.eta_destination_input.clear();
        self.fleet.eta_include_system_input.clear();
        self.current_screen = self.fleet_context_screen();
    }

    pub(crate) fn open_fleet_eta_with_selected_record(
        &mut self,
        fleet_record_index_1_based: usize,
    ) {
        if self
            .fleet_rows()
            .iter()
            .all(|row| row.fleet_record_index_1_based != fleet_record_index_1_based)
        {
            let message = if self.current_screen == ScreenId::FleetList {
                "Fleet unavailable".to_string()
            } else {
                "Selected fleet is no longer available.".to_string()
            };
            if self.current_screen == ScreenId::FleetList {
                self.show_fleet_list_dismiss_message(message);
            } else {
                self.fleet.menu_prompt_status =
                    self.show_fleet_prompt_feedback(PromptFeedback::error(message));
            }
            return;
        }
        self.clear_command_menu_notice();
        self.clear_fleet_menu_prompt();
        self.fleet.eta_fleet_record_index_1_based = Some(fleet_record_index_1_based);
        self.fleet.eta_status = None;
        self.fleet.eta_destination_input.clear();
        self.fleet.eta_include_system_input.clear();
        self.fleet.eta_mode = FleetEtaMode::EnteringDestination;
        self.current_screen = ScreenId::FleetEta;
    }

    fn submit_inline_fleet_change(&mut self) -> Result<(), String> {
        let raw = if self.fleet.menu_prompt_input.trim().is_empty() {
            self.fleet.menu_prompt_default_value.trim().to_string()
        } else {
            self.fleet.menu_prompt_input.trim().to_string()
        };
        let field = self
            .fleet
            .menu_prompt_change_field
            .ok_or_else(|| "Choose ROE, ID, or Speed first.".to_string())?;
        match field {
            FleetChangeField::Roe => {
                let roe = raw
                    .parse::<u8>()
                    .map_err(|_| "Enter an ROE from 0 to 10.".to_string())?;
                let checked_rows = self.checked_fleet_rows();
                if self.fleet_list_has_checked_selection() {
                    let mut successful = Vec::new();
                    let mut failure_detail = None;
                    for row in &checked_rows {
                        match self.game_data.set_fleet_rules_of_engagement(
                            self.player.record_index_1_based,
                            row.fleet_record_index_1_based,
                            roe,
                        ) {
                            Ok(_) => successful.push(row.fleet_record_index_1_based),
                            Err(err) => {
                                if failure_detail.is_none() {
                                    failure_detail = Some(match err {
                                        nc_data::GameStateMutationError::InvalidFleetPlayerInput {
                                            reason:
                                                nc_data::FleetPlayerInputValidationError::NonCombatFleetMustUseZeroRoe {
                                                    ..
                                                },
                                            ..
                                        } => "Support-only fleets must use ROE 0.".to_string(),
                                        nc_data::GameStateMutationError::InvalidFleetPlayerInput {
                                            reason:
                                                nc_data::FleetPlayerInputValidationError::RulesOfEngagementOutOfRange { .. },
                                            ..
                                        } => "Enter an ROE from 0 to 10.".to_string(),
                                        other => other.to_string(),
                                    });
                                }
                            }
                        }
                    }
                    if successful.is_empty() {
                        return Err(
                            failure_detail.unwrap_or_else(|| "Unable to change ROE.".to_string())
                        );
                    }
                    self.save_game_data().map_err(|err| err.to_string())?;
                    for record_index in &successful {
                        self.fleet.group_selected_fleets.remove(record_index);
                    }
                    let failure_count = checked_rows.len().saturating_sub(successful.len());
                    if failure_count == 0 {
                        self.clear_checked_fleet_selection();
                        self.show_fleet_context_success(
                            format!("Set ROE {} for {} checked fleets.", roe, checked_rows.len()),
                            true,
                        );
                    } else {
                        self.fleet.menu_prompt_status =
                            self.show_fleet_prompt_feedback(PromptFeedback::error(format!(
                                "Set ROE {} for {} fleets. {} {} remain selected: {}",
                                roe,
                                successful.len(),
                                failure_count,
                                if failure_count == 1 {
                                    "fleet"
                                } else {
                                    "fleets"
                                },
                                failure_detail
                                    .as_deref()
                                    .unwrap_or("Some fleets could not be changed.")
                            )));
                    }
                } else {
                    let row = self.prompt_context_fleet_row()?;
                    self.game_data
                        .set_fleet_rules_of_engagement(
                            self.player.record_index_1_based,
                            row.fleet_record_index_1_based,
                            roe,
                        )
                        .map_err(|err| match err {
                            nc_data::GameStateMutationError::InvalidFleetPlayerInput {
                                reason:
                                    nc_data::FleetPlayerInputValidationError::NonCombatFleetMustUseZeroRoe {
                                        ..
                                    },
                                ..
                            } => "Support-only fleets must use ROE 0.".to_string(),
                            nc_data::GameStateMutationError::InvalidFleetPlayerInput {
                                reason:
                                    nc_data::FleetPlayerInputValidationError::RulesOfEngagementOutOfRange { .. },
                                ..
                            } => "Enter an ROE from 0 to 10.".to_string(),
                            other => other.to_string(),
                        })?;
                    self.save_game_data().map_err(|err| err.to_string())?;
                    self.show_fleet_context_success(
                        format!("Fleet #{} ROE set to {}.", row.fleet_number, roe),
                        true,
                    );
                }
            }
            FleetChangeField::Id => {
                let row = self.prompt_context_fleet_row()?;
                let local_slot = raw
                    .parse::<u16>()
                    .map_err(|_| "Enter a fleet ID from 1 up.".to_string())?;
                self.game_data
                    .set_fleet_local_slot(
                        self.player.record_index_1_based,
                        row.fleet_record_index_1_based,
                        local_slot,
                    )
                    .map_err(|err| match err {
                        nc_data::GameStateMutationError::InvalidFleetLocalSlot { .. } => {
                            "Fleet ID is already in use.".to_string()
                        }
                        other => other.to_string(),
                    })?;
                self.save_game_data().map_err(|err| err.to_string())?;
                self.show_fleet_context_success(
                    format!(
                        "Fleet #{} renumbered to Fleet #{}.",
                        row.fleet_number, local_slot
                    ),
                    true,
                );
            }
            FleetChangeField::Speed => {
                let speed = raw
                    .parse::<u8>()
                    .map_err(|_| "Enter a speed from 1 up.".to_string())?;
                let checked_rows = self.checked_fleet_rows();
                if self.fleet_list_has_checked_selection() {
                    let mut successful = Vec::new();
                    let mut failure_detail = None;
                    for row in &checked_rows {
                        match self.game_data.set_fleet_speed(
                            self.player.record_index_1_based,
                            row.fleet_record_index_1_based,
                            speed,
                        ) {
                            Ok(_) => successful.push(row.fleet_record_index_1_based),
                            Err(err) => {
                                if failure_detail.is_none() {
                                    failure_detail = Some(match err {
                                        nc_data::GameStateMutationError::InvalidFleetSpeed {
                                            max,
                                            ..
                                        } => {
                                            format!("Enter a speed from 1 to {max}.")
                                        }
                                        other => other.to_string(),
                                    });
                                }
                            }
                        }
                    }
                    if successful.is_empty() {
                        return Err(
                            failure_detail.unwrap_or_else(|| "Unable to change speed.".to_string())
                        );
                    }
                    self.save_game_data().map_err(|err| err.to_string())?;
                    for record_index in &successful {
                        self.fleet.group_selected_fleets.remove(record_index);
                    }
                    let failure_count = checked_rows.len().saturating_sub(successful.len());
                    if failure_count == 0 {
                        self.clear_checked_fleet_selection();
                        self.show_fleet_context_success(
                            format!(
                                "Set speed {} for {} checked fleets.",
                                speed,
                                checked_rows.len()
                            ),
                            true,
                        );
                    } else {
                        self.fleet.menu_prompt_status =
                            self.show_fleet_prompt_feedback(PromptFeedback::error(format!(
                                "Set speed {} for {} fleets. {} {} remain selected: {}",
                                speed,
                                successful.len(),
                                failure_count,
                                if failure_count == 1 {
                                    "fleet"
                                } else {
                                    "fleets"
                                },
                                failure_detail
                                    .as_deref()
                                    .unwrap_or("Some fleets could not be changed.")
                            )));
                    }
                } else {
                    let row = self.prompt_context_fleet_row()?;
                    let applied = self
                        .game_data
                        .set_fleet_speed(
                            self.player.record_index_1_based,
                            row.fleet_record_index_1_based,
                            speed,
                        )
                        .map_err(|err| match err {
                            nc_data::GameStateMutationError::InvalidFleetSpeed { max, .. } => {
                                format!("Enter a speed from 1 to {max}.")
                            }
                            other => other.to_string(),
                        })?;
                    self.save_game_data().map_err(|err| err.to_string())?;
                    self.show_fleet_context_success(
                        if applied == speed {
                            format!("Fleet #{} speed set to {}.", row.fleet_number, speed)
                        } else {
                            format!(
                                "Fleet #{} is at its intended location or has no mission; its speed stays 0.",
                                row.fleet_number
                            )
                        },
                        true,
                    );
                }
            }
        }
        Ok(())
    }

    pub fn move_fleet_list(&mut self, delta: i8) {
        let ScreenId::FleetList = self.current_screen else {
            return;
        };
        let total = self.fleet_list_rows().len();
        if total == 0 {
            self.fleet.cursor = 0;
            return;
        }
        let next = self.fleet.cursor as isize + delta as isize;
        self.fleet.cursor = next.rem_euclid(total as isize) as usize;
        let visible_rows = self.fleet_list_visible_rows();
        sync_scroll_to_cursor(
            &mut self.fleet.scroll_offset,
            self.fleet.cursor,
            visible_rows,
        );
        self.fleet.list_status = None;
    }

    pub fn move_fleet_review(&mut self, delta: i8) {
        if self.current_screen != ScreenId::FleetReview {
            return;
        }
        let total = self.fleet_review_rows().len();
        if total == 0 {
            self.fleet.review_index = 0;
            return;
        }
        self.fleet.review_index = match delta {
            i8::MIN => 0,
            i8::MAX => total - 1,
            _ => crate::app::helpers::move_wrapped_cursor(
                self.fleet.review_index,
                delta as isize,
                total,
            ),
        };
        self.fleet.cursor = self.fleet.review_index;
        let visible_rows = if self.fleet.review_return_to_list {
            self.fleet_list_visible_rows()
        } else {
            self.fleet_group_visible_rows()
        };
        sync_scroll_to_cursor(
            &mut self.fleet.scroll_offset,
            self.fleet.cursor,
            visible_rows,
        );
    }

    pub fn append_fleet_list_char(&mut self, ch: char) {
        if self.current_screen != ScreenId::FleetList || !ch.is_ascii_digit() {
            return;
        }
        if self.fleet.list_input.len() >= 4 {
            return;
        }
        self.fleet.list_input.push(ch);
        if self.sync_fleet_list_cursor_to_input() {
            self.clear_fleet_list_input();
        }
        self.fleet.list_status = None;
    }

    pub fn append_fleet_list_filter_prompt_char(&mut self, ch: char) {
        if !matches!(
            self.current_screen,
            ScreenId::FleetListFilterPrompt | ScreenId::FleetListSortPrompt
        ) {
            return;
        }
        let allowed = match self.fleet.list_filter_prompt_mode {
            FleetListFilterPromptMode::Column => is_filter_column_char(ch),
            FleetListFilterPromptMode::Value => self
                .fleet
                .list_filter_pending_column
                .map(|column| is_filter_value_char(column.kind, ch))
                .unwrap_or(false),
        };
        if !allowed || self.fleet.list_filter_prompt_input.len() >= 24 {
            return;
        }
        self.fleet.list_filter_prompt_input.push(ch);
        self.fleet.list_filter_prompt_status = None;
        self.fleet.list_filter_prompt_dismiss_message = None;
    }

    pub fn backspace_fleet_list_input(&mut self) {
        if self.current_screen != ScreenId::FleetList {
            return;
        }
        self.fleet.list_input.pop();
        let _ = self.sync_fleet_list_cursor_to_input();
        self.fleet.list_status = None;
    }

    pub fn backspace_fleet_list_filter_prompt_input(&mut self) {
        if !matches!(
            self.current_screen,
            ScreenId::FleetListFilterPrompt | ScreenId::FleetListSortPrompt
        ) {
            return;
        }
        self.fleet.list_filter_prompt_input.pop();
        self.fleet.list_filter_prompt_status = None;
        self.fleet.list_filter_prompt_dismiss_message = None;
    }

    pub fn append_fleet_menu_prompt_char(&mut self, ch: char) {
        if !self.inline_fleet_menu_prompt_active_on_current_screen() {
            return;
        }
        let Some(mode) = self.fleet.menu_prompt_mode else {
            return;
        };
        let (allowed, max_len) = match mode {
            FleetMenuPromptMode::ChangeField => (ch.is_ascii_alphabetic(), 1),
            FleetMenuPromptMode::MergeCheckedConfirm => (matches!(ch, 'y' | 'Y' | 'n' | 'N'), 1),
            FleetMenuPromptMode::ChangeValue => {
                let max_len = match self.fleet.menu_prompt_change_field {
                    Some(FleetChangeField::Roe) | Some(FleetChangeField::Speed) => 2,
                    Some(FleetChangeField::Id) | None => 4,
                };
                (ch.is_ascii_digit(), max_len)
            }
            _ => (ch.is_ascii_digit(), 4),
        };
        if !allowed || self.fleet.menu_prompt_input.len() >= max_len {
            return;
        }
        self.fleet.menu_prompt_input.push(match mode {
            FleetMenuPromptMode::ChangeField => ch.to_ascii_uppercase(),
            _ => ch,
        });
        self.fleet.menu_prompt_status = None;
        if matches!(
            mode,
            FleetMenuPromptMode::ChangeField | FleetMenuPromptMode::MergeCheckedConfirm
        ) {
            self.submit_fleet_menu_prompt();
        }
    }

    pub fn append_fleet_eta_char(&mut self, ch: char) {
        if self.current_screen != ScreenId::FleetEta {
            return;
        }
        match self.fleet.eta_mode {
            FleetEtaMode::EnteringDestination => {
                if self.fleet.eta_destination_input.len() < 16 && is_coordinate_input_char(ch) {
                    self.fleet.eta_destination_input.push(ch);
                    self.fleet.eta_status = None;
                }
            }
            FleetEtaMode::ConfirmingSystemEntry => {
                if matches!(ch, 'y' | 'Y' | 'n' | 'N')
                    && self.fleet.eta_include_system_input.is_empty()
                {
                    self.fleet
                        .eta_include_system_input
                        .push(ch.to_ascii_uppercase());
                    self.fleet.eta_status = None;
                    self.submit_fleet_eta();
                }
            }
            FleetEtaMode::ShowingResult => {}
        }
    }

    pub fn backspace_fleet_menu_prompt_input(&mut self) {
        if !self.inline_fleet_menu_prompt_active_on_current_screen() {
            return;
        }
        self.fleet.menu_prompt_input.pop();
        self.fleet.menu_prompt_status = None;
    }

    pub fn backspace_fleet_eta_input(&mut self) {
        if self.current_screen != ScreenId::FleetEta {
            return;
        }
        match self.fleet.eta_mode {
            FleetEtaMode::EnteringDestination => {
                self.fleet.eta_destination_input.pop();
            }
            FleetEtaMode::ConfirmingSystemEntry => {
                self.fleet.eta_include_system_input.pop();
            }
            FleetEtaMode::ShowingResult => {}
        }
        self.fleet.eta_status = None;
    }

    pub fn submit_fleet_eta(&mut self) {
        if self.current_screen != ScreenId::FleetEta {
            return;
        }
        let Some(selected_row) = self.fleet_eta_selected_row() else {
            self.fleet.eta_status = Some("You have no active fleets.".to_string());
            self.open_fleet_eta();
            return;
        };
        match self.fleet.eta_mode {
            FleetEtaMode::EnteringDestination => {
                let default_destination = self.fleet_eta_default_destination();
                let Some(destination) = resolve_default_coords_input(
                    &self.fleet.eta_destination_input,
                    default_destination,
                ) else {
                    self.fleet.eta_status = Some("Enter coordinates like 10,13".to_string());
                    return;
                };
                let map_size =
                    nc_engine::map_size_for_player_count(self.game_data.conquest.player_count());
                if destination[0] == 0
                    || destination[1] == 0
                    || destination[0] > map_size
                    || destination[1] > map_size
                {
                    self.fleet.eta_status = Some(format!("Enter coordinates within 1..{map_size}"));
                    return;
                }
                self.fleet.eta_destination_input = format!("{},{}", destination[0], destination[1]);
                self.fleet.eta_include_system_input.clear();
                self.fleet.eta_status = None;
                self.fleet.eta_mode = FleetEtaMode::ConfirmingSystemEntry;
            }
            FleetEtaMode::ConfirmingSystemEntry => {
                let include_system =
                    resolve_yes_no_input(&self.fleet.eta_include_system_input, false);
                let destination = resolve_default_coords_input(
                    &self.fleet.eta_destination_input,
                    self.fleet_eta_default_destination(),
                )
                .unwrap_or(self.fleet_eta_default_destination());
                let result =
                    self.calculate_fleet_eta_message(&selected_row, destination, include_system);
                self.fleet.eta_status = Some(result);
                self.fleet.eta_include_system_input.clear();
                self.fleet.eta_mode = FleetEtaMode::ShowingResult;
            }
            FleetEtaMode::ShowingResult => {
                self.fleet.eta_status = None;
                self.fleet.eta_fleet_record_index_1_based = None;
                self.fleet.eta_destination_input.clear();
                self.fleet.eta_include_system_input.clear();
                self.current_screen = self.fleet_context_screen();
            }
        }
    }

    pub fn current_fleet_roe_by_id(&self, fleet_id: u16) -> Option<u8> {
        self.game_data
            .fleets
            .records
            .iter()
            .find(|fleet| {
                fleet.owner_empire_raw() as usize == self.player.record_index_1_based
                    && fleet.local_slot_word_raw() == fleet_id
            })
            .map(|fleet| fleet.rules_of_engagement())
    }

    pub(crate) fn fleet_list_rows(&self) -> Vec<FleetRow> {
        let mut rows = self.fleet_rows();
        rows.retain(|row| fleet_matches_filter(row, self.fleet.list_filter));
        if let Some(clause) = &self.fleet.list_filter_clause {
            rows.retain(|row| fleet_matches_clause(row, clause, &self.fleet.group_selected_fleets));
        }
        rows.sort_by(|left, right| match self.fleet.list_sort {
            FleetListSort::Id => apply_sort_direction(
                self.fleet.list_sort_direction,
                left.fleet_number.cmp(&right.fleet_number),
            ),
            FleetListSort::Selected => apply_sort_direction(
                self.fleet.list_sort_direction,
                self.fleet
                    .group_selected_fleets
                    .contains(&left.fleet_record_index_1_based)
                    .cmp(
                        &self
                            .fleet
                            .group_selected_fleets
                            .contains(&right.fleet_record_index_1_based),
                    ),
            )
            .then_with(|| right.fleet_number.cmp(&left.fleet_number)),
            FleetListSort::Location => apply_sort_direction(
                self.fleet.list_sort_direction,
                left.coords.cmp(&right.coords),
            )
            .then_with(|| right.fleet_number.cmp(&left.fleet_number)),
            FleetListSort::Order => apply_sort_direction(
                self.fleet.list_sort_direction,
                left.order_label.cmp(&right.order_label),
            )
            .then_with(|| right.fleet_number.cmp(&left.fleet_number)),
            FleetListSort::Target => apply_sort_direction(
                self.fleet.list_sort_direction,
                left.target_coords.cmp(&right.target_coords),
            )
            .then_with(|| right.fleet_number.cmp(&left.fleet_number)),
            FleetListSort::Speed => apply_sort_direction(
                self.fleet.list_sort_direction,
                left.current_speed.cmp(&right.current_speed),
            )
            .then_with(|| right.fleet_number.cmp(&left.fleet_number)),
            FleetListSort::Eta => apply_sort_direction(
                self.fleet.list_sort_direction,
                fleet_eta_sort_key(&left.list_eta_label)
                    .cmp(&fleet_eta_sort_key(&right.list_eta_label)),
            )
            .then_with(|| right.fleet_number.cmp(&left.fleet_number)),
            FleetListSort::Roe => apply_sort_direction(
                self.fleet.list_sort_direction,
                left.rules_of_engagement.cmp(&right.rules_of_engagement),
            )
            .then_with(|| right.fleet_number.cmp(&left.fleet_number)),
            FleetListSort::Armies => apply_sort_direction(
                self.fleet.list_sort_direction,
                left.loaded_armies.cmp(&right.loaded_armies),
            )
            .then_with(|| right.fleet_number.cmp(&left.fleet_number)),
            FleetListSort::Strength => apply_sort_direction(
                self.fleet.list_sort_direction,
                self.game_data
                    .fleets
                    .records
                    .get(left.fleet_record_index_1_based.saturating_sub(1))
                    .map(fleet_strength_key)
                    .cmp(
                        &self
                            .game_data
                            .fleets
                            .records
                            .get(right.fleet_record_index_1_based.saturating_sub(1))
                            .map(fleet_strength_key),
                    ),
            )
            .then_with(|| right.fleet_number.cmp(&left.fleet_number)),
        });
        rows
    }

    fn fleet_filter_default_value_for(&self, column: TableFilterColumn) -> String {
        let selected = self
            .fleet_list_rows()
            .get(self.fleet.cursor)
            .cloned()
            .or_else(|| self.fleet_rows().first().cloned());
        let Some(row) = selected else {
            return String::new();
        };
        match column.code {
            "id" => row.fleet_number.to_string(),
            "loc" => format!("{},{}", row.coords[0], row.coords[1]),
            "ord" => String::new(),
            "tar" => {
                if row.target_coords[0] == 0 || row.target_coords[1] == 0 {
                    String::new()
                } else {
                    format!("{},{}", row.target_coords[0], row.target_coords[1])
                }
            }
            "spd" => row.current_speed.to_string(),
            "eta" => row.list_eta_label,
            "roe" => row.rules_of_engagement.to_string(),
            "ars" => row.loaded_armies.to_string(),
            "shi" => String::new(),
            "sel" => {
                if self
                    .fleet
                    .group_selected_fleets
                    .contains(&row.fleet_record_index_1_based)
                {
                    "yes".to_string()
                } else {
                    "no".to_string()
                }
            }
            _ => String::new(),
        }
    }

    fn fleet_review_rows(&self) -> Vec<FleetRow> {
        if self.fleet.review_return_to_list {
            self.fleet_list_rows()
        } else {
            self.fleet_rows()
        }
    }

    pub(crate) fn fleet_rows(&self) -> Vec<FleetRow> {
        let fleet_numbers: std::collections::HashSet<u16> = self
            .active_owned_fleet_records()
            .map(|(_, fleet)| fleet.local_slot_word_raw())
            .collect();
        let mut rows = self
            .active_owned_fleet_records()
            .map(|(idx, fleet)| {
                let join_host_fleet_number =
                    if fleet.standing_order_kind() == nc_data::Order::JoinAnotherFleet {
                        Some(fleet.join_host_fleet_id_raw())
                            .filter(|number| fleet_numbers.contains(number))
                    } else {
                        None
                    };
                let order_label = if let Some(host_num) = join_host_fleet_number {
                    format!("Join Fleet #{host_num}")
                } else {
                    fleet.standing_order_summary()
                };
                FleetRow {
                    fleet_record_index_1_based: idx + 1,
                    fleet_number: fleet.local_slot_word_raw(),
                    coords: fleet.current_location_coords_raw(),
                    target_coords: fleet.standing_order_target_coords_raw(),
                    order_code: fleet.standing_order_code_raw(),
                    current_speed: fleet.current_speed(),
                    max_speed: fleet.max_speed(),
                    eta_label: fleet_eta_label(&self.game_data, idx),
                    list_eta_label: fleet_list_eta_label(&self.game_data, idx),
                    rules_of_engagement: fleet.rules_of_engagement(),
                    loaded_armies: fleet.army_count(),
                    order_label,
                    composition_label: fleet.ship_composition_summary(),
                    table_ships_label: fleet.ship_composition_table_summary(),
                    join_host_fleet_number,
                }
            })
            .collect::<Vec<_>>();
        rows.sort_by(|left, right| {
            left.order_code
                .cmp(&right.order_code)
                .then_with(|| right.fleet_number.cmp(&left.fleet_number))
        });
        rows
    }

    pub(crate) fn fleet_row_by_record_index(
        &self,
        fleet_record_index_1_based: usize,
    ) -> Option<FleetRow> {
        self.fleet_rows()
            .into_iter()
            .find(|row| row.fleet_record_index_1_based == fleet_record_index_1_based)
    }

    pub(crate) fn fleet_eta_selected_row(&self) -> Option<FleetRow> {
        self.fleet
            .eta_fleet_record_index_1_based
            .and_then(|idx| self.fleet_row_by_record_index(idx))
    }

    fn sync_fleet_list_cursor_to_input(&mut self) -> bool {
        let ScreenId::FleetList = self.current_screen else {
            return false;
        };
        let rows = self.fleet_list_rows();
        let raw_input = self.fleet.list_input.trim();
        if raw_input.starts_with('0')
            && raw_input.chars().all(|ch| ch.is_ascii_digit())
            && let Some(index) = rows
                .iter()
                .position(|row| format!("{:02}", row.fleet_number) == raw_input)
        {
            self.fleet.cursor = index;
            let visible_rows = self.fleet_list_visible_rows();
            sync_scroll_to_cursor(
                &mut self.fleet.scroll_offset,
                self.fleet.cursor,
                visible_rows,
            );
            return true;
        }
        let match_rows = rows
            .iter()
            .map(|row| vec![format!("{:02}", row.fleet_number)])
            .collect::<Vec<_>>();
        let Some(matched) =
            crate::screen::table_selection::find_typed_jump(&match_rows, 0, &self.fleet.list_input)
        else {
            return false;
        };
        self.fleet.cursor = matched.index;
        let visible_rows = self.fleet_list_visible_rows();
        sync_scroll_to_cursor(
            &mut self.fleet.scroll_offset,
            self.fleet.cursor,
            visible_rows,
        );
        matched.is_terminal_exact_match
    }

    pub fn cancel_fleet_menu_prompt(&mut self) {
        if self.inline_fleet_menu_prompt_active_on_current_screen() {
            self.clear_fleet_menu_prompt();
            self.current_screen = self.fleet_context_screen();
        }
    }

    fn fleet_menu_default_fleet_number(&self) -> Option<u16> {
        if self.fleet.menu_prompt_default_value.trim().is_empty() {
            self.strongest_owned_fleet_number()
        } else {
            self.fleet
                .menu_prompt_default_value
                .trim()
                .parse::<u16>()
                .ok()
        }
    }

    fn resolve_fleet_prompt_row_from_rows(
        &self,
        rows: &[FleetRow],
        default_fleet_number: Option<u16>,
        invalid_message: &str,
    ) -> Result<(usize, FleetRow), String> {
        let default_fleet_number =
            default_fleet_number.ok_or_else(|| "You have no active fleets.".to_string())?;
        let fleet_number = if self.fleet.menu_prompt_input.trim().is_empty() {
            default_fleet_number
        } else {
            self.fleet
                .menu_prompt_input
                .trim()
                .parse::<u16>()
                .map_err(|_| invalid_message.to_string())?
        };
        let index = rows
            .iter()
            .position(|row| row.fleet_number == fleet_number)
            .ok_or_else(|| format!("Fleet #{fleet_number} is not in your fleet list."))?;
        Ok((index, rows[index].clone()))
    }

    fn resolve_fleet_menu_prompt_selection(&self) -> Result<(usize, FleetRow), String> {
        let rows = self.fleet_rows();
        self.resolve_fleet_prompt_row_from_rows(
            &rows,
            self.fleet_menu_default_fleet_number(),
            "Enter one of your fleet numbers.",
        )
    }

    fn prompt_context_fleet_row(&self) -> Result<FleetRow, String> {
        let record_index = self
            .fleet
            .menu_prompt_context_fleet_record_index_1_based
            .ok_or_else(|| "Select a fleet first.".to_string())?;
        self.fleet_rows()
            .into_iter()
            .find(|row| row.fleet_record_index_1_based == record_index)
            .ok_or_else(|| "Selected fleet is no longer available.".to_string())
    }

    fn fleet_change_value_default(&self) -> Result<String, String> {
        if self.fleet_list_has_checked_selection() {
            let rows = self.checked_fleet_rows();
            return Ok(match self.fleet.menu_prompt_change_field {
                Some(FleetChangeField::Roe) => rows
                    .first()
                    .map(|row| row.rules_of_engagement)
                    .filter(|roe| rows.iter().all(|row| row.rules_of_engagement == *roe))
                    .map(|roe| roe.to_string())
                    .unwrap_or_default(),
                Some(FleetChangeField::Speed) => rows
                    .first()
                    .map(|row| row.current_speed)
                    .filter(|speed| rows.iter().all(|row| row.current_speed == *speed))
                    .map(|speed| speed.to_string())
                    .unwrap_or_default(),
                Some(FleetChangeField::Id) | None => String::new(),
            });
        }
        let row = self.prompt_context_fleet_row()?;
        Ok(match self.fleet.menu_prompt_change_field {
            Some(FleetChangeField::Roe) => row.rules_of_engagement.to_string(),
            Some(FleetChangeField::Id) => row.fleet_number.to_string(),
            Some(FleetChangeField::Speed) => row.current_speed.to_string(),
            None => String::new(),
        })
    }

    fn resolve_fleet_change_field(&self) -> Result<FleetChangeField, String> {
        let raw = if self.fleet.menu_prompt_input.trim().is_empty() {
            self.fleet.menu_prompt_default_value.trim().to_string()
        } else {
            self.fleet.menu_prompt_input.trim().to_string()
        };
        match raw.chars().next().map(|ch| ch.to_ascii_uppercase()) {
            Some('R') => Ok(FleetChangeField::Roe),
            Some('I') if !self.fleet_list_has_checked_selection() => Ok(FleetChangeField::Id),
            Some('S') => Ok(FleetChangeField::Speed),
            _ if self.fleet_list_has_checked_selection() => Err("Enter R or S.".to_string()),
            _ => Err("Enter R, I, or S.".to_string()),
        }
    }

    fn resolve_checked_merge_confirm(&self) -> Result<bool, String> {
        let raw = if self.fleet.menu_prompt_input.trim().is_empty() {
            self.fleet.menu_prompt_default_value.trim()
        } else {
            self.fleet.menu_prompt_input.trim()
        };
        match raw.chars().next().map(|ch| ch.to_ascii_uppercase()) {
            Some('Y') => Ok(true),
            Some('N') => Ok(false),
            _ => Err("Enter Y or N.".to_string()),
        }
    }

    pub(crate) fn merge_host_default_value(&self) -> String {
        let Some(source_record_index) = self.fleet.merge_source_record_index_1_based else {
            return String::new();
        };
        self.eligible_merge_host_rows(source_record_index)
            .first()
            .map(|row| row.fleet_number.to_string())
            .unwrap_or_default()
    }

    fn eligible_transfer_host_rows(&self) -> Vec<FleetRow> {
        let Some(donor_record_index) = self.fleet.transfer_donor_record_index_1_based else {
            return Vec::new();
        };
        let rows = self.fleet_rows();
        let Some(donor_row) = rows
            .iter()
            .find(|row| row.fleet_record_index_1_based == donor_record_index)
        else {
            return Vec::new();
        };
        let donor_coords = donor_row.coords;
        let mut rows = rows
            .into_iter()
            .filter(|row| {
                row.fleet_record_index_1_based != donor_record_index && row.coords == donor_coords
            })
            .collect::<Vec<_>>();
        rows.sort_by_key(|row| {
            (
                self.fleet_ship_total(row.fleet_record_index_1_based),
                row.fleet_number,
            )
        });
        rows
    }

    pub(crate) fn transfer_host_default_value(&self) -> String {
        self.eligible_transfer_host_rows()
            .first()
            .map(|row| row.fleet_number.to_string())
            .unwrap_or_default()
    }

    fn fleet_ship_total(&self, fleet_record_index_1_based: usize) -> u32 {
        self.game_data
            .fleets
            .records
            .get(fleet_record_index_1_based - 1)
            .map(|fleet| {
                u32::from(fleet.battleship_count())
                    + u32::from(fleet.cruiser_count())
                    + u32::from(fleet.destroyer_count())
                    + u32::from(fleet.troop_transport_count())
                    + u32::from(fleet.scout_count())
                    + u32::from(fleet.etac_count())
            })
            .unwrap_or(0)
    }

    fn co_located_merge_peer_rows(&self, source_record_index_1_based: usize) -> Vec<FleetRow> {
        let Some(source_row) = self.fleet_row_by_record_index(source_record_index_1_based) else {
            return Vec::new();
        };
        self.fleet_rows()
            .into_iter()
            .filter(|row| {
                row.fleet_record_index_1_based != source_record_index_1_based
                    && row.coords == source_row.coords
            })
            .collect()
    }

    fn eligible_merge_host_rows(&self, source_record_index_1_based: usize) -> Vec<FleetRow> {
        let Some(source_row) = self.fleet_row_by_record_index(source_record_index_1_based) else {
            return Vec::new();
        };
        let mut rows = self
            .co_located_merge_peer_rows(source_record_index_1_based)
            .into_iter()
            .filter(|row| row.fleet_number < source_row.fleet_number)
            .collect::<Vec<_>>();
        rows.sort_by_key(|row| {
            (
                self.fleet_ship_total(row.fleet_record_index_1_based),
                row.fleet_number,
            )
        });
        rows
    }

    pub(crate) fn eligible_merge_source_fleet_number(&self) -> Option<u16> {
        self.fleet_rows()
            .into_iter()
            .filter(|row| {
                !self
                    .eligible_merge_host_rows(row.fleet_record_index_1_based)
                    .is_empty()
            })
            .max_by_key(|row| {
                (
                    self.fleet_ship_total(row.fleet_record_index_1_based),
                    std::cmp::Reverse(row.fleet_number),
                )
            })
            .map(|row| row.fleet_number)
    }

    pub(crate) fn eligible_transfer_donor_fleet_number(&self) -> Option<u16> {
        self.fleet_rows()
            .into_iter()
            .filter(|row| self.fleet_ship_total(row.fleet_record_index_1_based) > 1)
            .filter(|row| {
                self.fleet_rows().iter().any(|other| {
                    other.fleet_record_index_1_based != row.fleet_record_index_1_based
                        && other.coords == row.coords
                })
            })
            .max_by_key(|row| {
                (
                    self.fleet_ship_total(row.fleet_record_index_1_based),
                    std::cmp::Reverse(row.fleet_number),
                )
            })
            .map(|row| row.fleet_number)
    }

    pub(crate) fn validate_merge_source_row(&self, row: &FleetRow) -> Result<(), String> {
        let peers = self.co_located_merge_peer_rows(row.fleet_record_index_1_based);
        if peers.is_empty() {
            return Err(format!(
                "Fleet #{} is not in a sector with another one of your fleets.",
                row.fleet_number
            ));
        }
        if peers
            .iter()
            .all(|peer| peer.fleet_number > row.fleet_number)
        {
            return Err("Fleets must be co-located in the same sector.".to_string());
        }
        Ok(())
    }

    pub(crate) fn validate_transfer_donor_row(&self, row: &FleetRow) -> Result<(), String> {
        if self.fleet_ship_total(row.fleet_record_index_1_based) <= 1 {
            return Err("Use merge instead".to_string());
        }
        let has_host = self.fleet_rows().iter().any(|other| {
            other.fleet_record_index_1_based != row.fleet_record_index_1_based
                && other.coords == row.coords
        });
        if !has_host {
            return Err(format!(
                "Fleet #{} is not in a sector with another one of your fleets.",
                row.fleet_number
            ));
        }
        Ok(())
    }

    fn resolve_merge_host_prompt_row(&self) -> Result<FleetRow, String> {
        let source_record_index = self
            .fleet
            .merge_source_record_index_1_based
            .ok_or_else(|| "Select the fleet that will merge first.".to_string())?;
        let source_row = self
            .fleet_row_by_record_index(source_record_index)
            .ok_or_else(|| "Selected fleet is no longer available.".to_string())?;
        let fleet_number = if self.fleet.menu_prompt_input.trim().is_empty() {
            self.fleet
                .menu_prompt_default_value
                .trim()
                .parse::<u16>()
                .map_err(|_| "Enter one of your fleet numbers.".to_string())?
        } else {
            self.fleet
                .menu_prompt_input
                .trim()
                .parse::<u16>()
                .map_err(|_| "Enter one of your fleet numbers.".to_string())?
        };
        let row = self
            .fleet_rows()
            .into_iter()
            .find(|row| row.fleet_number == fleet_number)
            .ok_or_else(|| format!("Fleet #{fleet_number} is not in your fleet list."))?;
        if row.fleet_record_index_1_based == source_record_index {
            return Err("Choose a different host fleet.".to_string());
        }
        if row.coords != source_row.coords {
            return Err(format!(
                "Fleet #{} is not in the same sector as Fleet #{}.",
                row.fleet_number, source_row.fleet_number
            ));
        }
        Ok(row)
    }

    fn resolve_transfer_host_prompt_row(&self) -> Result<FleetRow, String> {
        let donor_record_index = self
            .fleet
            .transfer_donor_record_index_1_based
            .ok_or_else(|| "Select a source fleet first.".to_string())?;
        let donor_row = self
            .fleet_row_by_record_index(donor_record_index)
            .ok_or_else(|| "Selected source fleet is no longer available.".to_string())?;
        let fleet_number = if self.fleet.menu_prompt_input.trim().is_empty() {
            self.fleet
                .menu_prompt_default_value
                .trim()
                .parse::<u16>()
                .map_err(|_| "Enter one of your fleet numbers.".to_string())?
        } else {
            self.fleet
                .menu_prompt_input
                .trim()
                .parse::<u16>()
                .map_err(|_| "Enter one of your fleet numbers.".to_string())?
        };
        let row = self
            .fleet_rows()
            .into_iter()
            .find(|row| row.fleet_number == fleet_number)
            .ok_or_else(|| format!("Fleet #{fleet_number} is not in your fleet list."))?;
        if row.fleet_record_index_1_based == donor_record_index {
            return Err("Choose a different destination fleet.".to_string());
        }
        if row.coords != donor_row.coords {
            return Err(format!(
                "Fleet #{} is not in the same sector as Fleet #{}.",
                row.fleet_number, donor_row.fleet_number
            ));
        }
        Ok(row)
    }

    pub fn submit_fleet_menu_prompt(&mut self) {
        if !self.inline_fleet_menu_prompt_active_on_current_screen() {
            return;
        }
        let Some(mode) = self.fleet.menu_prompt_mode else {
            return;
        };
        match mode {
            FleetMenuPromptMode::Review => match self.resolve_fleet_menu_prompt_selection() {
                Ok((index, row)) => {
                    self.fleet.cursor = index;
                    self.fleet.menu_prompt_input.clear();
                    self.fleet.menu_prompt_status = None;
                    self.fleet.menu_prompt_default_value = row.fleet_number.to_string();
                    self.open_fleet_review();
                }
                Err(err) => {
                    self.fleet.menu_prompt_status =
                        self.show_fleet_prompt_feedback(PromptFeedback::error(err))
                }
            },
            FleetMenuPromptMode::Order => match self.resolve_fleet_menu_prompt_selection() {
                Ok((index, row)) => {
                    self.fleet.cursor = index;
                    self.fleet.menu_prompt_input.clear();
                    self.fleet.menu_prompt_status = None;
                    self.fleet.menu_prompt_default_value = row.fleet_number.to_string();
                    self.open_fleet_order_with_selected_record(row.fleet_record_index_1_based);
                }
                Err(err) => {
                    self.fleet.menu_prompt_status =
                        self.show_fleet_prompt_feedback(PromptFeedback::error(err))
                }
            },
            FleetMenuPromptMode::ChangeFleet => match self.resolve_fleet_menu_prompt_selection() {
                Ok((_index, row)) => {
                    self.fleet.menu_prompt_context_fleet_record_index_1_based =
                        Some(row.fleet_record_index_1_based);
                    self.fleet.menu_prompt_change_field = None;
                    self.fleet.menu_prompt_mode = Some(FleetMenuPromptMode::ChangeField);
                    self.fleet.menu_prompt_input.clear();
                    self.fleet.menu_prompt_status = None;
                    self.fleet.menu_prompt_default_value = "R".to_string();
                }
                Err(err) => {
                    self.fleet.menu_prompt_status =
                        self.show_fleet_prompt_feedback(PromptFeedback::error(err))
                }
            },
            FleetMenuPromptMode::ChangeField => match self.resolve_fleet_change_field() {
                Ok(field) => {
                    self.fleet.menu_prompt_change_field = Some(field);
                    self.fleet.menu_prompt_mode = Some(FleetMenuPromptMode::ChangeValue);
                    self.fleet.menu_prompt_input.clear();
                    self.fleet.menu_prompt_status = None;
                    self.fleet.menu_prompt_default_value =
                        self.fleet_change_value_default().unwrap_or_default();
                }
                Err(err) => {
                    self.fleet.menu_prompt_status =
                        self.show_fleet_prompt_feedback(PromptFeedback::error(err))
                }
            },
            FleetMenuPromptMode::ChangeValue => {
                if let Err(err) = self.submit_inline_fleet_change() {
                    self.fleet.menu_prompt_status =
                        self.show_fleet_prompt_feedback(PromptFeedback::error(err));
                }
            }
            FleetMenuPromptMode::EtaFleet => match self.resolve_fleet_menu_prompt_selection() {
                Ok((index, row)) => {
                    self.fleet.cursor = index;
                    self.fleet.menu_prompt_input.clear();
                    self.fleet.menu_prompt_status = None;
                    self.fleet.menu_prompt_default_value = row.fleet_number.to_string();
                    self.open_fleet_eta_with_selected_record(row.fleet_record_index_1_based);
                }
                Err(err) => {
                    self.fleet.menu_prompt_status =
                        self.show_fleet_prompt_feedback(PromptFeedback::error(err))
                }
            },
            FleetMenuPromptMode::DetachFleet => match self.resolve_fleet_menu_prompt_selection() {
                Ok((index, row)) => {
                    self.fleet.cursor = index;
                    self.fleet.menu_prompt_input.clear();
                    self.fleet.menu_prompt_status = None;
                    if let Err(err) =
                        self.open_fleet_detach_with_selected_record(row.fleet_record_index_1_based)
                    {
                        self.fleet.menu_prompt_default_value = self
                            .largest_owned_fleet_number_by_ship_total()
                            .map(|value| value.to_string())
                            .unwrap_or_default();
                        self.fleet.menu_prompt_status =
                            self.show_fleet_prompt_feedback(PromptFeedback::error(err));
                    }
                }
                Err(err) => {
                    self.fleet.menu_prompt_default_value = self
                        .largest_owned_fleet_number_by_ship_total()
                        .map(|value| value.to_string())
                        .unwrap_or_default();
                    self.fleet.menu_prompt_status =
                        self.show_fleet_prompt_feedback(PromptFeedback::error(err));
                }
            },
            FleetMenuPromptMode::MergeSource => match self.resolve_fleet_menu_prompt_selection() {
                Ok((_index, row)) => {
                    if let Err(err) = self.validate_merge_source_row(&row) {
                        self.fleet.menu_prompt_default_value = self
                            .eligible_merge_source_fleet_number()
                            .map(|value| value.to_string())
                            .unwrap_or_default();
                        self.fleet.menu_prompt_status =
                            self.show_fleet_prompt_feedback(PromptFeedback::error(err));
                        return;
                    }
                    self.fleet.merge_source_record_index_1_based =
                        Some(row.fleet_record_index_1_based);
                    self.fleet.menu_prompt_mode = Some(FleetMenuPromptMode::MergeHost);
                    self.fleet.menu_prompt_input.clear();
                    self.fleet.menu_prompt_status = None;
                    self.fleet.menu_prompt_default_value = self.merge_host_default_value();
                }
                Err(err) => {
                    self.fleet.menu_prompt_status =
                        self.show_fleet_prompt_feedback(PromptFeedback::error(err))
                }
            },
            FleetMenuPromptMode::MergeHost => match self.resolve_merge_host_prompt_row() {
                Ok(row) => {
                    if let Err(err) = self.submit_inline_fleet_merge(row.fleet_record_index_1_based)
                    {
                        self.fleet.menu_prompt_status =
                            self.show_fleet_prompt_feedback(PromptFeedback::error(err));
                    }
                }
                Err(err) => {
                    self.fleet.menu_prompt_status =
                        self.show_fleet_prompt_feedback(PromptFeedback::error(err))
                }
            },
            FleetMenuPromptMode::MergeCheckedConfirm => {
                match self.resolve_checked_merge_confirm() {
                    Ok(false) => {
                        self.clear_fleet_menu_prompt();
                        self.current_screen = ScreenId::FleetList;
                    }
                    Ok(true) => {
                        let plan = match self.checked_merge_plan() {
                            Ok(plan) => plan,
                            Err(err) => {
                                self.fleet.menu_prompt_status =
                                    self.show_fleet_prompt_feedback(PromptFeedback::error(err));
                                return;
                            }
                        };
                        if let Err(err) = self.submit_checked_fleet_merge(plan) {
                            self.fleet.menu_prompt_status =
                                self.show_fleet_prompt_feedback(PromptFeedback::error(err));
                        }
                    }
                    Err(err) => {
                        self.fleet.menu_prompt_status =
                            self.show_fleet_prompt_feedback(PromptFeedback::error(err))
                    }
                }
            }
            FleetMenuPromptMode::TransferDonor => {
                match self.resolve_fleet_menu_prompt_selection() {
                    Ok((_index, row)) => {
                        if let Err(err) = self.validate_transfer_donor_row(&row) {
                            self.fleet.menu_prompt_status =
                                self.show_fleet_prompt_feedback(PromptFeedback::error(err));
                            return;
                        }
                        self.fleet.transfer_donor_record_index_1_based =
                            Some(row.fleet_record_index_1_based);
                        self.fleet.menu_prompt_mode = Some(FleetMenuPromptMode::TransferHost);
                        self.fleet.menu_prompt_input.clear();
                        self.fleet.menu_prompt_status = None;
                        self.fleet.menu_prompt_default_value = self.transfer_host_default_value();
                    }
                    Err(err) => {
                        self.fleet.menu_prompt_status =
                            self.show_fleet_prompt_feedback(PromptFeedback::error(err))
                    }
                }
            }
            FleetMenuPromptMode::TransferHost => match self.resolve_transfer_host_prompt_row() {
                Ok(row) => {
                    self.open_fleet_transfer_with_selected_records(row.fleet_record_index_1_based);
                }
                Err(err) => {
                    self.fleet.menu_prompt_status =
                        self.show_fleet_prompt_feedback(PromptFeedback::error(err))
                }
            },
            FleetMenuPromptMode::TransportFleet(mode) => match self
                .resolve_fleet_menu_prompt_selection()
            {
                Ok((_index, row)) => {
                    if let Err(err) = self
                        .open_fleet_transport_quantity_prompt(mode, row.fleet_record_index_1_based)
                    {
                        self.fleet.menu_prompt_status =
                            self.show_fleet_prompt_feedback(PromptFeedback::error(err));
                    }
                }
                Err(err) => {
                    self.fleet.menu_prompt_status =
                        self.show_fleet_prompt_feedback(PromptFeedback::error(err))
                }
            },
            FleetMenuPromptMode::TransportQuantity(_) => {
                self.planet.transport_qty_input = self.fleet.menu_prompt_input.clone();
                self.planet.transport_status = None;
                if let Err(err) = self.submit_planet_transport_qty() {
                    self.fleet.menu_prompt_status =
                        self.show_fleet_prompt_feedback(PromptFeedback::error(err.to_string()));
                    return;
                }
                if let Some(status) = self.planet.transport_status.take() {
                    self.fleet.menu_prompt_status =
                        self.show_fleet_prompt_feedback(PromptFeedback::error(status));
                }
            }
        }
    }

    pub(crate) fn handle_fleet_menu_prompt_key(
        &self,
        key: crossterm::event::KeyEvent,
    ) -> crate::app::Action {
        use crossterm::event::KeyCode;

        let mode = self.fleet.menu_prompt_mode;
        match key.code {
            KeyCode::Enter => crate::app::Action::Fleet(FleetAction::SubmitMenuPrompt),
            KeyCode::Backspace => crate::app::Action::Fleet(FleetAction::BackspaceMenuPromptInput),
            KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => {
                crate::app::Action::Fleet(FleetAction::CancelMenuPrompt)
            }
            KeyCode::Char(ch)
                if match mode {
                    Some(FleetMenuPromptMode::ChangeField) => ch.is_ascii_alphabetic(),
                    _ => ch.is_ascii_digit(),
                } =>
            {
                crate::app::Action::Fleet(FleetAction::AppendMenuPromptChar(ch))
            }
            _ => crate::app::Action::Noop,
        }
    }

    pub(crate) fn handle_fleet_eta_key(
        &self,
        key: crossterm::event::KeyEvent,
    ) -> crate::app::Action {
        use crossterm::event::KeyCode;

        match self.fleet.eta_mode {
            FleetEtaMode::EnteringDestination => match key.code {
                KeyCode::Enter => crate::app::Action::Fleet(FleetAction::SubmitEta),
                KeyCode::Backspace => crate::app::Action::Fleet(FleetAction::BackspaceEtaInput),
                KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => {
                    crate::app::Action::Fleet(FleetAction::CloseEta)
                }
                KeyCode::Char(ch) if is_coordinate_input_char(ch) => {
                    crate::app::Action::Fleet(FleetAction::AppendEtaChar(ch))
                }
                _ => crate::app::Action::Noop,
            },
            FleetEtaMode::ConfirmingSystemEntry => match key.code {
                KeyCode::Enter => crate::app::Action::Fleet(FleetAction::SubmitEta),
                KeyCode::Backspace => crate::app::Action::Fleet(FleetAction::BackspaceEtaInput),
                KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => {
                    crate::app::Action::Fleet(FleetAction::CloseEta)
                }
                KeyCode::Char(ch) if matches!(ch, 'y' | 'Y' | 'n' | 'N') => {
                    crate::app::Action::Fleet(FleetAction::AppendEtaChar(ch))
                }
                _ => crate::app::Action::Noop,
            },
            FleetEtaMode::ShowingResult => crate::app::Action::Fleet(FleetAction::SubmitEta),
        }
    }

    pub(crate) fn fleet_eta_default_destination(&self) -> [u8; 2] {
        let Some(row) = self.fleet_eta_selected_row() else {
            return [8, 2];
        };
        if row.target_coords[0] > 0 && row.target_coords[1] > 0 {
            row.target_coords
        } else {
            row.coords
        }
    }

    pub(crate) fn fleet_group_target_status_line(&self) -> String {
        if self.fleet.group_mode == crate::screen::FleetGroupOrderMode::SeekingNextClosest
            && let Some(question) = self.fleet_group_seek_next_closest()
        {
            return question.notice;
        }
        if self.fleet.group_mode == crate::screen::FleetGroupOrderMode::ConfirmingTarget
            && let (Some(mission_code), Ok(destination)) = (
                self.fleet.group_mission_code,
                self.resolve_fleet_group_split_target(),
            )
            && let Some(message) =
                self.fleet_group_confirmation_eta_message(mission_code, destination)
        {
            return message;
        }
        fleet_target_status_line(self.fleet.group_mission_code)
    }

    pub(crate) fn fleet_group_target_prompt(&self) -> String {
        match fleet_target_input_kind(self.fleet.group_mission_code) {
            FleetTargetInputKind::StarbaseId => "Starbase # ".to_string(),
            FleetTargetInputKind::FleetId => "Fleet # ".to_string(),
            FleetTargetInputKind::Coordinates => "Target ".to_string(),
            FleetTargetInputKind::None => "Target ".to_string(),
        }
    }

    pub(crate) fn fleet_group_target_default(&self) -> String {
        match fleet_target_input_kind(self.fleet.group_mission_code) {
            FleetTargetInputKind::StarbaseId => self
                .fleet_group_default_starbase()
                .map(|row| row.base_id.to_string())
                .unwrap_or_else(|| "1".to_string()),
            FleetTargetInputKind::FleetId => self
                .fleet_group_default_host_fleet()
                .map(|row| row.fleet_number.to_string())
                .unwrap_or_else(|| "1".to_string()),
            FleetTargetInputKind::Coordinates | FleetTargetInputKind::None => self
                .fleet_group_default_target_coords()
                .map(|target| format!("{},{}", target[0], target[1]))
                .unwrap_or_default(),
        }
    }
}
